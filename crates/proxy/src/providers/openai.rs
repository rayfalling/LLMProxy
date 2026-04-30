/// OpenAI 上游 provider adapter（/v1/chat/completions）
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use llm_core::{
    error::{FailoverTrigger, ProxyError},
    provider::{ExecContext, ProviderAdapter, StreamResult},
    schema::{
        CanonicalMessage, CanonicalRequest, CanonicalResponse, CanonicalStreamChunk,
        ContentPart, ImageData, Role, StopReason, StreamDelta, TokenUsage, ToolCall,
    },
};
use serde_json::{json, Value};
use std::time::Instant;
use uuid::Uuid;

const DEFAULT_BASE_URL: &str = "https://api.openai.com";

pub struct OpenAiAdapter;

#[async_trait]
impl ProviderAdapter for OpenAiAdapter {
    fn id(&self) -> &str {
        "openai"
    }

    async fn complete(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<CanonicalResponse, ProxyError> {
        let client = super::http_util::build_client(ctx)
              .map_err(|e| ProxyError::HttpClient(e))?;

        let body = canonical_to_openai_body(req);
        let url = format!("{}/v1/chat/completions", ctx.base_url.trim_end_matches('/'));

        let t0 = Instant::now();
        let resp = client
            .post(&url)
            .header("authorization", format!("Bearer {}", ctx.api_key))
            .header("content-type", "application/json")
            .apply_extra_headers(&ctx.extra_headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| if e.is_timeout() {
                ProxyError::UpstreamError {
                    provider: "openai".to_string(),
                    status: 0,
                    body: e.to_string(),
                    trigger: Some(FailoverTrigger::Timeout),
                }
            } else {
                ProxyError::HttpClient(e)
            })?;

        let status = resp.status().as_u16();
        let latency_ms = t0.elapsed().as_millis() as u64;

        if status != 200 {
            let body_text = resp.text().await.unwrap_or_default();
            let trigger = ProxyError::classify_upstream(status, &body_text);
            return Err(ProxyError::UpstreamError {
                provider: "openai".to_string(),
                status,
                body: body_text,
                trigger,
            });
        }

        let json: Value = resp.json().await.map_err(|e| ProxyError::HttpClient(e))?;
        Ok(openai_json_to_canonical(&json, req.request_id, latency_ms))
    }

    async fn complete_stream(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<StreamResult, ProxyError> {
        let client = super::http_util::build_client(ctx)
              .map_err(|e| ProxyError::HttpClient(e))?;

        let mut body = canonical_to_openai_body(req);
        body["stream"] = Value::Bool(true);
        body["stream_options"] = json!({ "include_usage": true });

        let url = format!("{}/v1/chat/completions", ctx.base_url.trim_end_matches('/'));

        let resp = client
            .post(&url)
            .header("authorization", format!("Bearer {}", ctx.api_key))
            .header("content-type", "application/json")
            .apply_extra_headers(&ctx.extra_headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| if e.is_timeout() {
                ProxyError::UpstreamError {
                    provider: "openai".to_string(),
                    status: 0,
                    body: e.to_string(),
                    trigger: Some(FailoverTrigger::Timeout),
                }
            } else {
                ProxyError::HttpClient(e)
            })?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body_text = resp.text().await.unwrap_or_default();
            let trigger = ProxyError::classify_upstream(status, &body_text);
            return Err(ProxyError::UpstreamError {
                provider: "openai".to_string(),
                status,
                body: body_text,
                trigger,
            });
        }

        let request_id = req.request_id;
        let chunk_stream = resp
            .bytes_stream()
            .map_err(|e| ProxyError::HttpClient(e))
            .map_ok(move |bytes| {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let chunks: Vec<Result<CanonicalStreamChunk, ProxyError>> = text
                    .lines()
                    .filter(|l| l.starts_with("data: "))
                    .filter_map(|line| {
                        let data = line.trim_start_matches("data: ");
                        if data.trim() == "[DONE]" {
                            return None;
                        }
                        let val: Value = serde_json::from_str(data).ok()?;
                        openai_chunk_to_canonical(&val, request_id)
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

fn canonical_to_openai_body(req: &CanonicalRequest) -> Value {
    let mut messages: Vec<Value> = vec![];

    if let Some(sys) = &req.system {
        messages.push(json!({ "role": "system", "content": sys }));
    }

    for msg in &req.messages {
        messages.push(convert_message(msg));
    }

    let mut body = json!({
        "model": req.model,
        "messages": messages,
    });

    if let Some(mt) = req.max_tokens {
        body["max_completion_tokens"] = Value::from(mt);
    }
    if let Some(t) = req.temperature {
        body["temperature"] = Value::from(t);
    }
    if let Some(p) = req.top_p {
        body["top_p"] = Value::from(p);
    }
    if !req.stop.is_empty() {
        body["stop"] = if req.stop.len() == 1 {
            Value::String(req.stop[0].clone())
        } else {
            json!(req.stop)
        };
    }
    if !req.tools.is_empty() {
        let tools: Vec<Value> = req
            .tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        body["tools"] = json!(tools);
    }

    // 透传 extra 字段
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
        Role::Tool => "tool",
        Role::System => "system",
    };

    // 纯文本消息
    let has_image = msg.content.iter().any(|p| matches!(p, ContentPart::Image { .. }));

    if !has_image {
        let text: String = msg
            .content
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let mut m = json!({ "role": role, "content": text });
        if let Some(id) = &msg.tool_call_id {
            m["tool_call_id"] = Value::String(id.clone());
        }
        if let Some(name) = &msg.name {
            m["name"] = Value::String(name.clone());
        }
        return m;
    }

    // 多模态消息
    let parts: Vec<Value> = msg
        .content
        .iter()
        .map(|p| match p {
            ContentPart::Text { text } => json!({ "type": "text", "text": text }),
            ContentPart::Image { data, .. } => {
                let url = match data {
                    ImageData::Url(u) => u.clone(),
                    ImageData::Base64(b64) => format!("data:image/jpeg;base64,{b64}"),
                };
                json!({ "type": "image_url", "image_url": { "url": url } })
            }
        })
        .collect();

    json!({ "role": role, "content": parts })
}

fn openai_json_to_canonical(json: &Value, request_id: Uuid, latency_ms: u64) -> CanonicalResponse {
    let model = json["model"].as_str().unwrap_or("").to_string();

    let choice = json["choices"].get(0).unwrap_or(&Value::Null);
    let message = &choice["message"];

    let content_text = message["content"].as_str().unwrap_or("").to_string();
    let content = if content_text.is_empty() {
        vec![]
    } else {
        vec![ContentPart::Text { text: content_text }]
    };

    let tool_calls: Option<Vec<ToolCall>> = message["tool_calls"]
        .as_array()
        .filter(|a| !a.is_empty())
        .map(|arr| {
            arr.iter()
                .filter_map(|tc| {
                    Some(ToolCall {
                        id: tc["id"].as_str()?.to_string(),
                        name: tc["function"]["name"].as_str()?.to_string(),
                        arguments: tc["function"]["arguments"].as_str()?.to_string(),
                    })
                })
                .collect()
        });

    let stop_reason = match choice["finish_reason"].as_str().unwrap_or("stop") {
        "stop" => StopReason::EndTurn,
        "length" => StopReason::MaxTokens,
        "tool_calls" => StopReason::ToolUse,
        "content_filter" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_string()),
    };

    let usage = TokenUsage {
        input_tokens: json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        cache_read_tokens: json["usage"]["prompt_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or(0) as u32,
        cache_write_tokens: 0,
    };

    CanonicalResponse {
        request_id,
        provider_id: "openai".to_string(),
        model,
        content,
        tool_calls,
        stop_reason,
        usage,
        latency_ms,
    }
}

fn openai_chunk_to_canonical(val: &Value, request_id: Uuid) -> Option<CanonicalStreamChunk> {
    let choice = val["choices"].get(0)?;
    let delta = &choice["delta"];

    // 用量 chunk
    if let Some(usage) = val["usage"].as_object() {
        let u = TokenUsage {
            input_tokens: usage.get("prompt_tokens")
                .and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            output_tokens: usage.get("completion_tokens")
                .and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
        };
        return Some(CanonicalStreamChunk {
            request_id,
            delta: StreamDelta::Usage { usage: u },
            finish_reason: None,
        });
    }

    let finish_reason = choice["finish_reason"].as_str().map(|r| match r {
        "stop" => StopReason::EndTurn,
        "length" => StopReason::MaxTokens,
        "tool_calls" => StopReason::ToolUse,
        "content_filter" => StopReason::ContentFilter,
        other => StopReason::Other(other.to_string()),
    });

    if let Some(text) = delta["content"].as_str() {
        return Some(CanonicalStreamChunk {
            request_id,
            delta: StreamDelta::Text { text: text.to_string() },
            finish_reason,
        });
    }

    if let Some(tcs) = delta["tool_calls"].as_array() {
        if let Some(tc) = tcs.first() {
            let id = tc["id"].as_str().unwrap_or("").to_string();
            let name = tc["function"]["name"].as_str().map(|s| s.to_string());
            let args = tc["function"]["arguments"].as_str().unwrap_or("").to_string();
            return Some(CanonicalStreamChunk {
                request_id,
                delta: StreamDelta::ToolCallArgs { id, name, arguments: args },
                finish_reason,
            });
        }
    }

    // finish 帧（空 delta）
    if finish_reason.is_some() {
        return Some(CanonicalStreamChunk {
            request_id,
            delta: StreamDelta::Text { text: String::new() },
            finish_reason,
        });
    }

    None
}

// ─── Extension trait ───────────────────────────────────────────────────────

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
