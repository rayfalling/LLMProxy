use crate::multimodal::pipeline::MultimodalRouter;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub multimodal: Arc<MultimodalRouter>,
}
