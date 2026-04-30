/// Claude 入站请求 → CanonicalRequest 转换
use uuid::Uuid;
use chrono::Utc;
use llm_core::{
    error::ProxyError,
    schema::{
        CanonicalMessage, CanonicalRequest, ContentPart, ImageData,
        OriginProtocol, Role, ToolDefinition,
    },
};
use super::types::{
    ClaudeContentBlock, ClaudeImageSource, ClaudeMessage, ClaudeMessageContent,
    ClaudeRequest, ClaudeRole, ClaudeSystem,
};

pub fn claude_to_canonical(
    req: ClaudeRequest,
    tenant_id: Uuid,
    api_key_id: Uuid,
) -> Result<CanonicalRequest, ProxyError> {
    let mut has_image = false;

    // 转换 system
    let system = req.system.map(|s| match s {
        ClaudeSystem::Text(t) => t,
        ClaudeSystem::Blocks(blocks) => blocks
            .into_iter()
            .map(|b| b.text)
            .collect::<Vec<_>>()
            .join("\n"),
    });

    // 转换 messages
    let messages = req
        .messages
        .into_iter()
        .map(|m| convert_message(m, &mut has_image))
        .collect::<Result<Vec<_>, _>>()?;

    // 转换 tools
    let tools = req
        .tools
        .into_iter()
        .map(|t| ToolDefinition {
            name: t.name,
            description: t.description,
            parameters: t.input_schema,
        })
        .collect();

    // 透传 thinking 等 beta 字段
    let mut extra = serde_json::json!({});
    if let Some(thinking) = req.thinking {
        extra["thinking"] = serde_json::to_value(thinking)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Some(tool_choice) = req.tool_choice {
        extra["tool_choice"] = serde_json::to_value(tool_choice)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Some(top_k) = req.top_k {
        extra["top_k"] = serde_json::Value::from(top_k);
    }
    if let Some(meta) = req.metadata {
        extra["metadata"] = serde_json::to_value(meta)
            .unwrap_or(serde_json::Value::Null);
    }

    Ok(CanonicalRequest {
        request_id: Uuid::new_v4(),
        created_at: Utc::now(),
        model: req.model,
        system,
        messages,
        max_tokens: Some(req.max_tokens),
        temperature: req.temperature,
        top_p: req.top_p,
        stop: req.stop_sequences,
        stream: req.stream.unwrap_or(false),
        tools,
        extra,
        origin_protocol: OriginProtocol::Claude,
        has_image,
        tenant_id,
        api_key_id,
    })
}

fn convert_message(
    msg: ClaudeMessage,
    has_image: &mut bool,
) -> Result<CanonicalMessage, ProxyError> {
    let role = match msg.role {
        ClaudeRole::User => Role::User,
        ClaudeRole::Assistant => Role::Assistant,
    };

    let content = match msg.content {
        ClaudeMessageContent::Text(t) => vec![ContentPart::Text { text: t }],
        ClaudeMessageContent::Blocks(blocks) => blocks
            .into_iter()
            .filter_map(|b| convert_block(b, has_image))
            .collect(),
    };

    Ok(CanonicalMessage {
        role,
        content,
        tool_call_id: None,
        name: None,
    })
}

fn convert_block(block: ClaudeContentBlock, has_image: &mut bool) -> Option<ContentPart> {
    match block {
        ClaudeContentBlock::Text { text } => Some(ContentPart::Text { text }),
        ClaudeContentBlock::Image { source } => {
            *has_image = true;
            let data = match source {
                ClaudeImageSource::Base64 { data, .. } => ImageData::Base64(data),
                ClaudeImageSource::Url { url } => ImageData::Url(url),
            };
            let media_type = match &data {
                _ => None, // media_type stored separately if needed
            };
            Some(ContentPart::Image { data, media_type })
        }
        ClaudeContentBlock::ToolUse { id, name, input } => {
            // tool_use 作为文本序列化透传
            Some(ContentPart::Text {
                text: format!("[tool_use:{id}] {name}({})", input),
            })
        }
        ClaudeContentBlock::ToolResult { content, .. } => {
            use super::types::ClaudeToolResultContent;
            match content {
                ClaudeToolResultContent::Text(t) => Some(ContentPart::Text { text: t }),
                ClaudeToolResultContent::Blocks(blocks) => {
                    let text = blocks
                        .into_iter()
                        .filter_map(|b| convert_block(b, has_image))
                        .filter_map(|p| match p {
                            ContentPart::Text { text } => Some(text),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    Some(ContentPart::Text { text })
                }
            }
        }
        ClaudeContentBlock::Document { source: _, title, context, .. } => {
            let parts: Vec<String> = [title, context]
                .into_iter()
                .flatten()
                .collect();
            Some(ContentPart::Text { text: parts.join("\n") })
        }
        ClaudeContentBlock::Thinking { thinking } => {
            Some(ContentPart::Text { text: format!("<thinking>{thinking}</thinking>") })
        }
        ClaudeContentBlock::RedactedThinking { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::claude::types::{ClaudeMessage, ClaudeMessageContent};

    #[test]
    fn claude_inbound_marks_image_and_origin_protocol() {
        let req = ClaudeRequest {
            model: "claude-sonnet-4".to_string(),
            messages: vec![ClaudeMessage {
                role: ClaudeRole::User,
                content: ClaudeMessageContent::Blocks(vec![
                    ClaudeContentBlock::Text { text: "look".to_string() },
                    ClaudeContentBlock::Image {
                        source: ClaudeImageSource::Url {
                            url: "https://example.com/a.png".to_string(),
                        },
                    },
                ]),
            }],
            max_tokens: 128,
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: vec![],
            stream: Some(false),
            tools: vec![],
            tool_choice: None,
            thinking: None,
            metadata: None,
        };

        let out = claude_to_canonical(req, uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
            .expect("convert");

        assert!(out.has_image);
        assert!(matches!(out.origin_protocol, llm_core::schema::OriginProtocol::Claude));
        assert_eq!(out.messages.len(), 1);
    }
}
