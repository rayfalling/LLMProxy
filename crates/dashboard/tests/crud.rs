//! Integration tests for the CRUD endpoints introduced by the
//! `webui-resource-crud-and-tenantless-login` change.
//!
//! Covers happy paths, validation errors, conflict (in-use / duplicate),
//! and tenant-isolation for the tenant-scoped resources.

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    body::{to_bytes, Body},
    http::{header, Method, Request, StatusCode},
    routing::{delete, get, post, put},
    Router,
};
use dashboard::{auth, handlers, state::AppState};
use llm_core::{config::AuthConfig, db::connect_and_migrate};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

const JWT_SECRET: &str = "test-secret-key";

const TENANT_A: &str = "10000000-0000-0000-0000-00000000000a";
const TENANT_B: &str = "10000000-0000-0000-0000-00000000000b";
const ADMIN_A: &str = "alice";
const ADMIN_B: &str = "bob";
const PASS: &str = "correct-password";

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
        .route(
            "/api/providers",
            get(handlers::list_providers).post(handlers::create_provider),
        )
        .route("/api/providers/{provider_id}", delete(handlers::delete_provider))
        .route(
            "/api/providers/{provider_id}/keys",
            get(handlers::list_provider_keys).post(handlers::create_provider_key),
        )
        .route(
            "/api/providers/{provider_id}/keys/{key_id}",
            delete(handlers::delete_provider_key),
        )
        .route(
            "/api/providers/{provider_id}/models",
            get(handlers::list_provider_models).post(handlers::create_provider_model),
        )
        .route(
            "/api/providers/{provider_id}/models/{model_name}",
            delete(handlers::delete_provider_model),
        )
        .route(
            "/api/aliases",
            get(handlers::list_aliases).post(handlers::create_alias),
        )
        .route("/api/aliases/{alias_name}", delete(handlers::delete_alias))
        .route(
            "/api/api-keys",
            get(handlers::list_api_keys).post(handlers::create_api_key),
        )
        .route("/api/api-keys/{api_key_id}", delete(handlers::delete_api_key))
        .route(
            "/api/key-pools",
            get(handlers::list_key_pool_mappings).post(handlers::create_key_pool),
        )
        .route(
            "/api/key-pools/{api_key_id}/{provider_id}",
            delete(handlers::delete_key_pool),
        )
        .route(
            "/api/vision-mappings",
            get(handlers::list_vision_mappings).post(handlers::create_vision_mapping),
        )
        .route(
            "/api/vision-mappings/{model_name}",
            put(handlers::update_vision_mapping).delete(handlers::delete_vision_mapping),
        )
        .with_state(state)
}

async fn seed_two_tenants() -> sqlx::SqlitePool {
    let pool = connect_and_migrate("sqlite::memory:").await.unwrap();
    for (id, name) in [(TENANT_A, "tenant-a"), (TENANT_B, "tenant-b")] {
        sqlx::query(
            "INSERT INTO tenants (id, name, status, created_at, updated_at) \
             VALUES (?, ?, 'active', datetime('now'), datetime('now'))",
        )
        .bind(id)
        .bind(name)
        .execute(&pool)
        .await
        .unwrap();
    }
    let salt = SaltString::from_b64("dGVzdHNhbHQxMjM0NQ").unwrap();
    let hash = Argon2::default()
        .hash_password(PASS.as_bytes(), &salt)
        .unwrap()
        .to_string();
    for (admin_id, tenant_id, username) in [
        ("admin-a", TENANT_A, ADMIN_A),
        ("admin-b", TENANT_B, ADMIN_B),
    ] {
        sqlx::query(
            "INSERT INTO tenant_admins (id, tenant_id, username, password_hash, status) \
             VALUES (?, ?, ?, ?, 'active')",
        )
        .bind(admin_id)
        .bind(tenant_id)
        .bind(username)
        .bind(&hash)
        .execute(&pool)
        .await
        .unwrap();
    }
    pool
}

async fn login_token(app: &Router, username: &str) -> String {
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"username": username, "password": PASS}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    body["token"].as_str().unwrap().to_string()
}

async fn send(app: &Router, method: Method, uri: &str, token: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"));
    let req = if let Some(b) = body {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
        builder.body(Body::from(b.to_string())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    let v = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, v)
}

// ── providers ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_provider_happy_path_then_list() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;
    let (status, body) = send(
        &app,
        Method::POST,
        "/api/providers",
        &token,
        Some(json!({
            "name": "openai",
            "display_name": "OpenAI",
            "base_url": "https://api.openai.com",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(body["name"], "openai");

    let (status, body) = send(&app, Method::GET, "/api/providers", &token, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn create_provider_invalid_name_returns_400() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;
    let (status, body) = send(
        &app,
        Method::POST,
        "/api/providers",
        &token,
        Some(json!({
            "name": "Bad Name!",
            "display_name": "X",
            "base_url": "https://x",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["error"], "invalid_field");
}

#[tokio::test]
async fn create_provider_duplicate_name_returns_409() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;
    let payload = json!({
        "name": "openai",
        "display_name": "OpenAI",
        "base_url": "https://api.openai.com",
    });
    let (s1, _) = send(&app, Method::POST, "/api/providers", &token, Some(payload.clone())).await;
    assert_eq!(s1, StatusCode::CREATED);
    let (s2, body) = send(&app, Method::POST, "/api/providers", &token, Some(payload)).await;
    assert_eq!(s2, StatusCode::CONFLICT);
    assert_eq!(body["error"], "name_taken");
}

#[tokio::test]
async fn delete_provider_then_404_on_second_delete() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;
    let (_, body) = send(
        &app,
        Method::POST,
        "/api/providers",
        &token,
        Some(json!({"name": "p1", "display_name": "P1", "base_url": "https://x"})),
    )
    .await;
    let pid = body["id"].as_str().unwrap().to_string();
    let (s1, _) = send(&app, Method::DELETE, &format!("/api/providers/{pid}"), &token, None).await;
    assert_eq!(s1, StatusCode::NO_CONTENT);
    let (s2, _) = send(&app, Method::DELETE, &format!("/api/providers/{pid}"), &token, None).await;
    assert_eq!(s2, StatusCode::NOT_FOUND);
}

// ── provider keys ────────────────────────────────────────────────────────────

#[tokio::test]
async fn provider_key_create_list_delete_with_masking() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;
    let (_, p) = send(
        &app,
        Method::POST,
        "/api/providers",
        &token,
        Some(json!({"name": "p1", "display_name": "P1", "base_url": "https://x"})),
    )
    .await;
    let pid = p["id"].as_str().unwrap().to_string();

    let (status, key) = send(
        &app,
        Method::POST,
        &format!("/api/providers/{pid}/keys"),
        &token,
        Some(json!({"label": "main", "plaintext_key": "sk-supersecret-1234567890"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {key}");
    let kid = key["id"].as_str().unwrap().to_string();

    let (s_list, list) = send(&app, Method::GET, &format!("/api/providers/{pid}/keys"), &token, None).await;
    assert_eq!(s_list, StatusCode::OK);
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let preview = arr[0]["key_preview"].as_str().unwrap();
    assert!(!preview.contains("supersecret"), "preview must not leak full key: {preview}");
    assert!(preview.starts_with("sk-s"));

    let (s_del, _) = send(
        &app,
        Method::DELETE,
        &format!("/api/providers/{pid}/keys/{kid}"),
        &token,
        None,
    )
    .await;
    assert_eq!(s_del, StatusCode::NO_CONTENT);
}

// ── aliases ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_alias_with_targets_then_blocks_provider_delete() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;

    let (_, p) = send(
        &app,
        Method::POST,
        "/api/providers",
        &token,
        Some(json!({"name": "p1", "display_name": "P1", "base_url": "https://x"})),
    )
    .await;
    let pid = p["id"].as_str().unwrap().to_string();

    // Register a model so the alias target is meaningful (not strictly enforced by FK).
    let (_, _) = send(
        &app,
        Method::POST,
        &format!("/api/providers/{pid}/models"),
        &token,
        Some(json!({"model_name": "gpt-4o-2024"})),
    )
    .await;

    let (status, body) = send(
        &app,
        Method::POST,
        "/api/aliases",
        &token,
        Some(json!({
            "alias_name": "gpt-4o",
            "route_strategy": "priority",
            "targets": [
                {"provider_id": pid, "model_name": "gpt-4o-2024", "priority": 0, "enabled": true}
            ]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_eq!(body["targets_count"], 1);

    // Provider deletion should now be blocked.
    let (s_del, body) = send(&app, Method::DELETE, &format!("/api/providers/{pid}"), &token, None).await;
    assert_eq!(s_del, StatusCode::CONFLICT);
    assert_eq!(body["error"], "in_use");

    // Delete alias unblocks deletion.
    let (s_alias_del, _) = send(&app, Method::DELETE, "/api/aliases/gpt-4o", &token, None).await;
    assert_eq!(s_alias_del, StatusCode::NO_CONTENT);
    let (s_del2, _) = send(&app, Method::DELETE, &format!("/api/providers/{pid}"), &token, None).await;
    assert_eq!(s_del2, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn create_alias_with_invalid_route_strategy_returns_400() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;
    let (status, body) = send(
        &app,
        Method::POST,
        "/api/aliases",
        &token,
        Some(json!({"alias_name": "x", "route_strategy": "round-robin", "targets": []})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "invalid_field");
}

// ── api keys (tenant-scoped) ─────────────────────────────────────────────────

#[tokio::test]
async fn create_api_key_returns_plaintext_once_then_list_only_prefix() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;
    let (status, body) = send(
        &app,
        Method::POST,
        "/api/api-keys",
        &token,
        Some(json!({"name": "ci"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    let plaintext = body["plaintext_key"].as_str().unwrap();
    assert!(plaintext.starts_with("lp_"), "key should be prefixed: {plaintext}");
    assert_eq!(plaintext.len(), 3 + 48, "lp_ + 48 hex chars");
    let prefix = body["prefix"].as_str().unwrap();
    assert_eq!(prefix.len(), 10);

    let (_, list) = send(&app, Method::GET, "/api/api-keys", &token, None).await;
    let arr = list.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["prefix"], prefix);
    assert!(arr[0].get("plaintext_key").is_none(), "list must not return plaintext");
}

#[tokio::test]
async fn api_keys_are_tenant_isolated() {
    let app = build_app(seed_two_tenants().await);
    let token_a = login_token(&app, ADMIN_A).await;
    let token_b = login_token(&app, ADMIN_B).await;

    let (_, key_a) = send(&app, Method::POST, "/api/api-keys", &token_a, Some(json!({}))).await;
    let id_a = key_a["id"].as_str().unwrap().to_string();

    // Tenant B's listing must not include A's key.
    let (_, list_b) = send(&app, Method::GET, "/api/api-keys", &token_b, None).await;
    assert_eq!(list_b.as_array().unwrap().len(), 0);

    // Tenant B cannot delete tenant A's key.
    let (status, _) = send(&app, Method::DELETE, &format!("/api/api-keys/{id_a}"), &token_b, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // But A can.
    let (status, _) = send(&app, Method::DELETE, &format!("/api/api-keys/{id_a}"), &token_a, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

// ── key-pool mappings ────────────────────────────────────────────────────────

#[tokio::test]
async fn create_key_pool_validates_provider_key_ownership() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;

    let (_, p1) = send(
        &app,
        Method::POST,
        "/api/providers",
        &token,
        Some(json!({"name": "p1", "display_name": "P1", "base_url": "https://x"})),
    )
    .await;
    let pid1 = p1["id"].as_str().unwrap().to_string();
    let (_, p2) = send(
        &app,
        Method::POST,
        "/api/providers",
        &token,
        Some(json!({"name": "p2", "display_name": "P2", "base_url": "https://y"})),
    )
    .await;
    let pid2 = p2["id"].as_str().unwrap().to_string();

    let (_, k2) = send(
        &app,
        Method::POST,
        &format!("/api/providers/{pid2}/keys"),
        &token,
        Some(json!({"plaintext_key": "sk-belongs-to-p2"})),
    )
    .await;
    let kid2 = k2["id"].as_str().unwrap().to_string();

    let (_, ak) = send(&app, Method::POST, "/api/api-keys", &token, Some(json!({}))).await;
    let api_key_id = ak["id"].as_str().unwrap().to_string();

    // Try to map p2's key to p1 — must 400.
    let (status, body) = send(
        &app,
        Method::POST,
        "/api/key-pools",
        &token,
        Some(json!({
            "api_key_id": api_key_id,
            "provider_id": pid1,
            "allowed_provider_key_ids": [kid2],
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "body: {body}");
    assert_eq!(body["error"], "invalid_field");

    // Correct mapping works.
    let (status, _) = send(
        &app,
        Method::POST,
        "/api/key-pools",
        &token,
        Some(json!({
            "api_key_id": api_key_id,
            "provider_id": pid2,
            "allowed_provider_key_ids": [kid2],
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Provider key now blocked from deletion.
    let (s_del, body) = send(
        &app,
        Method::DELETE,
        &format!("/api/providers/{pid2}/keys/{kid2}"),
        &token,
        None,
    )
    .await;
    assert_eq!(s_del, StatusCode::CONFLICT);
    assert_eq!(body["error"], "in_use");

    // Unmap then delete works.
    let (s_unmap, _) = send(
        &app,
        Method::DELETE,
        &format!("/api/key-pools/{api_key_id}/{pid2}"),
        &token,
        None,
    )
    .await;
    assert_eq!(s_unmap, StatusCode::NO_CONTENT);
    let (s_del2, _) = send(
        &app,
        Method::DELETE,
        &format!("/api/providers/{pid2}/keys/{kid2}"),
        &token,
        None,
    )
    .await;
    assert_eq!(s_del2, StatusCode::NO_CONTENT);
}

// ── vision mappings ──────────────────────────────────────────────────────────

#[tokio::test]
async fn vision_mapping_create_then_duplicate_409_then_delete() {
    let app = build_app(seed_two_tenants().await);
    let token = login_token(&app, ADMIN_A).await;
    let payload = json!({
        "model_name": "gpt-4o-vision",
        "vision_parser_alias": "vp",
        "generation_alias": "gen",
    });
    let (s1, _) = send(&app, Method::POST, "/api/vision-mappings", &token, Some(payload.clone())).await;
    assert_eq!(s1, StatusCode::CREATED);
    let (s2, body) = send(&app, Method::POST, "/api/vision-mappings", &token, Some(payload)).await;
    assert_eq!(s2, StatusCode::CONFLICT);
    assert_eq!(body["error"], "model_taken");

    let (s_del, _) = send(
        &app,
        Method::DELETE,
        "/api/vision-mappings/gpt-4o-vision",
        &token,
        None,
    )
    .await;
    assert_eq!(s_del, StatusCode::NO_CONTENT);
    let (s_del2, _) = send(
        &app,
        Method::DELETE,
        "/api/vision-mappings/gpt-4o-vision",
        &token,
        None,
    )
    .await;
    assert_eq!(s_del2, StatusCode::NOT_FOUND);
}
