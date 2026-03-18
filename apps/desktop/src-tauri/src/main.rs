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
    let abs_path = std::path::Path::new(&proj.path).join(&file_path);

    if !abs_path.exists() {
        return Err(format!("File not found: {}", abs_path.display()));
    }

    // Check if Obsidian is installed (macOS)
    let obsidian_installed = std::path::Path::new("/Applications/Obsidian.app").exists()
        || dirs::home_dir()
            .map(|h| h.join("Applications/Obsidian.app").exists())
            .unwrap_or(false);

    if obsidian_installed {
        // Use `open -a Obsidian <file_path>` — this opens the file directly
        // Obsidian will auto-detect the vault from the file's parent directory
        let status = std::process::Command::new("open")
            .args(["-a", "Obsidian", abs_path.to_str().unwrap_or("")])
            .spawn();
        match status {
            Ok(_) => Ok(serde_json::json!({ "opened_with": "obsidian", "path": abs_path.to_string_lossy() })),
            Err(_) => {
                // Fallback to system default
                let _ = std::process::Command::new("open").arg(&abs_path).spawn();
                Ok(serde_json::json!({ "opened_with": "system", "path": abs_path.to_string_lossy() }))
            }
        }
    } else {
        // Fallback: open with system default editor
        let _ = std::process::Command::new("open").arg(&abs_path).spawn();
        Ok(serde_json::json!({ "opened_with": "system", "path": abs_path.to_string_lossy() }))
    }
}

/// Add a new project: register vault path, index, and open in Obsidian
#[tauri::command]
fn add_project(
    state: State<AppState>,
    name: String,
    path: String,
) -> Result<serde_json::Value, String> {
    // Derive vault_name from folder name
    let vault_name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| name.clone());

    let proj = nexus_core::project::add_project(&state.pool, &name, &path, Some(&vault_name))
        .map_err(|e| e.to_string())?;

    // Auto-index
    let report = nexus_core::index_engine::index_project(&state.pool, &proj.id, false)
        .map_err(|e| e.to_string())?;

    // Auto-register in Obsidian
    let abs_path = std::path::Path::new(&path).canonicalize().unwrap_or(std::path::PathBuf::from(&path));
    let _ = std::process::Command::new("open")
        .args(["-a", "Obsidian", abs_path.to_str().unwrap_or(&path)])
        .spawn();

    Ok(serde_json::json!({
        "project": proj,
        "index_report": {
            "indexed": report.indexed,
            "errors": report.errors,
        }
    }))
}

/// Remove a project
#[tauri::command]
fn remove_project(state: State<AppState>, project_id: String) -> Result<(), String> {
    nexus_core::project::remove_project(&state.pool, &project_id).map_err(|e| e.to_string())
}

/// List documents in a project
#[tauri::command]
fn list_documents(
    state: State<AppState>,
    project_id: String,
) -> Result<Vec<nexus_core::search::DocumentInfo>, String> {
    nexus_core::search::list_documents(&state.pool, &project_id, None).map_err(|e| e.to_string())
}

/// Check and install Obsidian if not present (macOS only)
fn ensure_obsidian() {
    let installed = std::path::Path::new("/Applications/Obsidian.app").exists()
        || dirs::home_dir()
            .map(|h| h.join("Applications/Obsidian.app").exists())
            .unwrap_or(false);

    if !installed {
        eprintln!("Obsidian not found, attempting to install via brew...");
        let status = std::process::Command::new("brew")
            .args(["install", "--cask", "obsidian"])
            .status();
        match status {
            Ok(s) if s.success() => eprintln!("Obsidian installed successfully"),
            _ => eprintln!("Could not auto-install Obsidian. Install from https://obsidian.md"),
        }
    }
}

fn main() {
    // Auto-setup on first launch
    ensure_obsidian();
    register_mcp_server();

    let pool = sqlite::create_pool().expect("Failed to create database pool");
    sqlite::run_migrations(&pool).expect("Failed to run migrations");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState { pool })
        .invoke_handler(tauri::generate_handler![
            list_projects,
            search_documents,
            index_project,
            get_document,
            project_info,
            open_file,
            add_project,
            remove_project,
            list_documents,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
