use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::sqlite::DbPool;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk_id: String,
    pub document_id: String,
    pub file_path: String,
    pub project_name: String,
    pub heading_path: Option<String>,
    pub snippet: String,
    pub score: f64,
}

/// FTS5 keyword search
pub fn fts_search(
    pool: &DbPool,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<SearchResult>> {
    let conn = pool.get()?;

    // Empty query guard
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // Sanitize FTS5 query: wrap in quotes to prevent operator injection
    let safe_query = format!("\"{}\"", query.replace('"', "\"\""));

    let sql = if project_id.is_some() {
        "SELECT c.id, c.document_id, d.file_path, p.name, c.heading_path,
                snippet(chunks_fts, 0, '<b>', '</b>', '...', 32) as snippet,
                rank
         FROM chunks_fts
         JOIN chunks c ON chunks_fts.rowid = c.rowid
         JOIN documents d ON c.document_id = d.id
         JOIN projects p ON d.project_id = p.id
         WHERE chunks_fts MATCH ?1
         AND d.project_id = ?2
         ORDER BY rank
         LIMIT ?3"
    } else {
        "SELECT c.id, c.document_id, d.file_path, p.name, c.heading_path,
                snippet(chunks_fts, 0, '<b>', '</b>', '...', 32) as snippet,
                rank
         FROM chunks_fts
         JOIN chunks c ON chunks_fts.rowid = c.rowid
         JOIN documents d ON c.document_id = d.id
         JOIN projects p ON d.project_id = p.id
         WHERE chunks_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2"
    };

    let mut stmt = conn.prepare(sql)?;

    if let Some(pid) = project_id {
        let rows = stmt.query_map(params![safe_query, pid, limit as i64], map_result)?;
        let results = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    } else {
        let rows = stmt.query_map(params![safe_query, limit as i64], map_result)?;
        let results = rows.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(results)
    }
}

fn map_result(row: &rusqlite::Row) -> rusqlite::Result<SearchResult> {
    Ok(SearchResult {
        chunk_id: row.get(0)?,
        document_id: row.get(1)?,
        file_path: row.get(2)?,
        project_name: row.get(3)?,
        heading_path: row.get(4)?,
        snippet: row.get(5)?,
        score: row.get::<_, f64>(6).unwrap_or(0.0),
    })
}

/// List documents for a project, optionally filtered by tags
pub fn list_documents(
    pool: &DbPool,
    project_id: &str,
    tag_filter: Option<&str>,
) -> Result<Vec<DocumentInfo>> {
    let conn = pool.get()?;

    if let Some(tag) = tag_filter {
        let mut stmt = conn.prepare(
            "SELECT d.id, d.file_path, d.title, d.indexing_status, d.last_indexed
             FROM documents d
             JOIN document_tags dt ON d.id = dt.document_id
             JOIN tags t ON dt.tag_id = t.id
             WHERE d.project_id = ?1 AND t.name = ?2
             ORDER BY d.file_path"
        )?;
        let docs = stmt.query_map(params![project_id, tag], |row| {
            Ok(DocumentInfo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                title: row.get(2)?,
                indexing_status: row.get(3)?,
                last_indexed: row.get(4)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(docs)
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, file_path, title, indexing_status, last_indexed
             FROM documents
             WHERE project_id = ?1
             ORDER BY file_path"
        )?;
        let docs = stmt.query_map(params![project_id], |row| {
            Ok(DocumentInfo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                title: row.get(2)?,
                indexing_status: row.get(3)?,
                last_indexed: row.get(4)?,
            })
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(docs)
    }
}

/// Get document content by reading the file from disk
pub fn get_document_content(pool: &DbPool, project_id: &str, file_path: &str) -> Result<String> {
    let project = crate::project::get_project(pool, project_id)?;
    let full_path = std::path::Path::new(&project.path).join(file_path);
    let content = std::fs::read_to_string(&full_path)?;
    Ok(content)
}

/// Get document metadata
pub fn get_document_meta(pool: &DbPool, project_id: &str, file_path: &str) -> Result<DocumentMeta> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, file_path, title, frontmatter, indexing_status, content_hash, last_modified, last_indexed
         FROM documents
         WHERE project_id = ?1 AND file_path = ?2"
    )?;

    stmt.query_row(params![project_id, file_path], |row| {
        Ok(DocumentMeta {
            id: row.get(0)?,
            file_path: row.get(1)?,
            title: row.get(2)?,
            frontmatter: row.get::<_, Option<String>>(3)?,
            indexing_status: row.get(4)?,
            content_hash: row.get(5)?,
            last_modified: row.get(6)?,
            last_indexed: row.get(7)?,
        })
    }).map_err(|_| crate::error::NexusError::DocumentNotFound(file_path.to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: String,
    pub file_path: String,
    pub title: Option<String>,
    pub indexing_status: Option<String>,
    pub last_indexed: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DocumentMeta {
    pub id: String,
    pub file_path: String,
    pub title: Option<String>,
    pub frontmatter: Option<String>,
    pub indexing_status: Option<String>,
    pub content_hash: Option<String>,
    pub last_modified: Option<String>,
    pub last_indexed: Option<String>,
}

/// Vector similarity search using Ollama embeddings
pub fn vector_search(
    pool: &DbPool,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
    config: &crate::config::Config,
) -> Result<Vec<SearchResult>> {
    // Generate query embedding
    let query_embedding = crate::embedding::embed_text(config, query)?;

    let conn = pool.get()?;

    // Fetch all chunk embeddings (with optional project filter)
    let sql = if project_id.is_some() {
        "SELECT c.id, c.document_id, d.file_path, p.name, c.heading_path, c.content, ce.embedding
         FROM chunk_embeddings ce
         JOIN chunks c ON ce.chunk_id = c.id
         JOIN documents d ON c.document_id = d.id
         JOIN projects p ON d.project_id = p.id
         WHERE d.project_id = ?1"
    } else {
        "SELECT c.id, c.document_id, d.file_path, p.name, c.heading_path, c.content, ce.embedding
         FROM chunk_embeddings ce
         JOIN chunks c ON ce.chunk_id = c.id
         JOIN documents d ON c.document_id = d.id
         JOIN projects p ON d.project_id = p.id"
    };

    let mut stmt = conn.prepare(sql)?;

    let rows: Vec<(String, String, String, String, Option<String>, String, Vec<u8>)> = if let Some(pid) = project_id {
        stmt.query_map(params![pid], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?
    };

    // Compute cosine similarity and rank
    let mut scored: Vec<(f32, SearchResult)> = rows.into_iter().map(|(chunk_id, doc_id, file_path, proj_name, heading, content, emb_bytes)| {
        let emb = crate::embedding::bytes_to_embedding(&emb_bytes);
        let score = crate::embedding::cosine_similarity(&query_embedding, &emb);
        let snippet = if content.len() > 200 { format!("{}...", &content[..content.char_indices().nth(200).map_or(content.len(), |(i,_)| i)]) } else { content };
        (score, SearchResult {
            chunk_id,
            document_id: doc_id,
            file_path,
            project_name: proj_name,
            heading_path: heading,
            snippet,
            score: score as f64,
        })
    }).collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit);

    Ok(scored.into_iter().map(|(_, r)| r).collect())
}

/// Hybrid search: combine FTS5 keyword + vector similarity
pub fn hybrid_search(
    pool: &DbPool,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
    config: &crate::config::Config,
) -> Result<Vec<SearchResult>> {
    let weight = config.search.hybrid_weight; // 0.0 = keyword only, 1.0 = vector only

    // Get keyword results
    let keyword_results = fts_search(pool, query, project_id, limit * 2)?;

    // Get vector results (may fail if Ollama is not running)
    let vector_results = match vector_search(pool, query, project_id, limit * 2, config) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Vector search failed, falling back to keyword only: {}", e);
            return Ok(keyword_results.into_iter().take(limit).collect());
        }
    };

    // Merge and re-rank using reciprocal rank fusion
    let mut scores: std::collections::HashMap<String, (f64, SearchResult)> = std::collections::HashMap::new();

    for (rank, r) in keyword_results.iter().enumerate() {
        let rrf_score = (1.0 - weight) * (1.0 / (rank as f64 + 60.0));
        scores.entry(r.chunk_id.clone()).or_insert((0.0, r.clone())).0 += rrf_score;
    }

    for (rank, r) in vector_results.iter().enumerate() {
        let rrf_score = weight * (1.0 / (rank as f64 + 60.0));
        scores.entry(r.chunk_id.clone())
            .and_modify(|(s, _)| *s += rrf_score)
            .or_insert((rrf_score, r.clone()));
    }

    let mut merged: Vec<(f64, SearchResult)> = scores.into_values().collect();
    merged.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(limit);

    Ok(merged.into_iter().map(|(score, mut r)| { r.score = score; r }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::helpers::{test_pool, create_test_vault};

    fn setup_indexed_project(pool: &DbPool) -> crate::project::Project {
        let vault = create_test_vault();
        let proj = crate::project::add_project(pool, "search-test", vault.path().to_str().unwrap(), None).unwrap();
        crate::index_engine::index_project(pool, &proj.id, false).unwrap();
        // Keep vault alive by leaking (test only)
        std::mem::forget(vault);
        proj
    }

    #[test]
    fn test_fts_search_finds_rust() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "rust", Some(&proj.id), 10).unwrap();
        assert!(!results.is_empty(), "Should find 'rust' in indexed docs");
        assert!(results.iter().any(|r| r.file_path.contains("note1.md")));
    }

    #[test]
    fn test_fts_search_korean() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "한국어", Some(&proj.id), 10).unwrap();
        assert!(!results.is_empty(), "Should find Korean text");
        assert!(results.iter().any(|r| r.file_path.contains("korean.md")));
    }

    #[test]
    fn test_fts_search_no_results() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "xyznonexistent", Some(&proj.id), 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fts_search_empty_query() {
        let pool = test_pool();
        let results = fts_search(&pool, "", None, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fts_search_cross_project() {
        let pool = test_pool();
        let _proj = setup_indexed_project(&pool);

        // Search without project filter
        let results = fts_search(&pool, "programming", None, 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fts_search_has_snippet() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "ownership", Some(&proj.id), 10).unwrap();
        assert!(!results.is_empty());
        assert!(!results[0].snippet.is_empty(), "Snippet should not be empty");
    }

    #[test]
    fn test_fts_search_has_heading_path() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "ownership", Some(&proj.id), 10).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].heading_path.is_some(), "Should have heading path");
    }

    #[test]
    fn test_fts_search_limit() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "note", Some(&proj.id), 1).unwrap();
        assert!(results.len() <= 1);
    }

    #[test]
    fn test_list_documents() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let docs = list_documents(&pool, &proj.id, None).unwrap();
        assert!(docs.len() >= 3);
        assert!(docs.iter().all(|d| d.indexing_status.as_deref() == Some("done")));
    }

    #[test]
    fn test_list_documents_with_tag_filter() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let docs = list_documents(&pool, &proj.id, Some("rust")).unwrap();
        assert_eq!(docs.len(), 1);
        assert!(docs[0].file_path.contains("note1.md"));
    }

    #[test]
    fn test_get_document_content() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let content = get_document_content(&pool, &proj.id, "note1.md").unwrap();
        assert!(content.contains("First Note"));
        assert!(content.contains("ownership"));
    }

    #[test]
    fn test_get_document_meta() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let meta = get_document_meta(&pool, &proj.id, "note1.md").unwrap();
        assert_eq!(meta.title, Some("First Note".to_string()));
        assert!(meta.content_hash.is_some());
        assert_eq!(meta.indexing_status, Some("done".to_string()));
    }

    #[test]
    fn test_get_nonexistent_document() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let result = get_document_meta(&pool, &proj.id, "nonexistent.md");
        assert!(result.is_err());
    }

    #[test]
    fn test_fts_special_chars_safe() {
        let pool = test_pool();
        let _proj = setup_indexed_project(&pool);

        // FTS5 special operators should be safely handled
        let results = fts_search(&pool, "content:password OR heading_path:secret", None, 10).unwrap();
        // Should not crash, may return empty
        let _ = results;
    }
}
