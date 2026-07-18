use agc_api::{create_router, AppState};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

const TENANT: &str = "tenant-a";

async fn app() -> axum::Router {
    create_router(AppState::new())
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

fn span_json(agent_id: &str, level: &str, operation: &str, attributes: serde_json::Value) -> serde_json::Value {
    json!({
        "span_id": Uuid::new_v4(),
        "trace_id": Uuid::new_v4(),
        "parent_span_id": null,
        "agent_id": agent_id,
        "operation": operation,
        "level": level,
        "started_at": Utc::now().to_rfc3339(),
        "ended_at": null,
        "attributes": attributes,
    })
}

/// GET a tenant-scoped endpoint with the `X-Tenant-Id` header set.
async fn get_tenant(app: axum::Router, uri: &str, tenant: &str) -> axum::response::Response {
    app.oneshot(Request::builder().uri(uri).header("X-Tenant-Id", tenant).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

/// POST a tenant-scoped JSON endpoint (e.g. `/api/v1/traces`) with the
/// `X-Tenant-Id` header set.
async fn post_tenant_json(
    app: axum::Router,
    uri: &str,
    tenant: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .header("X-Tenant-Id", tenant)
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

/// POST a global (non-tenant-scoped) JSON endpoint, e.g. `/api/v1/policies`
/// (policies are shared governance across all tenants, not tenant-scoped).
async fn post_json(app: axum::Router, uri: &str, body: serde_json::Value) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
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
    let response = get_tenant(app().await, "/api/v1/traces/count", TENANT).await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["span_count"], 0);
    assert_eq!(json["tenant_id"], TENANT);
}

#[tokio::test]
async fn traces_count_without_tenant_header_is_rejected() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/api/v1/traces/count").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let json = body_json(response).await;
    assert_eq!(json["error"], "missing_tenant_id");
}

#[tokio::test]
async fn audit_count_starts_at_zero() {
    let response = get_tenant(app().await, "/api/v1/audit/count", TENANT).await;

    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
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
    let json = body_json(response).await;
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
async fn ingest_trace_is_allowed_with_no_policies_loaded() {
    let response = post_tenant_json(
        app().await,
        "/api/v1/traces",
        TENANT,
        span_json("agent-1", "info", "tool_call", json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let json = body_json(response).await;
    assert_eq!(json["policy_events"], 0);
    assert_eq!(json["tenant_id"], TENANT);
}

#[tokio::test]
async fn ingest_trace_without_tenant_header_is_rejected() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/traces")
                .header("content-type", "application/json")
                .body(Body::from(span_json("agent-1", "info", "tool_call", json!({})).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn ingest_trace_is_blocked_by_matching_block_rule() {
    let app_router = app().await;
    let policy = json!({
        "policy_id": "p1",
        "name": "Error gate",
        "agent_scope": [],
        "rules": [{
            "rule_id": "r1",
            "description": "Block errors",
            "condition": {"type": "span_level_at_least", "level": "error"},
            "action": {"type": "block", "reason": "too severe"}
        }]
    });
    let load = post_json(app_router.clone(), "/api/v1/policies", policy).await;
    assert_eq!(load.status(), StatusCode::CREATED);

    let response = post_tenant_json(
        app_router.clone(),
        "/api/v1/traces",
        TENANT,
        span_json("agent-1", "error", "risky_call", json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let json = body_json(response).await;
    assert_eq!(json["rule_id"], "r1");

    // A blocked span must not be persisted to the trace store.
    let count = get_tenant(app_router, "/api/v1/traces/count", TENANT).await;
    let count_json = body_json(count).await;
    assert_eq!(count_json["span_count"], 0);
}

#[tokio::test]
async fn ingest_trace_records_audit_entry_for_warn_rule_and_still_ingests() {
    let app_router = app().await;
    let policy = json!({
        "policy_id": "p1",
        "name": "Tool watch",
        "agent_scope": [],
        "rules": [{
            "rule_id": "r1",
            "description": "Warn on tools",
            "condition": {"type": "operation_matches", "pattern": "tool_*"},
            "action": {"type": "warn", "message": "tool call"}
        }]
    });
    post_json(app_router.clone(), "/api/v1/policies", policy).await;

    let response = post_tenant_json(
        app_router.clone(),
        "/api/v1/traces",
        TENANT,
        span_json("agent-1", "info", "tool_call", json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let json = body_json(response).await;
    assert_eq!(json["policy_events"], 1);

    let audit_count = get_tenant(app_router, "/api/v1/audit/count", TENANT).await;
    assert_eq!(body_json(audit_count).await["record_count"], 1);
}

#[tokio::test]
async fn get_trace_returns_ingested_spans() {
    let app_router = app().await;
    let trace_id = Uuid::new_v4();
    let mut span = span_json("agent-1", "info", "tool_call", json!({}));
    span["trace_id"] = json!(trace_id);

    let ingest = post_tenant_json(app_router.clone(), "/api/v1/traces", TENANT, span).await;
    assert_eq!(ingest.status(), StatusCode::CREATED);

    let response = get_tenant(app_router, &format!("/api/v1/traces/{trace_id}"), TENANT).await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["spans"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn get_trace_returns_404_for_unknown_trace() {
    let response = get_tenant(app().await, &format!("/api/v1/traces/{}", Uuid::new_v4()), TENANT).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn load_policy_increments_policy_count() {
    let app_router = app().await;
    let policy = json!({
        "policy_id": "p1",
        "name": "Test",
        "agent_scope": [],
        "rules": []
    });
    let response = post_json(app_router.clone(), "/api/v1/policies", policy).await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let count = app_router
        .oneshot(Request::builder().uri("/api/v1/policies/count").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(body_json(count).await["policy_count"], 1);
}

#[tokio::test]
async fn audit_list_is_paginated() {
    let app_router = app().await;
    for i in 0..3 {
        let span = span_json("agent-1", "error", &format!("op-{i}"), json!({}));
        let policy = json!({
            "policy_id": "p1", "name": "n", "agent_scope": [],
            "rules": [{"rule_id": "r1", "description": "d",
                "condition": {"type": "span_level_at_least", "level": "error"},
                "action": {"type": "warn", "message": "m"}}]
        });
        post_json(app_router.clone(), "/api/v1/policies", policy).await;
        post_tenant_json(app_router.clone(), "/api/v1/traces", TENANT, span).await;
    }

    let response = get_tenant(app_router, "/api/v1/audit?limit=2&offset=0", TENANT).await;
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert!(json["total"].as_u64().unwrap() >= 3);
    assert_eq!(json["records"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn audit_export_ndjson_has_correct_content_type() {
    let response = get_tenant(app().await, "/api/v1/audit/export.ndjson", TENANT).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-type").unwrap(), "application/x-ndjson");
}

#[tokio::test]
async fn audit_export_csv_has_correct_content_type_and_header() {
    let response = get_tenant(app().await, "/api/v1/audit/export.csv", TENANT).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/csv");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.starts_with("id,timestamp,agent_id,action,outcome,policy_id\n"));
}

#[tokio::test]
async fn ingest_trace_exports_via_otlp_when_telemetry_is_configured() {
    // Regression test for a real deadlock found during development: an
    // earlier OtlpExporter used a synchronous/simple span processor that
    // did its HTTP export inline on the calling thread, which hung forever
    // when record_span was invoked from inside an already-running Tokio
    // runtime (this axum handler, exactly like a real running server).
    // Wrapped in a timeout so a regression fails fast instead of hanging
    // the whole test suite again.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/traces"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let mut cfg = agc_api::default_config();
        cfg.telemetry.enabled = true;
        cfg.telemetry.endpoint = Some(format!("{}/v1/traces", server.uri()));
        cfg.telemetry.service_name = "agc-test".into();
        let state = AppState::from_config(&cfg).await;
        assert!(state.otlp.is_some(), "OTLP exporter should have been constructed");

        let app_router = create_router(state);
        let response = post_tenant_json(
            app_router,
            "/api/v1/traces",
            TENANT,
            span_json("agent-1", "info", "tool_call", json!({})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);

        // Give the batch processor's background thread a moment to flush;
        // real production traffic doesn't need this, tests polling a mock
        // server right after the response do.
        for _ in 0..20 {
            if !server.received_requests().await.unwrap().is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
    })
    .await
    .expect("ingest with OTLP telemetry did not complete within 10s");
}

#[tokio::test]
async fn otlp_managed_identity_token_fetch_failure_leaves_telemetry_enabled_without_a_header() {
    // Real end-to-end proof of the graceful-degradation path: with
    // use_managed_identity set, from_config tries the *real* default IMDS
    // endpoint (agc_azure::managed_identity::DEFAULT_IMDS_ENDPOINT), which
    // is unreachable here (this test doesn't run on Azure) and times out
    // after 2s (a real bug found and fixed earlier: IMDS silently hangs off
    // Azure without a client-side timeout). Confirms from_config still
    // builds a working OTLP exporter afterward instead of leaving telemetry
    // off or hanging the whole startup -- not just that the code compiles.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/v1/traces"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let mut cfg = agc_api::default_config();
        cfg.telemetry.enabled = true;
        cfg.telemetry.endpoint = Some(format!("{}/v1/traces", server.uri()));
        cfg.telemetry.service_name = "agc-test".into();
        cfg.telemetry.use_managed_identity = true;

        let state = AppState::from_config(&cfg).await;
        assert!(state.otlp.is_some(), "OTLP exporter should still be constructed after an IMDS timeout");
        assert!(!state.otlp_authenticated, "no token was actually obtained, so this must stay false even though it was requested");

        let app_router = create_router(state);
        let response = post_tenant_json(
            app_router,
            "/api/v1/traces",
            TENANT,
            span_json("agent-1", "info", "tool_call", json!({})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED, "ingestion still works even though the OTLP export has no auth header");
    })
    .await
    .expect("managed-identity fallback path did not complete within 10s");
}

#[tokio::test]
async fn policy_hot_reload_picks_up_a_new_file_and_enforces_it() {
    // Real end-to-end proof, not a unit test of load_policies_from_dir in
    // isolation: writes an actual YAML file to a real directory, waits for
    // the real filesystem watcher to notice, and confirms the policy it
    // loaded actually gates a real POST /api/v1/traces request.
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let dir = std::env::temp_dir().join(format!("agc-hot-reload-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let state = AppState::new();
        let _watcher = agc_api::spawn_policy_hot_reload(dir.clone(), state.policy.clone()).unwrap();
        let app_router = create_router(state);

        std::fs::write(
            dir.join("block-errors.yaml"),
            "policy_id: p1\nname: Error gate\nagent_scope: []\nrules:\n  - rule_id: r1\n    description: Block on error\n    condition:\n      type: span_level_at_least\n      level: error\n    action:\n      type: block\n      reason: too severe\n",
        )
        .unwrap();

        // Poll for the watcher's background task to have reloaded.
        let mut loaded = false;
        for _ in 0..40 {
            let response = app_router
                .clone()
                .oneshot(Request::builder().uri("/api/v1/policies/count").body(Body::empty()).unwrap())
                .await
                .unwrap();
            let json = body_json(response).await;
            if json["policy_count"] == 1 {
                loaded = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(loaded, "hot-reloaded policy never showed up in /api/v1/policies/count");

        // The hot-reloaded policy must actually gate ingestion, not just be counted.
        let response = post_tenant_json(
            app_router,
            "/api/v1/traces",
            TENANT,
            span_json("agent-1", "error", "risky_call", json!({})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        std::fs::remove_dir_all(&dir).unwrap();
    })
    .await
    .expect("policy hot-reload did not complete within 10s");
}

#[tokio::test]
async fn app_state_is_isolated_between_instances() {
    let app1 = create_router(AppState::new());
    let app2 = create_router(AppState::new());

    let r1 = get_tenant(app1, "/api/v1/traces/count", TENANT).await;
    let r2 = get_tenant(app2, "/api/v1/traces/count", TENANT).await;

    assert_eq!(r1.status(), StatusCode::OK);
    assert_eq!(r2.status(), StatusCode::OK);
}

#[tokio::test]
async fn tenants_have_isolated_trace_and_audit_stores() {
    let app_router = app().await;

    let ingest = post_tenant_json(
        app_router.clone(),
        "/api/v1/traces",
        "tenant-a",
        span_json("agent-1", "info", "tool_call", json!({})),
    )
    .await;
    assert_eq!(ingest.status(), StatusCode::CREATED);

    let a_count = get_tenant(app_router.clone(), "/api/v1/traces/count", "tenant-a").await;
    assert_eq!(body_json(a_count).await["span_count"], 1);

    // A different tenant must see none of tenant-a's data -- this is the
    // whole point of "tenant isolation in trace/audit stores", not just a
    // filtered view over one shared store.
    let b_count = get_tenant(app_router, "/api/v1/traces/count", "tenant-b").await;
    assert_eq!(body_json(b_count).await["span_count"], 0);
}

#[tokio::test]
async fn tenants_endpoint_lists_tenants_seen_so_far() {
    let app_router = app().await;

    let before = app_router
        .clone()
        .oneshot(Request::builder().uri("/api/v1/tenants").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(body_json(before).await["tenants"].as_array().unwrap().len(), 0);

    get_tenant(app_router.clone(), "/api/v1/traces/count", "tenant-a").await;
    get_tenant(app_router.clone(), "/api/v1/traces/count", "tenant-b").await;

    let after = app_router
        .oneshot(Request::builder().uri("/api/v1/tenants").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let tenants: Vec<String> =
        serde_json::from_value(body_json(after).await["tenants"].clone()).unwrap();
    assert_eq!(tenants, vec!["tenant-a".to_string(), "tenant-b".to_string()]);
}

#[tokio::test]
async fn policies_stay_global_across_tenants() {
    // Policies are explicitly NOT tenant-scoped per ROADMAP.md ("tenant
    // isolation in trace/audit stores" -- policy is deliberately excluded):
    // a policy loaded once must gate ingestion for every tenant.
    let app_router = app().await;
    let policy = json!({
        "policy_id": "p1", "name": "Error gate", "agent_scope": [],
        "rules": [{"rule_id": "r1", "description": "Block errors",
            "condition": {"type": "span_level_at_least", "level": "error"},
            "action": {"type": "block", "reason": "too severe"}}]
    });
    post_json(app_router.clone(), "/api/v1/policies", policy).await;

    for tenant in ["tenant-a", "tenant-b"] {
        let response = post_tenant_json(
            app_router.clone(),
            "/api/v1/traces",
            tenant,
            span_json("agent-1", "error", "risky_call", json!({})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "policy should gate tenant {tenant} too");
    }
}

fn hmac_jwt(secret: &str, roles: &[&str]) -> String {
    use jsonwebtoken::{Algorithm, EncodingKey, Header};
    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &json!({"roles": roles}),
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

async fn post_tenant_json_authed(
    app: axum::Router,
    uri: &str,
    tenant: &str,
    bearer: Option<&str>,
    body: serde_json::Value,
) -> axum::response::Response {
    let mut req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .header("X-Tenant-Id", tenant);
    if let Some(token) = bearer {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    app.oneshot(req.body(Body::from(body.to_string())).unwrap()).await.unwrap()
}

#[tokio::test]
async fn rbac_rejects_write_without_a_token_when_enabled() {
    let mut state = AppState::new();
    state.auth = agc_api::AuthConfig::hmac("s3cret");
    let app_router = create_router(state);

    let response = post_tenant_json_authed(
        app_router,
        "/api/v1/traces",
        TENANT,
        None,
        span_json("agent-1", "info", "tool_call", json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rbac_rejects_write_with_only_viewer_role() {
    let mut state = AppState::new();
    state.auth = agc_api::AuthConfig::hmac("s3cret");
    let app_router = create_router(state);
    let token = hmac_jwt("s3cret", &["viewer"]);

    let response = post_tenant_json_authed(
        app_router,
        "/api/v1/traces",
        TENANT,
        Some(&token),
        span_json("agent-1", "info", "tool_call", json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rbac_allows_write_with_admin_role_and_read_with_viewer_role() {
    let mut state = AppState::new();
    state.auth = agc_api::AuthConfig::hmac("s3cret");
    let app_router = create_router(state);
    let admin_token = hmac_jwt("s3cret", &["admin"]);
    let viewer_token = hmac_jwt("s3cret", &["viewer"]);

    let ingest = post_tenant_json_authed(
        app_router.clone(),
        "/api/v1/traces",
        TENANT,
        Some(&admin_token),
        span_json("agent-1", "info", "tool_call", json!({})),
    )
    .await;
    assert_eq!(ingest.status(), StatusCode::CREATED);

    let count = app_router
        .oneshot(
            Request::builder()
                .uri("/api/v1/traces/count")
                .header("X-Tenant-Id", TENANT)
                .header("Authorization", format!("Bearer {viewer_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(count.status(), StatusCode::OK);
    assert_eq!(body_json(count).await["span_count"], 1);
}

#[tokio::test]
async fn rbac_is_disabled_by_default_and_every_request_works_unauthenticated() {
    // AppState::new()'s default AuthConfig::Disabled must behave exactly
    // like this API did before RBAC existed -- every other test in this
    // file already relies on that, this test asserts it explicitly.
    let response = post_tenant_json(
        app().await,
        "/api/v1/traces",
        TENANT,
        span_json("agent-1", "info", "tool_call", json!({})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn compliance_report_reflects_real_policy_decisions_in_markdown_and_json() {
    // Real end-to-end proof: loads a block-action policy, sends a span that
    // trips it 3 times (crossing the "repeated blocks" threshold), then
    // reads the report back in both formats and checks the actual numbers,
    // not just that the endpoint returns 200.
    let app_router = app().await;
    let policy = json!({
        "policy_id": "p1", "name": "Error gate", "agent_scope": [],
        "rules": [{"rule_id": "r1", "description": "Block errors",
            "condition": {"type": "span_level_at_least", "level": "error"},
            "action": {"type": "block", "reason": "too severe"}}]
    });
    let load = post_json(app_router.clone(), "/api/v1/policies", policy).await;
    assert_eq!(load.status(), StatusCode::CREATED);

    for _ in 0..3 {
        let response = post_tenant_json(
            app_router.clone(),
            "/api/v1/traces",
            TENANT,
            span_json("noisy-agent", "error", "risky_call", json!({})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    let markdown_response = get_tenant(app_router.clone(), "/api/v1/compliance/report", TENANT).await;
    assert_eq!(markdown_response.status(), StatusCode::OK);
    assert_eq!(markdown_response.headers().get("content-type").unwrap(), "text/markdown");
    let body = axum::body::to_bytes(markdown_response.into_body(), usize::MAX).await.unwrap();
    let markdown = String::from_utf8(body.to_vec()).unwrap();
    assert!(markdown.contains(TENANT));
    assert!(markdown.contains("3 blocked"));
    assert!(markdown.contains("`p1`: 3"));
    assert!(markdown.contains("`noisy-agent`: 3 blocks"));
    assert!(markdown.contains("## Out of scope: Fairness, Inclusiveness"));

    let json_response = get_tenant(app_router.clone(), "/api/v1/compliance/report?format=json", TENANT).await;
    assert_eq!(json_response.status(), StatusCode::OK);
    let report = body_json(json_response).await;
    assert_eq!(report["tenant_id"], TENANT);
    assert_eq!(report["total_audit_records"], 3);
    assert_eq!(report["outcomes"]["blocked"], 3);
    assert_eq!(report["records_by_policy"], json!([["p1", 3]]));
    assert_eq!(report["repeated_block_agents"], json!([["noisy-agent", 3]]));
    assert_eq!(report["security"]["rbac_enabled"], false);
}

#[tokio::test]
async fn compliance_report_requires_a_tenant_header() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/api/v1/compliance/report").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn dashboard_serves_real_html_that_calls_every_rest_endpoint_it_needs() {
    // Not a browser render (none available in this environment), but a
    // real proof the served page is the actual dashboard, not a stub: it
    // requires no tenant header and no auth (it's a static asset; the
    // fetch() calls it makes from the browser hit the already-gated real
    // endpoints), and its JS references every endpoint it depends on.
    let response = app()
        .await
        .oneshot(Request::builder().uri("/dashboard").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/html; charset=utf-8");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("<title>Agent Governance Console</title>"));
    for endpoint in [
        "/health",
        "/api/v1/tenants",
        "/api/v1/policies/count",
        "/api/v1/traces/count",
        "/api/v1/audit?limit=",
        "/api/v1/compliance/report",
    ] {
        assert!(html.contains(endpoint), "dashboard JS should call {endpoint}");
    }
}
