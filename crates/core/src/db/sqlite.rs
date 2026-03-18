use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use crate::config::Config;
use crate::error::Result;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Create a connection pool to the SQLite database
pub fn create_pool() -> Result<DbPool> {
    Config::ensure_dirs()?;
    let db_path = Config::db_path();

    // Use with_init to apply PRAGMAs to EVERY connection in the pool
    let manager = SqliteConnectionManager::file(&db_path)
        .with_init(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA busy_timeout=5000;
                 PRAGMA foreign_keys=ON;
                 PRAGMA synchronous=NORMAL;"
            )
        });

    let pool = Pool::builder()
        .max_size(10)
        .build(manager)?;

    Ok(pool)
}

/// Run database migrations
pub fn run_migrations(pool: &DbPool) -> Result<()> {
    let mut conn = pool.get()?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );

        INSERT OR IGNORE INTO schema_version (version) VALUES (0);"
    )?;

    let version: i64 = conn.query_row(
        "SELECT MAX(version) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    if version < 1 {
        tracing::info!("Running migration V1: initial schema");
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../../migrations/V1__initial.sql"))?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?1)", params![1])?;
        tx.commit()?;
    }

    if version < 2 {
        tracing::info!("Running migration V2: embeddings");
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../../migrations/V2__embeddings.sql"))?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?1)", params![2])?;
        tx.commit()?;
    }

    tracing::info!("Database schema is up to date (version {})", std::cmp::max(version, 1));
    Ok(())
}

/// Helper: run a blocking DB operation on a separate thread (for async contexts)
pub async fn blocking<F, T>(pool: &DbPool, f: F) -> Result<T>
where
    F: FnOnce(&rusqlite::Connection) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        f(&conn)
    })
    .await
    .map_err(|e| crate::error::NexusError::Indexing(format!("task panicked: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pool_and_migrate() {
        // Use in-memory for testing
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::builder().max_size(1).build(manager).unwrap();

        let conn = pool.get().unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;"
        ).unwrap();

        // Run migration SQL directly
        conn.execute_batch(include_str!("../../migrations/V1__initial.sql")).unwrap();

        // Verify tables exist
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(count, 1);
    }
}
