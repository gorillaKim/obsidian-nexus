use anyhow::Result;
use nexus_core::db::sqlite::DbPool;

pub fn handle_index(pool: &DbPool, project_id: Option<&str>, all: bool, full: bool, format: &str) -> Result<()> {
    if all {
        let results = nexus_core::index_engine::index_all(pool, full)?;
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&results)?);
        } else {
            for (name, report) in &results {
                eprintln!(
                    "Project '{}': indexed={}, unchanged={}, skipped={}, errors={}",
                    name, report.indexed, report.unchanged, report.skipped, report.errors
                );
            }
        }
    } else if let Some(pid) = project_id {
        let report = nexus_core::index_engine::index_project(pool, pid, full)?;
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            eprintln!(
                "Indexed: {}, unchanged: {}, skipped: {}, errors: {}",
                report.indexed, report.unchanged, report.skipped, report.errors
            );
        }
    } else {
        anyhow::bail!("Specify a project ID or use --all");
    }
    Ok(())
}
