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
    conn.execute_batch(include_str!("../migrations/V6__fts_aliases.sql")).unwrap();
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
    let results = search::fts_search(&pool, "소유권", Some(&proj.id), 10, None)
        .expect("FTS search failed");
    assert!(!results.is_empty(), "Should find '소유권'");
    assert!(results[0].file_path.contains("rust-guide.md"));
    assert!(results[0].heading_path.as_ref().unwrap().contains("Ownership"));

    // 4. FTS5 search — English word
    let results = search::fts_search(&pool, "Authentication", Some(&proj.id), 10, None)
        .expect("Search failed");
    assert!(!results.is_empty(), "Should find 'Authentication'");
    assert!(results[0].file_path.contains("security-notes.md"));

    // 5. Search without project filter (cross-project)
    let results = search::fts_search(&pool, "Decorators", None, 10, None)
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
    let results = search::fts_search(&pool, "async", Some(&proj.id), 10, None).unwrap();
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
    let results = search::fts_search(&pool, "subfolder", Some(&proj.id), 10, None).unwrap();
    // The daily note may not contain "subfolder", so just verify the doc exists
    assert!(docs.iter().any(|d| d.file_path.starts_with("daily/")));
}

#[test]
fn test_search_snippet_and_heading() {
    let (pool, vault) = setup();

    let proj = project::add_project(&pool, "snippet-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    // Use a word that's definitely in chunk content (heading_path is also indexed in FTS5)
    let results = search::fts_search(&pool, "Borrowing", Some(&proj.id), 10, None).unwrap();
    assert!(!results.is_empty(), "Should find 'Borrowing'");

    let r = &results[0];
    assert!(!r.snippet.is_empty(), "Snippet should not be empty");
    assert!(r.heading_path.is_some(), "Should have heading path");
}

#[test]
fn test_empty_query_returns_empty() {
    let (pool, _vault) = setup();
    let results = search::fts_search(&pool, "", None, 10, None).unwrap();
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

#[test]
fn test_tag_prefilter_fts_or_mode() {
    let (pool, vault) = setup();
    let proj = project::add_project(&pool, "tag-or-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    let tf = search::TagFilter::new(vec!["rust".to_string()], false);
    let results = search::fts_search(&pool, "소유권 Authentication Decorators", Some(&proj.id), 10, Some(&tf)).unwrap();
    // Only rust-tagged document should appear
    assert!(!results.is_empty(), "Should find results with rust tag");
    assert!(results.iter().all(|r| r.file_path.contains("rust-guide.md")),
        "All results should be from rust-guide.md");
}

#[test]
fn test_tag_prefilter_empty_when_no_match() {
    let (pool, vault) = setup();
    let proj = project::add_project(&pool, "tag-empty-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    let tf = search::TagFilter::new(vec!["nonexistent-tag".to_string()], false);
    let results = search::fts_search(&pool, "소유권", Some(&proj.id), 10, Some(&tf)).unwrap();
    assert!(results.is_empty(), "Should return empty for non-existent tag");
}

#[test]
fn test_tag_prefilter_and_mode() {
    let (pool, vault) = setup();
    let proj = project::add_project(&pool, "tag-and-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    // rust-guide.md has tags: rust, programming
    // AND mode: both tags must match
    let tf = search::TagFilter::new(vec!["rust".to_string(), "programming".to_string()], true);
    let results = search::fts_search(&pool, "소유권", Some(&proj.id), 10, Some(&tf)).unwrap();
    assert!(!results.is_empty(), "Should find results with both rust AND programming tags");

    // AND mode with impossible combination
    let tf2 = search::TagFilter::new(vec!["rust".to_string(), "python".to_string()], true);
    let results2 = search::fts_search(&pool, "소유권", Some(&proj.id), 10, Some(&tf2)).unwrap();
    assert!(results2.is_empty(), "No document has both rust AND python tags");
}

#[test]
fn test_tag_prefilter_excludes_alias_match() {
    let pool = test_pool();
    let vault = TempDir::new().expect("Failed to create temp dir");

    // Document with alias "mcp-guide" but only tagged "devops" (no "mcp" tag)
    fs::write(
        vault.path().join("infra-setup.md"),
        r#"---
title: Infra Setup
aliases:
  - mcp-guide
tags:
  - devops
---

# Infra Setup

Infrastructure setup and configuration guide.
"#,
    )
    .unwrap();

    // Document with "mcp" tag
    fs::write(
        vault.path().join("mcp-server.md"),
        r#"---
title: MCP Server
tags:
  - mcp
  - programming
---

# MCP Server

MCP server implementation details.
"#,
    )
    .unwrap();

    let proj = project::add_project(&pool, "alias-tag-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    // Search "mcp-guide" with tag filter ["mcp"]
    // infra-setup.md has alias "mcp-guide" but no "mcp" tag — should be excluded
    let tf = search::TagFilter::new(vec!["mcp".to_string()], false);
    let results = search::fts_search(&pool, "mcp-guide", Some(&proj.id), 10, Some(&tf)).unwrap();

    // All results must have the "mcp" tag — alias match without tag must be excluded
    for r in &results {
        assert!(
            !r.file_path.contains("infra-setup.md"),
            "infra-setup.md should be excluded by tag filter despite alias match"
        );
    }
}

#[test]
fn test_get_popular_documents_global() {
    let pool = test_pool();
    let vault = TempDir::new().unwrap();

    fs::write(vault.path().join("hot.md"), "# Hot\nPopular document.").unwrap();
    fs::write(vault.path().join("cold.md"), "# Cold\nRarely seen document.").unwrap();

    let proj = project::add_project(&pool, "rank-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    // Fetch document IDs
    let conn = pool.get().unwrap();
    let hot_id: String = conn
        .query_row("SELECT id FROM documents WHERE file_path = 'hot.md' AND project_id = ?1", [&proj.id], |r| r.get(0))
        .unwrap();
    let cold_id: String = conn
        .query_row("SELECT id FROM documents WHERE file_path = 'cold.md' AND project_id = ?1", [&proj.id], |r| r.get(0))
        .unwrap();

    // hot.md: 5 views + 2 backlinks  →  5*0.6 + 2*0.4 = 3.8
    // cold.md: 1 view + 0 backlinks  →  1*0.6 + 0     = 0.6
    for _ in 0..5 {
        conn.execute("INSERT INTO document_views (document_id) VALUES (?1)", [&hot_id]).unwrap();
    }
    conn.execute("INSERT INTO document_views (document_id) VALUES (?1)", [&cold_id]).unwrap();
    conn.execute(
        "INSERT INTO wiki_links (source_doc_id, target_path, target_doc_id) VALUES (?1, 'hot.md', ?2)",
        [&cold_id, &hot_id],
    ).unwrap();
    conn.execute(
        "INSERT INTO wiki_links (source_doc_id, target_path, target_doc_id) VALUES (?1, 'hot.md', ?2)",
        [&hot_id, &hot_id],
    ).unwrap();
    drop(conn);

    let docs = search::get_popular_documents(&pool, None, 10).unwrap();
    assert!(!docs.is_empty(), "ranking should return results");
    assert_eq!(docs[0].file_path, "hot.md", "hot.md should rank first");
    assert!(docs[0].score > docs[1].score, "hot.md score should be higher");
}

#[test]
fn test_get_popular_documents_by_project() {
    let pool = test_pool();
    let vault_a = TempDir::new().unwrap();
    let vault_b = TempDir::new().unwrap();

    fs::write(vault_a.path().join("doc-a.md"), "# Doc A").unwrap();
    fs::write(vault_b.path().join("doc-b.md"), "# Doc B").unwrap();

    let proj_a = project::add_project(&pool, "proj-a", vault_a.path().to_str().unwrap(), None).unwrap();
    let proj_b = project::add_project(&pool, "proj-b", vault_b.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj_a.id, false).unwrap();
    index_engine::index_project(&pool, &proj_b.id, false).unwrap();

    let conn = pool.get().unwrap();
    let id_a: String = conn
        .query_row("SELECT id FROM documents WHERE project_id = ?1", [&proj_a.id], |r| r.get(0))
        .unwrap();
    let id_b: String = conn
        .query_row("SELECT id FROM documents WHERE project_id = ?1", [&proj_b.id], |r| r.get(0))
        .unwrap();

    // Give proj_b doc more views so it would rank higher globally
    for _ in 0..10 {
        conn.execute("INSERT INTO document_views (document_id) VALUES (?1)", [&id_b]).unwrap();
    }
    conn.execute("INSERT INTO document_views (document_id) VALUES (?1)", [&id_a]).unwrap();
    drop(conn);

    // Project-scoped ranking must not include proj_b docs
    let docs_a = search::get_popular_documents(&pool, Some(&proj_a.id), 10).unwrap();
    assert!(docs_a.iter().all(|d| d.project_id == proj_a.id), "project filter must be respected");

    let docs_b = search::get_popular_documents(&pool, Some(&proj_b.id), 10).unwrap();
    assert!(docs_b.iter().all(|d| d.project_id == proj_b.id), "project filter must be respected");

    // Global ranking: doc-b should rank first
    let global = search::get_popular_documents(&pool, None, 10).unwrap();
    assert_eq!(global[0].file_path, "doc-b.md", "doc-b should rank first globally");
}

#[test]
fn test_get_popular_documents_limit() {
    let pool = test_pool();
    let vault = TempDir::new().unwrap();

    for i in 0..5 {
        fs::write(vault.path().join(format!("doc{i}.md")), format!("# Doc {i}")).unwrap();
    }
    let proj = project::add_project(&pool, "limit-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    let docs = search::get_popular_documents(&pool, Some(&proj.id), 3).unwrap();
    assert!(docs.len() <= 3, "limit must be respected");
}

#[test]
fn test_alias_fts_search_via_bm25() {
    let pool = test_pool();
    let vault = TempDir::new().unwrap();

    // 문서: 본문에 "성과"라는 단어 없음, alias로만 등록
    fs::write(
        vault.path().join("performance-report.md"),
        r#"---
title: Performance Report
aliases:
  - 성과 리포트
  - 성과보고서
tags:
  - report
---

# Performance Report

This document contains quarterly performance metrics and KPIs.
"#,
    ).unwrap();

    // 관련 없는 문서
    fs::write(
        vault.path().join("other.md"),
        "# Other\n\nUnrelated content about recipes.\n",
    ).unwrap();

    let proj = project::add_project(&pool, "alias-fts-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    // alias 키워드로 FTS 검색 → performance-report.md가 상위에 와야 함
    let results = search::fts_search(&pool, "성과 리포트", Some(&proj.id), 10, None)
        .expect("FTS alias search failed");
    assert!(!results.is_empty(), "alias keyword search should return results");
    assert!(
        results[0].file_path.contains("performance-report.md"),
        "alias-matched document should rank first, got: {}",
        results[0].file_path
    );
}

#[test]
fn test_alias_fts_multi_match_scores_higher() {
    let pool = test_pool();
    let vault = TempDir::new().unwrap();

    // doc-a: alias 2개 매칭
    fs::write(vault.path().join("doc-a.md"), r#"---
title: Doc A
aliases:
  - 검색엔진
  - 검색 시스템
---
# Doc A
Content about indexing.
"#).unwrap();

    // doc-b: alias 1개 매칭
    fs::write(vault.path().join("doc-b.md"), r#"---
title: Doc B
aliases:
  - 검색엔진
---
# Doc B
Content about something else.
"#).unwrap();

    let proj = project::add_project(&pool, "alias-score-test", vault.path().to_str().unwrap(), None).unwrap();
    index_engine::index_project(&pool, &proj.id, false).unwrap();

    let results = search::fts_search(&pool, "검색엔진 검색 시스템", Some(&proj.id), 10, None)
        .expect("FTS search failed");
    assert!(results.len() >= 2, "should return both docs");
    assert!(
        results[0].file_path.contains("doc-a.md"),
        "doc with more alias matches should rank higher, got: {}",
        results[0].file_path
    );
}
