/// Claude Messages API 的入站请求类型
/// 覆盖 stable + beta 字段
use serde::{Deserialize, Serialize};

// ─── Content blocks ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeContentBlock {
    Text {
        text: String,
    },
    Image {
        source: ClaudeImageSource,
    },
    /// tool_use: 模型请求调用工具
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// tool_result: 用户返回工具结果
    ToolResult {
        tool_use_id: String,
        content: ClaudeToolResultContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// document block (beta)
    Document {
        source: ClaudeDocumentSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<ClaudeCitationsConfig>,
    },
    /// thinking block (extended thinking beta)
    Thinking {
        thinking: String,
    },
    /// redacted_thinking
    RedactedThinking {
        data: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeImageSource {
    Base64 {
        media_type: String,
        data: String,
    },
    Url {
        url: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudeToolResultContent {
    Text(String),
    Blocks(Vec<ClaudeContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeDocumentSource {
    Base64 {
        media_type: String,
        data: String,
    },
    Text {
        data: String,
    },
    Url {
        url: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCitationsConfig {
    pub enabled: bool,
}

// ─── Message ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: ClaudeRole,
    /// content 可以是纯文本字符串或 content block 列表
    pub content: ClaudeMessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudeMessageContent {
    Text(String),
    Blocks(Vec<ClaudeContentBlock>),
}

// ─── System prompt ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudeSystem {
    Text(String),
    Blocks(Vec<ClaudeSystemBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSystemBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<ClaudeCacheControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCacheControl {
    #[serde(rename = "type")]
    pub cache_type: String,  // "ephemeral"
}

// ─── Tool definition ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    /// cache_control (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<ClaudeCacheControl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeToolChoice {
    #[serde(rename = "type")]
    pub choice_type: String,  // "auto" | "any" | "tool"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
}

// ─── Thinking config (beta) ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeThinkingConfig {
    #[serde(rename = "type")]
    pub thinking_type: String,  // "enabled" | "disabled"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
}

// ─── Request ───────────────────────────────────────────────────────────────

/// Claude Messages API 请求（POST /v1/messages）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub messages: Vec<ClaudeMessage>,
    pub max_tokens: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<ClaudeSystem>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ClaudeToolDefinition>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ClaudeToolChoice>,

    /// extended thinking (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ClaudeThinkingConfig>,

    /// metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ClaudeMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

// ─── Response ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,  // "message"
    pub role: String,           // "assistant"
    pub model: String,
    pub content: Vec<ClaudeContentBlock>,
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    pub usage: ClaudeUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

// ─── Error response ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeErrorResponse {
    #[serde(rename = "type")]
    pub response_type: String,  // "error"
    pub error: ClaudeErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeErrorDetail {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}

// ─── Streaming event types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeStreamEvent {
    MessageStart {
        message: ClaudeStreamMessage,
    },
    ContentBlockStart {
        index: u32,
        content_block: ClaudeStreamContentBlock,
    },
    ContentBlockDelta {
        index: u32,
        delta: ClaudeStreamDelta,
    },
    ContentBlockStop {
        index: u32,
    },
    MessageDelta {
        delta: ClaudeMessageDeltaPayload,
        usage: ClaudeStreamUsage,
    },
    MessageStop,
    Ping,
    Error {
        error: ClaudeErrorDetail,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeStreamMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub role: String,
    pub model: String,
    pub content: Vec<serde_json::Value>,
    pub stop_reason: Option<String>,
    pub usage: ClaudeStreamUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeStreamContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClaudeStreamDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
    SignatureDelta { signature: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessageDeltaPayload {
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeStreamUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}
