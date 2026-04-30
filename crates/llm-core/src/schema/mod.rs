use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 规范内容块（文本或图片）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text {
        text: String,
    },
    Image {
        /// base64 编码的图片数据，或 URL
        data: ImageData,
        /// 可选 MIME 类型，如 "image/jpeg"
        media_type: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageData {
    /// URL 格式
    Url(String),
    /// base64 格式
    Base64(String),
}

/// 消息角色
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// 规范消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalMessage {
    pub role: Role,
    pub content: Vec<ContentPart>,
    /// tool call id（用于 tool 角色）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 消息名称（OpenAI name 字段）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

/// 工具调用请求（模型 -> 客户端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// 停止原因
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    StopSequence,
    ToolUse,
    ContentFilter,
    Other(String),
}

/// 规范请求（对外协议已被转换到此格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalRequest {
    /// 内部追踪 ID
    pub request_id: Uuid,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 逻辑模型名称（客户端请求的 model 字段）
    pub model: String,
    /// 系统提示（独立字段，兼容 Claude 和 OpenAI）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    pub messages: Vec<CanonicalMessage>,
    /// 最大输出 token
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 温度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// 停止序列
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    /// 是否开启流式输出
    pub stream: bool,
    /// 工具列表
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolDefinition>,
    /// 额外的原始 metadata（透传协议专有字段）
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub extra: serde_json::Value,
    /// 原始协议类型（用于回程格式化）
    pub origin_protocol: OriginProtocol,
    /// 是否包含图片（用于路由决策）
    pub has_image: bool,
    /// 请求所属 tenant id
    pub tenant_id: Uuid,
    /// 请求使用的 API key id
    pub api_key_id: Uuid,
}

/// 原始协议枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OriginProtocol {
    OpenAiChat,
    OpenAiResponses,
    OpenAiImages,
    OpenAiAudio,
    OpenAiRealtime,
    Claude,
}

/// 规范响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalResponse {
    pub request_id: Uuid,
    /// 实际使用的提供商
    pub provider_id: String,
    /// 实际使用的模型
    pub model: String,
    pub content: Vec<ContentPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
    pub latency_ms: u64,
}

/// Token 用量
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
}

/// 流式 chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalStreamChunk {
    pub request_id: Uuid,
    pub delta: StreamDelta,
    pub finish_reason: Option<StopReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamDelta {
    Text { text: String },
    ToolCallArgs { id: String, name: Option<String>, arguments: String },
    Usage { usage: TokenUsage },
}
