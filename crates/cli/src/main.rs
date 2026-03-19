use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "nexus", about = "Obsidian Nexus — Agent-friendly knowledge search")]
#[command(version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, default_value = "json", global = true)]
    format: String,

    /// Suppress non-essential output
    #[arg(long, global = true)]
    quiet: bool,

    /// Verbose output
    #[arg(long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage projects (vaults)
    Project {
        #[command(subcommand)]
        command: commands::project::ProjectCommands,
    },
    /// Index documents in a project
    Index {
        /// Project ID or name (omit for --all)
        project: Option<String>,
        /// Index all projects
        #[arg(long)]
        all: bool,
        /// Full re-index (ignore content hash)
        #[arg(long)]
        full: bool,
    },
    /// Search across indexed documents
    Search {
        /// Search query
        query: String,
        /// Limit to a specific project
        #[arg(long)]
        project: Option<String>,
        /// Search mode: keyword, vector, hybrid
        #[arg(long, default_value = "keyword")]
        mode: String,
        /// Max results
        #[arg(long, default_value = "20")]
        limit: usize,
    },
    /// Setup Obsidian Nexus (install dependencies, init DB)
    Setup,
    /// Set up librarian skill & subagent in a project (.mcp.json, .claude/agents, .claude/skills)
    Onboard {
        /// Target project root path (default: current directory)
        project_path: Option<String>,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Access document content and metadata
    Doc {
        #[command(subcommand)]
        command: commands::doc::DocCommands,
    },
    /// Watch vault for changes and auto-index
    Watch {
        /// Project ID or name (omit to watch all)
        project: Option<String>,
    },
    /// Check for updates and install new version
    Update {
        /// Only check, don't install
        #[arg(long)]
        check: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else if cli.quiet { "error" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(log_level)
        .with_target(false)
        .init();

    // Initialize database
    let pool = nexus_core::db::sqlite::create_pool()?;
    nexus_core::db::sqlite::run_migrations(&pool)?;

    // Setup and Onboard don't need DB
    if matches!(cli.command, Commands::Setup) {
        return commands::setup::handle_setup();
    }
    if let Commands::Onboard { ref project_path, force } = cli.command {
        return commands::onboard::handle_onboard(project_path.as_deref(), force).map_err(Into::into);
    }
    if let Commands::Update { check } = cli.command {
        return commands::update::handle_update(check, &cli.format);
    }

    match cli.command {
        Commands::Setup => unreachable!(),
        Commands::Onboard { .. } => unreachable!(),
        Commands::Project { command } => {
            commands::project::handle(&pool, command, &cli.format)?;
        }
        Commands::Index { project, all, full } => {
            commands::index::handle_index(&pool, project.as_deref(), all, full, &cli.format)?;
        }
        Commands::Search { query, project, mode, limit } => {
            commands::search::handle_search(&pool, &query, project.as_deref(), limit, &mode, &cli.format)?;
        }
        Commands::Doc { command } => {
            commands::doc::handle(&pool, command, &cli.format)?;
        }
        Commands::Watch { project } => {
            commands::watch::handle_watch(&pool, project.as_deref())?;
        }
        Commands::Update { .. } => unreachable!(),
    }

    Ok(())
}
