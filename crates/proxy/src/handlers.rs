use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use llm_core::{
    error::ProxyError,
    schema::{CanonicalStreamChunk, ContentPart, StreamDelta},
};
use chrono::Utc;
use uuid::Uuid;

use crate::{
    auth::ApiKeyContext,
    protocol::{
        claude::{
            inbound::claude_to_canonical,
            outbound::{
                canonical_chunk_to_claude_sse, canonical_to_claude_response,
                claude_stream_start_event, claude_stream_stop_event, proxy_error_to_claude_error,
            },
            types::ClaudeRequest,
        },
        openai::{
            inbound::openai_chat_to_canonical,
            outbound::{
                canonical_chunk_to_openai_sse, canonical_to_openai_response, openai_stream_done,
                proxy_error_to_openai_error,
            },
            types::OpenAiChatRequest,
        },
    },
    state::AppState,
};

pub async fn healthz() -> &'static str {
    "ok"
}

pub async fn openai_chat_completions(
    api: ApiKeyContext,
    State(state): State<AppState>,
    Json(req): Json<OpenAiChatRequest>,
) -> Result<Response, (StatusCode, Json<crate::auth::ApiError>)> {
    let stream = req.stream.unwrap_or(false);
    let canonical = openai_chat_to_canonical(req, api.tenant_id, api.api_key_id)
        .map_err(proxy_err_to_api_err)?;

    ensure_model_permitted(&state, api.api_key_id, &canonical.model).await?;

    tracing::info!(
        tenant_id = %api.tenant_id,
        api_key = %api.masked_key,
        model = %canonical.model,
        stream = canonical.stream,
        protocol = "openai_chat",
        "proxy request accepted"
    );

    let resp = match state.multimodal.route(canonical.clone()).await {
        Ok(v) => v,
        Err(e) => {
            persist_request_log(&state, &api, &canonical, None, Some(&e)).await?;
            return Err(proxy_err_to_api_err(e));
        }
    };

    persist_request_log(&state, &api, &canonical, Some(&resp), None).await?;

    if stream {
        let text = resp
            .content
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let mut sse = String::new();
        sse.push_str(&canonical_chunk_to_openai_sse(
            CanonicalStreamChunk {
                request_id: resp.request_id,
                delta: StreamDelta::Text { text },
                finish_reason: Some(resp.stop_reason.clone()),
            },
            &format!("chatcmpl-{}", resp.request_id.as_simple()),
            &resp.model,
            0,
        ));
        sse.push_str(&canonical_chunk_to_openai_sse(
            CanonicalStreamChunk {
                request_id: resp.request_id,
                delta: StreamDelta::Usage { usage: resp.usage.clone() },
                finish_reason: None,
            },
            &format!("chatcmpl-{}", resp.request_id.as_simple()),
            &resp.model,
            1,
        ));
        sse.push_str(openai_stream_done());

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        return Ok((headers, sse).into_response());
    }

    Ok((StatusCode::OK, Json(canonical_to_openai_response(resp))).into_response())
}

pub async fn claude_messages(
    api: ApiKeyContext,
    State(state): State<AppState>,
    Json(req): Json<ClaudeRequest>,
) -> Result<Response, (StatusCode, Json<crate::auth::ApiError>)> {
    let stream = req.stream.unwrap_or(false);
    let canonical = claude_to_canonical(req, api.tenant_id, api.api_key_id)
        .map_err(proxy_err_to_api_err)?;

    ensure_model_permitted(&state, api.api_key_id, &canonical.model).await?;

    tracing::info!(
        tenant_id = %api.tenant_id,
        api_key = %api.masked_key,
        model = %canonical.model,
        stream = canonical.stream,
        protocol = "claude",
        "proxy request accepted"
    );

    let resp = match state.multimodal.route(canonical.clone()).await {
        Ok(v) => v,
        Err(e) => {
            persist_request_log(&state, &api, &canonical, None, Some(&e)).await?;
            return Err(proxy_err_to_api_err(e));
        }
    };

    persist_request_log(&state, &api, &canonical, Some(&resp), None).await?;

    if stream {
        let text = resp
            .content
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let mut sse = String::new();
        sse.push_str(&claude_stream_start_event(resp.request_id, &resp.model));
        sse.push_str(&canonical_chunk_to_claude_sse(
            CanonicalStreamChunk {
                request_id: resp.request_id,
                delta: StreamDelta::Text { text },
                finish_reason: None,
            },
            &format!("msg_{}", resp.request_id.as_simple()),
            &resp.model,
            0,
        ));
        sse.push_str(&canonical_chunk_to_claude_sse(
            CanonicalStreamChunk {
                request_id: resp.request_id,
                delta: StreamDelta::Usage { usage: resp.usage.clone() },
                finish_reason: Some(resp.stop_reason.clone()),
            },
            &format!("msg_{}", resp.request_id.as_simple()),
            &resp.model,
            1,
        ));
        sse.push_str(&claude_stream_stop_event());

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/event-stream"));
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        return Ok((headers, sse).into_response());
    }

    Ok((StatusCode::OK, Json(canonical_to_claude_response(resp))).into_response())
}

fn proxy_err_to_api_err(e: ProxyError) -> (StatusCode, Json<crate::auth::ApiError>) {
    (
        StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(crate::auth::ApiError {
            error: "proxy_error".to_string(),
            message: e.to_string(),
        }),
    )
}

pub fn proxy_err_to_openai(e: ProxyError) -> (StatusCode, Json<serde_json::Value>) {
    let status = StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = proxy_error_to_openai_error("proxy_error", &e.to_string());
    (status, Json(serde_json::to_value(body).unwrap_or_default()))
}

pub fn proxy_err_to_claude(e: ProxyError) -> (StatusCode, Json<serde_json::Value>) {
    let status = StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = proxy_error_to_claude_error("proxy_error", &e.to_string());
    (status, Json(serde_json::to_value(body).unwrap_or_default()))
}

async fn ensure_model_permitted(
    state: &AppState,
    api_key_id: Uuid,
    model: &str,
) -> Result<(), (StatusCode, Json<crate::auth::ApiError>)> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT model_name FROM api_key_model_acl WHERE api_key_id = ? AND allowed = 1",
    )
    .bind(api_key_id.to_string())
    .fetch_all(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    // 若 ACL 为空，则默认放行；若有记录则必须命中
    if rows.is_empty() {
        return Ok(());
    }

    let allowed = rows.iter().any(|(m,)| m == model);
    if allowed {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(crate::auth::ApiError {
                error: "model_not_permitted".to_string(),
                message: format!("model `{model}` is not permitted for this api key"),
            }),
        ))
    }
}

async fn persist_request_log(
    state: &AppState,
    api: &ApiKeyContext,
    req: &llm_core::schema::CanonicalRequest,
    resp: Option<&llm_core::schema::CanonicalResponse>,
    err: Option<&ProxyError>,
) -> Result<(), (StatusCode, Json<crate::auth::ApiError>)> {
    let status = if err.is_some() { "error" } else { "success" };
    let provider_id = resp.map(|r| r.provider_id.clone());
    let provider_model = resp.map(|r| r.model.clone());
    let input_tokens = resp.map(|r| r.usage.input_tokens as i64).unwrap_or(0);
    let output_tokens = resp.map(|r| r.usage.output_tokens as i64).unwrap_or(0);
    let latency_ms = resp.map(|r| r.latency_ms as i64).unwrap_or(0);
    let failover_count = match err {
        Some(llm_core::error::ProxyError::AllProvidersExhausted { .. }) => 1,
        _ => 0,
    };
    let error_code = err.map(|e| e.to_string());

    sqlx::query(
        "INSERT INTO request_logs (
            id, tenant_id, api_key_id, request_id, model_alias, provider_id, provider_model,
            origin_protocol, status, input_tokens, output_tokens, latency_ms, failover_count,
            error_code, created_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::new_v4().to_string())
    .bind(api.tenant_id.to_string())
    .bind(api.api_key_id.to_string())
    .bind(req.request_id.to_string())
    .bind(req.model.clone())
    .bind(provider_id)
    .bind(provider_model)
    .bind(format!("{:?}", req.origin_protocol).to_lowercase())
    .bind(status)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(latency_ms)
    .bind(failover_count)
    .bind(error_code)
    .bind(Utc::now().to_rfc3339())
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    let hour_bucket = Utc::now().format("%Y-%m-%d %H:00:00").to_string();
    let metric_id = Uuid::new_v4().to_string();
    let is_error = if err.is_some() { 1 } else { 0 };

    sqlx::query(
        "INSERT INTO tenant_metrics_hourly (
            id, tenant_id, hour_bucket, request_count, error_count, failover_count,
            total_input_tokens, total_output_tokens, total_latency_ms, created_at, updated_at
         ) VALUES (?, ?, ?, 1, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
         ON CONFLICT(tenant_id, hour_bucket) DO UPDATE SET
            request_count = request_count + 1,
            error_count = error_count + excluded.error_count,
            failover_count = failover_count + excluded.failover_count,
            total_input_tokens = total_input_tokens + excluded.total_input_tokens,
            total_output_tokens = total_output_tokens + excluded.total_output_tokens,
            total_latency_ms = total_latency_ms + excluded.total_latency_ms,
            updated_at = datetime('now')",
    )
    .bind(metric_id)
    .bind(api.tenant_id.to_string())
    .bind(hour_bucket)
    .bind(is_error)
    .bind(failover_count)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(latency_ms)
    .execute(&state.pool)
    .await
    .map_err(crate::auth::internal_error)?;

    Ok(())
}
