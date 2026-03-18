/// Test helpers: in-memory DB pool with all migrations applied
#[cfg(test)]
pub mod helpers {
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;

    use crate::db::sqlite::DbPool;

    /// Create an in-memory database pool with all migrations applied
    pub fn test_pool() -> DbPool {
        let manager = SqliteConnectionManager::memory()
            .with_init(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode=WAL;
                     PRAGMA foreign_keys=ON;
                     PRAGMA busy_timeout=5000;"
                )
            });
        let pool = Pool::builder().max_size(1).build(manager).unwrap();

        let mut conn = pool.get().unwrap();

        // V1
        conn.execute_batch(include_str!("../migrations/V1__initial.sql")).unwrap();
        // V2
        conn.execute_batch(include_str!("../migrations/V2__embeddings.sql")).unwrap();

        pool
    }

    /// Create a temporary vault directory with sample markdown files
    pub fn create_test_vault() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("note1.md"),
            "---\ntitle: First Note\ntags:\n  - rust\n  - programming\n---\n\n# First Note\n\n## Introduction\n\nThis is the first note about Rust programming.\n\n## Details\n\nRust has a unique ownership system that ensures memory safety.\n",
        ).unwrap();

        std::fs::write(
            dir.path().join("note2.md"),
            "---\ntitle: Second Note\ntags:\n  - python\n---\n\n# Second Note\n\n## Overview\n\nPython is a popular programming language for data science.\n",
        ).unwrap();

        std::fs::write(
            dir.path().join("korean.md"),
            "---\ntitle: 한국어 노트\ntags:\n  - 한국어\n  - 테스트\n---\n\n# 한국어 테스트\n\n## 소개\n\n이것은 한국어로 작성된 테스트 문서입니다.\n멀티바이트 문자가 포함되어 있습니다.\n",
        ).unwrap();

        // Create a subdirectory with a note
        std::fs::create_dir_all(dir.path().join("subfolder")).unwrap();
        std::fs::write(
            dir.path().join("subfolder").join("nested.md"),
            "# Nested Note\n\nThis note is in a subfolder.\n",
        ).unwrap();

        // Create an excluded directory
        std::fs::create_dir_all(dir.path().join(".obsidian")).unwrap();
        std::fs::write(
            dir.path().join(".obsidian").join("config.json"),
            "{}",
        ).unwrap();

        dir
    }
}
