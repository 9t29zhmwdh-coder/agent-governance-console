use agc_api::{create_router, default_config, spawn_policy_hot_reload, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let mut cfg = default_config();
    if let Ok(path) = std::env::var("AGC_AUDIT_DB_PATH") {
        cfg.audit_db_path = Some(path.into());
    }
    if let Ok(endpoint) = std::env::var("AGC_TELEMETRY_ENDPOINT") {
        cfg.telemetry.enabled = true;
        cfg.telemetry.endpoint = Some(endpoint);
        cfg.telemetry.service_name =
            std::env::var("AGC_TELEMETRY_SERVICE_NAME").unwrap_or_else(|_| "agc".to_string());
    }

    let state = AppState::from_config(&cfg).expect("opening audit database");
    if let Some(path) = &cfg.audit_db_path {
        tracing::info!("Audit log persisted to {}", path.display());
    } else {
        tracing::info!("Audit log is in-memory only (set AGC_AUDIT_DB_PATH to persist)");
    }
    if state.otlp.is_some() {
        tracing::info!("OTLP telemetry export enabled to {}", cfg.telemetry.endpoint.as_deref().unwrap_or(""));
    } else {
        tracing::info!("Telemetry is disabled (set AGC_TELEMETRY_ENDPOINT to enable OTLP export)");
    }
    // Kept alive for main()'s whole lifetime (dropping it stops the watch).
    let _policy_watcher = match std::env::var("AGC_POLICY_DIR") {
        Ok(dir) => {
            let dir = std::path::PathBuf::from(dir);
            let initial = state
                .policy
                .try_lock()
                .expect("no other task holds the policy lock at startup")
                .load_policies_from_dir(&dir)
                .unwrap_or_else(|e| panic!("loading initial policies from {}: {e}", dir.display()));
            tracing::info!("Loaded {initial} policies from {} (hot-reload enabled)", dir.display());
            Some(spawn_policy_hot_reload(dir, state.policy.clone()).expect("starting policy directory watcher"))
        }
        Err(_) => {
            tracing::info!("No AGC_POLICY_DIR set: policies must be loaded via POST /api/v1/policies");
            None
        }
    };

    let app = create_router(state);

    let addr: std::net::SocketAddr = cfg.api_bind.parse().expect("invalid bind address");
    tracing::info!("AGC API listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
