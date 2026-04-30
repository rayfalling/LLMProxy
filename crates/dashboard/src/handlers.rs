use axum::{extract::{Path, Query, State}, http::StatusCode, Json};
use rand_core::{OsRng, RngCore};
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
            CAST(COALESCE(AVG(latency_ms), 0.0) AS REAL) AS avg_latency_ms
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

// ──────────────────────────────────────────────────────────────────────────
// CRUD: providers (global)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub display_name: String,
    pub base_url: String,
    pub auth_mode: Option<String>,
    pub auth_header: Option<String>,
}

fn bad_request(error: &str, message: &str, field: Option<&str>) -> (StatusCode, Json<crate::auth::ApiError>) {
    let msg = match field {
        Some(f) => format!("{message} (field: {f})"),
        None => message.to_string(),
    };
    (
        StatusCode::BAD_REQUEST,
        Json(crate::auth::ApiError { error: error.to_string(), message: msg }),
    )
}

fn conflict(error: &str, message: &str) -> (StatusCode, Json<crate::auth::ApiError>) {
    (
        StatusCode::CONFLICT,
        Json(crate::auth::ApiError { error: error.to_string(), message: message.to_string() }),
    )
}

fn not_found(error: &str, message: &str) -> (StatusCode, Json<crate::auth::ApiError>) {
    (
        StatusCode::NOT_FOUND,
        Json(crate::auth::ApiError { error: error.to_string(), message: message.to_string() }),
    )
}

fn valid_provider_name(s: &str) -> bool {
    let len = s.len();
    if !(2..=32).contains(&len) {
        return false;
    }
    let mut chars = s.chars();
    let Some(first) = chars.next() else { return false };
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn mask_secret(s: &str) -> String {
    let n = s.chars().count();
    if n <= 8 {
        return "*".repeat(n);
    }
    let head: String = s.chars().take(4).collect();
    let tail: String = s.chars().skip(n.saturating_sub(4)).collect();
    format!("{head}...{tail}")
}

fn generate_api_key() -> (String, String) {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let plaintext = format!("lp_{hex}");
    let prefix: String = plaintext.chars().take(10).collect();
    (plaintext, prefix)
}

pub async fn create_provider(
    _admin: TenantAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateProviderRequest>,
) -> Result<(StatusCode, Json<ProviderView>), (StatusCode, Json<crate::auth::ApiError>)> {
    if !valid_provider_name(&req.name) {
        return Err(bad_request(
            "invalid_field",
            "name must be 2-32 chars, start with a lowercase letter, contain only lowercase letters, digits, '_' or '-'",
            Some("name"),
        ));
    }
    if req.display_name.trim().is_empty() {
        return Err(bad_request("invalid_field", "display_name must not be empty", Some("display_name")));
    }
    if req.base_url.trim().is_empty() {
        return Err(bad_request("invalid_field", "base_url must not be empty", Some("base_url")));
    }
    let auth_mode = req.auth_mode.unwrap_or_else(|| "bearer".to_string());
    if !matches!(auth_mode.as_str(), "bearer" | "api-key-header") {
        return Err(bad_request("invalid_field", "auth_mode must be 'bearer' or 'api-key-header'", Some("auth_mode")));
    }
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM providers WHERE name = ?")
        .bind(&req.name)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if exists.is_some() {
        return Err(conflict("name_taken", "a provider with this name already exists"));
    }
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO providers (id, name, display_name, base_url, auth_mode, auth_header, enabled, health_state, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, 1, 'unknown', datetime('now'), datetime('now'))",
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.display_name)
    .bind(&req.base_url)
    .bind(&auth_mode)
    .bind(req.auth_header.as_deref())
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    Ok((
        StatusCode::CREATED,
        Json(ProviderView {
            id,
            name: req.name,
            display_name: req.display_name,
            enabled: 1,
            health_state: "unknown".to_string(),
        }),
    ))
}

pub async fn delete_provider(
    _admin: TenantAdmin,
    Path(provider_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<crate::auth::ApiError>)> {
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM providers WHERE id = ?")
        .bind(&provider_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if exists.is_none() {
        return Err(not_found("not_found", "provider not found"));
    }
    let referencing: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT ma.alias_name FROM model_alias_targets t \
         JOIN model_aliases ma ON ma.id = t.alias_id WHERE t.provider_id = ?",
    )
    .bind(&provider_id)
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    if !referencing.is_empty() {
        let aliases: Vec<String> = referencing.into_iter().map(|(n,)| n).collect();
        return Err(conflict("in_use", &format!("provider is referenced by aliases: {}", aliases.join(", "))));
    }
    let pooled: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM api_key_provider_key_pools \
         WHERE provider_key_id IN (SELECT id FROM provider_keys WHERE provider_id = ?)",
    )
    .bind(&provider_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    if let Some((n,)) = pooled {
        if n > 0 {
            return Err(conflict("in_use", "provider keys are referenced by tenant key-pool mappings; remove them first"));
        }
    }
    sqlx::query("DELETE FROM provider_keys WHERE provider_id = ?")
        .bind(&provider_id)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    sqlx::query("DELETE FROM provider_models WHERE provider_id = ?")
        .bind(&provider_id)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(&provider_id)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// ──────────────────────────────────────────────────────────────────────────
// CRUD: provider keys (global)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProviderKeyRequest {
    pub label: Option<String>,
    pub plaintext_key: String,
    pub priority: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ProviderKeyCreatedResponse {
    pub id: String,
    pub provider_id: String,
    pub label: Option<String>,
    pub priority: i64,
    pub enabled: i64,
}

#[derive(Debug, Serialize)]
pub struct ProviderKeyView {
    pub id: String,
    pub provider_id: String,
    pub label: Option<String>,
    pub enabled: i64,
    pub priority: i64,
    pub key_preview: String,
}

pub async fn list_provider_keys(
    _admin: TenantAdmin,
    Path(provider_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<ProviderKeyView>>, (StatusCode, Json<crate::auth::ApiError>)> {
    let rows: Vec<(String, String, Option<String>, i64, i64, String)> = sqlx::query_as(
        "SELECT id, provider_id, label, enabled, priority, key_ref FROM provider_keys \
         WHERE provider_id = ? ORDER BY priority, id",
    )
    .bind(&provider_id)
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    let out = rows
        .into_iter()
        .map(|(id, provider_id, label, enabled, priority, key_ref)| ProviderKeyView {
            id,
            provider_id,
            label,
            enabled,
            priority,
            key_preview: mask_secret(&key_ref),
        })
        .collect();
    Ok(Json(out))
}

pub async fn create_provider_key(
    _admin: TenantAdmin,
    Path(provider_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<CreateProviderKeyRequest>,
) -> Result<(StatusCode, Json<ProviderKeyCreatedResponse>), (StatusCode, Json<crate::auth::ApiError>)> {
    if req.plaintext_key.trim().is_empty() {
        return Err(bad_request("invalid_field", "plaintext_key must not be empty", Some("plaintext_key")));
    }
    let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM providers WHERE id = ?")
        .bind(&provider_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if exists.is_none() {
        return Err(not_found("not_found", "provider not found"));
    }
    let id = Uuid::new_v4().to_string();
    let priority = req.priority.unwrap_or(0);
    sqlx::query(
        "INSERT INTO provider_keys (id, provider_id, key_ref, label, enabled, priority, created_at) \
         VALUES (?, ?, ?, ?, 1, ?, datetime('now'))",
    )
    .bind(&id)
    .bind(&provider_id)
    .bind(&req.plaintext_key)
    .bind(req.label.as_deref())
    .bind(priority)
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    Ok((
        StatusCode::CREATED,
        Json(ProviderKeyCreatedResponse {
            id,
            provider_id,
            label: req.label,
            priority,
            enabled: 1,
        }),
    ))
}

pub async fn delete_provider_key(
    _admin: TenantAdmin,
    Path((provider_id, key_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<crate::auth::ApiError>)> {
    let referenced: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM api_key_provider_key_pools WHERE provider_key_id = ?",
    )
    .bind(&key_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    if let Some((n,)) = referenced {
        if n > 0 {
            return Err(conflict("in_use", "provider key is referenced by a key-pool mapping"));
        }
    }
    let res = sqlx::query("DELETE FROM provider_keys WHERE id = ? AND provider_id = ?")
        .bind(&key_id)
        .bind(&provider_id)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if res.rows_affected() == 0 {
        return Err(not_found("not_found", "provider key not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ──────────────────────────────────────────────────────────────────────────
// CRUD: provider models (global)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateProviderModelRequest {
    pub model_name: String,
    pub supports_vision: Option<bool>,
    pub supports_streaming: Option<bool>,
    pub supports_tools: Option<bool>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
}

pub async fn create_provider_model(
    _admin: TenantAdmin,
    Path(provider_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<CreateProviderModelRequest>,
) -> Result<(StatusCode, Json<ProviderModelView>), (StatusCode, Json<crate::auth::ApiError>)> {
    if req.model_name.trim().is_empty() {
        return Err(bad_request("invalid_field", "model_name must not be empty", Some("model_name")));
    }
    let provider_exists: Option<(String,)> = sqlx::query_as("SELECT id FROM providers WHERE id = ?")
        .bind(&provider_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if provider_exists.is_none() {
        return Err(not_found("not_found", "provider not found"));
    }
    let dup: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM provider_models WHERE provider_id = ? AND model_name = ?",
    )
    .bind(&provider_id)
    .bind(&req.model_name)
    .fetch_optional(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    if dup.is_some() {
        return Err(conflict("model_taken", "this model is already registered for the provider"));
    }
    let id = Uuid::new_v4().to_string();
    let supports_vision = req.supports_vision.unwrap_or(false);
    let supports_streaming = req.supports_streaming.unwrap_or(true);
    let supports_tools = req.supports_tools.unwrap_or(true);
    sqlx::query(
        "INSERT INTO provider_models (id, provider_id, model_name, enabled, supports_vision, supports_streaming, supports_tools, context_window, max_output_tokens, created_at, updated_at) \
         VALUES (?, ?, ?, 1, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(&id)
    .bind(&provider_id)
    .bind(&req.model_name)
    .bind(supports_vision as i64)
    .bind(supports_streaming as i64)
    .bind(supports_tools as i64)
    .bind(req.context_window)
    .bind(req.max_output_tokens)
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    Ok((
        StatusCode::CREATED,
        Json(ProviderModelView {
            id,
            provider_id,
            model_name: req.model_name,
            enabled: 1,
            supports_vision: supports_vision as i64,
            supports_streaming: supports_streaming as i64,
            supports_tools: supports_tools as i64,
            outbound_proxy_id: None,
        }),
    ))
}

pub async fn delete_provider_model(
    _admin: TenantAdmin,
    Path((provider_id, model_name)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<crate::auth::ApiError>)> {
    let referencing: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT ma.alias_name FROM model_alias_targets t \
         JOIN model_aliases ma ON ma.id = t.alias_id WHERE t.provider_id = ? AND t.model_name = ?",
    )
    .bind(&provider_id)
    .bind(&model_name)
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    if !referencing.is_empty() {
        let aliases: Vec<String> = referencing.into_iter().map(|(n,)| n).collect();
        return Err(conflict("in_use", &format!("provider model is referenced by aliases: {}", aliases.join(", "))));
    }
    let res = sqlx::query("DELETE FROM provider_models WHERE provider_id = ? AND model_name = ?")
        .bind(&provider_id)
        .bind(&model_name)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if res.rows_affected() == 0 {
        return Err(not_found("not_found", "provider model not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ──────────────────────────────────────────────────────────────────────────
// CRUD: model aliases (global)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateAliasRequest {
    pub alias_name: String,
    pub description: Option<String>,
    pub route_strategy: Option<String>,
    #[serde(default)]
    pub targets: Vec<AliasTargetInput>,
}

#[derive(Debug, Serialize)]
pub struct AliasCreatedResponse {
    pub id: String,
    pub alias_name: String,
    pub description: Option<String>,
    pub route_strategy: String,
    pub targets_count: usize,
}

pub async fn create_alias(
    _admin: TenantAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateAliasRequest>,
) -> Result<(StatusCode, Json<AliasCreatedResponse>), (StatusCode, Json<crate::auth::ApiError>)> {
    if req.alias_name.trim().is_empty() {
        return Err(bad_request("invalid_field", "alias_name must not be empty", Some("alias_name")));
    }
    let route_strategy = req.route_strategy.unwrap_or_else(|| "priority".to_string());
    if !matches!(route_strategy.as_str(), "priority" | "latency" | "cost") {
        return Err(bad_request("invalid_field", "route_strategy must be one of: priority, latency, cost", Some("route_strategy")));
    }
    for t in &req.targets {
        let exists: Option<(String,)> = sqlx::query_as("SELECT id FROM providers WHERE id = ?")
            .bind(&t.provider_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(crate::auth::internal_error)?;
        if exists.is_none() {
            return Err(bad_request(
                "invalid_field",
                &format!("target provider_id '{}' does not exist", t.provider_id),
                Some("targets"),
            ));
        }
    }
    let dup: Option<(String,)> = sqlx::query_as("SELECT id FROM model_aliases WHERE alias_name = ?")
        .bind(&req.alias_name)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if dup.is_some() {
        return Err(conflict("alias_taken", "an alias with this name already exists"));
    }
    let mut tx = state.pool.begin().await.map_err(crate::auth::internal_error)?;
    let alias_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO model_aliases (id, alias_name, description, route_strategy, created_at, updated_at) \
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(&alias_id)
    .bind(&req.alias_name)
    .bind(req.description.as_deref())
    .bind(&route_strategy)
    .execute(&mut *tx)
    .await
    .map_err(crate::auth::internal_error)?;
    let targets_count = req.targets.len();
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
        .bind(if t.enabled { 1_i64 } else { 0_i64 })
        .execute(&mut *tx)
        .await
        .map_err(crate::auth::internal_error)?;
    }
    tx.commit().await.map_err(crate::auth::internal_error)?;
    Ok((
        StatusCode::CREATED,
        Json(AliasCreatedResponse {
            id: alias_id,
            alias_name: req.alias_name,
            description: req.description,
            route_strategy,
            targets_count,
        }),
    ))
}

pub async fn delete_alias(
    _admin: TenantAdmin,
    Path(alias_name): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<crate::auth::ApiError>)> {
    let row: Option<(String,)> = sqlx::query_as("SELECT id FROM model_aliases WHERE alias_name = ?")
        .bind(&alias_name)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    let Some((alias_id,)) = row else {
        return Err(not_found("not_found", "alias not found"));
    };
    let mut tx = state.pool.begin().await.map_err(crate::auth::internal_error)?;
    sqlx::query("DELETE FROM model_alias_targets WHERE alias_id = ?")
        .bind(&alias_id)
        .execute(&mut *tx)
        .await
        .map_err(crate::auth::internal_error)?;
    sqlx::query("DELETE FROM failover_rules WHERE alias_id = ?")
        .bind(&alias_id)
        .execute(&mut *tx)
        .await
        .map_err(crate::auth::internal_error)?;
    sqlx::query("DELETE FROM model_aliases WHERE id = ?")
        .bind(&alias_id)
        .execute(&mut *tx)
        .await
        .map_err(crate::auth::internal_error)?;
    tx.commit().await.map_err(crate::auth::internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

// ──────────────────────────────────────────────────────────────────────────
// CRUD: tenant API keys (tenant-scoped)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: Option<String>,
    pub quota_rpm: Option<i64>,
    pub quota_tpm: Option<i64>,
    pub quota_daily_req: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyCreatedResponse {
    pub id: String,
    pub name: Option<String>,
    pub plaintext_key: String,
    pub prefix: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyView {
    pub id: String,
    pub name: Option<String>,
    pub prefix: String,
    pub status: String,
    pub quota_rpm: Option<i64>,
    pub quota_tpm: Option<i64>,
    pub quota_daily_req: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_api_keys(
    admin: TenantAdmin,
    State(state): State<AppState>,
) -> Result<Json<Vec<ApiKeyView>>, (StatusCode, Json<crate::auth::ApiError>)> {
    let rows: Vec<(String, Option<String>, String, String, Option<i64>, Option<i64>, Option<i64>, String, String)> =
        sqlx::query_as(
            "SELECT id, name, hashed_key, status, quota_rpm, quota_tpm, quota_daily_req, created_at, updated_at \
             FROM api_keys WHERE tenant_id = ? ORDER BY created_at DESC",
        )
        .bind(admin.tenant_id.to_string())
        .fetch_all(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    let out = rows
        .into_iter()
        .map(|(id, name, hashed_key, status, qrpm, qtpm, qdaily, created_at, updated_at)| ApiKeyView {
            id,
            name,
            prefix: hashed_key.chars().take(10).collect(),
            status,
            quota_rpm: qrpm,
            quota_tpm: qtpm,
            quota_daily_req: qdaily,
            created_at,
            updated_at,
        })
        .collect();
    Ok(Json(out))
}

pub async fn create_api_key(
    admin: TenantAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<(StatusCode, Json<ApiKeyCreatedResponse>), (StatusCode, Json<crate::auth::ApiError>)> {
    let (plaintext, prefix) = generate_api_key();
    let id = Uuid::new_v4().to_string();
    // NB: column is named `hashed_key` but the proxy compares the raw bearer token verbatim,
    // so we store plaintext here. Future migration should hash on both sides.
    sqlx::query(
        "INSERT INTO api_keys (id, tenant_id, hashed_key, name, status, quota_rpm, quota_tpm, quota_daily_req, created_at, updated_at) \
         VALUES (?, ?, ?, ?, 'active', ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(&id)
    .bind(admin.tenant_id.to_string())
    .bind(&plaintext)
    .bind(req.name.as_deref())
    .bind(req.quota_rpm)
    .bind(req.quota_tpm)
    .bind(req.quota_daily_req)
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    let created_at: (String,) = sqlx::query_as("SELECT created_at FROM api_keys WHERE id = ?")
        .bind(&id)
        .fetch_one(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    Ok((
        StatusCode::CREATED,
        Json(ApiKeyCreatedResponse {
            id,
            name: req.name,
            plaintext_key: plaintext,
            prefix,
            status: "active".to_string(),
            created_at: created_at.0,
        }),
    ))
}

pub async fn delete_api_key(
    admin: TenantAdmin,
    Path(api_key_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<crate::auth::ApiError>)> {
    let res = sqlx::query(
        "UPDATE api_keys SET status = 'revoked', updated_at = datetime('now') \
         WHERE id = ? AND tenant_id = ?",
    )
    .bind(&api_key_id)
    .bind(admin.tenant_id.to_string())
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    if res.rows_affected() == 0 {
        return Err(not_found("not_found", "api key not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ──────────────────────────────────────────────────────────────────────────
// CRUD: key-pool mappings (tenant-scoped)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateKeyPoolRequest {
    pub api_key_id: String,
    pub provider_id: String,
    pub allowed_provider_key_ids: Vec<String>,
}

pub async fn create_key_pool(
    admin: TenantAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateKeyPoolRequest>,
) -> Result<StatusCode, (StatusCode, Json<crate::auth::ApiError>)> {
    if req.allowed_provider_key_ids.is_empty() {
        return Err(bad_request(
            "invalid_field",
            "allowed_provider_key_ids must contain at least one provider_key id",
            Some("allowed_provider_key_ids"),
        ));
    }
    let owned: Option<(String,)> = sqlx::query_as("SELECT id FROM api_keys WHERE id = ? AND tenant_id = ?")
        .bind(&req.api_key_id)
        .bind(admin.tenant_id.to_string())
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if owned.is_none() {
        return Err(not_found("not_found", "api_key_id not found in this tenant"));
    }
    let prov: Option<(String,)> = sqlx::query_as("SELECT id FROM providers WHERE id = ?")
        .bind(&req.provider_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if prov.is_none() {
        return Err(not_found("not_found", "provider not found"));
    }
    for pk in &req.allowed_provider_key_ids {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM provider_keys WHERE id = ? AND provider_id = ?",
        )
        .bind(pk)
        .bind(&req.provider_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
        if row.is_none() {
            return Err(bad_request(
                "invalid_field",
                &format!("provider_key '{pk}' does not belong to provider '{}'", req.provider_id),
                Some("allowed_provider_key_ids"),
            ));
        }
    }
    let mut tx = state.pool.begin().await.map_err(crate::auth::internal_error)?;
    for pk in req.allowed_provider_key_ids {
        sqlx::query(
            "INSERT INTO api_key_provider_key_pools (id, tenant_id, api_key_id, provider_key_id, created_at) \
             VALUES (?, ?, ?, ?, datetime('now')) \
             ON CONFLICT(api_key_id, provider_key_id) DO NOTHING",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(admin.tenant_id.to_string())
        .bind(&req.api_key_id)
        .bind(pk)
        .execute(&mut *tx)
        .await
        .map_err(crate::auth::internal_error)?;
    }
    tx.commit().await.map_err(crate::auth::internal_error)?;
    Ok(StatusCode::CREATED)
}

pub async fn delete_key_pool(
    admin: TenantAdmin,
    Path((api_key_id, provider_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<crate::auth::ApiError>)> {
    let owned: Option<(String,)> = sqlx::query_as("SELECT id FROM api_keys WHERE id = ? AND tenant_id = ?")
        .bind(&api_key_id)
        .bind(admin.tenant_id.to_string())
        .fetch_optional(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if owned.is_none() {
        return Err(not_found("not_found", "api_key_id not found in this tenant"));
    }
    let res = sqlx::query(
        "DELETE FROM api_key_provider_key_pools \
         WHERE api_key_id = ? AND provider_key_id IN (SELECT id FROM provider_keys WHERE provider_id = ?)",
    )
    .bind(&api_key_id)
    .bind(&provider_id)
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    if res.rows_affected() == 0 {
        return Err(not_found("not_found", "no key-pool mappings to delete"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ──────────────────────────────────────────────────────────────────────────
// CRUD: vision mappings (global)
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateVisionMappingRequest {
    pub model_name: String,
    pub vision_parser_alias: Option<String>,
    pub generation_alias: Option<String>,
}

pub async fn create_vision_mapping(
    _admin: TenantAdmin,
    State(state): State<AppState>,
    Json(req): Json<CreateVisionMappingRequest>,
) -> Result<(StatusCode, Json<VisionMappingView>), (StatusCode, Json<crate::auth::ApiError>)> {
    if req.model_name.trim().is_empty() {
        return Err(bad_request("invalid_field", "model_name must not be empty", Some("model_name")));
    }
    let dup: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM model_vision_mappings WHERE model_name = ?",
    )
    .bind(&req.model_name)
    .fetch_optional(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    if dup.is_some() {
        return Err(conflict("model_taken", "a vision mapping for this model already exists"));
    }
    sqlx::query(
        "INSERT INTO model_vision_mappings (id, model_name, vision_parser_alias, generation_alias, created_at, updated_at) \
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&req.model_name)
    .bind(req.vision_parser_alias.as_deref())
    .bind(req.generation_alias.as_deref())
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;
    Ok((
        StatusCode::CREATED,
        Json(VisionMappingView {
            model_name: req.model_name,
            vision_parser_alias: req.vision_parser_alias,
            generation_alias: req.generation_alias,
        }),
    ))
}

pub async fn delete_vision_mapping(
    _admin: TenantAdmin,
    Path(model_name): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, Json<crate::auth::ApiError>)> {
    let res = sqlx::query("DELETE FROM model_vision_mappings WHERE model_name = ?")
        .bind(&model_name)
        .execute(&state.pool)
        .await
        .map_err(crate::auth::internal_error)?;
    if res.rows_affected() == 0 {
        return Err(not_found("not_found", "vision mapping not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}
