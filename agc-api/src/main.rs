use agc_api::{create_router, default_config, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = default_config();
    let state = AppState::new();
    let app = create_router(state);

    let addr: std::net::SocketAddr = cfg.api_bind.parse().expect("invalid bind address");
    tracing::info!("AGC API listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
