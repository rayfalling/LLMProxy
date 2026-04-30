/// Xiaomi AI (Mi AI) adapter
/// OpenAI 兼容接口，base_url = https://api.mistral.ai (示例) 或小米实际端点
/// 认证: Authorization: Bearer <key>
use async_trait::async_trait;
use llm_core::{
    error::ProxyError,
    provider::{ExecContext, ProviderAdapter, StreamResult},
    schema::{CanonicalRequest, CanonicalResponse},
};

pub struct XiaomiAdapter;

#[async_trait]
impl ProviderAdapter for XiaomiAdapter {
    fn id(&self) -> &str {
        "xiaomi"
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
