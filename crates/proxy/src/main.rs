#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod auth;
mod handlers;
mod protocol;
mod providers;
mod routing;
mod multimodal;
mod state;

use axum::{routing::{get, post}, Router};
use llm_core::db::connect_and_migrate;
use std::{collections::{HashMap, HashSet}, env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    multimodal::pipeline::{ModelVisionConfig, MultimodalRouter},
    providers::{
        anthropic::AnthropicAdapter, copilot::CopilotAdapter, deepseek::DeepSeekAdapter,
        google::GoogleAdapter, openai::OpenAiAdapter, openrouter::OpenRouterAdapter,
        xiaomi::XiaomiAdapter,
    },
    routing::{
        failover::{FailoverEngine, ProviderKeyInfo},
        registry::AliasRegistry,
    },
    state::AppState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "proxy=info,tower_http=info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://llmproxy.db".to_string());
    let host = env::var("PROXY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PROXY_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);

    let pool = connect_and_migrate(&db_url).await?;
    let alias_registry = Arc::new(AliasRegistry::new(pool.clone()));
    alias_registry.reload().await?;

    let (providers_map, keys_map, key_pool_mapping, model_proxy_map, vision_config) =
        load_runtime_from_db(&pool).await?;

    let failover = Arc::new(FailoverEngine::new(
        providers_map,
        keys_map,
        key_pool_mapping,
        model_proxy_map,
    ));
    let multimodal = Arc::new(MultimodalRouter::new(failover, alias_registry, vision_config));

    let state = AppState { pool, multimodal };

    let app = Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/v1/chat/completions", post(handlers::openai_chat_completions))
        .route("/v1/messages", post(handlers::claude_messages))
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("proxy listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn load_runtime_from_db(
    pool: &sqlx::SqlitePool,
) -> anyhow::Result<(
    HashMap<String, llm_core::provider::DynProvider>,
    HashMap<String, Vec<ProviderKeyInfo>>,
    HashMap<String, HashSet<String>>,
    HashMap<(String, String), Option<String>>,
    HashMap<String, ModelVisionConfig>,
)> {
    #[derive(sqlx::FromRow)]
    struct ProviderRow {
        id: String,
        name: String,
        base_url: String,
        enabled: i64,
    }

    #[derive(sqlx::FromRow)]
    struct ProviderKeyRow {
        id: String,
        provider_id: String,
        key_ref: String,
        enabled: i64,
    }

    #[derive(sqlx::FromRow)]
    struct KeyPoolRow {
        api_key_id: String,
        provider_key_id: String,
    }

    #[derive(sqlx::FromRow)]
    struct ModelRow {
        provider_id: String,
        model_name: String,
        supports_vision: i64,
        outbound_proxy_id: Option<String>,
    }

    #[derive(sqlx::FromRow)]
    struct ProxyRow {
        id: String,
        scheme: String,
        host: String,
        port: i64,
        username: Option<String>,
        password: Option<String>,
    }

    #[derive(sqlx::FromRow)]
    struct VisionMapRow {
        model_name: String,
        vision_parser_alias: Option<String>,
        generation_alias: Option<String>,
    }

    let provider_rows: Vec<ProviderRow> = sqlx::query_as(
        "SELECT id, name, base_url, enabled FROM providers WHERE enabled = 1",
    )
    .fetch_all(pool)
    .await?;

    let key_rows: Vec<ProviderKeyRow> = sqlx::query_as(
        "SELECT id, provider_id, key_ref, enabled FROM provider_keys WHERE enabled = 1",
    )
    .fetch_all(pool)
    .await?;

    let key_pool_rows: Vec<KeyPoolRow> = sqlx::query_as(
        "SELECT api_key_id, provider_key_id FROM api_key_provider_key_pools",
    )
    .fetch_all(pool)
    .await?;

    let model_rows: Vec<ModelRow> = sqlx::query_as(
        "SELECT provider_id, model_name, supports_vision, outbound_proxy_id FROM provider_models",
    )
    .fetch_all(pool)
    .await?;

    let proxy_rows: Vec<ProxyRow> = sqlx::query_as(
        "SELECT id, scheme, host, port, username, password FROM outbound_proxies WHERE enabled = 1",
    )
    .fetch_all(pool)
    .await?;

    let vision_map_rows: Vec<VisionMapRow> = sqlx::query_as(
        "SELECT model_name, vision_parser_alias, generation_alias FROM model_vision_mappings",
    )
    .fetch_all(pool)
    .await?;

    let proxy_url_map: HashMap<String, String> = proxy_rows
        .into_iter()
        .map(|r| {
            let auth = match (r.username.as_deref(), r.password.as_deref()) {
                (Some(u), Some(p)) => format!("{u}:{p}@"),
                _ => String::new(),
            };
            (r.id, format!("{}://{}{}:{}", r.scheme, auth, r.host, r.port))
        })
        .collect();

    let mut providers_map: HashMap<String, llm_core::provider::DynProvider> = HashMap::new();
    let mut provider_base: HashMap<String, String> = HashMap::new();

    for p in provider_rows {
        provider_base.insert(p.id.clone(), p.base_url.clone());
        let adapter: Option<llm_core::provider::DynProvider> = match p.name.as_str() {
            "openai" => Some(Arc::new(OpenAiAdapter)),
            "anthropic" => Some(Arc::new(AnthropicAdapter)),
            "copilot" => Some(Arc::new(CopilotAdapter)),
            "xiaomi" => Some(Arc::new(XiaomiAdapter)),
            "google" => Some(Arc::new(GoogleAdapter)),
            "openrouter" => Some(Arc::new(OpenRouterAdapter)),
            "deepseek" => Some(Arc::new(DeepSeekAdapter)),
            _ => None,
        };
        if let Some(a) = adapter {
            providers_map.insert(p.id, a);
        }
    }

    let mut keys_map: HashMap<String, Vec<ProviderKeyInfo>> = HashMap::new();
    for k in key_rows {
        let Some(base_url) = provider_base.get(&k.provider_id).cloned() else {
            continue;
        };
        keys_map.entry(k.provider_id.clone()).or_default().push(ProviderKeyInfo {
            provider_key_id: k.id,
            api_key: k.key_ref,
            outbound_proxy: None,
            base_url,
            extra_headers: vec![],
            provider_id: k.provider_id,
        });
    }

    let mut key_pool_mapping: HashMap<String, HashSet<String>> = HashMap::new();
    for r in key_pool_rows {
        key_pool_mapping
            .entry(r.api_key_id)
            .or_default()
            .insert(r.provider_key_id);
    }

    let mut model_proxy_map: HashMap<(String, String), Option<String>> = HashMap::new();
    let mut vision_config: HashMap<String, ModelVisionConfig> = HashMap::new();

    for m in model_rows {
        let outbound_proxy = m
            .outbound_proxy_id
            .as_ref()
            .and_then(|id| proxy_url_map.get(id))
            .cloned();

        model_proxy_map.insert((m.provider_id.clone(), m.model_name.clone()), outbound_proxy);

        vision_config.insert(
            m.model_name,
            ModelVisionConfig {
                supports_vision: m.supports_vision != 0,
                vision_parser_alias: None,
            },
        );
    }

    for row in vision_map_rows {
        if let Some(cfg) = vision_config.get_mut(&row.model_name) {
            cfg.vision_parser_alias = row.vision_parser_alias;
        } else {
            vision_config.insert(
                row.model_name,
                ModelVisionConfig {
                    supports_vision: false,
                    vision_parser_alias: row.vision_parser_alias,
                },
            );
        }

        let _ = row.generation_alias;
    }

    Ok((providers_map, keys_map, key_pool_mapping, model_proxy_map, vision_config))
}
