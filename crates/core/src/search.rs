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

/// DB 레벨 태그 사전 필터.
/// tags가 비어있으면 필터 없음.
#[derive(Debug, Clone, Default)]
pub struct TagFilter {
    pub tags: Vec<String>,
    pub match_all: bool,
}

impl TagFilter {
    pub fn new(tags: Vec<String>, match_all: bool) -> Self {
        Self { tags, match_all }
    }
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }
}

/// 태그 이름 목록으로 매칭 document_id 조회.
fn get_document_ids_by_tags(
    conn: &rusqlite::Connection,
    tags: &[String],
    project_id: Option<&str>,
    match_all: bool,
) -> Result<Option<Vec<String>>> {
    if tags.is_empty() {
        return Ok(None);
    }

    // Dedup 전에 소문자 변환 — "Rust"와 "rust"가 동일하게 처리되어야 함
    let mut lower_tags: Vec<String> = tags.iter().map(|t| t.to_lowercase()).collect();
    lower_tags.sort();
    lower_tags.dedup();
    let placeholders = lower_tags.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    let having_clause = if match_all {
        format!("HAVING COUNT(DISTINCT t.id) >= {}", lower_tags.len())
    } else {
        String::new()
    };

    let sql = if project_id.is_some() {
        format!(
            "SELECT dt.document_id
             FROM document_tags dt
             JOIN tags t ON dt.tag_id = t.id
             JOIN documents d ON dt.document_id = d.id
             WHERE LOWER(t.name) IN ({ph}) AND d.project_id = ?
             GROUP BY dt.document_id
             {having}",
            ph = placeholders,
            having = having_clause
        )
    } else {
        format!(
            "SELECT dt.document_id
             FROM document_tags dt
             JOIN tags t ON dt.tag_id = t.id
             WHERE LOWER(t.name) IN ({ph})
             GROUP BY dt.document_id
             {having}",
            ph = placeholders,
            having = having_clause
        )
    };

    let mut params_boxed: Vec<Box<dyn rusqlite::types::ToSql>> =
        lower_tags.iter().map(|t| Box::new(t.clone()) as Box<dyn rusqlite::types::ToSql>).collect();
    if let Some(pid) = project_id {
        params_boxed.push(Box::new(pid.to_string()));
    }

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_boxed.iter().map(|b| b.as_ref()).collect();
    let ids = stmt
        .query_map(param_refs.as_slice(), |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(Some(ids))
}

/// FTS5 keyword search
pub fn fts_search(
    pool: &DbPool,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
    tag_filter: Option<&TagFilter>,
) -> Result<Vec<SearchResult>> {
    let conn = pool.get()?;

    // Empty query guard
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    // 태그 사전 필터
    let tag_doc_ids: Option<Vec<String>> = if let Some(tf) = tag_filter {
        if !tf.is_empty() {
            match get_document_ids_by_tags(&conn, &tf.tags, project_id, tf.match_all)? {
                Some(ids) if ids.is_empty() => return Ok(vec![]),
                other => other,
            }
        } else {
            None
        }
    } else {
        None
    };

    // Sanitize FTS5 query: short queries use prefix matching, longer ones use phrase matching
    // Also split underscored terms (wiki_links → "wiki_links" OR "wiki" OR "links")
    let trimmed = query.trim();
    let safe_query = {
        let escaped = trimmed.replace('"', "\"\"");
        let mut parts: Vec<String> = vec![format!("\"{}\"", escaped)];

        // Split space-separated tokens for broader matching (multi-word queries)
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() > 1 {
            for word in &words {
                let w = word.trim();
                if !w.is_empty() {
                    let esc = w.replace('"', "\"\"");
                    parts.push(format!("\"{}\"", esc));
                }
            }
        }

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

    // Build dynamic SQL with optional project and tag filters
    let tag_in_clause = tag_doc_ids.as_ref().map(|ids| {
        let ph = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        format!("AND c.document_id IN ({})", ph)
    });

    let sql = {
        let mut s = String::from(
            "SELECT c.id, c.document_id, d.file_path, p.name, c.heading_path,
                    snippet(chunks_fts, 0, '<b>', '</b>', '...', 64) as snippet,
                    rank
             FROM chunks_fts
             JOIN chunks c ON chunks_fts.rowid = c.rowid
             JOIN documents d ON c.document_id = d.id
             JOIN projects p ON d.project_id = p.id
             WHERE chunks_fts MATCH ?"
        );
        if project_id.is_some() {
            s.push_str(" AND d.project_id = ?");
        }
        if let Some(ref clause) = tag_in_clause {
            s.push(' ');
            s.push_str(clause);
        }
        s.push_str(" ORDER BY rank LIMIT ?");
        s
    };

    let mut params_boxed: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    params_boxed.push(Box::new(safe_query.clone()));
    if let Some(pid) = project_id {
        params_boxed.push(Box::new(pid.to_string()));
    }
    if let Some(ref ids) = tag_doc_ids {
        for id in ids {
            params_boxed.push(Box::new(id.clone()));
        }
    }
    params_boxed.push(Box::new(limit as i64));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_boxed.iter().map(|b| b.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let results = stmt.query_map(param_refs.as_slice(), map_result)?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // Always resolve aliases and merge to front, but respect tag filter
    let alias_results = {
        let raw = resolve_alias_results(&conn, trimmed, project_id, limit);
        if let Some(ref ids) = tag_doc_ids {
            raw.into_iter()
                .filter(|r| ids.contains(&r.document_id))
                .collect()
        } else {
            raw
        }
    };
    let results = merge_alias_results(alias_results, results, limit);

    Ok(results)
}

/// Resolve aliases: find documents whose alias matches the query.
/// Returns chunks from matched documents, always executed regardless of other result counts.
fn resolve_alias_results(
    conn: &rusqlite::Connection,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
) -> Vec<SearchResult> {
    let pattern = format!("%{}%", query.trim());
    let sql = if project_id.is_some() {
        "SELECT c.id, c.document_id, d.file_path, p.name, c.heading_path,
                substr(c.content, 1, 200) as snippet, 0.0 as score
         FROM document_aliases da
         JOIN documents d ON da.document_id = d.id
         JOIN chunks c ON c.document_id = d.id
         JOIN projects p ON d.project_id = p.id
         WHERE da.alias LIKE ?1
           AND d.project_id = ?2
         GROUP BY c.document_id
         LIMIT ?3"
    } else {
        "SELECT c.id, c.document_id, d.file_path, p.name, c.heading_path,
                substr(c.content, 1, 200) as snippet, 0.0 as score
         FROM document_aliases da
         JOIN documents d ON da.document_id = d.id
         JOIN chunks c ON c.document_id = d.id
         JOIN projects p ON d.project_id = p.id
         WHERE da.alias LIKE ?1
         GROUP BY c.document_id
         LIMIT ?2"
    };

    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows_result = if let Some(pid) = project_id {
        stmt.query_map(params![pattern, pid, limit as i64], map_result)
    } else {
        stmt.query_map(params![pattern, limit as i64], map_result)
    };

    match rows_result {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => vec![],
    }
}

/// Merge alias results into the front of search results, deduplicating by document_id
fn merge_alias_results(alias_results: Vec<SearchResult>, main_results: Vec<SearchResult>, limit: usize) -> Vec<SearchResult> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut merged = Vec::with_capacity(limit);

    // Alias matches first
    for r in alias_results {
        if seen.insert(r.document_id.clone()) {
            merged.push(r);
        }
    }

    // Then main results
    for r in main_results {
        if merged.len() >= limit {
            break;
        }
        if seen.insert(r.document_id.clone()) {
            merged.push(r);
        }
    }

    merged
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
    tag_filter: Option<&TagFilter>,
) -> Result<Vec<SearchResult>> {
    // Generate and normalize query embedding
    let mut query_embedding = crate::embedding::embed_text(config, query)?;
    crate::embedding::normalize(&mut query_embedding);
    let query_bytes = crate::embedding::embedding_to_bytes(&query_embedding);

    let conn = pool.get()?;

    // 태그 사전 필터
    let tag_id_set: Option<std::collections::HashSet<String>> = if let Some(tf) = tag_filter {
        if !tf.is_empty() {
            match get_document_ids_by_tags(&conn, &tf.tags, project_id, tf.match_all)? {
                Some(ids) if ids.is_empty() => return Ok(vec![]),
                Some(ids) => Some(ids.into_iter().collect()),
                None => None,
            }
        } else {
            None
        }
    } else {
        None
    };

    // sqlite-vec KNN: fetch top-K*N candidates, then filter
    let fetch_limit = if tag_id_set.is_some() {
        limit * 5
    } else if project_id.is_some() {
        limit * 3
    } else {
        limit
    };

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

        // Filter by tag pre-filter
        if let Some(ref id_set) = tag_id_set {
            if !id_set.contains(&doc_id) {
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
    tag_filter: Option<&TagFilter>,
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
    let keyword_results = fts_search(pool, query, project_id, limit * 2, tag_filter)?;

    // Get vector results (may fail if Ollama is not running)
    let vector_results = match vector_search(pool, query, project_id, limit * 2, config, tag_filter) {
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

    let main_results: Vec<SearchResult> = merged.into_iter().map(|(score, mut r)| { r.score = score; r }).collect();

    // Always resolve aliases and merge to front
    let conn = pool.get()?;
    let alias_results = resolve_alias_results(&conn, query.trim(), project_id, limit);
    let results = merge_alias_results(alias_results, main_results, limit);

    Ok(results)
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

    // Collect unique document IDs for scoped batch queries
    let mut doc_ids: Vec<String> = results.iter().map(|r| r.document_id.clone()).collect();
    doc_ids.sort();
    doc_ids.dedup();
    let placeholders = doc_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    // Batch: tags per document (scoped)
    let mut doc_tags: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    {
        let sql = format!(
            "SELECT dt.document_id, t.name FROM tags t
             JOIN document_tags dt ON t.id = dt.tag_id
             WHERE dt.document_id IN ({})", placeholders
        );
        let mut tag_stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = doc_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let tag_rows = tag_stmt.query_map(params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        for (did, tag) in tag_rows {
            doc_tags.entry(did).or_default().push(tag);
        }
    }

    // Batch: backlink counts (scoped)
    let mut backlink_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    {
        let sql = format!(
            "SELECT target_doc_id, COUNT(*) FROM wiki_links
             WHERE target_doc_id IN ({}) GROUP BY target_doc_id", placeholders
        );
        let mut bl_stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = doc_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let bl_rows = bl_stmt.query_map(params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        for (did, count) in bl_rows {
            backlink_counts.insert(did, count);
        }
    }

    // Batch: view counts (scoped)
    let mut view_counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    {
        let sql = format!(
            "SELECT document_id, COUNT(*) FROM document_views
             WHERE document_id IN ({}) GROUP BY document_id", placeholders
        );
        let mut vc_stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = doc_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let vc_rows = vc_stmt.query_map(params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        for (did, count) in vc_rows {
            view_counts.insert(did, count);
        }
    }

    // Batch: document metadata (scoped)
    let mut doc_meta: std::collections::HashMap<String, (Option<String>, Option<String>)> = std::collections::HashMap::new();
    {
        let sql = format!(
            "SELECT id, title, last_modified FROM documents WHERE id IN ({})", placeholders
        );
        let mut meta_stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = doc_ids.iter().map(|id| id as &dyn rusqlite::types::ToSql).collect();
        let meta_rows = meta_stmt.query_map(params.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<String>>(2)?))
        })?.collect::<std::result::Result<Vec<_>, _>>()?;
        for (did, title, lm) in meta_rows {
            doc_meta.insert(did, (title, lm));
        }
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

/// Deprecated: 태그 필터링은 이제 검색 함수의 `tag_filter` 파라미터로 DB 레벨에서 수행됩니다.
/// 이 함수는 하위 호환성을 위해 유지됩니다.
#[deprecated(note = "Use tag_filter parameter in search functions instead")]
pub fn filter_by_tags(results: &mut Vec<SearchResult>, tags: &[&str], match_all: bool) {
    if tags.is_empty() { return; }
    results.retain(|r| {
        if let Some(ref result_tags) = r.tags {
            if match_all {
                tags.iter().all(|t| result_tags.iter().any(|rt| rt.eq_ignore_ascii_case(t)))
            } else {
                tags.iter().any(|t| result_tags.iter().any(|rt| rt.eq_ignore_ascii_case(t)))
            }
        } else {
            false
        }
    });
}

/// Record a document view for popularity tracking.
/// 같은 문서는 30분 이내 중복 기록하지 않음 (원자적 단일 쿼리로 TOCTOU 방지).
pub fn record_view(pool: &DbPool, document_id: &str) -> Result<()> {
    let conn = pool.get()?;
    conn.execute(
        "INSERT INTO document_views (document_id)
         SELECT ?1 WHERE NOT EXISTS (
             SELECT 1 FROM document_views
             WHERE document_id = ?1 AND viewed_at > datetime('now', '-30 minutes')
         )",
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

/// 파일 경로로 문서를 조회하여 열람 기록.
/// 문서가 없거나 실패해도 에러를 반환하지 않음 (fire-and-forget).
pub fn record_view_by_path(pool: &DbPool, project_id: &str, file_path: &str) {
    if let Ok(conn) = pool.get() {
        let doc_id: rusqlite::Result<String> = conn.query_row(
            "SELECT id FROM documents WHERE project_id = ?1 AND file_path = ?2 LIMIT 1",
            params![project_id, file_path],
            |row| row.get(0),
        );
        match doc_id {
            Ok(id) => {
                if let Err(e) = record_view(pool, &id) {
                    tracing::warn!("record_view_by_path: failed to record view for {}: {}", id, e);
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                tracing::warn!(
                    "record_view_by_path: document not found (project_id={}, file_path={})",
                    project_id,
                    file_path
                );
            }
            Err(e) => {
                tracing::warn!("record_view_by_path: db error (project_id={}, file_path={}): {}", project_id, file_path, e);
            }
        }
    }
}

// ─── 인기 문서 랭킹 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopularDoc {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub project_id: String,
    pub project_name: String,
    pub view_count: i64,
    pub backlink_count: i64,
    pub score: f64,
    pub last_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopProject {
    pub id: String,
    pub name: String,
    pub activity: i64,
}

/// 인기 문서 랭킹 조회.
/// score = view_count * 0.6 + backlink_count * 0.4
/// score가 모두 0이면 last_modified DESC 기준으로 최신 문서 정렬.
pub fn get_popular_documents(
    pool: &DbPool,
    project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<PopularDoc>> {
    let conn = pool.get()?;
    let limit_i = limit as i64;

    let base_sql = "
        SELECT d.id, d.file_path, COALESCE(d.title, d.file_path) AS title,
               d.project_id,
               p.name AS project_name,
               COALESCE(vc.view_count, 0) AS view_count,
               COALESCE(bl.backlink_count, 0) AS backlink_count,
               (COALESCE(vc.view_count, 0) * 0.6 + COALESCE(bl.backlink_count, 0) * 0.4) AS score,
               d.last_indexed
        FROM documents d
        JOIN projects p ON d.project_id = p.id
        LEFT JOIN (
            SELECT document_id, COUNT(*) AS view_count
            FROM document_views
            GROUP BY document_id
        ) vc ON vc.document_id = d.id
        LEFT JOIN (
            SELECT target_doc_id, COUNT(*) AS backlink_count
            FROM wiki_links
            WHERE target_doc_id IS NOT NULL
            GROUP BY target_doc_id
        ) bl ON bl.target_doc_id = d.id
        WHERE d.indexing_status = 'done'";

    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<PopularDoc> {
        Ok(PopularDoc {
            id: row.get(0)?,
            file_path: row.get(1)?,
            title: row.get(2)?,
            project_id: row.get(3)?,
            project_name: row.get(4)?,
            view_count: row.get(5)?,
            backlink_count: row.get(6)?,
            score: row.get(7)?,
            last_modified: row.get(8)?,
        })
    };

    // stmt를 바깥 스코프에 선언해야 MappedRows 라이프타임 에러 회피 가능
    let sql = if project_id.is_some() {
        format!("{} AND d.project_id = ?1 ORDER BY score DESC, d.last_indexed DESC LIMIT ?2", base_sql)
    } else {
        format!("{} ORDER BY score DESC, d.last_indexed DESC LIMIT ?1", base_sql)
    };
    let mut stmt = conn.prepare(&sql)?;

    let docs: Vec<PopularDoc> = if let Some(pid) = project_id {
        stmt.query_map(params![pid, limit_i], map_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        stmt.query_map(params![limit_i], map_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };

    Ok(docs)
}

/// 활동량 기준 상위 프로젝트 조회 (대시보드 탭 결정용).
/// activity = 프로젝트 내 총 view_count + backlink_count
pub fn get_top_projects(pool: &DbPool, limit: usize) -> Result<Vec<TopProject>> {
    let conn = pool.get()?;
    let limit_i = limit as i64;
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name,
                (COALESCE(vc.view_total, 0) + COALESCE(bl.link_total, 0)) AS activity
         FROM projects p
         LEFT JOIN (
             SELECT d.project_id, SUM(sub.vc) AS view_total
             FROM documents d
             JOIN (SELECT document_id, COUNT(*) AS vc FROM document_views GROUP BY document_id) sub
                  ON sub.document_id = d.id
             GROUP BY d.project_id
         ) vc ON vc.project_id = p.id
         LEFT JOIN (
             SELECT d.project_id, COUNT(wl.id) AS link_total
             FROM documents d
             JOIN wiki_links wl ON wl.target_doc_id = d.id
             GROUP BY d.project_id
         ) bl ON bl.project_id = p.id
         ORDER BY activity DESC
         LIMIT ?1",
    )?;
    let result = stmt
        .query_map(params![limit_i], |row| {
            Ok(TopProject {
                id: row.get(0)?,
                name: row.get(1)?,
                activity: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(result)
}

// ─── 관심 필요 문서 ─────────────────────────────────────────────────────────────

/// 관심 필요 문서 판별 기준 — 조정 시 여기만 수정
const ATTENTION_NEVER_VIEWED_GRACE_DAYS: i64 = 7; // 생성 후 이 기간은 미열람 제외
const ATTENTION_ORPHAN_GRACE_DAYS: i64 = 30; // 생성 후 이 기간은 고아 제외
const ATTENTION_ORPHAN_MAX_VIEWS: i64 = 3; // 이 조회수 미만 = 고아 후보
const ATTENTION_STALE_DAYS: i64 = 30; // 마지막 수정 후 이 기간 경과 = 오래됨
const ATTENTION_STALE_MAX_VIEWS: i64 = 5; // 이 조회수 미만 = 오래됨 후보

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionDoc {
    pub id: String,
    pub file_path: String,
    pub title: String,
    pub project_id: String,
    pub project_name: String,
    pub view_count: i64,
    pub backlink_count: i64,
    pub last_modified: Option<String>,
    pub created_at: Option<String>,
    pub reason: String, // "never_viewed" | "orphan" | "stale"
}

/// 관심이 필요한 문서 목록 조회.
/// reason 우선순위: never_viewed > orphan > stale
pub fn get_attention_documents(
    pool: &DbPool,
    project_id: Option<&str>,
    limit: usize,
) -> Result<Vec<AttentionDoc>> {
    let conn = pool.get()?;
    let limit_i = limit as i64;

    let project_filter = if project_id.is_some() {
        "AND d.project_id = ?1"
    } else {
        ""
    };

    let sql = format!(
        "WITH counts AS (
            SELECT d.id,
                   d.file_path,
                   COALESCE(d.title, d.file_path) AS title,
                   d.project_id,
                   p.name AS project_name,
                   d.last_modified,
                   d.created_at,
                   COALESCE(vc.view_count, 0) AS view_count,
                   COALESCE(bl.backlink_count, 0) AS backlink_count
            FROM documents d
            JOIN projects p ON d.project_id = p.id
            LEFT JOIN (
                SELECT document_id, COUNT(*) AS view_count
                FROM document_views
                GROUP BY document_id
            ) vc ON vc.document_id = d.id
            LEFT JOIN (
                SELECT target_doc_id, COUNT(*) AS backlink_count
                FROM wiki_links
                WHERE target_doc_id IS NOT NULL
                GROUP BY target_doc_id
            ) bl ON bl.target_doc_id = d.id
            WHERE d.indexing_status = 'done'
            {project_filter}
        ),
        labeled AS (
            SELECT *,
                   CASE
                       WHEN view_count = 0
                            AND (julianday('now') - julianday(created_at)) > {never_viewed_grace}
                           THEN 'never_viewed'
                       WHEN backlink_count = 0
                            AND view_count < {orphan_max_views}
                            AND (julianday('now') - julianday(created_at)) > {orphan_grace}
                           THEN 'orphan'
                       WHEN (julianday('now') - julianday(last_modified)) > {stale_days}
                            AND view_count < {stale_max_views}
                           THEN 'stale'
                       ELSE NULL
                   END AS reason
            FROM counts
        )
        SELECT id, file_path, title, project_id, project_name,
               view_count, backlink_count, last_modified, created_at, reason
        FROM labeled
        WHERE reason IS NOT NULL
        ORDER BY
            CASE reason
                WHEN 'never_viewed' THEN 1
                WHEN 'orphan' THEN 2
                WHEN 'stale' THEN 3
            END,
            created_at DESC
        LIMIT {limit_placeholder}",
        project_filter = project_filter,
        never_viewed_grace = ATTENTION_NEVER_VIEWED_GRACE_DAYS,
        orphan_grace = ATTENTION_ORPHAN_GRACE_DAYS,
        orphan_max_views = ATTENTION_ORPHAN_MAX_VIEWS,
        stale_days = ATTENTION_STALE_DAYS,
        stale_max_views = ATTENTION_STALE_MAX_VIEWS,
        limit_placeholder = if project_id.is_some() { "?2" } else { "?1" },
    );

    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<AttentionDoc> {
        Ok(AttentionDoc {
            id: row.get(0)?,
            file_path: row.get(1)?,
            title: row.get(2)?,
            project_id: row.get(3)?,
            project_name: row.get(4)?,
            view_count: row.get(5)?,
            backlink_count: row.get(6)?,
            last_modified: row.get(7)?,
            created_at: row.get(8)?,
            reason: row.get(9)?,
        })
    };

    let mut stmt = conn.prepare(&sql)?;
    let docs: Vec<AttentionDoc> = if let Some(pid) = project_id {
        stmt.query_map(params![pid, limit_i], map_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        stmt.query_map(params![limit_i], map_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    };

    Ok(docs)
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

        let results = fts_search(&pool, "rust", Some(&proj.id), 10, None).unwrap();
        assert!(!results.is_empty(), "Should find 'rust' in indexed docs");
        assert!(results.iter().any(|r| r.file_path.contains("note1.md")));
    }

    #[test]
    fn test_fts_search_korean() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "한국어", Some(&proj.id), 10, None).unwrap();
        assert!(!results.is_empty(), "Should find Korean text");
        assert!(results.iter().any(|r| r.file_path.contains("korean.md")));
    }

    #[test]
    fn test_fts_search_no_results() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "xyznonexistent", Some(&proj.id), 10, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fts_search_empty_query() {
        let pool = test_pool();
        let results = fts_search(&pool, "", None, 10, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fts_search_cross_project() {
        let pool = test_pool();
        let _proj = setup_indexed_project(&pool);

        // Search without project filter
        let results = fts_search(&pool, "programming", None, 10, None).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fts_search_has_snippet() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "ownership", Some(&proj.id), 10, None).unwrap();
        assert!(!results.is_empty());
        assert!(!results[0].snippet.is_empty(), "Snippet should not be empty");
    }

    #[test]
    fn test_fts_search_has_heading_path() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "ownership", Some(&proj.id), 10, None).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].heading_path.is_some(), "Should have heading path");
    }

    #[test]
    fn test_fts_search_limit() {
        let pool = test_pool();
        let proj = setup_indexed_project(&pool);

        let results = fts_search(&pool, "note", Some(&proj.id), 1, None).unwrap();
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
        let results = fts_search(&pool, "content:password OR heading_path:secret", None, 10, None).unwrap();
        // Should not crash, may return empty
        let _ = results;
    }
}
