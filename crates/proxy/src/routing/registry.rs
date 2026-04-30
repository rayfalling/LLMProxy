/// 逻辑模型别名注册表
/// 从数据库加载 model_alias 及其 targets，用于路由决策
use llm_core::error::ProxyError;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 单个路由目标
#[derive(Debug, Clone)]
pub struct AliasTarget {
    pub provider_id: String,
    pub provider_model: String,
    /// 越小优先级越高
    pub priority: i32,
    pub enabled: bool,
}

/// 别名路由策略
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteStrategy {
    /// 按固定优先级，失败则 failover
    Priority,
    /// 最低延迟（需要历史统计，降级为 Priority）
    Latency,
    /// 最低成本
    Cost,
}

impl From<&str> for RouteStrategy {
    fn from(s: &str) -> Self {
        match s {
            "latency" => RouteStrategy::Latency,
            "cost" => RouteStrategy::Cost,
            _ => RouteStrategy::Priority,
        }
    }
}

/// 一个逻辑别名的路由配置
#[derive(Debug, Clone)]
pub struct ModelAlias {
    pub alias_name: String,
    pub route_strategy: RouteStrategy,
    /// 按 priority ASC 排序
    pub targets: Vec<AliasTarget>,
    /// 触发 failover 的条件集合（数据库 failover_rules 表）
    pub failover_triggers: Vec<String>,
}

/// 全局别名注册表（内存缓存，定期刷新）
pub struct AliasRegistry {
    pool: SqlitePool,
    cache: RwLock<HashMap<String, Arc<ModelAlias>>>,
}

impl AliasRegistry {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// 从数据库加载所有别名到内存
    pub async fn reload(&self) -> Result<(), ProxyError> {
            #[derive(sqlx::FromRow)]
            struct AliasRow {
                alias_name: String,
                route_strategy: Option<String>,
                provider_id: String,
                provider_model: String,
                priority: i64,
                target_enabled: i64,
            }

            let rows: Vec<AliasRow> = sqlx::query_as(
                r#"SELECT ma.alias_name, ma.route_strategy,
                   mat.provider_id, mat.model_name AS provider_model,
                   mat.priority, mat.enabled AS target_enabled
                   FROM model_aliases ma
                   JOIN model_alias_targets mat ON mat.alias_id = ma.id
                   ORDER BY ma.alias_name, mat.priority ASC"#,
            )
            .fetch_all(&self.pool)
            .await?;

        let mut map: HashMap<String, ModelAlias> = HashMap::new();

        for row in rows {
            let entry = map.entry(row.alias_name.clone()).or_insert_with(|| ModelAlias {
                alias_name: row.alias_name.clone(),
                route_strategy: RouteStrategy::from(
                    row.route_strategy.as_deref().unwrap_or("priority"),
                ),
                targets: vec![],
                failover_triggers: vec![],
            });

            entry.targets.push(AliasTarget {
                provider_id: row.provider_id,
                provider_model: row.provider_model,
                    priority: row.priority as i32,
                    enabled: row.target_enabled != 0,
            });
        }

        // 加载 failover_rules
            #[derive(sqlx::FromRow)]
            struct RuleRow {
                alias_name: String,
                trigger: String,
            }

            let rules: Vec<RuleRow> = sqlx::query_as(
                "SELECT ma.alias_name, fr.trigger FROM failover_rules fr \
                 JOIN model_aliases ma ON ma.id = fr.alias_id WHERE fr.enabled = 1",
            )
            .fetch_all(&self.pool)
            .await?;

        for rule in rules {
            if let Some(entry) = map.get_mut(&rule.alias_name) {
                entry.failover_triggers.push(rule.trigger);
            }
        }

        let mut cache = self.cache.write().await;
        *cache = map.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();
        Ok(())
    }

    /// 查找别名，如果不存在则视为直连（alias_name == provider model name）
    pub async fn resolve(&self, model: &str) -> Option<Arc<ModelAlias>> {
        let cache = self.cache.read().await;
        cache.get(model).cloned()
    }
}
