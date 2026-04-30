/// Copilot / Azure OpenAI adapter
/// base_url 格式: https://<resource>.openai.azure.com/openai/deployments/<deployment>
/// 认证: api-key header (Azure) 或 Bearer token (GitHub Copilot)
use async_trait::async_trait;
use llm_core::{
    error::ProxyError,
    provider::{ExecContext, ProviderAdapter, StreamResult},
    schema::{CanonicalRequest, CanonicalResponse},
};

/// 复用 OpenAI adapter 的转换逻辑，只修改 URL 和认证头
pub struct CopilotAdapter;

#[async_trait]
impl ProviderAdapter for CopilotAdapter {
    fn id(&self) -> &str {
        "copilot"
    }

    async fn complete(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<CanonicalResponse, ProxyError> {
        // Azure endpoint 已在 ctx.base_url 中设置，直接转发给 OpenAI adapter 逻辑
        // 通过包装的 ctx 切换认证方式
        let ctx = azure_ctx(ctx);
        super::openai::OpenAiAdapter.complete(req, &ctx).await
    }

    async fn complete_stream(
        &self,
        req: &CanonicalRequest,
        ctx: &ExecContext,
    ) -> Result<StreamResult, ProxyError> {
        let ctx = azure_ctx(ctx);
        super::openai::OpenAiAdapter.complete_stream(req, &ctx).await
    }
}

/// Azure 用 api-key header 而非 Bearer，通过 extra_headers 注入
fn azure_ctx(ctx: &ExecContext) -> ExecContext {
    let mut headers = ctx.extra_headers.clone();
    // 移除已有的 authorization（OpenAI adapter 会加 Bearer），改用 api-key
    headers.push(("api-key".to_string(), ctx.api_key.clone()));
    ExecContext {
        api_key: String::new(),  // OpenAI adapter 会生成空的 Bearer header，Azure 忽略它
        outbound_proxy: ctx.outbound_proxy.clone(),
        base_url: ctx.base_url.clone(),
        extra_headers: headers,
    }
}
