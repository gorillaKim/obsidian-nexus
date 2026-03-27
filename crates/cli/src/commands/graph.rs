use anyhow::Result;
use clap::Subcommand;
use nexus_core::db::sqlite::DbPool;

#[derive(Subcommand)]
pub enum GraphCommands {
    /// Find related documents using RRF over link-distance and tag overlap
    Related {
        /// Project ID or name
        project: String,
        /// File path (relative to vault root)
        path: String,
        /// Max results
        #[arg(long, default_value = "10")]
        k: usize,
    },
    /// Find shortest link path between two documents (max 6 hops)
    Path {
        /// Project ID or name
        project: String,
        /// Source file path
        from: String,
        /// Target file path
        to: String,
    },
    /// Get all documents reachable within N link hops
    Cluster {
        /// Project ID or name
        project: String,
        /// File path (relative to vault root)
        path: String,
        /// Max hop depth (default: 2, max: 5)
        #[arg(long, default_value = "2")]
        depth: i64,
    },
}

pub fn handle(pool: &DbPool, cmd: GraphCommands, format: &str) -> Result<()> {
    match cmd {
        GraphCommands::Related { project, path, k } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let results = nexus_core::search::find_related(pool, &proj.id, &path, k)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                if results.is_empty() {
                    println!("No related documents found for: {}", path);
                } else {
                    for r in &results {
                        println!(
                            "[{:.4}] {} — {} ({})",
                            r.score,
                            r.file_path,
                            r.title.as_deref().unwrap_or(""),
                            r.signals.join(", ")
                        );
                    }
                }
            }
        }
        GraphCommands::Path { project, from, to } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let result = nexus_core::search::find_path(pool, &proj.id, &from, &to)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                match result {
                    Some(p) => {
                        println!("Path ({} hops): {}", p.hops, p.path.join(" → "));
                    }
                    None => println!("No path found between '{}' and '{}'", from, to),
                }
            }
        }
        GraphCommands::Cluster { project, path, depth } => {
            let proj = nexus_core::project::get_project(pool, &project)?;
            let nodes = nexus_core::search::get_cluster(pool, &proj.id, &path, depth)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&nodes)?);
            } else {
                if nodes.is_empty() {
                    println!("No linked documents found for: {}", path);
                } else {
                    for n in &nodes {
                        let tags = if n.tags.is_empty() {
                            String::new()
                        } else {
                            format!(" [{}]", n.tags.join(", "))
                        };
                        println!(
                            "(d={}) {} — {}{}",
                            n.distance,
                            n.file_path,
                            n.title.as_deref().unwrap_or(""),
                            tags
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
