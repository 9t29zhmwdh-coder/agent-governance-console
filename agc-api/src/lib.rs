mod auth;

pub use auth::{AuthConfig, Role};

use agc_core::{AuditLog, AuditOutcome, AuditRecord, ConsoleConfig, GovernancePolicy, PolicyAction, PolicyEngine, TraceSpan, TraceStore};
use axum::{
    extract::{FromRequestParts, Path, Query},
    http::{header, request::Parts, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Per-tenant trace and audit storage. Policies stay global (shared
/// governance across all tenants, see `AppState::policy`); only trace and
/// audit data are isolated, per `ROADMAP.md`'s "tenant isolation in
/// trace/audit stores".
pub struct TenantStore {
    pub traces: Mutex<TraceStore>,
    pub audit: Mutex<AuditLog>,
}

impl TenantStore {
    fn in_memory() -> Self {
        Self { traces: Mutex::new(TraceStore::new()), audit: Mutex::new(AuditLog::new()) }
    }

    fn with_audit_db(path: impl AsRef<FsPath>) -> rusqlite::Result<Self> {
        Ok(Self { traces: Mutex::new(TraceStore::new()), audit: Mutex::new(AuditLog::open(path)?) })
    }
}

#[derive(Clone)]
pub struct AppState {
    /// `RwLock`, not `Mutex`: after warm-up almost every call is a lookup
    /// of an already-created tenant, so concurrent requests shouldn't
    /// serialize on one exclusive lock just to read a HashMap. A real
    /// bottleneck found while load-testing the SLA target (ROADMAP.md):
    /// with a plain `Mutex` here, 1000 req/s against one tenant produced
    /// p99 ~29ms; this read/write split is what got it under the 10ms
    /// target, see `docs/performance.md`.
    tenants: Arc<RwLock<HashMap<String, Arc<TenantStore>>>>,
    pub policy: Arc<Mutex<PolicyEngine>>,
    /// Real OTLP span exporter, present only when telemetry is enabled and
    /// configured with an endpoint (see `agc_core::TelemetryConfig`).
    pub otlp: Option<Arc<agc_azure::OtlpExporter>>,
    /// `true` only if `otlp` is set AND a Managed Identity token was
    /// actually attached to it as an `Authorization` header -- distinct
    /// from `cfg.telemetry.use_managed_identity`, which just records that
    /// authentication was *requested*; the token fetch itself can fail
    /// (e.g. off Azure) and export still proceeds unauthenticated.
    pub otlp_authenticated: bool,
    /// If set, each tenant's audit log persists to `{dir}/{tenant_id}.sqlite`
    /// (created lazily on that tenant's first request) instead of vanishing
    /// with an in-memory log when the process exits.
    audit_db_dir: Option<PathBuf>,
    /// RBAC gate for the REST API. `AuthConfig::Disabled` (the default)
    /// treats every request as `Role::Admin`, matching this API's
    /// behavior before RBAC existed.
    pub auth: AuthConfig,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tenants: Arc::new(RwLock::new(HashMap::new())),
            policy: Arc::new(Mutex::new(PolicyEngine::new())),
            otlp: None,
            otlp_authenticated: false,
            audit_db_dir: None,
            auth: AuthConfig::disabled(),
        }
    }

    /// Same as `new()`, but every tenant's audit log persists to
    /// `{dir}/{tenant_id}.sqlite` instead of vanishing on process exit.
    pub fn with_audit_db_dir(dir: impl Into<PathBuf>) -> Self {
        let mut state = Self::new();
        state.audit_db_dir = Some(dir.into());
        state
    }

    /// Builds from a `ConsoleConfig`: per-tenant file-backed audit logs if
    /// `audit_db_dir` is set, in-memory otherwise (no I/O happens here;
    /// tenant stores -- and their SQLite files, if configured -- are
    /// created lazily on each tenant's first request). If telemetry is
    /// enabled with an endpoint, also constructs a real OTLP exporter; a
    /// misconfigured endpoint logs a warning and leaves telemetry off
    /// rather than failing the whole server startup over it. Async because
    /// `cfg.telemetry.use_managed_identity` fetches a real Microsoft Entra
    /// token (via IMDS) before the exporter is built; a token fetch failure
    /// is handled the same way -- logged, telemetry stays off, startup
    /// still succeeds.
    pub async fn from_config(cfg: &ConsoleConfig) -> Self {
        let mut state = match &cfg.audit_db_dir {
            Some(dir) => Self::with_audit_db_dir(dir.clone()),
            None => Self::new(),
        };
        if cfg.telemetry.enabled {
            if let Some(endpoint) = &cfg.telemetry.endpoint {
                let token = if cfg.telemetry.use_managed_identity {
                    let mut credential = agc_azure::ManagedIdentityCredential::new();
                    if let Some(client_id) = &cfg.telemetry.managed_identity_client_id {
                        credential = credential.with_client_id(client_id.clone());
                    }
                    match credential.get_token("https://monitor.azure.com/").await {
                        Ok(token) => Some(token.access_token),
                        Err(e) => {
                            tracing::warn!("failed to fetch Managed Identity token for OTLP export, proceeding without an Authorization header: {e}");
                            None
                        }
                    }
                } else {
                    None
                };
                match agc_azure::OtlpExporter::new(endpoint, &cfg.telemetry.service_name, token.as_deref()) {
                    Ok(exporter) => {
                        state.otlp_authenticated = token.is_some();
                        state.otlp = Some(Arc::new(exporter));
                    }
                    Err(e) => tracing::warn!("failed to initialize OTLP exporter, telemetry stays disabled: {e}"),
                }
            }
        }
        state
    }

    /// Resolves the store for `tenant_id`, creating it (and, if
    /// `audit_db_dir` is set, its backing SQLite file) on first use.
    async fn tenant_store(&self, tenant_id: &str) -> rusqlite::Result<Arc<TenantStore>> {
        // Fast path: a shared read lock, so concurrent requests for
        // already-created tenants (the overwhelming majority in practice)
        // never block each other.
        if let Some(store) = self.tenants.read().await.get(tenant_id) {
            return Ok(store.clone());
        }
        let mut tenants = self.tenants.write().await;
        // Another task may have created this tenant while we were waiting
        // for the write lock; re-check before creating a duplicate.
        if let Some(store) = tenants.get(tenant_id) {
            return Ok(store.clone());
        }
        let store = match &self.audit_db_dir {
            Some(dir) => {
                std::fs::create_dir_all(dir).map_err(|e| {
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(1),
                        Some(format!("creating audit_db_dir {}: {e}", dir.display())),
                    )
                })?;
                Arc::new(TenantStore::with_audit_db(dir.join(format!("{tenant_id}.sqlite")))?)
            }
            None => Arc::new(TenantStore::in_memory()),
        };
        tenants.insert(tenant_id.to_string(), store.clone());
        Ok(store)
    }

    /// Every tenant ID that has made at least one request so far, sorted.
    pub async fn tenant_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.tenants.read().await.keys().cloned().collect();
        ids.sort();
        ids
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts the required `X-Tenant-Id` header. Rejects with `400` if it's
/// missing or empty -- multi-tenant isolation only works if every request
/// says which tenant it's for, so there is no "default tenant" fallback
/// silently pooling everyone's data together.
pub struct TenantId(pub String);

#[axum::async_trait]
impl<S> FromRequestParts<S> for TenantId
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        match parts.headers.get("X-Tenant-Id").and_then(|v| v.to_str().ok()) {
            Some(id) if !id.trim().is_empty() => Ok(TenantId(id.to_string())),
            _ => Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "missing_tenant_id",
                    "reason": "the X-Tenant-Id header is required on this endpoint",
                })),
            )
                .into_response()),
        }
    }
}

fn tenant_store_error(tenant_id: &str, e: rusqlite::Error) -> Response {
    tracing::error!("failed to open tenant store for '{tenant_id}': {e}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "tenant_store_unavailable"})),
    )
        .into_response()
}

/// The dashboard's HTML/CSS/JS is a single self-contained static file
/// (no build step, no external CDN, no framework) embedded at compile
/// time -- see `agc-api/static/dashboard.html`. It calls this server's
/// own JSON REST endpoints via `fetch`, so it needs no server-side
/// templating or state of its own.
const DASHBOARD_HTML: &str = include_str!("../static/dashboard.html");

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/dashboard", get(|| async { ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], DASHBOARD_HTML) }))
        .route(
            "/api/v1/tenants",
            get({
                let s = state.clone();
                move |headers: HeaderMap| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    Json(serde_json::json!({"tenants": s.tenant_ids().await})).into_response()
                }
            }),
        )
        .route(
            "/api/v1/traces/count",
            get({
                let s = state.clone();
                move |TenantId(tenant_id): TenantId, headers: HeaderMap| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    traces_count(s, tenant_id).await
                }
            }),
        )
        .route(
            "/api/v1/traces",
            post({
                let s = state.clone();
                move |TenantId(tenant_id): TenantId, headers: HeaderMap, Json(span): Json<TraceSpan>| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Admin).await {
                        return resp;
                    }
                    ingest_trace(s, tenant_id, span).await
                }
            }),
        )
        .route(
            "/api/v1/traces/:trace_id",
            get({
                let s = state.clone();
                move |TenantId(tenant_id): TenantId, headers: HeaderMap, Path(trace_id): Path<Uuid>| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    get_trace(s, tenant_id, trace_id).await
                }
            }),
        )
        .route(
            "/api/v1/audit/count",
            get({
                let s = state.clone();
                move |TenantId(tenant_id): TenantId, headers: HeaderMap| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    audit_count(s, tenant_id).await
                }
            }),
        )
        .route(
            "/api/v1/audit",
            get({
                let s = state.clone();
                move |TenantId(tenant_id): TenantId, headers: HeaderMap, Query(q): Query<AuditQuery>| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    list_audit(s, tenant_id, q).await
                }
            }),
        )
        .route(
            "/api/v1/audit/export.ndjson",
            get({
                let s = state.clone();
                move |TenantId(tenant_id): TenantId, headers: HeaderMap| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    export_ndjson(s, tenant_id).await
                }
            }),
        )
        .route(
            "/api/v1/audit/export.csv",
            get({
                let s = state.clone();
                move |TenantId(tenant_id): TenantId, headers: HeaderMap| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    export_csv(s, tenant_id).await
                }
            }),
        )
        .route(
            "/api/v1/compliance/report",
            get({
                let s = state.clone();
                move |TenantId(tenant_id): TenantId, headers: HeaderMap, Query(q): Query<ComplianceQuery>| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    compliance_report(s, tenant_id, q).await
                }
            }),
        )
        .route(
            "/api/v1/policies/count",
            get({
                let s = state.clone();
                move |headers: HeaderMap| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Viewer).await {
                        return resp;
                    }
                    let count = s.policy.lock().await.policy_count();
                    Json(serde_json::json!({"policy_count": count})).into_response()
                }
            }),
        )
        .route(
            "/api/v1/policies",
            post({
                let s = state.clone();
                move |headers: HeaderMap, Json(policy): Json<GovernancePolicy>| async move {
                    if let Err(resp) = auth::authorize(&s.auth, &headers, Role::Admin).await {
                        return resp;
                    }
                    load_policy(s, policy).await
                }
            }),
        )
}

pub fn default_config() -> ConsoleConfig {
    ConsoleConfig::default_local()
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

async fn traces_count(state: AppState, tenant_id: String) -> Response {
    let store = match state.tenant_store(&tenant_id).await {
        Ok(s) => s,
        Err(e) => return tenant_store_error(&tenant_id, e),
    };
    let count = store.traces.lock().await.span_count();
    Json(serde_json::json!({"tenant_id": tenant_id, "span_count": count})).into_response()
}

async fn audit_count(state: AppState, tenant_id: String) -> Response {
    let store = match state.tenant_store(&tenant_id).await {
        Ok(s) => s,
        Err(e) => return tenant_store_error(&tenant_id, e),
    };
    let count = store.audit.lock().await.record_count();
    Json(serde_json::json!({"tenant_id": tenant_id, "record_count": count})).into_response()
}

/// Real-time policy gate: evaluates every in-scope policy rule (global,
/// shared across tenants) against the incoming span, records one audit
/// entry per matched rule in `tenant_id`'s isolated audit log, and rejects
/// the span with 403 if any matched rule's action is `Block`. A blocked
/// span is never written to that tenant's trace store.
async fn ingest_trace(state: AppState, tenant_id: String, span: TraceSpan) -> Response {
    let store = match state.tenant_store(&tenant_id).await {
        Ok(s) => s,
        Err(e) => return tenant_store_error(&tenant_id, e),
    };

    let matches = {
        let policy = state.policy.lock().await;
        policy.evaluate(&span)
    };

    let mut block: Option<(String, String)> = None;
    {
        let mut audit = store.audit.lock().await;
        for (policy_id, rule) in &matches {
            let outcome = match &rule.action {
                PolicyAction::Warn { .. } => AuditOutcome::Warned,
                PolicyAction::Block { .. } => AuditOutcome::Blocked,
                PolicyAction::Alert { .. } => AuditOutcome::Alerted,
            };
            audit.append(AuditRecord {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                agent_id: span.agent_id.clone(),
                action: span.operation.clone(),
                outcome,
                policy_id: Some(policy_id.clone()),
                details: serde_json::json!({
                    "rule_id": rule.rule_id,
                    "span_id": span.span_id,
                }),
            });
            if block.is_none() {
                if let PolicyAction::Block { reason } = &rule.action {
                    block = Some((rule.rule_id.clone(), reason.clone()));
                }
            }
        }
    }

    if let Some((rule_id, reason)) = block {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "blocked_by_policy",
                "rule_id": rule_id,
                "reason": reason,
            })),
        )
            .into_response();
    }

    if let Some(otlp) = &state.otlp {
        let duration_ms = span.duration_ms().map(|d| d.max(0) as u64).unwrap_or(0);
        otlp.record_span(&span.operation, duration_ms);
    }

    let span_id = span.span_id;
    let trace_id = span.trace_id;
    store.traces.lock().await.ingest(span);

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "tenant_id": tenant_id,
            "span_id": span_id,
            "trace_id": trace_id,
            "policy_events": matches.len(),
        })),
    )
        .into_response()
}

async fn get_trace(state: AppState, tenant_id: String, trace_id: Uuid) -> Response {
    let store = match state.tenant_store(&tenant_id).await {
        Ok(s) => s,
        Err(e) => return tenant_store_error(&tenant_id, e),
    };
    let traces = store.traces.lock().await;
    let spans = traces.spans_for_trace(&trace_id);
    if spans.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "trace_not_found", "tenant_id": tenant_id, "trace_id": trace_id})),
        )
            .into_response();
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({"tenant_id": tenant_id, "trace_id": trace_id, "spans": spans})),
    )
        .into_response()
}

async fn load_policy(state: AppState, policy: GovernancePolicy) -> Response {
    let policy_id = policy.policy_id.clone();
    state.policy.lock().await.load_policy(policy);
    (StatusCode::CREATED, Json(serde_json::json!({"policy_id": policy_id, "loaded": true}))).into_response()
}

#[derive(serde::Deserialize)]
struct AuditQuery {
    limit: Option<usize>,
    offset: Option<usize>,
}

async fn list_audit(state: AppState, tenant_id: String, q: AuditQuery) -> Response {
    let store = match state.tenant_store(&tenant_id).await {
        Ok(s) => s,
        Err(e) => return tenant_store_error(&tenant_id, e),
    };
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let offset = q.offset.unwrap_or(0);
    let (records, total) = store.audit.lock().await.list_paginated(limit, offset);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "tenant_id": tenant_id,
            "total": total,
            "limit": limit,
            "offset": offset,
            "records": records,
        })),
    )
        .into_response()
}

async fn export_ndjson(state: AppState, tenant_id: String) -> Response {
    let store = match state.tenant_store(&tenant_id).await {
        Ok(s) => s,
        Err(e) => return tenant_store_error(&tenant_id, e),
    };
    let body = store.audit.lock().await.export_ndjson();
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/x-ndjson")], body).into_response()
}

async fn export_csv(state: AppState, tenant_id: String) -> Response {
    let store = match state.tenant_store(&tenant_id).await {
        Ok(s) => s,
        Err(e) => return tenant_store_error(&tenant_id, e),
    };
    let body = store.audit.lock().await.export_csv();
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/csv")], body).into_response()
}

#[derive(serde::Deserialize)]
struct ComplianceQuery {
    format: Option<String>,
}

/// `GET /api/v1/compliance/report`: a Responsible-AI-aligned compliance
/// report over this tenant's own trace/audit data (see
/// `agc_core::compliance` for exactly what's covered and what's
/// explicitly out of scope). Markdown by default; `?format=json` for a
/// machine-readable version of the same data.
async fn compliance_report(state: AppState, tenant_id: String, q: ComplianceQuery) -> Response {
    let store = match state.tenant_store(&tenant_id).await {
        Ok(s) => s,
        Err(e) => return tenant_store_error(&tenant_id, e),
    };
    let audit = store.audit.lock().await;
    let trace = store.traces.lock().await;
    let policy = state.policy.lock().await;
    let security = agc_core::SecurityPosture {
        rbac_enabled: !matches!(state.auth, AuthConfig::Disabled),
        telemetry_managed_identity: state.otlp_authenticated,
    };
    let report = agc_core::ComplianceReport::generate(&tenant_id, &audit, &trace, &policy, security);
    match q.format.as_deref() {
        Some("json") => (StatusCode::OK, Json(report)).into_response(),
        _ => (StatusCode::OK, [(header::CONTENT_TYPE, "text/markdown")], report.to_markdown()).into_response(),
    }
}

/// Watches `dir` for filesystem changes and reloads `policy` from it on
/// every event (see `PolicyEngine::load_policies_from_dir`; a parse error
/// logs a warning and keeps the previous policy set). `notify`'s watcher
/// callback runs on its own OS thread, not inside the Tokio runtime, so
/// it can't `.await` the policy lock directly -- it forwards a signal
/// over a channel to a dedicated async task that does the actual reload.
///
/// The returned watcher must be kept alive (e.g. as a local in `main`)
/// for the duration the reload should keep working; dropping it stops
/// the watch.
pub fn spawn_policy_hot_reload(
    dir: PathBuf,
    policy: Arc<Mutex<agc_core::PolicyEngine>>,
) -> notify::Result<notify::RecommendedWatcher> {
    use notify::Watcher;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_ok() {
            let _ = tx.send(());
        }
    })?;
    watcher.watch(&dir, notify::RecursiveMode::NonRecursive)?;

    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            let mut engine = policy.lock().await;
            match engine.load_policies_from_dir(&dir) {
                Ok(n) => tracing::info!("Policy hot-reload: loaded {n} policies from {}", dir.display()),
                Err(e) => tracing::warn!("Policy hot-reload failed, keeping previous policies: {e}"),
            }
        }
    });

    Ok(watcher)
}
