use anyhow::Result;
use clap::Subcommand;
use nexus_core::db::sqlite::DbPool;

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Register a vault path as a project
    Add {
        /// Project name
        #[arg(long)]
        name: String,
        /// Path to the Obsidian vault
        #[arg(long)]
        path: String,
        /// Obsidian vault name (for deep links)
        #[arg(long)]
        vault: Option<String>,
    },
    /// List all registered projects
    List,
    /// Remove a project (only removes index, not files)
    Remove {
        /// Project ID or name
        id: String,
    },
    /// Show project details
    Info {
        /// Project ID or name
        id: String,
    },
    /// Update vault path (for relocated vaults)
    Update {
        /// Project ID or name
        id: String,
        /// New path to the vault
        #[arg(long)]
        path: String,
    },
}

pub fn handle(pool: &DbPool, cmd: ProjectCommands, format: &str) -> Result<()> {
    match cmd {
        ProjectCommands::Add { name, path, vault } => {
            let project = nexus_core::project::add_project(pool, &name, &path, vault.as_deref())?;
            print_output(format, &project)?;
            eprintln!("Project '{}' added successfully.", project.name);
        }
        ProjectCommands::List => {
            let projects = nexus_core::project::list_projects(pool)?;
            print_output(format, &projects)?;
        }
        ProjectCommands::Remove { id } => {
            nexus_core::project::remove_project(pool, &id)?;
            eprintln!("Project '{}' removed.", id);
        }
        ProjectCommands::Info { id } => {
            let (project, stats) = nexus_core::project::project_info(pool, &id)?;
            let info = serde_json::json!({
                "project": project,
                "stats": stats,
            });
            print_output(format, &info)?;
        }
        ProjectCommands::Update { id, path } => {
            let project = nexus_core::project::update_project_path(pool, &id, &path)?;
            print_output(format, &project)?;
            eprintln!("Project path updated to: {}", project.path);
        }
    }
    Ok(())
}

fn print_output<T: serde::Serialize>(format: &str, data: &T) -> Result<()> {
    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
        "text" => {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
        _ => {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
    }
    Ok(())
}
