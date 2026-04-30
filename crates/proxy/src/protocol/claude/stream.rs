/// Claude SSE 流解析工具函数
use crate::protocol::claude::types::ClaudeStreamEvent;

/// 从 SSE 行中解析 Claude 事件
pub fn parse_sse_line(line: &str) -> Option<ClaudeStreamEvent> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    serde_json::from_str(data).ok()
}
