use thiserror::Error;

/// 统一错误分类（用于回退触发判断）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailoverTrigger {
    /// 余额不足 / 付费相关
    InsufficientBalance,
    /// 速率限制 429
    RateLimited,
    /// 上游 5xx
    UpstreamServerError,
    /// 请求超时
    Timeout,
    /// 模型已下线或不可用
    ModelOffline,
    /// 管理员手动禁用
    ManuallyDisabled,
}

/// 代理层核心错误类型
#[derive(Debug, Error)]
pub enum ProxyError {
    // ── 上游提供商错误 ──────────────────────────────────────────
    #[error("upstream provider error: {status} {body}")]
    UpstreamError {
        provider: String,
        status: u16,
        body: String,
        trigger: Option<FailoverTrigger>,
    },

    #[error("upstream request timeout after {timeout_ms}ms")]
    UpstreamTimeout { provider: String, timeout_ms: u64 },

    // ── 路由 / 配置错误 ─────────────────────────────────────────
    #[error("no eligible provider found for model `{model}`")]
    NoEligibleProvider { model: String },

    #[error("model `{model}` not found in alias registry")]
    ModelNotFound { model: String },

    #[error("all providers exhausted for model `{model}`")]
    AllProvidersExhausted { model: String },

    // ── 鉴权 / 租户 ─────────────────────────────────────────────
    #[error("invalid or expired API key")]
    InvalidApiKey,

    #[error("quota exceeded for key `{key_id}`")]
    QuotaExceeded { key_id: String },

    #[error("rate limit exceeded for key `{key_id}`")]
    RateLimitExceeded { key_id: String },

    #[error("model `{model}` not permitted for key `{key_id}`")]
    ModelNotPermitted { model: String, key_id: String },

    // ── 协议 / 请求错误 ─────────────────────────────────────────
    #[error("invalid request: {message}")]
    InvalidRequest { message: String },

    #[error("unsupported protocol: {protocol}")]
    UnsupportedProtocol { protocol: String },

    // ── 内部错误 ────────────────────────────────────────────────
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("http client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl ProxyError {
    /// 根据上游 HTTP 状态码推断回退触发类型
    pub fn classify_upstream(status: u16, body: &str) -> Option<FailoverTrigger> {
        match status {
            429 => Some(FailoverTrigger::RateLimited),
            500..=599 => Some(FailoverTrigger::UpstreamServerError),
            402 | 403 => {
                // 判断 body 关键词区分余额不足 vs 权限
                let lower = body.to_lowercase();
                if lower.contains("insufficient")
                    || lower.contains("balance")
                    || lower.contains("credit")
                    || lower.contains("quota")
                    || lower.contains("billing")
                {
                    Some(FailoverTrigger::InsufficientBalance)
                } else {
                    None
                }
            }
            404 => {
                let lower = body.to_lowercase();
                if lower.contains("model") && (lower.contains("not found") || lower.contains("decommission")) {
                    Some(FailoverTrigger::ModelOffline)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// 是否应该触发回退迁移
    pub fn should_failover(&self) -> bool {
        match self {
            ProxyError::UpstreamError { trigger, .. } => trigger.is_some(),
            ProxyError::UpstreamTimeout { .. } => true,
            _ => false,
        }
    }

    /// 对应的 HTTP 状态码（用于返回给客户端）
    pub fn http_status(&self) -> u16 {
        match self {
            ProxyError::InvalidApiKey => 401,
            ProxyError::QuotaExceeded { .. } => 429,
            ProxyError::RateLimitExceeded { .. } => 429,
            ProxyError::ModelNotPermitted { .. } => 403,
            ProxyError::InvalidRequest { .. } => 400,
            ProxyError::UnsupportedProtocol { .. } => 400,
            ProxyError::NoEligibleProvider { .. } => 503,
            ProxyError::ModelNotFound { .. } => 404,
            ProxyError::AllProvidersExhausted { .. } => 503,
            ProxyError::UpstreamError { status, .. } => *status,
            ProxyError::UpstreamTimeout { .. } => 504,
            _ => 500,
        }
    }
}
