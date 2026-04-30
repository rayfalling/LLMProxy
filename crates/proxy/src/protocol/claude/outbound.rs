/// CanonicalResponse → Claude Messages API 响应格式
use llm_core::schema::{
    CanonicalResponse, CanonicalStreamChunk, ContentPart, StopReason, StreamDelta,
};
use super::types::{
    ClaudeContentBlock, ClaudeErrorDetail, ClaudeErrorResponse, ClaudeResponse,
    ClaudeStreamContentBlock, ClaudeStreamDelta, ClaudeStreamEvent,
    ClaudeStreamMessage, ClaudeStreamUsage, ClaudeUsage, ClaudeMessageDeltaPayload,
};
use uuid::Uuid;

pub fn canonical_to_claude_response(resp: CanonicalResponse) -> ClaudeResponse {
    let content: Vec<ClaudeContentBlock> = resp
        .content
        .into_iter()
        .filter_map(|part| match part {
            ContentPart::Text { text } => Some(ClaudeContentBlock::Text { text }),
            ContentPart::Image { .. } => None,
        })
        .collect();

    let stop_reason = Some(stop_reason_to_claude(&resp.stop_reason));

    ClaudeResponse {
        id: format!("msg_{}", resp.request_id.as_simple()),
        response_type: "message".to_string(),
        role: "assistant".to_string(),
        model: resp.model,
        content,
        stop_reason,
        stop_sequence: None,
        usage: ClaudeUsage {
            input_tokens: resp.usage.input_tokens,
            output_tokens: resp.usage.output_tokens,
            cache_creation_input_tokens: if resp.usage.cache_write_tokens > 0 {
                Some(resp.usage.cache_write_tokens)
            } else {
                None
            },
            cache_read_input_tokens: if resp.usage.cache_read_tokens > 0 {
                Some(resp.usage.cache_read_tokens)
            } else {
                None
            },
        },
    }
}

/// 将 CanonicalStreamChunk 转换为 Claude SSE 事件字符串
pub fn canonical_chunk_to_claude_sse(
    chunk: CanonicalStreamChunk,
    message_id: &str,
    model: &str,
    index: u32,
) -> String {
    let event = match chunk.delta {
        StreamDelta::Text { text } => ClaudeStreamEvent::ContentBlockDelta {
            index,
            delta: ClaudeStreamDelta::TextDelta { text },
        },
        StreamDelta::ToolCallArgs { id: _, name: _, arguments } => {
            ClaudeStreamEvent::ContentBlockDelta {
                index,
                delta: ClaudeStreamDelta::InputJsonDelta {
                    partial_json: arguments,
                },
            }
        }
        StreamDelta::Usage { usage } => ClaudeStreamEvent::MessageDelta {
            delta: ClaudeMessageDeltaPayload {
                stop_reason: chunk.finish_reason.map(|r| stop_reason_to_claude(&r)),
                stop_sequence: None,
            },
            usage: ClaudeStreamUsage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
                cache_creation_input_tokens: if usage.cache_write_tokens > 0 {
                    Some(usage.cache_write_tokens)
                } else {
                    None
                },
                cache_read_input_tokens: if usage.cache_read_tokens > 0 {
                    Some(usage.cache_read_tokens)
                } else {
                    None
                },
            },
        },
    };

    let data = serde_json::to_string(&event).unwrap_or_default();
    format!("data: {data}\n\n")
}

/// 生成流式开始事件
pub fn claude_stream_start_event(request_id: Uuid, model: &str) -> String {
    let event = ClaudeStreamEvent::MessageStart {
        message: ClaudeStreamMessage {
            id: format!("msg_{}", request_id.as_simple()),
            msg_type: "message".to_string(),
            role: "assistant".to_string(),
            model: model.to_string(),
            content: vec![],
            stop_reason: None,
            usage: ClaudeStreamUsage {
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_input_tokens: None,
                cache_read_input_tokens: None,
            },
        },
    };
    let data = serde_json::to_string(&event).unwrap_or_default();
    format!("data: {data}\n\nevent: message_start\n\n")
}

pub fn claude_stream_stop_event() -> String {
    let event = ClaudeStreamEvent::MessageStop;
    let data = serde_json::to_string(&event).unwrap_or_default();
    format!("data: {data}\n\nevent: message_stop\n\n")
}

/// 错误响应
pub fn proxy_error_to_claude_error(error_type: &str, message: &str) -> ClaudeErrorResponse {
    ClaudeErrorResponse {
        response_type: "error".to_string(),
        error: ClaudeErrorDetail {
            error_type: error_type.to_string(),
            message: message.to_string(),
        },
    }
}

fn stop_reason_to_claude(reason: &StopReason) -> String {
    match reason {
        StopReason::EndTurn => "end_turn".to_string(),
        StopReason::MaxTokens => "max_tokens".to_string(),
        StopReason::StopSequence => "stop_sequence".to_string(),
        StopReason::ToolUse => "tool_use".to_string(),
        StopReason::ContentFilter => "content_filter".to_string(),
        StopReason::Other(s) => s.clone(),
    }
}
