use anyhow::Result;
use nexus_core::db::sqlite::DbPool;

pub fn handle_ranking(
    pool: &DbPool,
    project: Option<&str>,
    limit: usize,
    format: &str,
) -> Result<()> {
    let project_id = if let Some(p) = project {
        let proj = nexus_core::project::get_project(pool, p)?;
        Some(proj.id)
    } else {
        None
    };

    let docs = nexus_core::search::get_popular_documents(pool, project_id.as_deref(), limit)?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&docs)?);
    } else {
        let scope = project.unwrap_or("전체");
        println!("📊 랭킹 — {scope} (상위 {limit}개)\n");
        for (i, doc) in docs.iter().enumerate() {
            println!(
                "{}. [{}] {}",
                i + 1,
                doc.project_name,
                doc.title,
            );
            println!(
                "   조회 {}  백링크 {}  점수 {:.2}",
                doc.view_count, doc.backlink_count, doc.score
            );
            println!("   {}", doc.file_path);
            println!();
        }
    }

    Ok(())
}
