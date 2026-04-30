use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
    Json,
};
use argon2::PasswordVerifier;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub tenant_id: String,
    pub username: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub tenant_id: String,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct TenantAdmin {
    pub tenant_id: Uuid,
    pub username: String,
}

#[derive(Debug, FromRow)]
struct TenantAdminRow {
    admin_id: String,
    tenant_id: String,
    username: String,
    password_hash: String,
}

pub async fn login(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ApiError>)> {
    let row: Option<TenantAdminRow> = sqlx::query_as(
        "SELECT ta.id AS admin_id, ta.tenant_id, ta.username, ta.password_hash \
         FROM tenant_admins ta JOIN tenants t ON t.id = ta.tenant_id \
         WHERE ta.username = ? AND ta.status = 'active' AND t.status = 'active'",
    )
    .bind(&req.username)
    .fetch_optional(&state.pool)
    .await
    .map_err(internal_error)?;

    let Some(row) = row else {
        return Err(unauthorized("invalid_credentials", "invalid username or password"));
    };

    let verify_ok = argon2::password_hash::PasswordHash::new(&row.password_hash)
        .ok()
        .and_then(|parsed| {
            argon2::Argon2::default()
                .verify_password(req.password.as_bytes(), &parsed)
                .ok()
        })
        .is_some();

    if !verify_ok {
        return Err(unauthorized("invalid_credentials", "invalid username or password"));
    }

    let now = Utc::now();
    let exp = now + Duration::seconds(state.auth.jwt_expiry_secs as i64);

    let claims = JwtClaims {
        sub: row.admin_id,
        tenant_id: row.tenant_id.clone(),
        username: row.username.clone(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.auth.jwt_secret.as_bytes()),
    )
    .map_err(internal_error)?;

    Ok(Json(LoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: state.auth.jwt_expiry_secs,
        tenant_id: row.tenant_id,
        username: row.username,
    }))
}

impl<S> FromRequestParts<S> for TenantAdmin
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);

        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| unauthorized("missing_token", "missing Authorization header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| unauthorized("invalid_token", "invalid bearer token"))?;

        let token_data = decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(state.auth.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| unauthorized("invalid_token", "token validation failed"))?;

        let tenant_id = Uuid::parse_str(&token_data.claims.tenant_id)
            .map_err(|_| unauthorized("invalid_token", "invalid tenant id in token"))?;

        Ok(TenantAdmin {
            tenant_id,
            username: token_data.claims.username,
        })
    }
}

fn unauthorized(code: &str, message: &str) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ApiError {
            error: code.to_string(),
            message: message.to_string(),
        }),
    )
}

pub fn internal_error<E: std::fmt::Display>(e: E) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            error: "internal_error".to_string(),
            message: e.to_string(),
        }),
    )
}
