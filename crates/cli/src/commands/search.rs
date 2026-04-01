use anyhow::Result;
use nexus_core::db::sqlite::DbPool;

#[allow(clippy::too_many_arguments)]
pub fn handle_search(
    pool: &DbPool,
    query: Option<&str>,
    project_id: Option<&str>,
    limit: usize,
    offset: usize,
    mode: &str,
    sort_by: &str,
    date_from: Option<&str>,
    date_to: Option<&str>,
    tags: Option<&str>,
    tag_match_all: bool,
    format: &str,
) -> Result<()> {
    let resolved_pid = if let Some(pid) = project_id {
        let proj = nexus_core::project::get_project(pool, pid)?;
        Some(proj.id)
    } else {
        None
    };

    // 태그 필터
    let tag_strings: Vec<String> = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    let tag_filter = if tag_strings.is_empty() {
        None
    } else {
        Some(nexus_core::search::TagFilter::new(tag_strings, tag_match_all))
    };

    // 날짜 필터
    let date_filter = if date_from.is_some() || date_to.is_some() {
        Some(nexus_core::search::DateFilter {
            date_from: date_from.map(str::to_string),
            date_to: date_to.map(str::to_string),
            field: nexus_core::search::DateField::LastModified,
        })
    } else {
        None
    };

    let cli_sort_by = match sort_by {
        "date_desc" => nexus_core::search::SortBy::DateDesc,
        "date_asc" => nexus_core::search::SortBy::DateAsc,
        _ => nexus_core::search::SortBy::Relevance,
    };

    let results = if let Some(q) = query {
        match mode {
            "vector" => {
                let config = nexus_core::Config::load()?;
                nexus_core::search::vector_search(pool, q, resolved_pid.as_deref(), limit, offset, &config, tag_filter.as_ref(), date_filter.as_ref())?
            }
            "hybrid" => {
                let config = nexus_core::Config::load()?;
                nexus_core::search::hybrid_search(pool, q, resolved_pid.as_deref(), limit, offset, &config, tag_filter.as_ref(), date_filter.as_ref())?
            }
            _ => {
                nexus_core::search::fts_search(pool, q, resolved_pid.as_deref(), limit, offset, tag_filter.as_ref(), date_filter.as_ref())?
            }
        }
    } else {
        // filter-only mode
        nexus_core::search::filter_search(pool, resolved_pid.as_deref(), limit, offset, tag_filter.as_ref(), date_filter.as_ref(), cli_sort_by)?
    };

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        if results.is_empty() {
            println!("No results.");
            return Ok(());
        }
        for r in &results {
            println!("[{}] {} — {}", r.project_name, r.file_path, r.heading_path.as_deref().unwrap_or(""));
            println!("  {}", r.snippet);
            println!();
        }
        println!("({} results, offset={})", results.len(), offset);
    }
    Ok(())
}

pub fn handle_get_docs(
    pool: &DbPool,
    paths: &[String],
    project: Option<&str>,
    format: &str,
) -> Result<()> {
    let paths: Vec<&str> = paths.iter().map(String::as_str).take(5).collect();

    let mut success: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let mut errors: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();

    for path in paths {
        let (proj_id, file_path) = if let Some(p) = project {
            match nexus_core::project::get_project(pool, p) {
                Ok(proj) => (proj.id, path.to_string()),
                Err(e) => { errors.insert(path.to_string(), e.to_string()); continue; }
            }
        } else {
            let parts: Vec<&str> = path.splitn(2, '/').collect();
            if parts.len() < 2 {
                errors.insert(path.to_string(), "Use 'project/path' format or --project".to_string());
                continue;
            }
            match nexus_core::project::get_project(pool, parts[0]) {
                Ok(proj) => (proj.id, parts[1].to_string()),
                Err(e) => { errors.insert(path.to_string(), e.to_string()); continue; }
            }
        };

        match nexus_core::search::get_document_content(pool, &proj_id, &file_path) {
            Ok(content) => { success.insert(path.to_string(), content); }
            Err(e) => { errors.insert(path.to_string(), e.to_string()); }
        }
    }

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({ "success": success, "errors": errors }))?);
    } else {
        for (path, content) in &success {
            println!("=== {} ===", path);
            println!("{}", content);
            println!();
        }
        for (path, err) in &errors {
            eprintln!("ERROR {}: {}", path, err);
        }
    }
    Ok(())
}
