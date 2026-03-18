use anyhow::Result;
use nexus_core::db::sqlite::DbPool;

pub fn handle_search(
    pool: &DbPool,
    query: &str,
    project_id: Option<&str>,
    limit: usize,
    mode: &str,
    format: &str,
) -> Result<()> {
    // Resolve project name to ID if needed
    let resolved_pid = if let Some(pid) = project_id {
        let proj = nexus_core::project::get_project(pool, pid)?;
        Some(proj.id)
    } else {
        None
    };

    let results = match mode {
        "vector" => {
            let config = nexus_core::Config::load()?;
            nexus_core::search::vector_search(pool, query, resolved_pid.as_deref(), limit, &config)?
        }
        "hybrid" => {
            let config = nexus_core::Config::load()?;
            nexus_core::search::hybrid_search(pool, query, resolved_pid.as_deref(), limit, &config)?
        }
        _ => {
            // "keyword" or default
            nexus_core::search::fts_search(pool, query, resolved_pid.as_deref(), limit)?
        }
    };

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        for r in &results {
            println!("[{}] {} — {}", r.project_name, r.file_path, r.heading_path.as_deref().unwrap_or(""));
            println!("  {}", r.snippet);
            println!();
        }
    }
    Ok(())
}
