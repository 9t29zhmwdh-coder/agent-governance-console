use agc_core::{AuditLog, ConsoleConfig, PolicyEngine, TraceStore};
use axum::{routing::get, Json, Router};
use std::sync::Arc;
use tokio::sync::Mutex;

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
            "/api/v1/policies/count",
            get({
                let s = state.clone();
                move || async move {
                    let count = s.policy.lock().await.policy_count();
                    Json(serde_json::json!({"policy_count": count}))
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
