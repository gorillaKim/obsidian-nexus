// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use nexus_core::db::sqlite::{self, DbPool};
use nexus_core::project::Project;
use nexus_core::search::SearchResult;
use tauri::State;

struct AppState {
    pool: DbPool,
}

#[tauri::command]
fn list_projects(state: State<AppState>) -> Result<Vec<Project>, String> {
    nexus_core::project::list_projects(&state.pool).map_err(|e| e.to_string())
}

#[tauri::command]
fn search_documents(
    state: State<AppState>,
    query: String,
    project_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let resolved_pid = if let Some(ref pid) = project_id {
        let proj = nexus_core::project::get_project(&state.pool, pid).map_err(|e| e.to_string())?;
        Some(proj.id)
    } else {
        None
    };

    nexus_core::search::fts_search(
        &state.pool,
        &query,
        resolved_pid.as_deref(),
        limit.unwrap_or(20),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn index_project(state: State<AppState>, project_id: String) -> Result<String, String> {
    let report = nexus_core::index_engine::index_project(&state.pool, &project_id, false)
        .map_err(|e| e.to_string())?;
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_document(
    state: State<AppState>,
    project_id: String,
    file_path: String,
) -> Result<String, String> {
    nexus_core::search::get_document_content(&state.pool, &project_id, &file_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn project_info(
    state: State<AppState>,
    project_id: String,
) -> Result<serde_json::Value, String> {
    let (proj, stats) = nexus_core::project::project_info(&state.pool, &project_id)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "project": proj,
        "stats": {
            "doc_count": stats.doc_count,
            "chunk_count": stats.chunk_count,
            "pending_count": stats.pending_count,
        }
    }))
}

/// Register MCP server in Claude Code config on first launch
fn register_mcp_server() {
    let mcp_binary = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("nexus-mcp-server")))
        .unwrap_or_default();

    if !mcp_binary.exists() {
        // In dev mode, try the release build path
        let alt = dirs::home_dir()
            .map(|h| h.join("gorillaProject/obsidian-nexus/target/release/nexus-mcp-server"));
        if let Some(ref alt_path) = alt {
            if !alt_path.exists() {
                return;
            }
        }
    }

    // Claude Code config: ~/.claude.json (Claude Desktop) or ~/.claude/claude_desktop_config.json
    let config_paths = [
        dirs::home_dir().map(|h| h.join(".claude/claude_desktop_config.json")),
        dirs::home_dir().map(|h| h.join(".claude.json")),
    ];

    for config_path in config_paths.iter().flatten() {
        let mcp_path = if mcp_binary.exists() {
            mcp_binary.to_string_lossy().to_string()
        } else {
            dirs::home_dir()
                .map(|h| h.join("gorillaProject/obsidian-nexus/target/release/nexus-mcp-server"))
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };

        let mut config: serde_json::Value = if config_path.exists() {
            let content = std::fs::read_to_string(config_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        // Check if already registered
        if config.get("mcpServers")
            .and_then(|s| s.get("nexus"))
            .is_some()
        {
            return; // Already registered
        }

        // Add nexus MCP server
        let servers = config.as_object_mut().unwrap()
            .entry("mcpServers")
            .or_insert(serde_json::json!({}));
        servers.as_object_mut().unwrap().insert(
            "nexus".to_string(),
            serde_json::json!({
                "command": mcp_path,
                "args": []
            }),
        );

        // Write config
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&config) {
            if std::fs::write(config_path, content).is_ok() {
                eprintln!("Nexus MCP server registered in {}", config_path.display());
                return;
            }
        }
    }
}

fn main() {
    // Auto-register MCP server on first launch
    register_mcp_server();

    let pool = sqlite::create_pool().expect("Failed to create database pool");
    sqlite::run_migrations(&pool).expect("Failed to run migrations");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState { pool })
        .invoke_handler(tauri::generate_handler![
            list_projects,
            search_documents,
            index_project,
            get_document,
            project_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
