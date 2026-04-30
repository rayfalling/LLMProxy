mod auth;
mod handlers;
mod state;

use axum::{routing::{get, post, put}, Router};
use llm_core::{config::AuthConfig, db::connect_and_migrate};
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            env::var("RUST_LOG").unwrap_or_else(|_| "dashboard=info,tower_http=info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://llmproxy.db".to_string());
    let host = env::var("DASHBOARD_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("DASHBOARD_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8081);

    let auth = AuthConfig {
        jwt_secret: env::var("JWT_SECRET").unwrap_or_else(|_| "change-me-in-production".to_string()),
        jwt_expiry_secs: env::var("JWT_EXPIRY_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24 * 3600),
    };

    let pool = connect_and_migrate(&db_url).await?;
    let state = AppState {
        pool,
        auth: Arc::new(auth),
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/api/auth/login", post(auth::login))
        .route("/api/me", get(handlers::me))
        .route("/api/providers", get(handlers::list_providers))
        .route("/api/providers/{provider_id}/enabled", put(handlers::set_provider_enabled))
        .route("/api/providers/{provider_id}/models", get(handlers::list_provider_models))
        .route(
            "/api/providers/{provider_id}/models/{model_name}/enabled",
            put(handlers::set_provider_model_enabled),
        )
        .route("/api/aliases", get(handlers::list_aliases))
        .route(
            "/api/aliases/{alias_name}/strategy",
            put(handlers::update_alias_route_strategy),
        )
        .route(
            "/api/aliases/{alias_name}/targets",
            put(handlers::update_alias_targets),
        )
        .route("/api/key-pools", get(handlers::list_key_pool_mappings))
        .route(
            "/api/key-pools/{api_key_id}",
            put(handlers::update_key_pool_mapping),
        )
        .route("/api/events/failovers", get(handlers::list_failover_events))
        .route("/api/stats", get(handlers::tenant_stats))
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("dashboard listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}
