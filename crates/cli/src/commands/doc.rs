use anyhow::Result;
use clap::Subcommand;
use nexus_core::db::sqlite::DbPool;

#[derive(Subcommand)]
pub enum DocCommands {
    /// Get document content
    Get {
        /// Project ID or name
        project: String,
        /// File path (relative to vault root)
        path: String,
    },
    /// Get document metadata
    Meta {
        /// Project ID or name
        project: String,
        /// File path (relative to vault root)
        path: String,
    },
    /// List documents in a project
    List {
        /// Project ID or name
        project: String,
        /// Filter by tag
        #[arg(long)]
        tags: Option<String>,
    },
}

pub fn handle(pool: &DbPool, cmd: DocCommands, format: &str) -> Result<()> {
    match cmd {
        DocCommands::Get { project, path } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let content = nexus_core::search::get_document_content(pool, &proj.id, &path)?;
            println!("{}", content);
        }
        DocCommands::Meta { project, path } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let meta = nexus_core::search::get_document_meta(pool, &proj.id, &path)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&meta)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&meta)?);
            }
        }
        DocCommands::List { project, tags } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let docs = nexus_core::search::list_documents(pool, &proj.id, tags.as_deref())?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&docs)?);
            } else {
                for d in &docs {
                    println!("{} [{}] {}", d.file_path, d.indexing_status.as_deref().unwrap_or("?"), d.title.as_deref().unwrap_or(""));
                }
            }
        }
    }
    Ok(())
}
