use async_trait::async_trait;
use crate::schema::{CanonicalRequest, CanonicalResponse, CanonicalStreamChunk};
use crate::error::ProxyError;
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;

pub type StreamResult = Pin<Box<dyn Stream<Item = Result<CanonicalStreamChunk, ProxyError>> + Send>>;

/// 调用上游时的执行上下文：包含该次调用所用的密钥和代理配置
#[derive(Debug, Clone)]
pub struct ExecContext {
    /// 该次调用使用的上游 API key
    pub api_key: String,
    /// 可选出站 SOCKS5/HTTP 代理 URL（如 "socks5://127.0.0.1:1080"）
    pub outbound_proxy: Option<String>,
    /// 该上游 provider 的 base URL（支持覆盖）
    pub base_url: String,
    /// 额外的自定义请求头（如 anthropic-beta）
    pub extra_headers: Vec<(String, String)>,
}

/// 所有上游提供商必须实现此 trait
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// 提供商唯一标识，如 "openai" / "anthropic"
    fn id(&self) -> &str;

    /// 非流式请求
    async fn complete(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<CanonicalResponse, ProxyError>;

    /// 流式请求
    async fn complete_stream(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<StreamResult, ProxyError>;

    /// 检测此 adapter + key 组合是否在线
    async fn health_check(&self, ctx: &ExecContext) -> bool {
        let _ = ctx;
        true
    }
}

pub type DynProvider = Arc<dyn ProviderAdapter>;

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
