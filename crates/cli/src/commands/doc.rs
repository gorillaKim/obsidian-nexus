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
    /// Resolve an alias to a document
    ResolveAlias {
        /// Alias to resolve
        alias: String,
        /// Project ID or name (optional, searches all projects if omitted)
        #[arg(long)]
        project: Option<String>,
    },
    /// Extract a specific section by heading
    Section {
        /// Project ID or name
        project: String,
        /// File path (relative to vault root)
        path: String,
        /// Heading text to extract
        heading: String,
    },
    /// Get documents that link TO this document (backlinks)
    Backlinks {
        /// Project ID or name
        project: String,
        /// File path (relative to vault root)
        path: String,
    },
    /// Get documents that this document links to (forward links)
    Links {
        /// Project ID or name
        project: String,
        /// File path (relative to vault root)
        path: String,
    },
    /// Get table of contents for a document
    Toc {
        /// Project ID or name
        project: String,
        /// File path (relative to vault root)
        path: String,
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
        DocCommands::ResolveAlias { alias, project } => {
            if let Some(proj_name) = project {
                let proj = nexus_core::project::get_project(pool, &proj_name)?;
                let result = nexus_core::search::resolve_by_alias(pool, &proj.id, &alias)?;
                match result {
                    Some(doc) => {
                        if format == "json" {
                            println!("{}", serde_json::to_string_pretty(&doc)?);
                        } else {
                            println!("{} — {}", doc.file_path, doc.title.as_deref().unwrap_or(""));
                        }
                    }
                    None => {
                        if format == "json" {
                            println!("null");
                        } else {
                            println!("No document found for alias: {}", alias);
                        }
                    }
                }
            } else {
                // Search all projects
                let projects = nexus_core::project::list_projects(pool)?;
                let mut found = false;
                for proj in &projects {
                    if let Ok(Some(doc)) = nexus_core::search::resolve_by_alias(pool, &proj.id, &alias) {
                        if format == "json" {
                            println!("{}", serde_json::to_string_pretty(&doc)?);
                        } else {
                            println!("[{}] {} — {}", proj.name, doc.file_path, doc.title.as_deref().unwrap_or(""));
                        }
                        found = true;
                        break;
                    }
                }
                if !found {
                    if format == "json" {
                        println!("null");
                    } else {
                        println!("No document found for alias: {}", alias);
                    }
                }
            }
        }
        DocCommands::Section { project, path, heading } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let section = nexus_core::search::get_section(pool, &proj.id, &path, &heading, None)?;
            println!("{}", section);
        }
        DocCommands::Backlinks { project, path } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let backlinks = nexus_core::search::get_backlinks(pool, &proj.id, &path)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&backlinks)?);
            } else {
                if backlinks.is_empty() {
                    println!("No backlinks found for: {}", path);
                } else {
                    for bl in &backlinks {
                        println!("{} — {}", bl.source_file_path, bl.source_title.as_deref().unwrap_or(""));
                    }
                }
            }
        }
        DocCommands::Links { project, path } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let links = nexus_core::search::get_forward_links(pool, &proj.id, &path)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&links)?);
            } else {
                if links.is_empty() {
                    println!("No links found in: {}", path);
                } else {
                    for link in &links {
                        let status = if link.resolved { "✓" } else { "✗" };
                        println!("[{}] {} — {}", status, link.target_path, link.display_text.as_deref().unwrap_or(""));
                    }
                }
            }
        }
        DocCommands::Toc { project, path } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let toc = nexus_core::search::get_toc(pool, &proj.id, &path)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&toc)?);
            } else {
                for entry in &toc {
                    let indent = "  ".repeat(entry.level.saturating_sub(1));
                    println!("{}{}. {}", indent, entry.level, entry.heading);
                }
            }
        }
    }
    Ok(())
}
