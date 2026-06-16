use agc_core::{ConsoleConfig, TraceStore, AuditLog, PolicyEngine};
use axum::{Router, routing::get, Json};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    traces: Arc<Mutex<TraceStore>>,
    audit:  Arc<Mutex<AuditLog>>,
    policy: Arc<Mutex<PolicyEngine>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = ConsoleConfig::default_local();
    let state = AppState {
        traces: Arc::new(Mutex::new(TraceStore::new())),
        audit:  Arc::new(Mutex::new(AuditLog::new())),
        policy: Arc::new(Mutex::new(PolicyEngine::new())),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/traces/count", get({
            let s = state.clone();
            move || async move {
                let count = s.traces.lock().await.span_count();
                Json(serde_json::json!({"span_count": count}))
            }
        }))
        .route("/api/v1/audit/count", get({
            let s = state.clone();
            move || async move {
                let count = s.audit.lock().await.record_count();
                Json(serde_json::json!({"record_count": count}))
            }
        }))
        .route("/api/v1/policies/count", get({
            let s = state.clone();
            move || async move {
                let count = s.policy.lock().await.policy_count();
                Json(serde_json::json!({"policy_count": count}))
            }
        }));

    let addr: std::net::SocketAddr = cfg.api_bind.parse().expect("invalid bind address");
    tracing::info!("AGC API listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}
