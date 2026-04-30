/// 固定优先级 failover 引擎
/// 按 AliasTarget priority 顺序尝试，遇到可 failover 的错误则切换下一个
use llm_core::{
    error::{FailoverTrigger, ProxyError},
    provider::{DynProvider, ExecContext},
    schema::{CanonicalRequest, CanonicalResponse},
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::registry::{AliasTarget, ModelAlias, RouteStrategy};

/// 每个 provider_key 在数据库中的运行时信息
#[derive(Debug, Clone)]
pub struct ProviderKeyInfo {
    pub provider_key_id: String,
    pub api_key: String,
    pub outbound_proxy: Option<String>,
    pub base_url: String,
    pub extra_headers: Vec<(String, String)>,
    /// 按 provider_id 索引
    pub provider_id: String,
}

/// Failover 引擎
pub struct FailoverEngine {
    /// provider_id -> DynProvider
    providers: HashMap<String, DynProvider>,
    /// provider_id -> 可用 key 列表（轮询或单 key）
    keys: HashMap<String, Vec<ProviderKeyInfo>>,
    /// downstream api_key_id -> allowed provider_key_ids
    key_pool_mapping: HashMap<String, HashSet<String>>,
    /// (provider_id, provider_model) -> outbound proxy url
    model_outbound_proxy: HashMap<(String, String), Option<String>>,
    /// 禁用的 provider_id 集合（手动 disable）
    disabled: std::sync::RwLock<std::collections::HashSet<String>>,
}

impl FailoverEngine {
    pub fn new(
        providers: HashMap<String, DynProvider>,
        keys: HashMap<String, Vec<ProviderKeyInfo>>,
        key_pool_mapping: HashMap<String, HashSet<String>>,
        model_outbound_proxy: HashMap<(String, String), Option<String>>,
    ) -> Self {
        Self {
            providers,
            keys,
            key_pool_mapping,
            model_outbound_proxy,
            disabled: std::sync::RwLock::new(Default::default()),
        }
    }

    pub fn disable_provider(&self, provider_id: &str) {
        self.disabled.write().unwrap().insert(provider_id.to_string());
    }

    pub fn enable_provider(&self, provider_id: &str) {
        self.disabled.write().unwrap().remove(provider_id);
    }

    /// 执行带 failover 的请求
    pub async fn execute(
        &self,
        alias: &ModelAlias,
        req: &CanonicalRequest,
    ) -> Result<CanonicalResponse, ProxyError> {
        let targets = self.order_targets(alias);

        let mut last_err = ProxyError::AllProvidersExhausted {
            model: req.model.clone(),
        };

        for target in &targets {
            if !target.enabled {
                continue;
            }

            // 检查手动禁用
            if self.disabled.read().unwrap().contains(&target.provider_id) {
                continue;
            }

            let Some(adapter) = self.providers.get(&target.provider_id) else {
                continue;
            };

            let Some(key_info) = self.pick_key(req.api_key_id.to_string().as_str(), &target.provider_id) else {
                continue;
            };

            let mut req = req.clone();
            req.model = target.provider_model.clone();

            let outbound_proxy = self
                .model_outbound_proxy
                .get(&(target.provider_id.clone(), target.provider_model.clone()))
                .cloned()
                .unwrap_or_else(|| key_info.outbound_proxy.clone());

            let ctx = ExecContext {
                api_key: key_info.api_key.clone(),
                outbound_proxy,
                base_url: key_info.base_url.clone(),
                extra_headers: key_info.extra_headers.clone(),
            };

            match adapter.complete(&req, &ctx).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    if self.should_failover(&e, alias) {
                        last_err = e;
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        Err(last_err)
    }

    /// 根据 route_strategy 对 targets 排序
    fn order_targets<'a>(&self, alias: &'a ModelAlias) -> Vec<&'a AliasTarget> {
        let mut targets: Vec<&AliasTarget> = alias.targets.iter().collect();

        match alias.route_strategy {
            RouteStrategy::Priority | RouteStrategy::Latency | RouteStrategy::Cost => {
                // 目前均按 priority ASC（latency/cost 需历史数据支持，后续优化）
                targets.sort_by_key(|t| t.priority);
            }
        }

        targets
    }

    fn pick_key(&self, downstream_api_key_id: &str, provider_id: &str) -> Option<&ProviderKeyInfo> {
        let keys = self.keys.get(provider_id)?;

        match self.key_pool_mapping.get(downstream_api_key_id) {
            Some(allowed) if !allowed.is_empty() => keys
                .iter()
                .find(|k| allowed.contains(&k.provider_key_id)),
            _ => keys.first(),
        }
    }

    fn should_failover(&self, err: &ProxyError, alias: &ModelAlias) -> bool {
        let trigger = match err {
            ProxyError::UpstreamError { trigger, .. } => trigger.as_ref(),
            _ => return false,
        };

        let Some(trigger) = trigger else {
            return false;
        };

        let trigger_str = match trigger {
            FailoverTrigger::InsufficientBalance => "insufficient_balance",
            FailoverTrigger::RateLimited => "rate_limited",
            FailoverTrigger::UpstreamServerError => "server_error",
            FailoverTrigger::Timeout => "timeout",
            FailoverTrigger::ModelOffline => "model_offline",
            FailoverTrigger::ManuallyDisabled => "manually_disabled",
        };

        alias.failover_triggers.iter().any(|t| t == trigger_str)
            || alias.failover_triggers.is_empty()  // 默认所有触发器都 failover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::registry::{AliasTarget, ModelAlias, RouteStrategy};

    #[test]
    fn failover_trigger_mapping_matches_rule() {
        let engine = FailoverEngine::new(HashMap::new(), HashMap::new(), HashMap::new(), HashMap::new());
        let alias = ModelAlias {
            alias_name: "gpt-4o".to_string(),
            route_strategy: RouteStrategy::Priority,
            targets: vec![AliasTarget {
                provider_id: "openai".to_string(),
                provider_model: "gpt-4o".to_string(),
                priority: 0,
                enabled: true,
            }],
            failover_triggers: vec!["rate_limited".to_string()],
        };

        let err = ProxyError::UpstreamError {
            provider: "openai".to_string(),
            status: 429,
            body: "rate limited".to_string(),
            trigger: Some(FailoverTrigger::RateLimited),
        };

        assert!(engine.should_failover(&err, &alias));
    }
}
