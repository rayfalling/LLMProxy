/// DeepSeek official adapter
/// https://api.deepseek.com/v1/chat/completions
/// OpenAI 兼容接口
use async_trait::async_trait;
use llm_core::{
    error::ProxyError,
    provider::{ExecContext, ProviderAdapter, StreamResult},
    schema::{CanonicalRequest, CanonicalResponse},
};

pub struct DeepSeekAdapter;

#[async_trait]
impl ProviderAdapter for DeepSeekAdapter {
    fn id(&self) -> &str {
        "deepseek"
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
