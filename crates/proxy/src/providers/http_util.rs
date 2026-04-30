/// 根据 ExecContext 构建带出站代理支持的 reqwest::Client
use llm_core::provider::ExecContext;
use reqwest::Client;

pub fn build_client(ctx: &ExecContext) -> Result<Client, reqwest::Error> {
    let mut builder = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(10));

    if let Some(proxy_url) = &ctx.outbound_proxy {
        let proxy = reqwest::Proxy::all(proxy_url.as_str())
            .unwrap_or_else(|_| reqwest::Proxy::all("http://invalid").unwrap());
        builder = builder.proxy(proxy);
    }

    builder.build()
}

/// 将 SSE 响应体解析为行迭代（过滤空行和注释行）
pub fn sse_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| !l.is_empty() && !l.starts_with(':'))
}
