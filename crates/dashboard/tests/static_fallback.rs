//! Integration tests for the embedded SPA static-asset fallback.

use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    routing::{get, post},
    Router,
};
use dashboard::{auth, bootstrap, state::AppState, static_assets};
use llm_core::{config::AuthConfig, db::connect_and_migrate};
use std::sync::Arc;
use tower::ServiceExt;

fn build_app() -> Router {
    // We don't actually exercise the DB here, but AppState requires a pool.
    let pool = futures::executor::block_on(connect_and_migrate("sqlite::memory:")).unwrap();
    let state = AppState {
        pool,
        auth: Arc::new(AuthConfig {
            jwt_secret: "test".into(),
            jwt_expiry_secs: 3600,
        }),
    };
    Router::new()
        .route("/api/setup/status", get(bootstrap::setup_status))
        .route("/api/auth/login", post(auth::login))
        .fallback(static_assets::serve_spa)
        .with_state(state)
}

async fn fetch(app: &Router, path: &str) -> (StatusCode, String, String) {
    let req = Request::builder()
        .method(Method::GET)
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, content_type, String::from_utf8_lossy(&bytes).into_owned())
}

#[tokio::test]
async fn root_path_returns_html_index() {
    let app = build_app();
    let (status, ctype, body) = fetch(&app, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(ctype.starts_with("text/html"), "unexpected content-type: {ctype}");
    assert!(body.contains("<html") || body.contains("<!doctype"), "expected HTML; got: {body}");
}

#[tokio::test]
async fn unknown_spa_route_returns_index_html() {
    let app = build_app();
    let (status, ctype, _body) = fetch(&app, "/dashboard/anything/deep").await;
    assert_eq!(status, StatusCode::OK);
    assert!(ctype.starts_with("text/html"));
}

#[tokio::test]
async fn api_routes_are_not_shadowed_by_fallback() {
    let app = build_app();
    let (status, ctype, _body) = fetch(&app, "/api/setup/status").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        ctype.starts_with("application/json"),
        "expected JSON for /api/setup/status, got {ctype}"
    );
}
