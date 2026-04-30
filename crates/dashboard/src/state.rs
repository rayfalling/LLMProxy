use llm_core::config::AuthConfig;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub auth: Arc<AuthConfig>,
}
