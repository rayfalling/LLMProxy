//! Dashboard integration tests (S6.6)
//!
//! Tests: login, JWT auth middleware, provider list/enable, alias strategy —
//! all against an in-memory SQLite DB seeded with known data.

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    routing::{get, post, put},
    Router,
};
use dashboard::{auth, handlers, state::AppState};
use llm_core::{config::AuthConfig, db::connect_and_migrate};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt; // Router::oneshot

// ── constants ─────────────────────────────────────────────────────────────────

const JWT_SECRET: &str = "test-secret-key";
const TENANT_ID: &str = "10000000-0000-0000-0000-000000000001";
const ADMIN_USER: &str = "admin";
const ADMIN_PASS: &str = "correct-password";

// ── router builder ────────────────────────────────────────────────────────────

fn build_app(pool: sqlx::SqlitePool) -> Router {
    let state = AppState {
        pool,
        auth: Arc::new(AuthConfig {
            jwt_secret: JWT_SECRET.to_string(),
            jwt_expiry_secs: 3600,
        }),
    };
    Router::new()
        .route("/api/auth/login", post(auth::login))
        .route("/api/me", get(handlers::me))
        .route("/api/providers", get(handlers::list_providers))
        .route("/api/providers/{provider_id}/enabled", put(handlers::set_provider_enabled))
        .route("/api/aliases", get(handlers::list_aliases))
        .route("/api/aliases/{alias_name}/strategy", put(handlers::update_alias_route_strategy))
        .route("/api/stats", get(handlers::tenant_stats))
        .with_state(state)
}

// ── DB seed ───────────────────────────────────────────────────────────────────

async fn setup_db() -> sqlx::SqlitePool {
    let pool = connect_and_migrate("sqlite::memory:").await.unwrap();

    sqlx::query(
        "INSERT INTO tenants (id, name, status, created_at, updated_at)
         VALUES (?, 'test-tenant', 'active', datetime('now'), datetime('now'))",
    )
    .bind(TENANT_ID)
    .execute(&pool)
    .await
    .unwrap();

    let salt = SaltString::from_b64("dGVzdHNhbHQxMjM0NQ").unwrap();
    let hash = Argon2::default()
        .hash_password(ADMIN_PASS.as_bytes(), &salt)
        .unwrap()
        .to_string();

    sqlx::query(
        "INSERT INTO tenant_admins (id, tenant_id, username, password_hash, status)
         VALUES ('admin-1', ?, ?, ?, 'active')",
    )
    .bind(TENANT_ID)
    .bind(ADMIN_USER)
    .bind(&hash)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO providers (id, name, display_name, base_url, enabled, health_state, \
         created_at, updated_at) \
         VALUES ('prov-1', 'openai', 'OpenAI', 'https://api.openai.com', 1, 'healthy', \
         datetime('now'), datetime('now'))",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO model_aliases (id, alias_name, route_strategy, created_at, updated_at)
         VALUES ('alias-1', 'gpt-4o', 'priority', datetime('now'), datetime('now'))",
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

// ── login helper ─────────────────────────────────────────────────────────────

async fn do_login(app: &Router, username: &str, password: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"tenant_name": "test-tenant", "username": username, "password": password})
                .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_login_success() {
    let app = build_app(setup_db().await);
    let (status, body) = do_login(&app, ADMIN_USER, ADMIN_PASS).await;
    assert_eq!(status, StatusCode::OK, "login should return 200; body: {body}");
    assert!(body["token"].is_string(), "response should contain JWT token");
    assert_eq!(body["token_type"], "Bearer");
}

#[tokio::test]
async fn test_login_wrong_password_returns_401() {
    let app = build_app(setup_db().await);
    let (status, _) = do_login(&app, ADMIN_USER, "wrong-password").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_without_token_returns_401() {
    let app = build_app(setup_db().await);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/me")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_me_with_valid_token_returns_identity() {
    let app = build_app(setup_db().await);
    let (_, login_body) = do_login(&app, ADMIN_USER, ADMIN_PASS).await;
    let token = login_body["token"].as_str().unwrap().to_string();

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["username"], ADMIN_USER);
    assert_eq!(body["tenant_id"], TENANT_ID);
}

#[tokio::test]
async fn test_list_providers_returns_seeded_provider() {
    let app = build_app(setup_db().await);
    let (_, login_body) = do_login(&app, ADMIN_USER, ADMIN_PASS).await;
    let token = login_body["token"].as_str().unwrap().to_string();

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/providers")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    let providers = json.as_array().unwrap();
    assert_eq!(providers.len(), 1);
    assert_eq!(providers[0]["name"], "openai");
    assert_eq!(providers[0]["enabled"], 1);
}

#[tokio::test]
async fn test_set_provider_enabled_toggles_flag() {
    let pool = setup_db().await;
    let app = build_app(pool);
    let (_, login_body) = do_login(&app, ADMIN_USER, ADMIN_PASS).await;
    let token = login_body["token"].as_str().unwrap().to_string();

    let req = Request::builder()
        .method(Method::PUT)
        .uri("/api/providers/prov-1/enabled")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"enabled": false}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Re-list to verify
    let req2 = Request::builder()
        .method(Method::GET)
        .uri("/api/providers")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    let bytes = to_bytes(resp2.into_body(), 1 << 20).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json[0]["enabled"], 0, "provider should now be disabled");
}

#[tokio::test]
async fn test_list_aliases_returns_seeded_alias() {
    let app = build_app(setup_db().await);
    let (_, login_body) = do_login(&app, ADMIN_USER, ADMIN_PASS).await;
    let token = login_body["token"].as_str().unwrap().to_string();

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/aliases")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    let aliases = json.as_array().unwrap();
    assert_eq!(aliases.len(), 1);
    assert_eq!(aliases[0]["alias_name"], "gpt-4o");
}

#[tokio::test]
async fn test_update_alias_strategy_persists() {
    let pool = setup_db().await;
    let app = build_app(pool);
    let (_, login_body) = do_login(&app, ADMIN_USER, ADMIN_PASS).await;
    let token = login_body["token"].as_str().unwrap().to_string();

    let req = Request::builder()
        .method(Method::PUT)
        .uri("/api/aliases/gpt-4o/strategy")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"route_strategy": "latency"}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify via list
    let req2 = Request::builder()
        .method(Method::GET)
        .uri("/api/aliases")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp2 = app.oneshot(req2).await.unwrap();
    let bytes = to_bytes(resp2.into_body(), 1 << 20).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json[0]["route_strategy"], "latency");
}

#[tokio::test]
async fn test_stats_empty_logs_returns_zeros_not_500() {
    // Regression: GET /api/stats used to 500 on a fresh DB because
    // `COALESCE(AVG(latency_ms), 0)` returned INTEGER, mismatching the f64
    // decode target. Fix casts the column to REAL.
    let app = build_app(setup_db().await);
    let (_, login_body) = do_login(&app, ADMIN_USER, ADMIN_PASS).await;
    let token = login_body["token"].as_str().unwrap().to_string();

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/stats")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "stats on empty logs must not 500");

    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["total_requests"], 0);
    assert_eq!(body["total_input_tokens"], 0);
    assert_eq!(body["total_output_tokens"], 0);
    assert_eq!(body["avg_latency_ms"], 0.0);
    assert_eq!(body["qps_last_hour"], 0.0);
    assert_eq!(body["error_rate_last_hour"], 0.0);
    assert_eq!(body["failover_count_last_hour"], 0);
}

#[tokio::test]
async fn test_stats_with_seeded_logs_aggregates() {
    let pool = setup_db().await;
    // Seed 4 request_logs, mix of statuses + non-trivial latencies.
    for (idx, (status, lat, fail)) in [
        ("success", 100i64, 0i64),
        ("success", 200, 0),
        ("error", 300, 1),
        ("success", 400, 0),
    ]
    .iter()
    .enumerate()
    {
        sqlx::query(
            "INSERT INTO request_logs (id, tenant_id, api_key_id, request_id, model_alias, \
             origin_protocol, status, latency_ms, input_tokens, output_tokens, \
             failover_count, created_at) \
             VALUES (?, ?, 'key-1', ?, 'gpt-4o', 'openai', ?, ?, 10, 20, ?, datetime('now'))",
        )
        .bind(format!("log-{idx}"))
        .bind(TENANT_ID)
        .bind(format!("req-{idx}"))
        .bind(*status)
        .bind(*lat)
        .bind(*fail)
        .execute(&pool)
        .await
        .unwrap();
    }

    let app = build_app(pool);
    let (_, login_body) = do_login(&app, ADMIN_USER, ADMIN_PASS).await;
    let token = login_body["token"].as_str().unwrap().to_string();

    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/stats")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["total_requests"], 4);
    assert_eq!(body["total_input_tokens"], 40);
    assert_eq!(body["total_output_tokens"], 80);
    assert_eq!(body["avg_latency_ms"], 250.0);
    assert_eq!(body["failover_count_last_hour"], 1);
    // 1 error / 4 total
    let err_rate = body["error_rate_last_hour"].as_f64().unwrap();
    assert!((err_rate - 0.25).abs() < 1e-9, "error_rate={err_rate}");
}
