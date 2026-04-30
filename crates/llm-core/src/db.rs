use sqlx::sqlite::SqlitePoolOptions;
pub use sqlx::SqlitePool;

pub async fn connect_and_migrate(url: &str) -> Result<SqlitePool, sqlx::Error> {
    // 确保 SQLite 文件目录存在（file::memory: 不需要）
    if let Some(path) = url.strip_prefix("sqlite://") {
        if path != ":memory:" && !path.starts_with(":memory:") {
            if let Some(parent) = std::path::Path::new(path).parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(16)
        .connect_with(
            url.parse::<sqlx::sqlite::SqliteConnectOptions>()?
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .foreign_keys(true),
        )
        .await?;

    sqlx::migrate!("../../migrations").run(&pool).await.map_err(|e| {
        sqlx::Error::Configuration(format!("migration failed: {e}").into())
    })?;

    Ok(pool)
}

