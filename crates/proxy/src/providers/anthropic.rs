/// Anthropic (Claude) 上游 provider adapter
/// 将 CanonicalRequest 转为 Anthropic Messages API 请求并回转
use async_trait::async_trait;
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use llm_core::{
    error::ProxyError,
    provider::{ExecContext, ProviderAdapter, StreamResult},
    schema::{
        CanonicalMessage, CanonicalRequest, CanonicalResponse, CanonicalStreamChunk,
        ContentPart, ImageData, Role, StopReason, StreamDelta, TokenUsage,
    },
};
use serde_json::{json, Value};
use std::time::Instant;
use uuid::Uuid;

const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicAdapter;

#[async_trait]
impl ProviderAdapter for AnthropicAdapter {
    fn id(&self) -> &str {
        "anthropic"
    }

    async fn complete(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<CanonicalResponse, ProxyError> {
        let client = super::http_util::build_client(ctx)
              .map_err(|e| ProxyError::HttpClient(e))?;

        let body = canonical_to_anthropic_body(req);
        let url = format!("{}/v1/messages", ctx.base_url.trim_end_matches('/'));

        let t0 = Instant::now();
        let resp = client
            .post(&url)
            .header("x-api-key", &ctx.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .apply_extra_headers(&ctx.extra_headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProxyError::UpstreamError {
                        provider: "anthropic".to_string(),
                        status: 0,
                        body: e.to_string(),
                        trigger: Some(llm_core::error::FailoverTrigger::Timeout),
                    }
                } else {
                    ProxyError::HttpClient(e)
                }
            })?;

        let status = resp.status().as_u16();
        let latency_ms = t0.elapsed().as_millis() as u64;

        if status != 200 {
            let body_text = resp.text().await.unwrap_or_default();
            let trigger = ProxyError::classify_upstream(status, &body_text);
            return Err(ProxyError::UpstreamError {
                provider: "anthropic".to_string(),
                status,
                body: body_text,
                trigger,
            });
        }

        let json: Value = resp.json().await.map_err(|e| ProxyError::HttpClient(e))?;
        Ok(anthropic_json_to_canonical(&json, req.request_id, latency_ms))
    }

    async fn complete_stream(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<StreamResult, ProxyError> {
        let client = super::http_util::build_client(ctx)
              .map_err(|e| ProxyError::HttpClient(e))?;

        let mut body = canonical_to_anthropic_body(req);
        body["stream"] = Value::Bool(true);

        let url = format!("{}/v1/messages", ctx.base_url.trim_end_matches('/'));

        let resp = client
            .post(&url)
            .header("x-api-key", &ctx.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .apply_extra_headers(&ctx.extra_headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProxyError::UpstreamError {
                        provider: "anthropic".to_string(),
                        status: 0,
                        body: e.to_string(),
                        trigger: Some(llm_core::error::FailoverTrigger::Timeout),
                    }
                } else {
                    ProxyError::HttpClient(e)
                }
            })?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body_text = resp.text().await.unwrap_or_default();
            let trigger = ProxyError::classify_upstream(status, &body_text);
            return Err(ProxyError::UpstreamError {
                provider: "anthropic".to_string(),
                status,
                body: body_text,
                trigger,
            });
        }

        let request_id = req.request_id;
        let byte_stream = resp.bytes_stream().map_err(|e| ProxyError::HttpClient(e));

        // SSE: 每个 data: 行 -> CanonicalStreamChunk
        let chunk_stream = byte_stream
            .map_ok(move |bytes| {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let chunks: Vec<Result<CanonicalStreamChunk, ProxyError>> = text
                    .lines()
                    .filter(|l| l.starts_with("data: "))
                    .filter_map(|line| {
                        let data = line.trim_start_matches("data: ");
                        let val: Value = serde_json::from_str(data).ok()?;
                        anthropic_event_to_chunk(&val, request_id)
                    })
                    .map(Ok)
                    .collect();
                futures::stream::iter(chunks)
            })
            .try_flatten();

        Ok(Box::pin(chunk_stream))
    }
}

// ─── Conversion helpers ────────────────────────────────────────────────────

fn canonical_to_anthropic_body(req: &CanonicalRequest) -> Value {
    let messages: Vec<Value> = req.messages.iter().map(convert_message).collect();

    let mut body = json!({
        "model": req.model,
        "messages": messages,
        "max_tokens": req.max_tokens.unwrap_or(4096),
    });

    if let Some(sys) = &req.system {
        body["system"] = Value::String(sys.clone());
    }
    if let Some(t) = req.temperature {
        body["temperature"] = Value::from(t);
    }
    if let Some(p) = req.top_p {
        body["top_p"] = Value::from(p);
    }
    if !req.stop.is_empty() {
        body["stop_sequences"] = json!(req.stop);
    }
    if !req.tools.is_empty() {
        let tools: Vec<Value> = req
            .tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            })
            .collect();
        body["tools"] = json!(tools);
    }

    // 透传 extra 字段（thinking / tool_choice / top_k 等）
    if let Value::Object(map) = &req.extra {
        for (k, v) in map {
            body[k] = v.clone();
        }
    }

    body
}

fn convert_message(msg: &CanonicalMessage) -> Value {
    let role = match msg.role {
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "user",
        Role::System => "user",
    };

    let content: Vec<Value> = msg.content.iter().map(|p| match p {
        ContentPart::Text { text } => json!({ "type": "text", "text": text }),
        ContentPart::Image { data, media_type } => {
            match data {
                ImageData::Base64(b64) => json!({
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": media_type.as_deref().unwrap_or("image/jpeg"),
                        "data": b64,
                    }
                }),
                ImageData::Url(url) => json!({
                    "type": "image",
                    "source": { "type": "url", "url": url }
                }),
            }
        }
    }).collect();

    if content.len() == 1 {
        if let Some(Value::String(text)) = content[0].get("text") {
            return json!({ "role": role, "content": text });
        }
    }

    json!({ "role": role, "content": content })
}

fn anthropic_json_to_canonical(json: &Value, request_id: Uuid, latency_ms: u64) -> CanonicalResponse {
    let model = json["model"].as_str().unwrap_or("").to_string();

    let content: Vec<ContentPart> = json["content"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|block| {
            if block["type"] == "text" {
                Some(ContentPart::Text {
                    text: block["text"].as_str().unwrap_or("").to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    let stop_reason = match json["stop_reason"].as_str().unwrap_or("end_turn") {
        "end_turn" => StopReason::EndTurn,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        "tool_use" => StopReason::ToolUse,
        other => StopReason::Other(other.to_string()),
    };

    let usage = TokenUsage {
        input_tokens: json["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: json["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
        cache_read_tokens: json["usage"]["cache_read_input_tokens"].as_u64().unwrap_or(0) as u32,
        cache_write_tokens: json["usage"]["cache_creation_input_tokens"].as_u64().unwrap_or(0) as u32,
    };

    CanonicalResponse {
        request_id,
        provider_id: "anthropic".to_string(),
        model,
        content,
        tool_calls: None,
        stop_reason,
        usage,
        latency_ms,
    }
}

fn anthropic_event_to_chunk(val: &Value, request_id: Uuid) -> Option<CanonicalStreamChunk> {
    let event_type = val["type"].as_str()?;

    match event_type {
        "content_block_delta" => {
            let delta = &val["delta"];
            match delta["type"].as_str()? {
                "text_delta" => {
                    let text = delta["text"].as_str()?.to_string();
                    Some(CanonicalStreamChunk {
                        request_id,
                        delta: StreamDelta::Text { text },
                        finish_reason: None,
                    })
                }
                "input_json_delta" => {
                    let args = delta["partial_json"].as_str()?.to_string();
                    Some(CanonicalStreamChunk {
                        request_id,
                        delta: StreamDelta::ToolCallArgs {
                            id: String::new(),
                            name: None,
                            arguments: args,
                        },
                        finish_reason: None,
                    })
                }
                _ => None,
            }
        }
        "message_delta" => {
            let stop_reason = val["delta"]["stop_reason"].as_str().map(|r| match r {
                "end_turn" => StopReason::EndTurn,
                "max_tokens" => StopReason::MaxTokens,
                "stop_sequence" => StopReason::StopSequence,
                "tool_use" => StopReason::ToolUse,
                other => StopReason::Other(other.to_string()),
            });

            let usage = TokenUsage {
                input_tokens: 0,
                output_tokens: val["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
            };

            Some(CanonicalStreamChunk {
                request_id,
                delta: StreamDelta::Usage { usage },
                finish_reason: stop_reason,
            })
        }
        _ => None,
    }
}

// ─── Extension trait for adding extra headers ──────────────────────────────

trait RequestBuilderExt {
    fn apply_extra_headers(self, headers: &[(String, String)]) -> Self;
}

impl RequestBuilderExt for reqwest::RequestBuilder {
    fn apply_extra_headers(mut self, headers: &[(String, String)]) -> Self {
        for (k, v) in headers {
            self = self.header(k.as_str(), v.as_str());
        }
        self
    }
}
