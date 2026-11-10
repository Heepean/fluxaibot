//! SQLite persistence layer.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::ConnectOptions;
use std::path::PathBuf;
use std::result::Result as StdResult;

pub type StorageResult<T> = StdResult<T, String>;

pub struct Storage {
    pub pool: sqlx::SqlitePool,
    pub data_dir: PathBuf,
}

impl Storage {
    pub async fn new(data_dir: PathBuf) -> StorageResult<Self> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data dir {:?}: {}", data_dir, e))?;
        let options = SqliteConnectOptions::new()
            .filename(data_dir.join("fluxaibot.sqlite3"))
            .create_if_missing(true)
            .disable_statement_logging();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .map_err(|e| format!("Failed to connect to application database: {}", e))?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| format!("Failed to run migrations: {}", e))?;

        Ok(Storage { pool, data_dir })
    }

    pub fn pool(&self) -> &sqlx::SqlitePool {
        &self.pool
    }

    pub async fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        sqlx::query_scalar("SELECT value FROM _meta WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| format!("Failed to read application setting {key}: {e}"))
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO _meta (key, value, updated_at) VALUES (?, ?, datetime('now')) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("Failed to save application setting {key}: {e}"))?;
        Ok(())
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir.join("backups")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.data_dir.join("runtime")
    }

    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn database_is_persistent_between_connections() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("fluxaibot.sqlite3");

        let storage = Storage::new(directory.path().to_path_buf()).await.unwrap();
        assert!(database_path.is_file());
        storage.close().await;

        let reopened = Storage::new(directory.path().to_path_buf()).await.unwrap();
        reopened.close().await;
    }

    #[tokio::test]
    async fn settings_are_persistent_between_connections() {
        let directory = tempfile::tempdir().unwrap();
        let storage = Storage::new(directory.path().to_path_buf()).await.unwrap();
        storage
            .set_setting("models_folder", r"C:\Users\user\Models")
            .await
            .unwrap();
        storage.close().await;

        let reopened = Storage::new(directory.path().to_path_buf()).await.unwrap();
        assert_eq!(
            reopened.get_setting("models_folder").await.unwrap(),
            Some(r"C:\Users\user\Models".to_string())
        );
        reopened.close().await;
    }
}
