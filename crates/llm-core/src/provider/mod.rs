use async_trait::async_trait;
use crate::schema::{CanonicalRequest, CanonicalResponse, CanonicalStreamChunk};
use crate::error::ProxyError;
use futures::Stream;
use std::pin::Pin;

pub type StreamResult = Pin<Box<dyn Stream<Item = Result<CanonicalStreamChunk, ProxyError>> + Send>>;

/// 所有上游提供商必须实现此 trait
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// 提供商唯一标识，如 "openai" / "anthropic"
    fn id(&self) -> &str;

    /// 非流式请求
    async fn complete(&self, req: &CanonicalRequest) -> Result<CanonicalResponse, ProxyError>;

    /// 流式请求
    async fn complete_stream(&self, req: &CanonicalRequest) -> Result<StreamResult, ProxyError>;

    /// 检测此 adapter 是否在线（用于健康检查）
    async fn health_check(&self) -> bool {
        true
    }
}

/// 提供商模型能力描述
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub model_name: String,
    pub supports_vision: bool,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub context_window: u32,
    pub max_output_tokens: u32,
}
