/// OpenRouter adapter
/// https://openrouter.ai/api/v1/chat/completions
/// OpenAI 兼容，额外支持 HTTP-Referer / X-Title 头
use async_trait::async_trait;
use llm_core::{
    error::ProxyError,
    provider::{ExecContext, ProviderAdapter, StreamResult},
    schema::{CanonicalRequest, CanonicalResponse},
};

pub struct OpenRouterAdapter;

#[async_trait]
impl ProviderAdapter for OpenRouterAdapter {
    fn id(&self) -> &str {
        "openrouter"
    }

    async fn complete(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<CanonicalResponse, ProxyError> {
        super::openai::OpenAiAdapter.complete(req, ctx).await
    }

    async fn complete_stream(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<StreamResult, ProxyError> {
        super::openai::OpenAiAdapter.complete_stream(req, ctx).await
    }
}
