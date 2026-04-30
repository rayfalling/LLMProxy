//! First-boot setup: create the initial tenant + tenant-admin while the
//! database is empty. After the first successful call this endpoint is
//! permanently locked (returns 409).

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use axum::{extract::State, http::StatusCode, Json};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::{internal_error, ApiError};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SetupRequest {
    pub tenant_name: String,
    pub username: String,
    pub password: String,
    pub password_confirm: String,
}

#[derive(Debug, Serialize)]
pub struct SetupResponse {
    pub success: bool,
    pub tenant_id: String,
    pub admin_id: String,
}

#[derive(Debug, Serialize)]
pub struct SetupStatusResponse {
    pub initialized: bool,
}

/// Returns true when at least one tenant row exists.
pub async fn is_initialized(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tenants")
        .fetch_one(pool)
        .await?;
    Ok(row.0 > 0)
}

/// `GET /api/setup/status` — public, no JWT required.
pub async fn setup_status(
    State(state): State<AppState>,
) -> Result<Json<SetupStatusResponse>, (StatusCode, Json<ApiError>)> {
    let initialized = is_initialized(&state.pool)
        .await
        .map_err(internal_error)?;
    Ok(Json(SetupStatusResponse { initialized }))
}

/// `POST /api/setup` — public, no JWT required. Idempotent-once: succeeds
/// while the database has zero tenants, returns 409 afterwards.
pub async fn setup(
    State(state): State<AppState>,
    Json(req): Json<SetupRequest>,
) -> Result<Json<SetupResponse>, (StatusCode, Json<ApiError>)> {
    // ── input validation ────────────────────────────────────────────────────
    let tenant_name = req.tenant_name.trim();
    let username = req.username.trim();

    if tenant_name.is_empty() {
        return Err(bad_request("invalid_tenant_name", "tenant_name must not be empty"));
    }
    if username.is_empty() {
        return Err(bad_request("invalid_username", "username must not be empty"));
    }
    if req.password.len() < 8 {
        return Err(bad_request(
            "invalid_password",
            "password must be at least 8 characters",
        ));
    }
    if req.password != req.password_confirm {
        return Err(bad_request(
            "password_mismatch",
            "password and password_confirm must match",
        ));
    }

    // ── transactional insert: re-check inside the transaction so concurrent
    //    setup calls cannot both win ──────────────────────────────────────
    let mut tx = state.pool.begin().await.map_err(internal_error)?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tenants")
        .fetch_one(&mut *tx)
        .await
        .map_err(internal_error)?;

    if count.0 > 0 {
        return Err((
            StatusCode::CONFLICT,
            Json(ApiError {
                error: "already_initialized".to_string(),
                message: "setup has already been completed".to_string(),
            }),
        ));
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(|e| internal_error(format!("hash error: {e}")))?
        .to_string();

    let tenant_id = Uuid::new_v4().to_string();
    let admin_id = Uuid::new_v4().to_string();

    sqlx::query(
        "INSERT INTO tenants (id, name, status, created_at, updated_at) \
         VALUES (?, ?, 'active', datetime('now'), datetime('now'))",
    )
    .bind(&tenant_id)
    .bind(tenant_name)
    .execute(&mut *tx)
    .await
    .map_err(internal_error)?;

    sqlx::query(
        "INSERT INTO tenant_admins (id, tenant_id, username, password_hash, status) \
         VALUES (?, ?, ?, ?, 'active')",
    )
    .bind(&admin_id)
    .bind(&tenant_id)
    .bind(username)
    .bind(&password_hash)
    .execute(&mut *tx)
    .await
    .map_err(internal_error)?;

    tx.commit().await.map_err(internal_error)?;

    Ok(Json(SetupResponse {
        success: true,
        tenant_id,
        admin_id,
    }))
}

fn bad_request(code: &str, message: &str) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            error: code.to_string(),
            message: message.to_string(),
        }),
    )
}
