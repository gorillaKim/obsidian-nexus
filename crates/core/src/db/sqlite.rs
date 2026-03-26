use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use crate::config::Config;
use crate::error::Result;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Register sqlite-vec as auto extension (called once before any connections)
fn register_sqlite_vec() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });
}

/// Public wrapper for tests to register sqlite-vec
pub fn register_sqlite_vec_for_test() {
    register_sqlite_vec();
}

/// Create a connection pool to the SQLite database
pub fn create_pool() -> Result<DbPool> {
    Config::ensure_dirs()?;
    let db_path = Config::db_path();

    // Register sqlite-vec globally before creating any connections
    register_sqlite_vec();

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

    if version < 3 {
        tracing::info!("Running migration V3: sqlite-vec");
        // sqlite-vec extension must be loaded before creating vec0 virtual table
        let config = Config::load().unwrap_or_default();
        let dimensions = config.embedding.dimensions;
        conn.execute_batch(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
                chunk_id TEXT PRIMARY KEY,
                embedding float[{dimensions}]
            );"
        ))?;
        conn.execute("INSERT INTO schema_version (version) VALUES (?1)", params![3])?;
    }

    if version < 4 {
        tracing::info!("Running migration V4: wiki links and aliases");
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../../migrations/V4__links.sql"))?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?1)", params![4])?;
        tx.commit()?;
    }

    if version < 5 {
        tracing::info!("Running migration V5: search enhancements");
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../../migrations/V5__search_enhancements.sql"))?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?1)", params![5])?;
        tx.commit()?;
    }

    if version < 6 {
        tracing::info!("Running migration V6: FTS5 aliases column");
        let tx = conn.transaction()?;
        tx.execute_batch(include_str!("../../migrations/V6__fts_aliases.sql"))?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?1)", params![6])?;
        tx.commit()?;
    }

    const LATEST_VERSION: i32 = 6;
    tracing::info!("Database schema is up to date (version {})", LATEST_VERSION);
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
