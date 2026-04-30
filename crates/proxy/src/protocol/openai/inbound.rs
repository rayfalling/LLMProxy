/// OpenAI ChatCompletion 入站请求 → CanonicalRequest
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
    OpenAiChatRequest, OpenAiContentPart, OpenAiMessage, OpenAiMessageContent,
    OpenAiStop,
};

pub fn openai_chat_to_canonical(
    req: OpenAiChatRequest,
    tenant_id: Uuid,
    api_key_id: Uuid,
) -> Result<CanonicalRequest, ProxyError> {
    let mut has_image = false;
    let mut system: Option<String> = None;
    let mut messages: Vec<CanonicalMessage> = Vec::new();

    for msg in req.messages {
        if msg.role == "system" {
            // system 提取到顶层字段
            let text = extract_text_content(&msg);
            match system {
                None => system = Some(text),
                Some(ref mut s) => {
                    s.push('\n');
                    s.push_str(&text);
                }
            }
        } else {
            messages.push(convert_message(msg, &mut has_image)?);
        }
    }

    let max_tokens = req
        .max_completion_tokens
        .or(req.max_tokens);

    let stop = match req.stop {
        None => vec![],
        Some(OpenAiStop::Single(s)) => vec![s],
        Some(OpenAiStop::Multiple(v)) => v,
    };

    let tools: Vec<ToolDefinition> = req
        .tools
        .into_iter()
        .map(|t| ToolDefinition {
            name: t.function.name,
            description: t.function.description,
            parameters: t.function.parameters.unwrap_or(serde_json::json!({})),
        })
        .collect();

    // 把 tool_choice / response_format 等透传到 extra
    let mut extra = serde_json::json!({});
    if let Some(tc) = req.tool_choice {
        extra["tool_choice"] = serde_json::to_value(tc).unwrap_or_default();
    }
    if let Some(rf) = req.response_format {
        extra["response_format"] = serde_json::to_value(rf).unwrap_or_default();
    }
    if let Some(seed) = req.seed {
        extra["seed"] = serde_json::Value::from(seed);
    }
    if let Some(re) = req.reasoning_effort {
        extra["reasoning_effort"] = serde_json::Value::String(re);
    }
    if let Some(freq) = req.frequency_penalty {
        extra["frequency_penalty"] = serde_json::Value::from(freq);
    }
    if let Some(pres) = req.presence_penalty {
        extra["presence_penalty"] = serde_json::Value::from(pres);
    }
    if let Some(user) = req.user {
        extra["user"] = serde_json::Value::String(user);
    }

    Ok(CanonicalRequest {
        request_id: Uuid::new_v4(),
        created_at: Utc::now(),
        model: req.model,
        system,
        messages,
        max_tokens,
        temperature: req.temperature,
        top_p: req.top_p,
        stop,
        stream: req.stream.unwrap_or(false),
        tools,
        extra,
        origin_protocol: OriginProtocol::OpenAiChat,
        has_image,
        tenant_id,
        api_key_id,
    })
}

fn extract_text_content(msg: &OpenAiMessage) -> String {
    match &msg.content {
        None => String::new(),
        Some(OpenAiMessageContent::Text(t)) => t.clone(),
        Some(OpenAiMessageContent::Parts(parts)) => parts
            .iter()
            .filter_map(|p| match p {
                OpenAiContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn convert_message(
    msg: OpenAiMessage,
    has_image: &mut bool,
) -> Result<CanonicalMessage, ProxyError> {
    let role = match msg.role.as_str() {
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "tool" => Role::Tool,
        _ => Role::User,
    };

    let content = match msg.content {
        None => vec![],
        Some(OpenAiMessageContent::Text(t)) => vec![ContentPart::Text { text: t }],
        Some(OpenAiMessageContent::Parts(parts)) => parts
            .into_iter()
            .map(|p| convert_content_part(p, has_image))
            .collect(),
    };

    Ok(CanonicalMessage {
        role,
        content,
        tool_call_id: msg.tool_call_id,
        name: msg.name,
    })
}

fn convert_content_part(part: OpenAiContentPart, has_image: &mut bool) -> ContentPart {
    match part {
        OpenAiContentPart::Text { text } => ContentPart::Text { text },
        OpenAiContentPart::ImageUrl { image_url } => {
            *has_image = true;
            // data URL → Base64，否则 Url
            let data = if image_url.url.starts_with("data:") {
                // data:<mime>;base64,<data>
                let b64 = image_url
                    .url
                    .splitn(2, ',')
                    .nth(1)
                    .unwrap_or("")
                    .to_string();
                let media_type = image_url
                    .url
                    .splitn(2, ';')
                    .next()
                    .and_then(|s| s.strip_prefix("data:"))
                    .map(|s| s.to_string());
                return ContentPart::Image {
                    data: ImageData::Base64(b64),
                    media_type,
                };
            } else {
                ImageData::Url(image_url.url)
            };
            ContentPart::Image { data, media_type: None }
        }
        OpenAiContentPart::InputAudio { input_audio } => {
            // 音频暂时作为文本透传 base64
            ContentPart::Text {
                text: format!("[audio:{}]{}", input_audio.format, input_audio.data),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::openai::types::{
        OpenAiChatRequest, OpenAiImageUrl, OpenAiMessage, OpenAiMessageContent,
        OpenAiContentPart,
    };

    #[test]
    fn openai_inbound_data_url_image_to_base64() {
        let req = OpenAiChatRequest {
            model: "gpt-4o-mini".to_string(),
            messages: vec![OpenAiMessage {
                role: "user".to_string(),
                content: Some(OpenAiMessageContent::Parts(vec![
                    OpenAiContentPart::ImageUrl {
                        image_url: OpenAiImageUrl {
                            url: "data:image/png;base64,Zm9vYmFy".to_string(),
                            detail: None,
                        },
                    },
                ])),
                name: None,
                tool_calls: vec![],
                tool_call_id: None,
                refusal: None,
            }],
            max_tokens: Some(64),
            max_completion_tokens: None,
            temperature: None,
            top_p: None,
            n: None,
            stream: Some(false),
            stream_options: None,
            stop: None,
            tools: vec![],
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
            seed: None,
            frequency_penalty: None,
            presence_penalty: None,
            logprobs: None,
            top_logprobs: None,
            user: None,
            reasoning_effort: None,
        };

        let out = openai_chat_to_canonical(req, uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
            .expect("convert");

        assert!(out.has_image);
        assert!(matches!(out.origin_protocol, llm_core::schema::OriginProtocol::OpenAiChat));
        assert!(matches!(out.messages[0].content[0], ContentPart::Image { .. }));
    }
}
