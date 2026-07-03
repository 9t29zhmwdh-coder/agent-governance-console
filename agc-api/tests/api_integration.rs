use agc_api::{create_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

async fn app() -> axum::Router {
    create_router(AppState::new())
}

#[tokio::test]
async fn health_returns_ok() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn traces_count_starts_at_zero() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/traces/count")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["span_count"], 0);
}

#[tokio::test]
async fn audit_count_starts_at_zero() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit/count")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["record_count"], 0);
}

#[tokio::test]
async fn policies_count_starts_at_zero() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/policies/count")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["policy_count"], 0);
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn app_state_is_isolated_between_instances() {
    let app1 = create_router(AppState::new());
    let app2 = create_router(AppState::new());

    let r1 = app1
        .oneshot(Request::builder().uri("/api/v1/traces/count").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let r2 = app2
        .oneshot(Request::builder().uri("/api/v1/traces/count").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(r2.status(), StatusCode::OK);
}
