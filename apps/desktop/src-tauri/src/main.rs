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
    mode: Option<String>,
    hybrid_weight: Option<f64>,
    min_vector_score: Option<f64>,
    tags: Option<Vec<String>>,
) -> Result<Vec<SearchResult>, String> {
    let resolved_pid = if let Some(ref pid) = project_id {
        let proj = nexus_core::project::get_project(&state.pool, pid).map_err(|e| e.to_string())?;
        Some(proj.id)
    } else {
        None
    };

    let mut config = nexus_core::Config::load().unwrap_or_default();
    if let Some(w) = hybrid_weight {
        config.search.hybrid_weight = w;
    }
    if let Some(s) = min_vector_score {
        config.search.min_vector_score = s;
    }

    let limit = limit.unwrap_or(20);
    let mode = mode.as_deref().unwrap_or("hybrid");
    let use_popularity = project_id.is_some();

    let mut results = match mode {
        "keyword" => nexus_core::search::fts_search(
            &state.pool, &query, resolved_pid.as_deref(), limit,
        ).map_err(|e| e.to_string())?,
        "vector" => nexus_core::search::vector_search(
            &state.pool, &query, resolved_pid.as_deref(), limit, &config,
        ).map_err(|e| e.to_string())?,
        _ => nexus_core::search::hybrid_search(
            &state.pool, &query, resolved_pid.as_deref(), limit, &config,
        ).map_err(|e| e.to_string())?,
    };

    // Enrich with metadata + apply tag filter
    nexus_core::search::enrich_results(&state.pool, &mut results, use_popularity)
        .map_err(|e| e.to_string())?;

    if let Some(ref tag_list) = tags {
        let tag_refs: Vec<&str> = tag_list.iter().map(|s| s.as_str()).collect();
        nexus_core::search::filter_by_tags(&mut results, &tag_refs);
    }

    Ok(results)
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

/// Current platform target triple for sidecar binary suffix
fn target_triple() -> &'static str {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    { "aarch64-apple-darwin" }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    { "x86_64-apple-darwin" }
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    { "x86_64-unknown-linux-gnu" }
    #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
    { "x86_64-pc-windows-msvc" }
    #[cfg(all(target_arch = "aarch64", target_os = "windows"))]
    { "aarch64-pc-windows-msvc" }
}

/// Find a sidecar binary bundled with the app.
/// Tauri places sidecars next to the exe with a target triple suffix.
fn find_sidecar(name: &str) -> Option<String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // 1. Bundled sidecar (with target triple suffix)
            let sidecar_name = format!("{}-{}", name, target_triple());
            let candidate = dir.join(&sidecar_name);
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
            // 2. Same directory without suffix (dev build)
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate.to_string_lossy().to_string());
            }
        }
    }
    // 3. Cargo release build path (dev mode fallback)
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(format!("gorillaProject/obsidian-nexus/target/release/{}", name));
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// Find the MCP server binary path
fn find_mcp_binary() -> Option<String> {
    find_sidecar("nexus-mcp-server")
}

/// Find the CLI binary path
fn find_cli_binary() -> Option<String> {
    find_sidecar("nexus")
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

/// AI tool config targets for MCP server registration.
/// Only registers when the tool's config directory already exists (i.e. the tool is installed).
/// Easily extensible: add a new entry to support additional AI tools.
struct McpTarget {
    name: &'static str,
    /// Path relative to home directory
    config_rel: &'static str,
}

const MCP_TARGETS: &[McpTarget] = &[
    McpTarget { name: "Claude Desktop", config_rel: ".claude/claude_desktop_config.json" },
    McpTarget { name: "Claude Code",    config_rel: ".claude/settings.json" },
    McpTarget { name: "Gemini CLI",     config_rel: ".gemini/settings.json" },
    // To add more tools, append here:
    // McpTarget { name: "Cursor",      config_rel: ".cursor/mcp.json" },
    // McpTarget { name: "Windsurf",    config_rel: ".codeium/windsurf/mcp_config.json" },
];

/// Register MCP server in all detected AI tool configs
fn register_mcp_server() {
    let mcp_path = match find_mcp_binary() {
        Some(p) => p,
        None => return,
    };

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };

    for target in MCP_TARGETS {
        let config_path = home.join(target.config_rel);
        // Only register if the tool's config directory exists (tool is installed)
        if let Some(parent) = config_path.parent() {
            if parent.exists() {
                if register_in_config(&config_path, &mcp_path) {
                    eprintln!("MCP registered in {} ({})", target.name, config_path.display());
                }
            }
        }
    }
}

/// MCP status for each AI tool: name, installed (config dir exists), registered (nexus entry exists)
#[derive(serde::Serialize)]
struct McpStatus {
    name: String,
    installed: bool,
    registered: bool,
}

#[tauri::command]
fn mcp_status() -> Vec<McpStatus> {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return vec![],
    };
    MCP_TARGETS.iter().map(|t| {
        let config_path = home.join(t.config_rel);
        let installed = config_path.parent().map(|p| p.exists()).unwrap_or(false);
        let registered = if installed && config_path.exists() {
            std::fs::read_to_string(&config_path).ok()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                .and_then(|v| v.get("mcpServers")?.get("nexus").cloned())
                .is_some()
        } else {
            false
        };
        McpStatus { name: t.name.to_string(), installed, registered }
    }).collect()
}

/// Get bundled CLI binary path (for terminal integration)
#[tauri::command]
fn cli_path() -> Result<String, String> {
    find_cli_binary().ok_or_else(|| "CLI binary not found".to_string())
}

/// Get bundled MCP server binary path
#[tauri::command]
fn mcp_path() -> Result<String, String> {
    find_mcp_binary().ok_or_else(|| "MCP server binary not found".to_string())
}

#[tauri::command]
fn mcp_register(name: String) -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Cannot find home directory")?;
    let mcp_path = find_mcp_binary().ok_or("MCP server binary not found")?;
    let target = MCP_TARGETS.iter().find(|t| t.name == name)
        .ok_or(format!("Unknown target: {}", name))?;
    let config_path = home.join(target.config_rel);
    if register_in_config(&config_path, &mcp_path) {
        Ok(format!("Registered in {}", name))
    } else {
        Err(format!("Failed to register in {}", name))
    }
}

/// Simple URL encoding for Obsidian URI scheme
fn urlencoding(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'/' => {
            String::from(b as char)
        }
        _ => format!("%{:02X}", b),
    }).collect()
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
        // Use Obsidian URI scheme to open the specific file in the vault
        // obsidian://open?vault=VAULT_NAME&file=FILE_PATH (without .md extension)
        // vault_name must match Obsidian's registered vault name (= folder name, not on-config.json name)
        let vault_folder_name = std::path::Path::new(&proj.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| proj.name.clone());
        let file_without_ext = if file_path.ends_with(".md") {
            &file_path[..file_path.len() - 3]
        } else {
            &file_path
        };
        let uri = format!(
            "obsidian://open?vault={}&file={}",
            urlencoding(&vault_folder_name),
            urlencoding(&file_without_ext),
        );
        let status = std::process::Command::new("open")
            .arg(&uri)
            .spawn();
        match status {
            Ok(_) => Ok(serde_json::json!({ "opened_with": "obsidian_uri", "uri": uri, "path": abs_path.to_string_lossy() })),
            Err(_) => {
                // Fallback to open -a Obsidian
                let _ = std::process::Command::new("open")
                    .args(["-a", "Obsidian", abs_path.to_str().unwrap_or("")])
                    .spawn();
                Ok(serde_json::json!({ "opened_with": "obsidian_fallback", "path": abs_path.to_string_lossy() }))
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

/// Sync project name from on-config.json
#[tauri::command]
fn sync_vault_config(state: State<AppState>, project_id: String) -> Result<serde_json::Value, String> {
    let updated = nexus_core::project::sync_vault_config(&state.pool, &project_id)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!(updated))
}

/// Detect Obsidian vaults under a directory
#[tauri::command]
fn detect_vaults(path: String) -> Result<Vec<(String, String)>, String> {
    nexus_core::project::detect_vaults(&path).map_err(|e| e.to_string())
}

/// Auto-detect and register all vaults under a directory
#[tauri::command]
fn auto_add_vaults(state: State<AppState>, path: String) -> Result<serde_json::Value, String> {
    let projects = nexus_core::project::auto_add_vaults(&state.pool, &path)
        .map_err(|e| e.to_string())?;

    // Auto-index each newly added vault
    for proj in &projects {
        let _ = nexus_core::index_engine::index_project(&state.pool, &proj.id, false);
    }

    Ok(serde_json::json!({
        "added": projects.len(),
        "projects": projects,
    }))
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

/// Install CLI symlinks to ~/.local/bin so `nexus` and `nexus-mcp-server` are available in PATH.
/// Also ensures ~/.local/bin is in the user's shell PATH.
fn install_cli_symlinks() {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };
    let bin_dir = home.join(".local/bin");

    // Create ~/.local/bin if it doesn't exist
    if !bin_dir.exists() {
        if std::fs::create_dir_all(&bin_dir).is_err() {
            return;
        }
    }

    for name in &["nexus", "nexus-mcp-server"] {
        if let Some(sidecar_path) = find_sidecar(name) {
            let link_path = bin_dir.join(name);
            // Skip if already pointing to the correct target
            if link_path.is_symlink() {
                if let Ok(target) = std::fs::read_link(&link_path) {
                    if target.to_string_lossy() == sidecar_path {
                        continue;
                    }
                }
                let _ = std::fs::remove_file(&link_path);
            }
            if !link_path.exists() {
                match std::os::unix::fs::symlink(&sidecar_path, &link_path) {
                    Ok(_) => eprintln!("CLI symlink created: {} -> {}", link_path.display(), sidecar_path),
                    Err(e) => eprintln!("Failed to create symlink {}: {}", link_path.display(), e),
                }
            }
        }
    }

    // Ensure ~/.local/bin is in PATH via ~/.zshrc (macOS default shell)
    let zshrc = home.join(".zshrc");
    let path_line = "export PATH=\"$HOME/.local/bin:$PATH\"";
    let already_set = std::fs::read_to_string(&zshrc)
        .map(|c| c.contains(".local/bin"))
        .unwrap_or(false);
    if !already_set {
        let entry = format!("\n# Obsidian Nexus CLI\n{}\n", path_line);
        if std::fs::OpenOptions::new().create(true).append(true).open(&zshrc)
            .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()))
            .is_ok()
        {
            eprintln!("Added ~/.local/bin to PATH in ~/.zshrc");
        }
    }
}

fn main() {
    // Auto-setup on first launch
    ensure_obsidian();
    register_mcp_server();
    install_cli_symlinks();

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
            detect_vaults,
            auto_add_vaults,
            sync_vault_config,
            mcp_status,
            mcp_register,
            mcp_path,
            cli_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
