//! Integration tests for the first-boot setup endpoint
//! (`POST /api/setup`, `GET /api/setup/status`).

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    routing::{get, post},
    Router,
};
use dashboard::{auth, bootstrap, state::AppState};
use llm_core::{config::AuthConfig, db::connect_and_migrate};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

const JWT_SECRET: &str = "test-secret-key";

fn build_app(pool: sqlx::SqlitePool) -> Router {
    let state = AppState {
        pool,
        auth: Arc::new(AuthConfig {
            jwt_secret: JWT_SECRET.to_string(),
            jwt_expiry_secs: 3600,
        }),
    };
    Router::new()
        .route("/api/setup", post(bootstrap::setup))
        .route("/api/setup/status", get(bootstrap::setup_status))
        .route("/api/auth/login", post(auth::login))
        .with_state(state)
}

async fn empty_pool() -> sqlx::SqlitePool {
    connect_and_migrate("sqlite::memory:").await.unwrap()
}

async fn post_setup(app: &Router, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/setup")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let parsed = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
}

async fn get_status(app: &Router) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/setup/status")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let parsed = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
}

#[tokio::test]
async fn status_reports_uninitialized_for_empty_db() {
    let app = build_app(empty_pool().await);
    let (status, body) = get_status(&app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["initialized"], false);
}

#[tokio::test]
async fn setup_creates_tenant_and_admin_when_empty() {
    let app = build_app(empty_pool().await);
    let (status, body) = post_setup(
        &app,
        json!({
            "tenant_name": "acme",
            "username": "admin",
            "password": "supersecret",
            "password_confirm": "supersecret"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["success"], true);
    assert!(body["tenant_id"].is_string());
    assert!(body["admin_id"].is_string());

    // status flips to initialized
    let (_, status_body) = get_status(&app).await;
    assert_eq!(status_body["initialized"], true);
}

#[tokio::test]
async fn setup_then_login_with_created_admin_succeeds() {
    let app = build_app(empty_pool().await);

    let (status, _) = post_setup(
        &app,
        json!({
            "tenant_name": "acme",
            "username": "alice",
            "password": "supersecret",
            "password_confirm": "supersecret"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "tenant_name": "acme",
                "username": "alice",
                "password": "supersecret"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let login_status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(login_status, StatusCode::OK, "login body: {body}");
    assert!(body["token"].is_string());
}

#[tokio::test]
async fn setup_returns_409_when_already_initialized() {
    let app = build_app(empty_pool().await);
    let (first_status, _) = post_setup(
        &app,
        json!({
            "tenant_name": "first",
            "username": "admin",
            "password": "supersecret",
            "password_confirm": "supersecret"
        }),
    )
    .await;
    assert_eq!(first_status, StatusCode::OK);

    let (second_status, body) = post_setup(
        &app,
        json!({
            "tenant_name": "second",
            "username": "admin2",
            "password": "supersecret",
            "password_confirm": "supersecret"
        }),
    )
    .await;
    assert_eq!(second_status, StatusCode::CONFLICT);
    assert_eq!(body["error"], "already_initialized");
}

#[tokio::test]
async fn setup_rejects_short_password() {
    let app = build_app(empty_pool().await);
    let (status, body) = post_setup(
        &app,
        json!({
            "tenant_name": "acme",
            "username": "admin",
            "password": "short",
            "password_confirm": "short"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_password");
}

#[tokio::test]
async fn setup_rejects_password_mismatch() {
    let app = build_app(empty_pool().await);
    let (status, body) = post_setup(
        &app,
        json!({
            "tenant_name": "acme",
            "username": "admin",
            "password": "supersecret",
            "password_confirm": "differentsecret"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "password_mismatch");
}

#[tokio::test]
async fn setup_rejects_empty_tenant_or_username() {
    let app = build_app(empty_pool().await);

    let (s1, b1) = post_setup(
        &app,
        json!({
            "tenant_name": "   ",
            "username": "admin",
            "password": "supersecret",
            "password_confirm": "supersecret"
        }),
    )
    .await;
    assert_eq!(s1, StatusCode::BAD_REQUEST);
    assert_eq!(b1["error"], "invalid_tenant_name");

    let (s2, b2) = post_setup(
        &app,
        json!({
            "tenant_name": "acme",
            "username": "",
            "password": "supersecret",
            "password_confirm": "supersecret"
        }),
    )
    .await;
    assert_eq!(s2, StatusCode::BAD_REQUEST);
    assert_eq!(b2["error"], "invalid_username");
}
