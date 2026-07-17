use agc_core::{AuditLog, AuditOutcome, AuditRecord, ConsoleConfig, GovernancePolicy, PolicyAction, PolicyEngine, TraceSpan, TraceStore};
use axum::{
    extract::{Path, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub traces: Arc<Mutex<TraceStore>>,
    pub audit: Arc<Mutex<AuditLog>>,
    pub policy: Arc<Mutex<PolicyEngine>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            traces: Arc::new(Mutex::new(TraceStore::new())),
            audit: Arc::new(Mutex::new(AuditLog::new())),
            policy: Arc::new(Mutex::new(PolicyEngine::new())),
        }
    }

    /// Same as `new()`, but the audit log persists to a SQLite file at
    /// `path` so records survive a process restart instead of vanishing
    /// with the in-memory log `new()` uses.
    pub fn with_audit_db(path: impl AsRef<std::path::Path>) -> rusqlite::Result<Self> {
        Ok(Self {
            traces: Arc::new(Mutex::new(TraceStore::new())),
            audit: Arc::new(Mutex::new(AuditLog::open(path)?)),
            policy: Arc::new(Mutex::new(PolicyEngine::new())),
        })
    }

    /// Builds from a `ConsoleConfig`: file-backed audit log if
    /// `audit_db_path` is set, in-memory otherwise.
    pub fn from_config(cfg: &ConsoleConfig) -> rusqlite::Result<Self> {
        match &cfg.audit_db_path {
            Some(path) => Self::with_audit_db(path),
            None => Ok(Self::new()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/api/v1/traces/count",
            get({
                let s = state.clone();
                move || async move {
                    let count = s.traces.lock().await.span_count();
                    Json(serde_json::json!({"span_count": count}))
                }
            }),
        )
        .route(
            "/api/v1/traces",
            post({
                let s = state.clone();
                move |Json(span): Json<TraceSpan>| async move { ingest_trace(s, span).await }
            }),
        )
        .route(
            "/api/v1/traces/:trace_id",
            get({
                let s = state.clone();
                move |Path(trace_id): Path<Uuid>| async move { get_trace(s, trace_id).await }
            }),
        )
        .route(
            "/api/v1/audit/count",
            get({
                let s = state.clone();
                move || async move {
                    let count = s.audit.lock().await.record_count();
                    Json(serde_json::json!({"record_count": count}))
                }
            }),
        )
        .route(
            "/api/v1/audit",
            get({
                let s = state.clone();
                move |Query(q): Query<AuditQuery>| async move { list_audit(s, q).await }
            }),
        )
        .route(
            "/api/v1/audit/export.ndjson",
            get({
                let s = state.clone();
                move || async move { export_ndjson(s).await }
            }),
        )
        .route(
            "/api/v1/audit/export.csv",
            get({
                let s = state.clone();
                move || async move { export_csv(s).await }
            }),
        )
        .route(
            "/api/v1/policies/count",
            get({
                let s = state.clone();
                move || async move {
                    let count = s.policy.lock().await.policy_count();
                    Json(serde_json::json!({"policy_count": count}))
                }
            }),
        )
        .route(
            "/api/v1/policies",
            post({
                let s = state.clone();
                move |Json(policy): Json<GovernancePolicy>| async move { load_policy(s, policy).await }
            }),
        )
}

pub fn default_config() -> ConsoleConfig {
    ConsoleConfig::default_local()
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

/// Real-time policy gate: evaluates every in-scope policy rule against the
/// incoming span, records one audit entry per matched rule, and rejects the
/// span with 403 if any matched rule's action is `Block`. A blocked span is
/// never written to the trace store.
async fn ingest_trace(state: AppState, span: TraceSpan) -> Response {
    let matches = {
        let policy = state.policy.lock().await;
        policy.evaluate(&span)
    };

    let mut block: Option<(String, String)> = None;
    {
        let mut audit = state.audit.lock().await;
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

    let span_id = span.span_id;
    let trace_id = span.trace_id;
    state.traces.lock().await.ingest(span);

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "span_id": span_id,
            "trace_id": trace_id,
            "policy_events": matches.len(),
        })),
    )
        .into_response()
}

async fn get_trace(state: AppState, trace_id: Uuid) -> Response {
    let traces = state.traces.lock().await;
    let spans = traces.spans_for_trace(&trace_id);
    if spans.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "trace_not_found", "trace_id": trace_id})),
        )
            .into_response();
    }
    (StatusCode::OK, Json(serde_json::json!({"trace_id": trace_id, "spans": spans}))).into_response()
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

async fn list_audit(state: AppState, q: AuditQuery) -> Response {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let offset = q.offset.unwrap_or(0);
    let (records, total) = state.audit.lock().await.list_paginated(limit, offset);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "total": total,
            "limit": limit,
            "offset": offset,
            "records": records,
        })),
    )
        .into_response()
}

async fn export_ndjson(state: AppState) -> Response {
    let body = state.audit.lock().await.export_ndjson();
    (StatusCode::OK, [(header::CONTENT_TYPE, "application/x-ndjson")], body).into_response()
}

async fn export_csv(state: AppState) -> Response {
    let body = state.audit.lock().await.export_csv();
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/csv")], body).into_response()
}
