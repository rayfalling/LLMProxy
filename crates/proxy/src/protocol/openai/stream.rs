/// OpenAI SSE 流解析
use super::types::OpenAiStreamChunk;

/// 从 SSE 行中解析 OpenAI stream chunk
pub fn parse_sse_line(line: &str) -> Option<OpenAiStreamChunk> {
    let data = line.strip_prefix("data: ")?;
    if data.trim() == "[DONE]" {
        return None;
    }
    serde_json::from_str(data).ok()
}
