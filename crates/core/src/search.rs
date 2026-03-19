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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backlink_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
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

    // Sanitize FTS5 query: short queries use prefix matching, longer ones use phrase matching
    // Also split underscored terms (wiki_links → "wiki_links" OR "wiki" OR "links")
    let trimmed = query.trim();
    let safe_query = {
        let escaped = trimmed.replace('"', "\"\"");
        let mut parts: Vec<String> = vec![format!("\"{}\"", escaped)];

        // Split underscore terms into individual words for broader matching
        if trimmed.contains('_') {
            for word in trimmed.split('_') {
                let w = word.trim();
                if !w.is_empty() {
                    parts.push(format!("\"{}\"", w.replace('"', "\"\"")));
                }
            }
        }

        // Short query: add prefix matching (escaped inside quotes)
        if trimmed.chars().count() <= 3 {
            parts.push(format!("\"{}\"*", escaped));
        }

        parts.join(" OR ")
    };

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
        tags: None,
        backlink_count: None,
        view_count: None,
        last_modified: None,
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
    let vault_path = std::fs::canonicalize(&project.path)
        .map_err(|_| crate::error::NexusError::PathNotFound(project.path.clone()))?;
    let full_path = vault_path.join(file_path);
    let full_path = std::fs::canonicalize(&full_path)
        .map_err(|_| crate::error::NexusError::DocumentNotFound(file_path.to_string()))?;
    // Path traversal guard
    if !full_path.starts_with(&vault_path) {
        return Err(crate::error::NexusError::DocumentNotFound(
            "Path traversal detected".to_string()
        ));
    }
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

/// Vector similarity search using sqlite-vec KNN
pub fn vector_search(
    pool: &DbPool,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
    config: &crate::config::Config,
) -> Result<Vec<SearchResult>> {
    // Generate and normalize query embedding
    let mut query_embedding = crate::embedding::embed_text(config, query)?;
    crate::embedding::normalize(&mut query_embedding);
    let query_bytes = crate::embedding::embedding_to_bytes(&query_embedding);

    let conn = pool.get()?;

    // sqlite-vec KNN: fetch top-K*2 candidates, then filter by project if needed
    let fetch_limit = if project_id.is_some() { limit * 3 } else { limit };

    let sql = "SELECT v.chunk_id, v.distance, c.document_id, d.file_path, p.name,
                      c.heading_path, c.content, d.project_id
               FROM vec_chunks v
               JOIN chunks c ON v.chunk_id = c.id
               JOIN documents d ON c.document_id = d.id
               JOIN projects p ON d.project_id = p.id
               WHERE v.embedding MATCH ?1
                 AND k = ?2
               ORDER BY v.distance";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![query_bytes, fetch_limit as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,   // chunk_id
            row.get::<_, f64>(1)?,      // distance
            row.get::<_, String>(2)?,   // document_id
            row.get::<_, String>(3)?,   // file_path
            row.get::<_, String>(4)?,   // project_name
            row.get::<_, Option<String>>(5)?, // heading_path
            row.get::<_, String>(6)?,   // content
            row.get::<_, String>(7)?,   // project_id
        ))
    })?.collect::<std::result::Result<Vec<_>, _>>()?;

    // Convert L2 distance to similarity score (normalized vectors: similarity ≈ 1 - distance²/2)
    let min_score = config.search.min_vector_score;
    let mut results: Vec<SearchResult> = Vec::new();

    for (chunk_id, distance, doc_id, file_path, proj_name, heading, content, pid) in rows {
        // Filter by project if specified
        if let Some(filter_pid) = project_id {
            if pid != filter_pid {
                continue;
            }
        }

        // Convert L2 distance to cosine similarity for normalized vectors
        let similarity = 1.0 - (distance * distance) / 2.0;

        if similarity < min_score {
            continue;
        }

        let snippet = if content.len() > 200 {
            format!("{}...", &content[..content.char_indices().nth(200).map_or(content.len(), |(i,_)| i)])
        } else {
            content
        };

        results.push(SearchResult {
            chunk_id,
            document_id: doc_id,
            file_path,
            project_name: proj_name,
            heading_path: heading,
            snippet,
            score: similarity,
            tags: None,
            backlink_count: None,
            view_count: None,
            last_modified: None,
        });

        if results.len() >= limit {
            break;
        }
    }

    Ok(results)
}

/// Hybrid search: combine FTS5 keyword + vector similarity
pub fn hybrid_search(
    pool: &DbPool,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
    config: &crate::config::Config,
) -> Result<Vec<SearchResult>> {
    // Dynamic weight: short queries lean toward keyword, long queries use configured weight
    let char_count = query.trim().chars().count();
    let weight = if char_count <= 2 {
        config.search.hybrid_weight * 0.3 // heavily keyword-biased
    } else if char_count <= 4 {
        config.search.hybrid_weight * 0.6 // moderately keyword-biased
    } else {
        config.search.hybrid_weight // use configured weight as-is
    };

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

/// Enrich search results with metadata (tags, backlink_count, view_count, last_modified)
/// and optionally apply metadata-based reranking with title boost
pub fn enrich_results(
    pool: &DbPool,
    results: &mut Vec<SearchResult>,
    use_popularity: bool,
) -> Result<()> {
    if results.is_empty() { return Ok(()); }
    let conn = pool.get()?;

    // Collect unique document IDs for batch queries
    let mut doc_ids: Vec<String> = results.iter().map(|r| r.document_id.clone()).collect();
    doc_ids.sort();
    doc_ids.dedup();

    // Batch: tags per document
    let mut doc_tags: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut tag_stmt = conn.prepare_cached(
        "SELECT dt.document_id, t.name FROM tags t
         JOIN document_tags dt ON t.id = dt.tag_id"
    )?;
    let tag_rows = tag_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?.collect::<std::result::Result<Vec<_>, _>>()?;
    for (did, tag) in tag_rows {
        doc_tags.entry(did).or_default().push(tag);
    }

    // Batch: backlink counts
    let mut backlink_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut bl_stmt = conn.prepare_cached(
        "SELECT target_doc_id, COUNT(*) FROM wiki_links WHERE target_doc_id IS NOT NULL GROUP BY target_doc_id"
    )?;
    let bl_rows = bl_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?.collect::<std::result::Result<Vec<_>, _>>()?;
    for (did, count) in bl_rows {
        backlink_counts.insert(did, count);
    }

    // Batch: view counts
    let mut view_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut vc_stmt = conn.prepare_cached(
        "SELECT document_id, COUNT(*) FROM document_views GROUP BY document_id"
    )?;
    let vc_rows = vc_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?.collect::<std::result::Result<Vec<_>, _>>()?;
    for (did, count) in vc_rows {
        view_counts.insert(did, count);
    }

    // Batch: document metadata (title, last_modified)
    let mut doc_meta: std::collections::HashMap<String, (Option<String>, Option<String>)> = std::collections::HashMap::new();
    let mut meta_stmt = conn.prepare_cached(
        "SELECT id, title, last_modified FROM documents"
    )?;
    let meta_rows = meta_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?))
    })?.collect::<std::result::Result<Vec<_>, _>>()?;
    for (did, title, lm) in meta_rows {
        doc_meta.insert(did, (title, lm));
    }

    // Apply metadata to results + reranking
    for r in results.iter_mut() {
        r.tags = Some(doc_tags.get(&r.document_id).cloned().unwrap_or_default());
        r.backlink_count = Some(*backlink_counts.get(&r.document_id).unwrap_or(&0));
        r.view_count = Some(*view_counts.get(&r.document_id).unwrap_or(&0));
        if let Some((_, ref lm)) = doc_meta.get(&r.document_id) {
            r.last_modified = lm.clone();
        }

        let mut boost = 0.0_f64;

        // Title match boost: if heading_path top-level matches document title
        if let Some((Some(ref title), _)) = doc_meta.get(&r.document_id) {
            if let Some(ref heading) = r.heading_path {
                let first_heading = heading.split(" > ").next().unwrap_or("");
                if first_heading == title.as_str() {
                    boost += 0.10;
                }
            }
        }

        // Heading depth boost: top-level sections get priority
        if let Some(ref heading) = r.heading_path {
            if !heading.contains(" > ") {
                boost += 0.05;
            }
        }

        if use_popularity {
            let bl = r.backlink_count.unwrap_or(0) as f64;
            boost += (bl * 0.02).min(0.20);
            let vc = r.view_count.unwrap_or(0) as f64;
            if vc > 0.0 {
                boost += ((vc + 1.0).ln() * 0.03).min(0.15);
            }
        }

        r.score *= 1.0 + boost;
    }
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    Ok(())
}

/// Filter search results by tags (post-enrich). Keeps results that have ANY of the specified tags.
pub fn filter_by_tags(results: &mut Vec<SearchResult>, tags: &[&str]) {
    if tags.is_empty() { return; }
    results.retain(|r| {
        if let Some(ref result_tags) = r.tags {
            tags.iter().any(|t| result_tags.iter().any(|rt| rt.eq_ignore_ascii_case(t)))
        } else {
            false
        }
    });
}

/// Record a document view for popularity tracking
pub fn record_view(pool: &DbPool, document_id: &str) -> Result<()> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO document_views (document_id) VALUES (?1)",
        params![document_id],
    )?;
    Ok(())
}

/// Get a specific section of a document by heading path
pub fn get_section(pool: &DbPool, project_id: &str, file_path: &str, heading: &str) -> Result<String> {
    let content = get_document_content(pool, project_id, file_path)?;
    let mut in_section = false;
    let mut section_level = 0usize;
    let mut result = String::new();

    for line in content.lines() {
        if line.starts_with('#') {
            let level = line.chars().take_while(|c| *c == '#').count();
            let title = line.trim_start_matches('#').trim();
            if title.eq_ignore_ascii_case(heading) || line.contains(heading) {
                in_section = true;
                section_level = level;
                result.push_str(line);
                result.push('\n');
                continue;
            }
            if in_section && level <= section_level {
                break; // Hit same or higher level heading, stop
            }
        }
        if in_section {
            result.push_str(line);
            result.push('\n');
        }
    }

    if result.is_empty() {
        Err(crate::error::NexusError::DocumentNotFound(
            format!("Section '{}' not found in {}", heading, file_path)
        ))
    } else {
        Ok(result)
    }
}

/// Backlink result
#[derive(Debug, Serialize, Deserialize)]
pub struct BacklinkResult {
    pub source_file_path: String,
    pub source_title: Option<String>,
    pub display_text: Option<String>,
}

/// Forward link result
#[derive(Debug, Serialize, Deserialize)]
pub struct LinkResult {
    pub target_path: String,
    pub display_text: Option<String>,
    pub resolved: bool,
}

/// Get documents that link TO this document (backlinks)
pub fn get_backlinks(pool: &DbPool, project_id: &str, file_path: &str) -> Result<Vec<BacklinkResult>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT d.file_path, d.title, wl.display_text
         FROM wiki_links wl
         JOIN documents d ON wl.source_doc_id = d.id
         JOIN documents target_d ON wl.target_doc_id = target_d.id
         WHERE target_d.project_id = ?1 AND target_d.file_path = ?2"
    )?;
    let results = stmt.query_map(params![project_id, file_path], |row| {
        Ok(BacklinkResult {
            source_file_path: row.get(0)?,
            source_title: row.get(1)?,
            display_text: row.get(2)?,
        })
    })?.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(results)
}

/// Get documents that this document links TO (forward links)
pub fn get_forward_links(pool: &DbPool, project_id: &str, file_path: &str) -> Result<Vec<LinkResult>> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT wl.target_path, wl.display_text, wl.target_doc_id
         FROM wiki_links wl
         JOIN documents d ON wl.source_doc_id = d.id
         WHERE d.project_id = ?1 AND d.file_path = ?2"
    )?;
    let results = stmt.query_map(params![project_id, file_path], |row| {
        let target_doc_id: Option<String> = row.get(2)?;
        Ok(LinkResult {
            target_path: row.get(0)?,
            display_text: row.get(1)?,
            resolved: target_doc_id.is_some(),
        })
    })?.collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(results)
}

/// Resolve a document by alias
pub fn resolve_by_alias(pool: &DbPool, project_id: &str, alias: &str) -> Result<Option<DocumentInfo>> {
    let conn = pool.get()?;
    let result = conn.query_row(
        "SELECT d.id, d.file_path, d.title, d.indexing_status, d.last_indexed
         FROM document_aliases da
         JOIN documents d ON da.document_id = d.id
         WHERE d.project_id = ?1 AND da.alias = ?2
         LIMIT 1",
        params![project_id, alias],
        |row| {
            Ok(DocumentInfo {
                id: row.get(0)?,
                file_path: row.get(1)?,
                title: row.get(2)?,
                indexing_status: row.get(3)?,
                last_indexed: row.get(4)?,
            })
        },
    );
    match result {
        Ok(info) => Ok(Some(info)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
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
