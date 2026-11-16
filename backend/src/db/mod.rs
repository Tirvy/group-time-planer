use std::time::Duration;

use sqlx::{
    SqlitePool,
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

use crate::config::Config;

/// SQLite is the only datastore in the MVP. These constants define the connection
/// behaviour required by the architecture document.
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Write transactions acquire SQLite's write lock up front, so they cannot fail
/// later while upgrading a read lock to a write lock.
pub const BEGIN_IMMEDIATE: &str = "BEGIN IMMEDIATE";

/// Migrations live at the repository root and are embedded at compile time.
pub static MIGRATOR: Migrator = sqlx::migrate!("../migrations");

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("failed to create database directory {path}: {source}")]
    CreateDir {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to open SQLite database {path}: {source}")]
    Connect {
        path: std::path::PathBuf,
        #[source]
        source: sqlx::Error,
    },
    #[error("SQLite invariant check failed: {0}")]
    Invariant(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

/// Ensures the data directory exists and opens a bounded pool.
///
/// The database file is created only when missing; existing data is never deleted
/// or recreated by startup, migrations or deployment.
pub async fn connect(config: &Config) -> Result<SqlitePool, DbError> {
    if let Some(dir) = config.database_dir() {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|source| DbError::CreateDir {
                path: dir.to_path_buf(),
                source,
            })?;
    }

    let options = SqliteConnectOptions::new()
        .filename(&config.database_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(BUSY_TIMEOUT);

    let pool = SqlitePoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_with(options)
        .await
        .map_err(|source| DbError::Connect {
            path: config.database_path.clone(),
            source,
        })?;

    verify_pragmas(&pool).await?;
    Ok(pool)
}

/// Applies pending migrations. Migrations are idempotent and never drop or
/// recreate tables, so existing data survives restarts and upgrades.
pub async fn migrate(pool: &SqlitePool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

/// Verifies that per-connection and database-wide settings are actually active.
/// This guards against configuration silently not being applied.
pub async fn verify_pragmas(pool: &SqlitePool) -> Result<(), DbError> {
    let mut connection = pool.acquire().await?;

    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(&mut *connection)
        .await?;
    if foreign_keys != 1 {
        return Err(DbError::Invariant(format!(
            "foreign_keys expected 1, got {foreign_keys}"
        )));
    }

    let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
        .fetch_one(&mut *connection)
        .await?;
    if busy_timeout < BUSY_TIMEOUT.as_millis() as i64 {
        return Err(DbError::Invariant(format!(
            "busy_timeout expected at least {} ms, got {busy_timeout} ms",
            BUSY_TIMEOUT.as_millis()
        )));
    }

    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&mut *connection)
        .await?;
    if !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(DbError::Invariant(format!(
            "journal_mode expected wal, got {journal_mode}"
        )));
    }

    Ok(())
}

/// Cheap liveness probe used by the health endpoint.
pub async fn ping(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(path: std::path::PathBuf, max_connections: u32) -> Config {
        Config {
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            database_path: path,
            db_max_connections: max_connections,
            ..Config::default()
        }
    }

    #[tokio::test]
    async fn creates_parent_directory_and_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/data/app.db");

        let pool = connect(&test_config(path.clone(), 2)).await.unwrap();

        assert!(path.exists(), "database file should be created on demand");
        assert!(
            dir.path().join("nested/data").is_dir(),
            "parent directory should be created automatically"
        );

        pool.close().await;
    }

    #[tokio::test]
    async fn applies_pragmas_on_every_connection() {
        let dir = tempfile::tempdir().unwrap();
        let pool = connect(&test_config(dir.path().join("app.db"), 2))
            .await
            .unwrap();

        let mut first = pool.acquire().await.unwrap();
        let mut second = pool.acquire().await.unwrap();

        for connection in [&mut *first, &mut *second] {
            let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
                .fetch_one(&mut *connection)
                .await
                .unwrap();
            assert_eq!(foreign_keys, 1);

            let busy_timeout: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
                .fetch_one(&mut *connection)
                .await
                .unwrap();
            assert!(busy_timeout >= BUSY_TIMEOUT.as_millis() as i64);
        }

        drop((first, second));
        pool.close().await;
    }

    #[tokio::test]
    async fn reuses_existing_database_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.db");

        let pool = connect(&test_config(path.clone(), 1)).await.unwrap();
        sqlx::query("CREATE TABLE probe (value INTEGER NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO probe (value) VALUES (42)")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let pool = connect(&test_config(path, 1)).await.unwrap();
        let value: i64 = sqlx::query_scalar("SELECT value FROM probe")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(value, 42, "existing data must survive a reconnect");
        pool.close().await;
    }

    #[tokio::test]
    async fn ping_reports_closed_pool_as_error() {
        let dir = tempfile::tempdir().unwrap();
        let pool = connect(&test_config(dir.path().join("app.db"), 1))
            .await
            .unwrap();
        pool.close().await;
        assert!(ping(&pool).await.is_err());
    }
}
