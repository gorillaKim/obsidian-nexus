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
        /// Search query (optional — omit to use filter-only mode)
        query: Option<String>,
        /// Limit to a specific project
        #[arg(long)]
        project: Option<String>,
        /// Search mode: keyword, vector, hybrid
        #[arg(long, default_value = "keyword")]
        mode: String,
        /// Max results
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Pagination offset
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Filter: start date (ISO 8601, e.g. 2025-01-01)
        #[arg(long)]
        date_from: Option<String>,
        /// Filter: end date inclusive (ISO 8601, e.g. 2025-12-31)
        #[arg(long)]
        date_to: Option<String>,
        /// Sort order: relevance, date_desc, date_asc
        #[arg(long, default_value = "relevance")]
        sort_by: String,
        /// Filter by tags (comma-separated, e.g. rust,devlog)
        #[arg(long)]
        tags: Option<String>,
        /// Require ALL tags to match (AND mode). Default: OR
        #[arg(long)]
        tag_match_all: bool,
    },
    /// Read multiple documents at once (up to 5)
    GetDocs {
        /// File paths (space-separated, use project/path format or with --project)
        paths: Vec<String>,
        /// Project name or ID
        #[arg(long)]
        project: Option<String>,
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
    /// Show popular document rankings (by view count + backlinks)
    Ranking {
        /// Limit to a specific project
        #[arg(long)]
        project: Option<String>,
        /// Max results
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    /// Check for updates and install new version
    Update {
        /// Only check, don't install
        #[arg(long)]
        check: bool,
        /// Force check (ignore 24h cache)
        #[arg(long)]
        force: bool,
    },
    /// Graph traversal: related documents, shortest path, link cluster
    Graph {
        #[command(subcommand)]
        command: commands::graph::GraphCommands,
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

    // Commands that don't need DB
    if matches!(cli.command, Commands::Setup) {
        return commands::setup::handle_setup();
    }
    if let Commands::Onboard { ref project_path, force } = cli.command {
        return commands::onboard::handle_onboard(project_path.as_deref(), force);
    }
    if let Commands::Update { check, force } = cli.command {
        return commands::update::handle_update(check, force, &cli.format);
    }

    // Initialize database
    let pool = nexus_core::db::sqlite::create_pool()?;
    nexus_core::db::sqlite::run_migrations(&pool)?;

    match cli.command {
        Commands::Setup => unreachable!(),
        Commands::Onboard { .. } => unreachable!(),
        Commands::Project { command } => {
            commands::project::handle(&pool, command, &cli.format)?;
        }
        Commands::Index { project, all, full } => {
            commands::index::handle_index(&pool, project.as_deref(), all, full, &cli.format)?;
        }
        Commands::Search { query, project, mode, limit, offset, date_from, date_to, sort_by, tags, tag_match_all } => {
            commands::search::handle_search(&pool, query.as_deref(), project.as_deref(), limit, offset, &mode, &sort_by, date_from.as_deref(), date_to.as_deref(), tags.as_deref(), tag_match_all, &cli.format)?;
        }
        Commands::GetDocs { paths, project } => {
            commands::search::handle_get_docs(&pool, &paths, project.as_deref(), &cli.format)?;
        }
        Commands::Doc { command } => {
            commands::doc::handle(&pool, command, &cli.format)?;
        }
        Commands::Watch { project } => {
            commands::watch::handle_watch(&pool, project.as_deref())?;
        }
        Commands::Ranking { project, limit } => {
            commands::ranking::handle_ranking(&pool, project.as_deref(), limit, &cli.format)?;
        }
        Commands::Graph { command } => {
            commands::graph::handle(&pool, command, &cli.format)?;
        }
        Commands::Update { .. } => unreachable!(),
    }

    Ok(())
}
