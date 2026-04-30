use axum::{
    extract::{FromRef, FromRequestParts},
    http::{header, request::Parts, StatusCode},
    Json,
};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ApiKeyContext {
    pub tenant_id: Uuid,
    pub api_key_id: Uuid,
    pub quota_rpm: Option<i64>,
    pub quota_tpm: Option<i64>,
    pub quota_daily_req: Option<i64>,
    pub masked_key: String,
}

#[derive(Debug, FromRow)]
struct ApiKeyRow {
    id: String,
    tenant_id: String,
    quota_rpm: Option<i64>,
    quota_tpm: Option<i64>,
    quota_daily_req: Option<i64>,
}

impl<S> FromRequestParts<S> for ApiKeyContext
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = AppState::from_ref(state);
        let raw_key = extract_api_key(parts)
            .ok_or_else(|| unauthorized("missing_api_key", "missing API key in Authorization or x-api-key"))?;

        let row: Option<ApiKeyRow> = sqlx::query_as(
            "SELECT id, tenant_id, quota_rpm, quota_tpm, quota_daily_req \
             FROM api_keys WHERE hashed_key = ? AND status = 'active'",
        )
        .bind(raw_key)
        .fetch_optional(&state.pool)
        .await
        .map_err(internal_error)?;

        let Some(row) = row else {
            return Err(unauthorized("invalid_api_key", "API key not found or inactive"));
        };

        let tenant_id = Uuid::parse_str(&row.tenant_id)
            .map_err(|_| unauthorized("invalid_api_key", "invalid tenant id in db"))?;
        let api_key_id = Uuid::parse_str(&row.id)
            .map_err(|_| unauthorized("invalid_api_key", "invalid api key id in db"))?;

        // 请求前配额校验（基于 request_logs 的滚动统计）
        if let Some(limit) = row.quota_rpm {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) AS c FROM request_logs \
                 WHERE api_key_id = ? AND created_at >= datetime('now', '-1 minute')",
            )
            .bind(&row.id)
            .fetch_one(&state.pool)
            .await
            .map_err(internal_error)?;
            if count >= limit {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ApiError {
                        error: "rate_limit_exceeded".to_string(),
                        message: "requests per minute exceeded".to_string(),
                    }),
                ));
            }
        }

        if let Some(limit) = row.quota_tpm {
            let (tokens,): (i64,) = sqlx::query_as(
                "SELECT COALESCE(SUM(input_tokens + output_tokens), 0) AS t FROM request_logs \
                 WHERE api_key_id = ? AND created_at >= datetime('now', '-1 minute')",
            )
            .bind(&row.id)
            .fetch_one(&state.pool)
            .await
            .map_err(internal_error)?;
            if tokens >= limit {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ApiError {
                        error: "token_rate_limit_exceeded".to_string(),
                        message: "tokens per minute exceeded".to_string(),
                    }),
                ));
            }
        }

        if let Some(limit) = row.quota_daily_req {
            let (count,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) AS c FROM request_logs \
                 WHERE api_key_id = ? AND date(created_at) = date('now')",
            )
            .bind(&row.id)
            .fetch_one(&state.pool)
            .await
            .map_err(internal_error)?;
            if count >= limit {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ApiError {
                        error: "daily_quota_exceeded".to_string(),
                        message: "daily request quota exceeded".to_string(),
                    }),
                ));
            }
        }

        let masked_key = mask_api_key(raw_key);

        Ok(ApiKeyContext {
            tenant_id,
            api_key_id,
            quota_rpm: row.quota_rpm,
            quota_tpm: row.quota_tpm,
            quota_daily_req: row.quota_daily_req,
            masked_key,
        })
    }
}

fn extract_api_key(parts: &Parts) -> Option<&str> {
    if let Some(v) = parts.headers.get("x-api-key").and_then(|h| h.to_str().ok()) {
        if !v.trim().is_empty() {
            return Some(v.trim());
        }
    }

    parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|s| !s.is_empty())
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

fn mask_api_key(raw: &str) -> String {
    if raw.len() <= 8 {
        return "***".to_string();
    }
    format!("{}***{}", &raw[..4], &raw[raw.len() - 4..])
}
