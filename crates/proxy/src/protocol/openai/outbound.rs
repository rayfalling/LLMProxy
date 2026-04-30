/// CanonicalResponse → OpenAI ChatCompletion 响应格式
use chrono::Utc;
use llm_core::schema::{
    CanonicalResponse, CanonicalStreamChunk, ContentPart, StopReason, StreamDelta,
};
use super::types::{
    OpenAiChatResponse, OpenAiChoice, OpenAiChoiceMessage, OpenAiCompletionTokenDetails,
    OpenAiErrorDetail, OpenAiErrorResponse, OpenAiPromptTokenDetails,
    OpenAiStreamChunk, OpenAiStreamChoice, OpenAiStreamDelta,
    OpenAiStreamFunctionDelta, OpenAiStreamToolCall, OpenAiToolCall,
    OpenAiFunctionCall, OpenAiUsage,
};
use uuid::Uuid;

pub fn canonical_to_openai_response(resp: CanonicalResponse) -> OpenAiChatResponse {
    let content: Option<String> = {
        let text: String = resp
            .content
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        if text.is_empty() { None } else { Some(text) }
    };

    let tool_calls: Vec<OpenAiToolCall> = resp
        .tool_calls
        .unwrap_or_default()
        .into_iter()
        .map(|tc| OpenAiToolCall {
            id: tc.id,
            call_type: "function".to_string(),
            function: OpenAiFunctionCall {
                name: tc.name,
                arguments: tc.arguments,
            },
        })
        .collect();

    let finish_reason = Some(stop_reason_to_openai(&resp.stop_reason));

    let usage = OpenAiUsage {
        prompt_tokens: resp.usage.input_tokens,
        completion_tokens: resp.usage.output_tokens,
        total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
        prompt_tokens_details: if resp.usage.cache_read_tokens > 0 {
            Some(OpenAiPromptTokenDetails {
                cached_tokens: Some(resp.usage.cache_read_tokens),
                audio_tokens: None,
            })
        } else {
            None
        },
        completion_tokens_details: None,
    };

    OpenAiChatResponse {
        id: format!("chatcmpl-{}", resp.request_id.as_simple()),
        object: "chat.completion".to_string(),
        created: Utc::now().timestamp(),
        model: resp.model,
        choices: vec![OpenAiChoice {
            index: 0,
            message: OpenAiChoiceMessage {
                role: "assistant".to_string(),
                content,
                tool_calls,
                refusal: None,
            },
            finish_reason,
        }],
        usage,
        system_fingerprint: None,
    }
}

/// CanonicalStreamChunk → OpenAI SSE 行
pub fn canonical_chunk_to_openai_sse(
    chunk: CanonicalStreamChunk,
    response_id: &str,
    model: &str,
    chunk_index: u32,
) -> String {
    let (delta, finish_reason, usage) = match chunk.delta {
        StreamDelta::Text { text } => (
            OpenAiStreamDelta {
                role: if chunk_index == 0 { Some("assistant".to_string()) } else { None },
                content: Some(text),
                tool_calls: vec![],
            },
            chunk.finish_reason.map(|r| stop_reason_to_openai(&r)),
            None,
        ),
        StreamDelta::ToolCallArgs { id, name, arguments } => (
            OpenAiStreamDelta {
                role: None,
                content: None,
                tool_calls: vec![OpenAiStreamToolCall {
                    index: 0,
                    id: Some(id),
                    call_type: Some("function".to_string()),
                    function: Some(OpenAiStreamFunctionDelta {
                        name,
                        arguments: Some(arguments),
                    }),
                }],
            },
            chunk.finish_reason.map(|r| stop_reason_to_openai(&r)),
            None,
        ),
        StreamDelta::Usage { usage } => (
            OpenAiStreamDelta {
                role: None,
                content: None,
                tool_calls: vec![],
            },
            chunk.finish_reason.map(|r| stop_reason_to_openai(&r)),
            Some(OpenAiUsage {
                prompt_tokens: usage.input_tokens,
                completion_tokens: usage.output_tokens,
                total_tokens: usage.input_tokens + usage.output_tokens,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            }),
        ),
    };

    let stream_chunk = OpenAiStreamChunk {
        id: response_id.to_string(),
        object: "chat.completion.chunk".to_string(),
        created: Utc::now().timestamp(),
        model: model.to_string(),
        choices: vec![OpenAiStreamChoice {
            index: 0,
            delta,
            finish_reason,
        }],
        usage,
    };

    let data = serde_json::to_string(&stream_chunk).unwrap_or_default();
    format!("data: {data}\n\n")
}

pub fn openai_stream_done() -> &'static str {
    "data: [DONE]\n\n"
}

pub fn proxy_error_to_openai_error(error_type: &str, message: &str) -> OpenAiErrorResponse {
    OpenAiErrorResponse {
        error: OpenAiErrorDetail {
            message: message.to_string(),
            error_type: error_type.to_string(),
            param: None,
            code: None,
        },
    }
}

fn stop_reason_to_openai(reason: &StopReason) -> String {
    match reason {
        StopReason::EndTurn => "stop".to_string(),
        StopReason::MaxTokens => "length".to_string(),
        StopReason::StopSequence => "stop".to_string(),
        StopReason::ToolUse => "tool_calls".to_string(),
        StopReason::ContentFilter => "content_filter".to_string(),
        StopReason::Other(s) => s.clone(),
    }
}
