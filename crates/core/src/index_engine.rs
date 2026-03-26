use rusqlite::params;
use std::path::Path;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::config::Config;
use crate::db::sqlite::DbPool;
use crate::error::{NexusError, Result};
use crate::indexer;
use crate::project;

/// Index a single project (incremental: skip unchanged files)
pub fn index_project(pool: &DbPool, project_id: &str, full: bool) -> Result<IndexReport> {
    let proj = project::get_project(pool, project_id)?;
    let config = Config::load().unwrap_or_default();
    let vault_path = Path::new(&proj.path);

    if !vault_path.is_dir() {
        return Err(NexusError::PathNotFound(proj.path.clone()));
    }

    let mut report = IndexReport::default();

    // Walk all .md files
    for entry in WalkDir::new(vault_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
    {
        let abs_path = entry.path();
        let rel_path = abs_path.strip_prefix(vault_path)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .to_string();

        // Check exclude patterns
        if config.is_excluded(abs_path) {
            report.skipped += 1;
            continue;
        }

        // Read file
        let content = match std::fs::read_to_string(abs_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read {}: {}", rel_path, e);
                report.errors += 1;
                continue;
            }
        };

        // Check if file changed (content hash comparison)
        let new_hash = indexer::compute_hash(&content);
        if !full {
            if let Some(existing_hash) = get_existing_hash(pool, &proj.id, &rel_path)? {
                if existing_hash == new_hash {
                    report.unchanged += 1;
                    continue;
                }
            }
        }

        // UoW: index this file atomically
        match index_single_file(pool, &proj.id, &rel_path, &content, &new_hash, &config) {
            Ok(_) => report.indexed += 1,
            Err(e) => {
                tracing::error!("Failed to index {}: {}", rel_path, e);
                mark_error(pool, &proj.id, &rel_path);
                report.errors += 1;
            }
        }
    }

    // Clean up ghost documents (in DB but deleted from disk)
    {
        let conn = pool.get()?;
        let mut ghost_stmt = conn.prepare(
            "SELECT id, file_path FROM documents WHERE project_id = ?1"
        )?;
        let all_docs: Vec<(String, String)> = ghost_stmt.query_map(params![proj.id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        drop(ghost_stmt);
        for (doc_id, doc_path) in &all_docs {
            let abs = vault_path.join(doc_path);
            if !abs.exists() {
                conn.execute("DELETE FROM documents WHERE id = ?1", params![doc_id])?;
                tracing::info!("Removed ghost document: {}", doc_path);
            }
        }
    }

    // Update project last_indexed_at only if no errors occurred
    if report.errors == 0 {
        let conn = pool.get()?;
        conn.execute(
            "UPDATE projects SET last_indexed_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![proj.id],
        )?;
    }

    Ok(report)
}

/// UoW: Index a single file atomically using a SQLite transaction
fn index_single_file(
    pool: &DbPool,
    project_id: &str,
    file_path: &str,
    content: &str,
    content_hash: &str,
    config: &Config,
) -> Result<()> {
    let mut conn = pool.get()?;

    // Parse markdown before starting transaction (no DB needed)
    let parsed = indexer::parse_markdown(content, config.indexer.chunk_size, config.indexer.chunk_overlap);

    // Generate embeddings outside transaction (network call, avoid holding lock)
    // Normalize embeddings for sqlite-vec (L2 distance ≈ cosine distance when normalized)
    // embed_text is enriched with title + aliases for the first chunk to improve vector recall.
    // FTS-stored content is NOT modified — only the embedding input differs.
    let embed_title = parsed.title.as_deref().unwrap_or("");
    let embed_file_stem = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .replace('-', " ");
    let embeddings: Vec<Option<Vec<f32>>> = parsed.chunks.iter().enumerate().map(|(i, chunk)| {
        let text = build_embed_text(embed_title, &embed_file_stem, &parsed.aliases, &chunk.content, i);
        match crate::embedding::embed_text(config, &text) {
            Ok(mut vec) => {
                crate::embedding::normalize(&mut vec);
                Some(vec)
            }
            Err(e) => {
                tracing::warn!("Embedding failed for chunk: {}", e);
                None
            }
        }
    }).collect();

    // Begin transaction — all DB writes are atomic
    let tx = conn.transaction()?;

    // 1. Upsert document with status 'indexing'
    let doc_id = get_or_create_document(&tx, project_id, file_path)?;
    tx.execute(
        "UPDATE documents SET indexing_status = 'indexing' WHERE id = ?1",
        params![doc_id],
    )?;

    // 2. Delete old embeddings + chunks (FTS5 triggers handle fts cleanup)
    tx.execute("DELETE FROM vec_chunks WHERE chunk_id IN (SELECT id FROM chunks WHERE document_id = ?1)", params![doc_id])?;
    // Also clean legacy table if it exists
    let _ = tx.execute("DELETE FROM chunk_embeddings WHERE chunk_id IN (SELECT id FROM chunks WHERE document_id = ?1)", params![doc_id]);
    tx.execute("DELETE FROM chunks WHERE document_id = ?1", params![doc_id])?;

    // 3. Delete old tags
    tx.execute("DELETE FROM document_tags WHERE document_id = ?1", params![doc_id])?;

    // 4. Insert new chunks + embeddings
    // Derive title and file stem for searchability injection
    let doc_title = parsed.title.as_deref().unwrap_or("");
    let file_stem = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .replace('-', " ");

    for (i, chunk) in parsed.chunks.iter().enumerate() {
        let chunk_id = Uuid::new_v4().to_string();
        // Inject title + filename into first chunk so FTS5 can match by title/filename
        let content = if i == 0 && !doc_title.is_empty() {
            format!("{} {}\n{}", doc_title, file_stem, chunk.content)
        } else {
            chunk.content.clone()
        };
        // Store aliases only on first chunk to avoid redundant repetition across chunks
        let chunk_aliases: Option<String> = if i == 0 && !parsed.aliases.is_empty() {
            let limited: Vec<&str> = parsed.aliases.iter().take(MAX_ALIAS_COUNT).map(|s| s.as_str()).collect();
            Some(limited.join(", "))
        } else {
            None
        };
        tx.execute(
            "INSERT INTO chunks (id, document_id, chunk_index, content, heading_path, start_line, end_line, aliases)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                chunk_id,
                doc_id,
                chunk.index as i64,
                content,
                chunk.heading_path,
                chunk.start_line as i64,
                chunk.end_line as i64,
                chunk_aliases,
            ],
        )?;

        if let Some(Some(emb_vec)) = embeddings.get(i) {
            let emb_bytes = crate::embedding::embedding_to_bytes(emb_vec);
            tx.execute(
                "INSERT INTO vec_chunks (chunk_id, embedding) VALUES (?1, ?2)",
                params![chunk_id, emb_bytes],
            )?;
        }
    }

    // 5. Insert tags
    for tag_name in &parsed.tags {
        tx.execute(
            "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
            params![tag_name],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO document_tags (document_id, tag_id)
             SELECT ?1, id FROM tags WHERE name = ?2",
            params![doc_id, tag_name],
        )?;
    }

    // 6. Delete old wiki links and aliases
    tx.execute("DELETE FROM wiki_links WHERE source_doc_id = ?1", params![doc_id])?;
    tx.execute("DELETE FROM document_aliases WHERE document_id = ?1", params![doc_id])?;

    // 7. Insert wiki links (resolve target_doc_id within same project)
    for link in &parsed.wiki_links {
        // Try to resolve: match target against file_path
        // Supports: exact path, with .md, filename-only (Obsidian shortest-path match)
        let target_with_md = if link.target.ends_with(".md") {
            link.target.clone()
        } else {
            format!("{}.md", link.target)
        };
        // Also try matching just the filename part (e.g. "rust-ownership" matches "references/rust-ownership.md")
        let filename_match = format!("%/{}.md", link.target);
        let target_doc_id: Option<String> = tx.query_row(
            "SELECT id FROM documents WHERE project_id = ?1
             AND (file_path = ?2 OR file_path = ?3 OR file_path LIKE ?4)",
            params![project_id, link.target, target_with_md, filename_match],
            |row| row.get(0),
        ).ok();

        tx.execute(
            "INSERT INTO wiki_links (source_doc_id, target_path, display_text, target_doc_id) VALUES (?1, ?2, ?3, ?4)",
            params![doc_id, link.target, link.display, target_doc_id],
        )?;
    }

    // 8. Insert aliases
    for alias in &parsed.aliases {
        tx.execute(
            "INSERT INTO document_aliases (document_id, alias) VALUES (?1, ?2)",
            params![doc_id, alias],
        )?;
    }

    // 9. Update document metadata
    let frontmatter_json = parsed.frontmatter
        .map(|v| serde_json::to_string(&v).unwrap_or_default());

    tx.execute(
        "UPDATE documents SET
            title = ?1,
            content_hash = ?2,
            frontmatter = ?3,
            indexing_status = 'done',
            last_indexed = CURRENT_TIMESTAMP
         WHERE id = ?4",
        params![parsed.title, content_hash, frontmatter_json, doc_id],
    )?;

    // Commit transaction — all or nothing
    tx.commit()?;

    Ok(())
}

fn get_or_create_document(conn: &rusqlite::Connection, project_id: &str, file_path: &str) -> Result<String> {
    // Try to find existing
    let existing: Option<String> = conn.query_row(
        "SELECT id FROM documents WHERE project_id = ?1 AND file_path = ?2",
        params![project_id, file_path],
        |row| row.get(0),
    ).ok();

    if let Some(id) = existing {
        return Ok(id);
    }

    // Create new
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO documents (id, project_id, file_path) VALUES (?1, ?2, ?3)",
        params![id, project_id, file_path],
    )?;
    Ok(id)
}

fn get_existing_hash(pool: &DbPool, project_id: &str, file_path: &str) -> Result<Option<String>> {
    let conn = pool.get()?;
    let hash: Option<String> = conn.query_row(
        "SELECT content_hash FROM documents WHERE project_id = ?1 AND file_path = ?2",
        params![project_id, file_path],
        |row| row.get(0),
    ).ok();
    Ok(hash)
}

fn mark_error(pool: &DbPool, project_id: &str, file_path: &str) {
    if let Ok(conn) = pool.get() {
        let _ = conn.execute(
            "UPDATE documents SET indexing_status = 'error' WHERE project_id = ?1 AND file_path = ?2",
            params![project_id, file_path],
        );
    }
}

/// Index all projects
pub fn index_all(pool: &DbPool, full: bool) -> Result<Vec<(String, IndexReport)>> {
    let projects = project::list_projects(pool)?;
    let mut results = Vec::new();
    for proj in projects {
        let report = index_project(pool, &proj.id, full)?;
        results.push((proj.name, report));
    }
    Ok(results)
}

#[derive(Debug, Default, serde::Serialize)]
pub struct IndexReport {
    pub indexed: usize,
    pub unchanged: usize,
    pub skipped: usize,
    pub errors: usize,
}

/// Maximum number of aliases included in embedding text and stored per chunk.
const MAX_ALIAS_COUNT: usize = 5;

/// Build the text input for embedding a chunk.
/// For the first chunk (index 0), prepends title, file stem, and up to MAX_ALIAS_COUNT aliases.
/// Remaining chunks use raw content to avoid redundant prefix storage.
pub(crate) fn build_embed_text(
    title: &str,
    file_stem: &str,
    aliases: &[String],
    content: &str,
    chunk_index: usize,
) -> String {
    if chunk_index != 0 {
        return content.to_string();
    }
    let alias_part = if aliases.is_empty() {
        String::new()
    } else {
        let limited: Vec<&str> = aliases.iter().take(MAX_ALIAS_COUNT).map(|s| s.as_str()).collect();
        format!("별칭: {}\n", limited.join(", "))
    };
    format!("제목: {} {}\n{}{}", title, file_stem, alias_part, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::helpers::{test_pool, create_test_vault};

    fn add_test_project(pool: &DbPool) -> (crate::project::Project, tempfile::TempDir) {
        let vault = create_test_vault();
        let proj = crate::project::add_project(pool, "test", vault.path().to_str().unwrap(), None).unwrap();
        (proj, vault)
    }

    #[test]
    fn test_index_project_basic() {
        let pool = test_pool();
        let (proj, _vault) = add_test_project(&pool);

        let report = index_project(&pool, &proj.id, false).unwrap();
        assert!(report.indexed >= 3); // note1.md, note2.md, korean.md, subfolder/nested.md
        assert_eq!(report.errors, 0);
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn test_incremental_indexing() {
        let pool = test_pool();
        let (proj, _vault) = add_test_project(&pool);

        // First index
        let r1 = index_project(&pool, &proj.id, false).unwrap();
        assert!(r1.indexed >= 3);

        // Second index — all unchanged
        let r2 = index_project(&pool, &proj.id, false).unwrap();
        assert_eq!(r2.indexed, 0);
        assert!(r2.unchanged >= 3);
    }

    #[test]
    fn test_full_reindex() {
        let pool = test_pool();
        let (proj, _vault) = add_test_project(&pool);

        // First index
        index_project(&pool, &proj.id, false).unwrap();

        // Full re-index ignores content hash
        let r2 = index_project(&pool, &proj.id, true).unwrap();
        assert!(r2.indexed >= 3);
        assert_eq!(r2.unchanged, 0);
    }

    #[test]
    fn test_documents_created_with_correct_status() {
        let pool = test_pool();
        let (proj, _vault) = add_test_project(&pool);
        index_project(&pool, &proj.id, false).unwrap();

        let conn = pool.get().unwrap();
        let done_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE project_id = ?1 AND indexing_status = 'done'",
            rusqlite::params![proj.id],
            |row| row.get(0),
        ).unwrap();
        assert!(done_count >= 3);
    }

    #[test]
    fn test_chunks_created() {
        let pool = test_pool();
        let (proj, _vault) = add_test_project(&pool);
        index_project(&pool, &proj.id, false).unwrap();

        let conn = pool.get().unwrap();
        let chunk_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks c JOIN documents d ON c.document_id = d.id WHERE d.project_id = ?1",
            rusqlite::params![proj.id],
            |row| row.get(0),
        ).unwrap();
        assert!(chunk_count > 0, "Expected chunks to be created");
    }

    #[test]
    fn test_tags_extracted() {
        let pool = test_pool();
        let (proj, _vault) = add_test_project(&pool);
        index_project(&pool, &proj.id, false).unwrap();

        let conn = pool.get().unwrap();
        let tag_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT t.name) FROM tags t
             JOIN document_tags dt ON t.id = dt.tag_id
             JOIN documents d ON dt.document_id = d.id
             WHERE d.project_id = ?1",
            rusqlite::params![proj.id],
            |row| row.get(0),
        ).unwrap();
        // note1 has rust+programming, note2 has python, korean has 한국어+테스트
        assert!(tag_count >= 4, "Expected at least 4 tags, got {}", tag_count);
    }

    #[test]
    fn test_fts5_populated() {
        let pool = test_pool();
        let (proj, _vault) = add_test_project(&pool);
        index_project(&pool, &proj.id, false).unwrap();

        let conn = pool.get().unwrap();
        let fts_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chunks_fts WHERE chunks_fts MATCH '\"rust\"'",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(fts_count > 0, "FTS5 should have entries for 'rust'");
    }

    #[test]
    fn test_excluded_files_skipped() {
        let pool = test_pool();
        let vault = create_test_vault();
        // .obsidian directory exists in test vault
        let proj = crate::project::add_project(&pool, "excl-test", vault.path().to_str().unwrap(), None).unwrap();

        index_project(&pool, &proj.id, false).unwrap();

        let conn = pool.get().unwrap();
        let has_obsidian: i64 = conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE project_id = ?1 AND file_path LIKE '%.obsidian%'",
            rusqlite::params![proj.id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(has_obsidian, 0, ".obsidian files should be excluded");
    }

    #[test]
    fn test_index_nonexistent_project() {
        let pool = test_pool();
        let result = index_project(&pool, "nonexistent-id", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_project_info_after_indexing() {
        let pool = test_pool();
        let (proj, _vault) = add_test_project(&pool);
        index_project(&pool, &proj.id, false).unwrap();

        let (_, stats) = crate::project::project_info(&pool, &proj.id).unwrap();
        assert!(stats.doc_count >= 3);
        assert!(stats.chunk_count > 0);
        assert_eq!(stats.pending_count, 0);
    }

    #[test]
    fn test_build_embed_text_with_aliases() {
        let aliases = vec!["별칭A".to_string(), "AliasB".to_string()];
        let text = build_embed_text("My Title", "my-file", &aliases, "본문 내용", 0);
        assert!(text.contains("제목: My Title my-file"), "title prefix missing");
        assert!(text.contains("별칭: 별칭A, AliasB"), "aliases prefix missing");
        assert!(text.contains("본문 내용"), "content missing");
    }

    #[test]
    fn test_build_embed_text_no_aliases() {
        let text = build_embed_text("Title", "file", &[], "content", 0);
        assert!(text.contains("제목: Title file"));
        assert!(!text.contains("별칭:"), "should not have alias prefix when no aliases");
        assert!(text.contains("content"));
    }

    #[test]
    fn test_build_embed_text_non_first_chunk() {
        let aliases = vec!["SomeAlias".to_string()];
        let text = build_embed_text("Title", "file", &aliases, "content", 1);
        assert_eq!(text, "content", "non-first chunk must return raw content only");
    }

    #[test]
    fn test_build_embed_text_alias_limit() {
        let aliases: Vec<String> = (0..8).map(|i| format!("Alias{i}")).collect();
        let text = build_embed_text("T", "f", &aliases, "body", 0);
        // 최대 5개만 포함되어야 함
        let alias_line = text.lines().find(|l| l.starts_with("별칭:")).unwrap();
        let count = alias_line.split(',').count();
        assert_eq!(count, 5, "alias count should be capped at 5, got {count}");
    }

    #[test]
    fn test_index_all() {
        let pool = test_pool();
        let v1 = create_test_vault();
        let v2 = tempfile::tempdir().unwrap();
        std::fs::write(v2.path().join("only.md"), "# Only\n\nSingle file.\n").unwrap();

        crate::project::add_project(&pool, "p1", v1.path().to_str().unwrap(), None).unwrap();
        crate::project::add_project(&pool, "p2", v2.path().to_str().unwrap(), None).unwrap();

        let results = index_all(&pool, false).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|(_, r)| r.errors == 0));
    }
}
