use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{auth::TenantAdmin, state::AppState};

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub tenant_id: String,
    pub username: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ProviderView {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub enabled: i64,
    pub health_state: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TenantStats {
    pub total_requests: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub avg_latency_ms: f64,
    pub qps_last_hour: f64,
    pub p50_latency_ms_last_hour: f64,
    pub p95_latency_ms_last_hour: f64,
    pub error_rate_last_hour: f64,
    pub failover_count_last_hour: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct ProviderModelView {
    pub id: String,
    pub provider_id: String,
    pub model_name: String,
    pub enabled: i64,
    pub supports_vision: i64,
    pub supports_streaming: i64,
    pub supports_tools: i64,
    pub outbound_proxy_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetEnabledRequest {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub ok: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct AliasView {
    pub id: String,
    pub alias_name: String,
    pub description: Option<String>,
    pub route_strategy: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AliasTargetInput {
    pub provider_id: String,
    pub model_name: String,
    pub priority: i32,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAliasTargetsRequest {
    pub targets: Vec<AliasTargetInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRouteStrategyRequest {
    pub route_strategy: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct FailoverEventView {
    pub request_id: String,
    pub model_alias: String,
    pub provider_id: Option<String>,
    pub provider_model: Option<String>,
    pub failover_count: i64,
    pub error_code: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct FailoverEventQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct KeyPoolMappingView {
    pub api_key_id: String,
    pub provider_key_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateKeyPoolRequest {
    pub provider_key_ids: Vec<String>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct VisionMappingView {
    pub model_name: String,
    pub vision_parser_alias: Option<String>,
    pub generation_alias: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVisionMappingRequest {
    pub vision_parser_alias: Option<String>,
    pub generation_alias: Option<String>,
}

pub async fn me(admin: TenantAdmin) -> Json<MeResponse> {
    Json(MeResponse {
        tenant_id: admin.tenant_id.to_string(),
        username: admin.username,
    })
}

pub async fn list_providers(
    _admin: TenantAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<ProviderView>>, (axum::http::StatusCode, Json<crate::auth::ApiError>)> {
    let rows: Vec<ProviderView> = sqlx::query_as(
        "SELECT id, name, display_name, enabled, health_state FROM providers ORDER BY name",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(rows))
}

pub async fn set_provider_enabled(
    _admin: TenantAdmin,
    Path(provider_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<SetEnabledRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<crate::auth::ApiError>)> {
    sqlx::query("UPDATE providers SET enabled = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(if req.enabled { 1 } else { 0 })
        .bind(provider_id)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;

    Ok(Json(ActionResponse { ok: true }))
}

pub async fn list_provider_models(
    _admin: TenantAdmin,
    Path(provider_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ProviderModelView>>, (StatusCode, Json<crate::auth::ApiError>)> {
    let rows: Vec<ProviderModelView> = sqlx::query_as(
        "SELECT id, provider_id, model_name, enabled, supports_vision, supports_streaming, supports_tools, outbound_proxy_id \
         FROM provider_models WHERE provider_id = ? ORDER BY model_name",
    )
    .bind(provider_id)
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(rows))
}

pub async fn set_provider_model_enabled(
    _admin: TenantAdmin,
    Path((provider_id, model_name)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(req): Json<SetEnabledRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<crate::auth::ApiError>)> {
    sqlx::query(
        "UPDATE provider_models SET enabled = ?, updated_at = datetime('now') \
         WHERE provider_id = ? AND model_name = ?",
    )
    .bind(if req.enabled { 1 } else { 0 })
    .bind(provider_id)
    .bind(model_name)
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(ActionResponse { ok: true }))
}

pub async fn list_aliases(
    _admin: TenantAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<AliasView>>, (StatusCode, Json<crate::auth::ApiError>)> {
    let rows: Vec<AliasView> = sqlx::query_as(
        "SELECT id, alias_name, description, route_strategy FROM model_aliases ORDER BY alias_name",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(rows))
}

pub async fn update_alias_route_strategy(
    _admin: TenantAdmin,
    Path(alias_name): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateRouteStrategyRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<crate::auth::ApiError>)> {
    if !matches!(req.route_strategy.as_str(), "priority" | "latency" | "cost") {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::auth::ApiError {
                error: "invalid_route_strategy".to_string(),
                message: "route_strategy must be one of: priority, latency, cost".to_string(),
            }),
        ));
    }

    sqlx::query(
        "UPDATE model_aliases SET route_strategy = ?, updated_at = datetime('now') WHERE alias_name = ?",
    )
    .bind(req.route_strategy)
    .bind(alias_name)
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(ActionResponse { ok: true }))
}

pub async fn update_alias_targets(
    _admin: TenantAdmin,
    Path(alias_name): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateAliasTargetsRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<crate::auth::ApiError>)> {
    let alias_id: Option<(String,)> = sqlx::query_as("SELECT id FROM model_aliases WHERE alias_name = ?")
        .bind(&alias_name)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;

    let Some((alias_id,)) = alias_id else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(crate::auth::ApiError {
                error: "alias_not_found".to_string(),
                message: "alias not found".to_string(),
            }),
        ));
    };

    sqlx::query("DELETE FROM model_alias_targets WHERE alias_id = ?")
        .bind(&alias_id)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;

    for t in req.targets {
        sqlx::query(
            "INSERT INTO model_alias_targets (id, alias_id, provider_id, model_name, priority, enabled, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&alias_id)
        .bind(t.provider_id)
        .bind(t.model_name)
        .bind(t.priority)
        .bind(if t.enabled { 1 } else { 0 })
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    }

    Ok(Json(ActionResponse { ok: true }))
}

pub async fn list_key_pool_mappings(
    admin: TenantAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<KeyPoolMappingView>>, (StatusCode, Json<crate::auth::ApiError>)> {
    let rows: Vec<KeyPoolMappingView> = sqlx::query_as(
        "SELECT m.api_key_id, m.provider_key_id
         FROM api_key_provider_key_pools m
         JOIN api_keys ak ON ak.id = m.api_key_id
         WHERE ak.tenant_id = ?",
    )
    .bind(admin.tenant_id.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(rows))
}

pub async fn update_key_pool_mapping(
    admin: TenantAdmin,
    Path(api_key_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateKeyPoolRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<crate::auth::ApiError>)> {
    let owned: Option<(String,)> = sqlx::query_as("SELECT id FROM api_keys WHERE id = ? AND tenant_id = ?")
        .bind(&api_key_id)
        .bind(admin.tenant_id.to_string())
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;

    if owned.is_none() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(crate::auth::ApiError {
                error: "forbidden".to_string(),
                message: "api key not in current tenant".to_string(),
            }),
        ));
    }

    sqlx::query("DELETE FROM api_key_provider_key_pools WHERE api_key_id = ?")
        .bind(&api_key_id)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;

    for provider_key_id in req.provider_key_ids {
        sqlx::query(
            "INSERT INTO api_key_provider_key_pools (id, tenant_id, api_key_id, provider_key_id, created_at) \
             VALUES (?, ?, ?, ?, datetime('now'))",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(admin.tenant_id.to_string())
        .bind(&api_key_id)
        .bind(provider_key_id)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    }

    Ok(Json(ActionResponse { ok: true }))
}

pub async fn list_failover_events(
    admin: TenantAdmin,
    Query(q): Query<FailoverEventQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<FailoverEventView>>, (StatusCode, Json<crate::auth::ApiError>)> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows: Vec<FailoverEventView> = sqlx::query_as(
        "SELECT request_id, model_alias, provider_id, provider_model, failover_count, error_code, created_at
         FROM request_logs
         WHERE tenant_id = ? AND failover_count > 0
         ORDER BY created_at DESC
         LIMIT ?",
    )
    .bind(admin.tenant_id.to_string())
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(rows))
}

pub async fn list_vision_mappings(
    _admin: TenantAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<VisionMappingView>>, (StatusCode, Json<crate::auth::ApiError>)> {
    let rows: Vec<VisionMappingView> = sqlx::query_as(
        "SELECT model_name, vision_parser_alias, generation_alias FROM model_vision_mappings ORDER BY model_name",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(rows))
}

pub async fn update_vision_mapping(
    _admin: TenantAdmin,
    Path(model_name): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateVisionMappingRequest>,
) -> Result<Json<ActionResponse>, (StatusCode, Json<crate::auth::ApiError>)> {
    sqlx::query(
        "INSERT INTO model_vision_mappings (id, model_name, vision_parser_alias, generation_alias, created_at, updated_at)
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))
         ON CONFLICT(model_name) DO UPDATE SET
             vision_parser_alias = excluded.vision_parser_alias,
             generation_alias = excluded.generation_alias,
             updated_at = datetime('now')",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(model_name)
    .bind(req.vision_parser_alias)
    .bind(req.generation_alias)
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(Json(ActionResponse { ok: true }))
}

pub async fn tenant_stats(
    admin: TenantAdmin,
    State(state): State<AppState>,
) -> Result<Json<TenantStats>, (axum::http::StatusCode, Json<crate::auth::ApiError>)> {
    let row: Option<(i64, i64, i64, f64)> = sqlx::query_as(
        "SELECT
            COUNT(*) AS total_requests,
            COALESCE(SUM(input_tokens), 0) AS total_input_tokens,
            COALESCE(SUM(output_tokens), 0) AS total_output_tokens,
            COALESCE(AVG(latency_ms), 0) AS avg_latency_ms
         FROM request_logs
         WHERE tenant_id = ?",
    )
    .bind(admin.tenant_id.to_string())
    .fetch_optional(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    let latencies: Vec<(i64,)> = sqlx::query_as(
        "SELECT latency_ms FROM request_logs
         WHERE tenant_id = ? AND created_at >= datetime('now', '-1 hour')
         ORDER BY latency_ms ASC",
    )
    .bind(admin.tenant_id.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    let counts: Option<(i64, i64, i64)> = sqlx::query_as(
        "SELECT
            COUNT(*) AS total,
            COALESCE(SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END), 0) AS errors,
            COALESCE(SUM(failover_count), 0) AS failovers
         FROM request_logs
         WHERE tenant_id = ? AND created_at >= datetime('now', '-1 hour')",
    )
    .bind(admin.tenant_id.to_string())
    .fetch_optional(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    let p50 = percentile_from_sorted(&latencies, 0.50);
    let p95 = percentile_from_sorted(&latencies, 0.95);
    let (last_hour_total, last_hour_errors, last_hour_failovers) = counts.unwrap_or((0, 0, 0));
    let qps_last_hour = (last_hour_total as f64) / 3600.0;
    let error_rate_last_hour = if last_hour_total > 0 {
        (last_hour_errors as f64) / (last_hour_total as f64)
    } else {
        0.0
    };

    let (total_requests, total_input_tokens, total_output_tokens, avg_latency_ms) =
        row.unwrap_or((0, 0, 0, 0.0));

    Ok(Json(TenantStats {
        total_requests,
        total_input_tokens,
        total_output_tokens,
        avg_latency_ms,
        qps_last_hour,
        p50_latency_ms_last_hour: p50,
        p95_latency_ms_last_hour: p95,
        error_rate_last_hour,
        failover_count_last_hour: last_hour_failovers,
    }))
}

fn percentile_from_sorted(values: &[(i64,)], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let n = values.len();
    let rank = ((n as f64 - 1.0) * p).round() as usize;
    values[rank.min(n - 1)].0 as f64
}
