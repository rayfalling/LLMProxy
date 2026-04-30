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

    let resp = state.multimodal.route(canonical).await.map_err(proxy_err_to_api_err)?;

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

    let resp = state.multimodal.route(canonical).await.map_err(proxy_err_to_api_err)?;

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
