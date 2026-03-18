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

/// Find the MCP server binary path
fn find_mcp_binary() -> Option<String> {
    // 1. Same directory as current exe
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("nexus-mcp-server");
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    // 2. Release build path (dev mode)
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join("gorillaProject/obsidian-nexus/target/release/nexus-mcp-server");
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// Register MCP server in a JSON config file under "mcpServers.nexus"
fn register_in_config(config_path: &std::path::Path, mcp_path: &str) -> bool {
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(config_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Already registered?
    if config.get("mcpServers").and_then(|s| s.get("nexus")).is_some() {
        return true;
    }

    let servers = config.as_object_mut().unwrap()
        .entry("mcpServers")
        .or_insert(serde_json::json!({}));
    servers.as_object_mut().unwrap().insert(
        "nexus".to_string(),
        serde_json::json!({ "command": mcp_path, "args": [] }),
    );

    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(&config) {
        if std::fs::write(config_path, content).is_ok() {
            eprintln!("Nexus MCP server registered in {}", config_path.display());
            return true;
        }
    }
    false
}

/// Register MCP server in both Claude Desktop and Claude Code configs
fn register_mcp_server() {
    let mcp_path = match find_mcp_binary() {
        Some(p) => p,
        None => return,
    };

    if let Some(home) = dirs::home_dir() {
        // Claude Desktop
        register_in_config(&home.join(".claude/claude_desktop_config.json"), &mcp_path);
        // Claude Code (settings.json)
        register_in_config(&home.join(".claude/settings.json"), &mcp_path);
    }
}

/// Open a file: try Obsidian deeplink first, fallback to system default editor
#[tauri::command]
fn open_file(
    state: State<AppState>,
    project_id: String,
    file_path: String,
) -> Result<serde_json::Value, String> {
    let proj = nexus_core::project::get_project(&state.pool, &project_id)
        .map_err(|e| e.to_string())?;
    let vault_name = proj.vault_name.as_deref().unwrap_or(&proj.name);
    let abs_path = std::path::Path::new(&proj.path).join(&file_path);

    // Try Obsidian deeplink
    let obsidian_uri = format!(
        "obsidian://open?vault={}&file={}",
        urlencoding::encode(vault_name),
        urlencoding::encode(&file_path),
    );

    // Check if Obsidian is installed (macOS)
    let obsidian_installed = std::path::Path::new("/Applications/Obsidian.app").exists()
        || dirs::home_dir()
            .map(|h| h.join("Applications/Obsidian.app").exists())
            .unwrap_or(false);

    if obsidian_installed {
        // Open via Obsidian deeplink
        let _ = std::process::Command::new("open").arg(&obsidian_uri).spawn();
        Ok(serde_json::json!({ "opened_with": "obsidian", "uri": obsidian_uri }))
    } else if abs_path.exists() {
        // Fallback: open with system default editor
        let _ = std::process::Command::new("open").arg(&abs_path).spawn();
        Ok(serde_json::json!({ "opened_with": "system", "path": abs_path.to_string_lossy() }))
    } else {
        Err(format!("File not found: {}", abs_path.display()))
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
            open_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
