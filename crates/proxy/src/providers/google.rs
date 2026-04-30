/// Google Gemini adapter
/// 使用 Gemini OpenAI-compat endpoint: https://generativelanguage.googleapis.com/v1beta/openai/
/// 认证: Authorization: Bearer <key>
use async_trait::async_trait;
use llm_core::{
    error::ProxyError,
    provider::{ExecContext, ProviderAdapter, StreamResult},
    schema::{CanonicalRequest, CanonicalResponse},
};

pub struct GoogleAdapter;

#[async_trait]
impl ProviderAdapter for GoogleAdapter {
    fn id(&self) -> &str {
        "google"
    }

    async fn complete(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<CanonicalResponse, ProxyError> {
        // Gemini OpenAI compat 层，直接复用 OpenAI adapter
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
