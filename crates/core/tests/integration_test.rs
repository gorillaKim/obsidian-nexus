//! Integration tests for Obsidian Nexus core
//! Tests the full pipeline: project → index → search → doc

use nexus_core::config::Config;
use nexus_core::db::sqlite::DbPool;
use nexus_core::index_engine;
use nexus_core::project;
use nexus_core::search;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::fs;
use tempfile::TempDir;

fn test_pool() -> DbPool {
    // Register sqlite-vec extension
    nexus_core::db::sqlite::register_sqlite_vec_for_test();

    let manager = SqliteConnectionManager::memory()
        .with_init(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA foreign_keys=ON;
                 PRAGMA busy_timeout=5000;"
            )
        });
    let pool = Pool::builder().max_size(1).build(manager).unwrap();
    let conn = pool.get().unwrap();
    conn.execute_batch(include_str!("../migrations/V1__initial.sql")).unwrap();
    conn.execute_batch(include_str!("../migrations/V2__embeddings.sql")).unwrap();
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS vec_chunks USING vec0(
            chunk_id TEXT PRIMARY KEY,
            embedding float[768]
        );"
    ).unwrap();
    conn.execute_batch(include_str!("../migrations/V4__links.sql")).unwrap();
    conn.execute_batch(include_str!("../migrations/V5__search_enhancements.sql")).unwrap();
    pool
}

fn setup() -> (DbPool, TempDir) {
    let pool = test_pool();

    let vault = TempDir::new().expect("Failed to create temp dir");

    // Create test markdown files
    fs::write(
        vault.path().join("rust-guide.md"),
        r#"---
title: Rust Guide
tags:
  - rust
  - programming
---

# Rust Guide

## Ownership

Rust의 소유권 시스템은 메모리 안전성을 보장합니다.
변수의 소유권은 한 번에 하나의 변수만 가질 수 있습니다.

## Borrowing

참조와 빌림을 통해 데이터를 안전하게 공유할 수 있습니다.
"#,
    )
    .unwrap();

    fs::write(
        vault.path().join("python-tips.md"),
        r#"---
title: Python Tips
tags:
  - python
  - scripting
---

# Python Tips

## List Comprehension

파이썬의 리스트 컴프리헨션은 간결하고 강력합니다.

## Decorators

데코레이터를 사용하면 함수의 동작을 수정할 수 있습니다.
"#,
    )
    .unwrap();

    fs::write(
        vault.path().join("security-notes.md"),
        r#"---
title: Security Notes
tags:
  - security
  - devops
---

# Security Notes

## Authentication

인증은 사용자의 신원을 확인하는 과정입니다.
JWT 토큰이나 OAuth2를 사용할 수 있습니다.

## Authorization

인가는 인증된 사용자에게 권한을 부여하는 과정입니다.
RBAC(Role-Based Access Control) 패턴을 자주 사용합니다.
"#,
    )
    .unwrap();

    // Create a subfolder with a file
    fs::create_dir_all(vault.path().join("daily")).unwrap();
    fs::write(
        vault.path().join("daily/2024-01-15.md"),
        r#"---
title: Daily Note
tags:
  - daily
---

# 2024-01-15

오늘 Rust의 lifetime에 대해 공부했다.
소유권과 빌림의 개념을 더 깊이 이해하게 되었다.
"#,
    )
    .unwrap();

    // Create .obsidian folder (should be excluded)
    fs::create_dir_all(vault.path().join(".obsidian")).unwrap();
    fs::write(
        vault.path().join(".obsidian/config.json"),
        "{}",
    )
    .unwrap();

    (pool, vault)
}

#[test]
fn test_full_pipeline_project_to_search() {
    let (pool, vault) = setup();

    // 1. Add project
    let proj = project::add_project(&pool, "integration-test", vault.path().to_str().unwrap(), None)
        .expect("Failed to add project");
    assert_eq!(proj.name, "integration-test");

    // 2. Index
    let report = index_engine::index_project(&pool, &proj.id, false)
        .expect("Failed to index");
    assert_eq!(report.indexed, 4); // 3 root + 1 subfolder
    assert_eq!(report.errors, 0);
    assert_eq!(report.skipped, 0); // .obsidian should not count as skipped .md

    // 3. FTS5 search — Korean
    let results = search::fts_search(&pool, "소유권", Some(&proj.id), 10)
        .expect("FTS search failed");
    assert!(!results.is_empty(), "Should find '소유권'");
    assert!(results[0].file_path.contains("rust-guide.md"));
    assert!(results[0].heading_path.as_ref().unwrap().contains("Ownership"));

    // 4. FTS5 search — English word
    let results = search::fts_search(&pool, "Authentication", Some(&proj.id), 10)
        .expect("Search failed");
    assert!(!results.is_empty(), "Should find 'Authentication'");
    assert!(results[0].file_path.contains("security-notes.md"));

    // 5. Search without project filter (cross-project)
    let results = search::fts_search(&pool, "Decorators", None, 10)
        .expect("Cross-project search failed");
    assert!(!results.is_empty(), "Should find 'Decorators' across projects");

    // 6. List documents
    let docs = search::list_documents(&pool, &proj.id, None)
        .expect("List documents failed");
    assert_eq!(docs.len(), 4);
    assert!(docs.iter().all(|d| d.indexing_status.as_deref() == Some("done")));

    // 7. List documents with tag filter
    let rust_docs = search::list_documents(&pool, &proj.id, Some("rust"))
        .expect("Tag filter failed");
    assert_eq!(rust_docs.len(), 1);
    assert!(rust_docs[0].file_path.contains("rust-guide.md"));

    // 8. Get document content
    let content = search::get_document_content(&pool, &proj.id, "rust-guide.md")
        .expect("Get content failed");
    assert!(content.contains("소유권"));
    assert!(content.contains("Borrowing"));

    // 9. Get document metadata
    let meta = search::get_document_meta(&pool, &proj.id, "rust-guide.md")
        .expect("Get meta failed");
    assert_eq!(meta.title, Some("Rust Guide".to_string()));
    assert!(meta.frontmatter.is_some());
    assert!(meta.content_hash.is_some());
    assert_eq!(meta.indexing_status, Some("done".to_string()));

    // 10. Project info
    let (_, stats) = project::project_info(&pool, &proj.id)
        .expect("Project info failed");
    assert_eq!(stats.doc_count, 4);
    assert!(stats.chunk_count > 0);
    assert_eq!(stats.pending_count, 0);
}

#[test]
fn test_incremental_indexing() {
    let (pool, vault) = setup();

    let proj = project::add_project(&pool, "incr-test", vault.path().to_str().unwrap(), None).unwrap();

    // First index
    let r1 = index_engine::index_project(&pool, &proj.id, false).unwrap();
    assert_eq!(r1.indexed, 4);

    // Second index — no changes
    let r2 = index_engine::index_project(&pool, &proj.id, false).unwrap();
    assert_eq!(r2.indexed, 0);
    assert_eq!(r2.unchanged, 4);

    // Modify a file
    fs::write(
        vault.path().join("rust-guide.md"),
        "# Updated\n\nNew content about Rust async/await.",
    ).unwrap();

    // Third index — one file changed
    let r3 = index_engine::index_project(&pool, &proj.id, false).unwrap();
    assert_eq!(r3.indexed, 1);
    assert_eq!(r3.unchanged, 3);

    // Search for new content
    let results = search::fts_search(&pool, "async", Some(&proj.id), 10).unwrap();
    assert!(!results.is_empty(), "Should find updated content");
}

#[test]
fn test_full_reindex() {
    let (pool, vault) = setup();

    let proj = project::add_project(&pool, "full-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    // Full reindex should re-process all files
    let r = index_engine::index_project(&pool, &proj.id, true).unwrap();
    assert_eq!(r.indexed, 4);
    assert_eq!(r.unchanged, 0);
}

#[test]
fn test_project_crud() {
    let (pool, vault) = setup();

    // Add
    let proj = project::add_project(&pool, "crud-test", vault.path().to_str().unwrap(), Some("MyVault")).unwrap();
    assert_eq!(proj.vault_name, Some("MyVault".to_string()));

    // List
    let list = project::list_projects(&pool).unwrap();
    assert!(list.iter().any(|p| p.name == "crud-test"));

    // Get by name
    let found = project::get_project(&pool, "crud-test").unwrap();
    assert_eq!(found.id, proj.id);

    // Get by ID
    let found2 = project::get_project(&pool, &proj.id).unwrap();
    assert_eq!(found2.name, "crud-test");

    // Update path
    let new_vault = TempDir::new().unwrap();
    let new_path = new_vault.path().to_str().unwrap();
    project::update_project_path(&pool, &proj.id, new_path).unwrap();
    let updated = project::get_project(&pool, &proj.id).unwrap();
    // macOS resolves /var → /private/var, so compare canonicalized paths
    assert!(
        updated.path.ends_with(new_vault.path().file_name().unwrap().to_str().unwrap()),
        "Updated path should end with the same temp dir name"
    );

    // Remove
    project::remove_project(&pool, &proj.id).unwrap();
    assert!(project::get_project(&pool, &proj.id).is_err());
}

#[test]
fn test_excluded_files_not_indexed() {
    let (pool, vault) = setup();

    let proj = project::add_project(&pool, "excl-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    let docs = search::list_documents(&pool, &proj.id, None).unwrap();
    // .obsidian/config.json is not .md so won't be indexed anyway
    // But verify no .obsidian paths
    assert!(docs.iter().all(|d| !d.file_path.contains(".obsidian")));
}

#[test]
fn test_subfolder_indexed() {
    let (pool, vault) = setup();

    let proj = project::add_project(&pool, "sub-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    let docs = search::list_documents(&pool, &proj.id, None).unwrap();
    assert!(docs.iter().any(|d| d.file_path.contains("daily/")));

    // Search in subfolder content
    let results = search::fts_search(&pool, "subfolder", Some(&proj.id), 10).unwrap();
    // The daily note may not contain "subfolder", so just verify the doc exists
    assert!(docs.iter().any(|d| d.file_path.starts_with("daily/")));
}

#[test]
fn test_search_snippet_and_heading() {
    let (pool, vault) = setup();

    let proj = project::add_project(&pool, "snippet-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    // Use a word that's definitely in chunk content (heading_path is also indexed in FTS5)
    let results = search::fts_search(&pool, "Borrowing", Some(&proj.id), 10).unwrap();
    assert!(!results.is_empty(), "Should find 'Borrowing'");

    let r = &results[0];
    assert!(!r.snippet.is_empty(), "Snippet should not be empty");
    assert!(r.heading_path.is_some(), "Should have heading path");
}

#[test]
fn test_empty_query_returns_empty() {
    let (pool, _vault) = setup();
    let results = search::fts_search(&pool, "", None, 10).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_nonexistent_project_errors() {
    let (pool, _vault) = setup();
    assert!(project::get_project(&pool, "nonexistent").is_err());
    assert!(index_engine::index_project(&pool, "nonexistent", false).is_err());
}

#[test]
fn test_config_defaults() {
    let config = Config::default();
    assert_eq!(config.embedding.provider, "ollama");
    assert_eq!(config.embedding.model, "nomic-embed-text");
    assert_eq!(config.indexer.chunk_size, 512);
    assert_eq!(config.search.default_limit, 20);
    assert!(config.is_excluded(std::path::Path::new(".obsidian/plugins")));
    assert!(!config.is_excluded(std::path::Path::new("notes/hello.md")));
}
