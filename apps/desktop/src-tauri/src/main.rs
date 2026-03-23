// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use nexus_core::db::sqlite::{self, DbPool};
use nexus_core::project::Project;
use nexus_core::search::SearchResult;
use nexus_agent::cli_detector::{self, DetectedAgent};
use nexus_agent::cli_bridge::{SidecarManager, BridgeResponse};
use nexus_agent::session::{SessionManager, SessionMeta};
use nexus_agent::prompt::{PromptLoader, PromptContext};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::oneshot;

/// Per-session completion channels: notified when result/cancelled/error arrives.
type PendingMessages = Arc<Mutex<HashMap<String, oneshot::Sender<bool>>>>;

struct AppState {
    pool: DbPool,
    session_manager: Arc<SessionManager>,
    prompt_loader: PromptLoader,
    sidecar: Arc<SidecarManager>,
    pending_messages: PendingMessages,
    reader_started: Arc<AtomicBool>,
}

/// Background reader thread: reads all sidecar responses, routes to frontend,
/// and signals waiting chat_send_message tasks via oneshot channels.
fn spawn_background_reader(
    sidecar: Arc<SidecarManager>,
    app: AppHandle,
    pending: PendingMessages,
    session_manager: Arc<SessionManager>,
) {
    let Some(mut reader) = sidecar.take_reader() else {
        eprintln!("[reader] Failed to take reader — background reader not started");
        return;
    };

    std::thread::spawn(move || {
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    eprintln!("[reader] Sidecar EOF — process exited");
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue; // skip blank lines, not EOF
                    }
                    let resp: BridgeResponse = match serde_json::from_str(trimmed) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("[reader] JSON parse error: {} (line: {})", e, trimmed);
                            continue;
                        }
                    };

                    // Skip init acks — they would reset frontend status mid-stream
                    if resp.msg_type == "init" {
                        continue;
                    }

                    let session_id = resp.session_id.clone();
                    let is_terminal = matches!(
                        resp.msg_type.as_str(),
                        "result" | "error" | "cancelled"
                    );

                    // Emit to the correct session's frontend channel
                    let _ = app.emit(&format!("chat-stream:{}", session_id), &resp);

                    if is_terminal {
                        let success = resp.msg_type == "result";
                        if success {
                            let _ = session_manager.increment_message_count(&session_id);
                        }
                        if let Ok(mut pending) = pending.lock() {
                            if let Some(tx) = pending.remove(&session_id) {
                                let _ = tx.send(success);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[reader] Read error: {}", e);
                    break;
                }
            }
        }

        // Sidecar exited or errored — unblock any waiting chat_send_message calls
        if let Ok(mut pending) = pending.lock() {
            for (sid, tx) in pending.drain() {
                let _ = app.emit(&format!("chat-stream:{}", sid), serde_json::json!({
                    "type": "error",
                    "sessionId": sid,
                    "code": "sidecar_exit",
                    "message": "사이드카 프로세스가 종료되었습니다",
                    "retryable": true
                }));
                let _ = tx.send(false);
            }
        }
    });
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
    tag_match_all: Option<bool>,
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

    let tag_filter = tags.as_ref().and_then(|tag_list| {
        if tag_list.is_empty() { None }
        else {
            Some(nexus_core::search::TagFilter::new(
                tag_list.clone(),
                tag_match_all.unwrap_or(false),
            ))
        }
    });

    let mut results = match mode {
        "keyword" => nexus_core::search::fts_search(
            &state.pool, &query, resolved_pid.as_deref(), limit, tag_filter.as_ref(),
        ).map_err(|e| e.to_string())?,
        "vector" => nexus_core::search::vector_search(
            &state.pool, &query, resolved_pid.as_deref(), limit, &config, tag_filter.as_ref(),
        ).map_err(|e| e.to_string())?,
        _ => nexus_core::search::hybrid_search(
            &state.pool, &query, resolved_pid.as_deref(), limit, &config, tag_filter.as_ref(),
        ).map_err(|e| e.to_string())?,
    };

    // Enrich with metadata
    nexus_core::search::enrich_results(&state.pool, &mut results, use_popularity)
        .map_err(|e| e.to_string())?;

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
    // 3. ~/.local/bin fallback (symlinked during install)
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(format!(".local/bin/{}", name));
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
    find_sidecar("obs-nexus")
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
    if let Some(existing) = config.get("mcpServers").and_then(|s| s.get("nexus")) {
        let existing_path = existing.get("command")
            .and_then(|c| c.as_str())
            .map(std::path::Path::new);
        let is_valid = existing_path.map(|p| {
            p.exists() && !p.to_string_lossy().contains(' ')
        }).unwrap_or(false);
        if is_valid {
            return true; // 경로 유효 → 그대로 유지
        }
        // 경로 무효 또는 공백 포함 → 아래에서 덮어쓰기
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

    // Path traversal guard: ensure resolved path stays within the project directory
    let abs_path = abs_path.canonicalize().map_err(|e| format!("Invalid path: {}", e))?;
    let vault_path = std::path::Path::new(&proj.path)
        .canonicalize()
        .map_err(|e| format!("Invalid project path: {}", e))?;
    if !abs_path.starts_with(&vault_path) {
        return Err("Access denied: path is outside the project directory".to_string());
    }

    // Check if Obsidian is installed (macOS)
    let obsidian_installed = std::path::Path::new("/Applications/Obsidian.app").exists()
        || dirs::home_dir()
            .map(|h| h.join("Applications/Obsidian.app").exists())
            .unwrap_or(false);

    if obsidian_installed {
        // Use Obsidian URI with absolute path to avoid vault name conflicts
        // (e.g., multiple vaults named "docs")
        let uri = format!(
            "obsidian://open?path={}",
            urlencoding(&abs_path.to_string_lossy()),
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

    for name in &["obs-nexus", "nexus-mcp-server"] {
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

// === System Status ===

#[derive(serde::Serialize)]
struct SystemStatus {
    mcp_binary: ComponentStatus,
    obs_nexus_binary: ComponentStatus,
    mcp_registrations: Vec<McpStatus>,
    cli_agents: Vec<CliAgentStatus>,
    ollama: ComponentStatus,
    obsidian: ComponentStatus,
}

#[derive(serde::Serialize)]
struct ComponentStatus {
    installed: bool,
    detail: Option<String>, // path, version, or error detail
}

#[derive(serde::Serialize)]
struct CliAgentStatus {
    cli: String,
    installed: bool,
    path: Option<String>,
    version: Option<String>,
    authenticated: bool,
    failure_reason: Option<String>,
}

fn check_ollama() -> ComponentStatus {
    // Check if ollama binary exists
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let installed = std::process::Command::new(&shell)
        .args(["-l", "-c", "which ollama"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !installed {
        return ComponentStatus { installed: false, detail: None };
    }

    // Check if ollama server is running
    let running = std::process::Command::new("curl")
        .args(["-sf", "--max-time", "1", "http://localhost:11434/api/version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    ComponentStatus {
        installed: true,
        detail: Some(if running { "실행 중".to_string() } else { "설치됨 (미실행)".to_string() }),
    }
}

fn check_obsidian() -> ComponentStatus {
    let installed = std::path::Path::new("/Applications/Obsidian.app").exists()
        || dirs::home_dir()
            .map(|h| h.join("Applications/Obsidian.app").exists())
            .unwrap_or(false);
    ComponentStatus { installed, detail: None }
}

#[tauri::command]
async fn system_status() -> SystemStatus {
    let mcp_binary = {
        let path = find_mcp_binary();
        ComponentStatus { installed: path.is_some(), detail: path }
    };

    let obs_nexus_binary = {
        let path = find_cli_binary();
        ComponentStatus { installed: path.is_some(), detail: path }
    };

    let mcp_registrations = {
        let home = dirs::home_dir().unwrap_or_default();
        MCP_TARGETS.iter().map(|t| {
            let config_path = home.join(t.config_rel);
            let installed = config_path.parent().map(|p| p.exists()).unwrap_or(false);
            let registered = if installed && config_path.exists() {
                std::fs::read_to_string(&config_path).ok()
                    .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                    .and_then(|v| v.get("mcpServers")?.get("nexus").cloned())
                    .is_some()
            } else { false };
            McpStatus { name: t.name.to_string(), installed, registered }
        }).collect()
    };

    let cli_agents = tokio::task::spawn_blocking(|| {
        let detected = cli_detector::detect_agents();

        detected.into_iter().map(|agent| {
            let installed = !agent.path.as_os_str().is_empty();
            CliAgentStatus {
                cli: agent.cli.to_string(),
                installed,
                path: if installed { Some(agent.path.to_string_lossy().to_string()) } else { None },
                version: if !agent.version.is_empty() { Some(agent.version) } else { None },
                authenticated: agent.authenticated,
                failure_reason: agent.failure_reason,
            }
        }).collect()
    }).await.unwrap_or_default();

    let ollama = tokio::task::spawn_blocking(check_ollama).await.unwrap_or(ComponentStatus { installed: false, detail: None });
    let obsidian = check_obsidian();

    SystemStatus { mcp_binary, obs_nexus_binary, mcp_registrations, cli_agents, ollama, obsidian }
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// === CLI Diagnostics ===

#[derive(serde::Serialize)]
struct CliDiagnostics {
    cli: String,
    which_result: String,
    direct_exec_stdout: String,
    direct_exec_stderr: String,
    direct_exec_exit: String,
    shell_exec_stdout: String,
    shell_exec_stderr: String,
    shell_exec_exit: String,
    shell_used: String,
    nvm_path: String,
    nvm_exec_stdout: String,
    nvm_exec_exit: String,
    find_cli_path_result: String,
}

#[tauri::command]
async fn diagnose_cli(cli: String) -> CliDiagnostics {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let cli_name = cli.clone();
    let cli_for_err = cli.clone();

    tokio::task::spawn_blocking(move || {
        // Step 1: which
        let which_result = std::process::Command::new(&shell)
            .args(["-l", "-c", &format!("which {}", cli)])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|e| format!("which failed: {}", e));

        let binary_path = which_result.clone();

        // Step 2: direct execution
        let (direct_stdout, direct_stderr, direct_exit) = if !binary_path.is_empty() && !binary_path.starts_with("which failed") {
            std::process::Command::new(&binary_path)
                .arg("--version")
                .output()
                .map(|o| (
                    String::from_utf8_lossy(&o.stdout).trim().to_string(),
                    String::from_utf8_lossy(&o.stderr).trim().to_string(),
                    o.status.to_string(),
                ))
                .unwrap_or_else(|e| (String::new(), e.to_string(), "spawn failed".to_string()))
        } else {
            (String::new(), "binary not found".to_string(), "N/A".to_string())
        };

        // Step 3: shell execution with name
        let (shell_stdout, shell_stderr, shell_exit) = std::process::Command::new(&shell)
            .args(["-l", "-c", &format!("{} --version", cli_name)])
            .output()
            .map(|o| (
                String::from_utf8_lossy(&o.stdout).trim().to_string(),
                String::from_utf8_lossy(&o.stderr).trim().to_string(),
                o.status.to_string(),
            ))
            .unwrap_or_else(|e| (String::new(), e.to_string(), "spawn failed".to_string()));

        // Step 4: nvm direct path check
        let home = dirs::home_dir().unwrap_or_default();
        let nvm_base = home.join(".nvm/versions/node");
        let nvm_path = if nvm_base.exists() {
            let mut entries: Vec<_> = std::fs::read_dir(&nvm_base)
                .into_iter().flatten().flatten()
                .map(|e| e.path().join("bin").join(&cli_name))
                .filter(|p| p.exists())
                .collect();
            entries.sort();
            entries.into_iter().last()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "nvm bin not found".to_string())
        } else {
            "~/.nvm not found".to_string()
        };

        let (nvm_stdout, nvm_exit) = if !nvm_path.starts_with("nvm") && !nvm_path.starts_with("~") {
            // Run with enriched PATH so node scripts (#!/usr/bin/env node) can find node
            let nvm_bin_dir = std::path::Path::new(&nvm_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let current_path = std::env::var("PATH").unwrap_or_default();
            let enriched_path = format!("{}:{}", nvm_bin_dir, current_path);
            std::process::Command::new(&nvm_path)
                .arg("--version")
                .env("PATH", enriched_path)
                .output()
                .map(|o| (
                    String::from_utf8_lossy(&o.stdout).trim().to_string(),
                    o.status.to_string(),
                ))
                .unwrap_or_else(|e| (e.to_string(), "spawn failed".to_string()))
        } else {
            (String::new(), "N/A".to_string())
        };

        // Step 5: what find_cli_path actually returns
        let find_result = nexus_agent::cli_detector::find_cli_path_pub(&cli_name)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "None".to_string());

        CliDiagnostics {
            cli: cli_name,
            which_result,
            direct_exec_stdout: direct_stdout,
            direct_exec_stderr: direct_stderr,
            direct_exec_exit: direct_exit,
            shell_exec_stdout: shell_stdout,
            shell_exec_stderr: shell_stderr,
            shell_exec_exit: shell_exit,
            shell_used: shell,
            nvm_path,
            nvm_exec_stdout: nvm_stdout,
            nvm_exec_exit: nvm_exit,
            find_cli_path_result: find_result,
        }
    }).await.unwrap_or_else(|e| CliDiagnostics {
        cli: cli_for_err,
        which_result: format!("task error: {}", e),
        direct_exec_stdout: String::new(),
        direct_exec_stderr: String::new(),
        direct_exec_exit: String::new(),
        shell_exec_stdout: String::new(),
        shell_exec_stderr: String::new(),
        shell_exec_exit: String::new(),
        shell_used: String::new(),
        nvm_path: String::new(),
        nvm_exec_stdout: String::new(),
        nvm_exec_exit: String::new(),
        find_cli_path_result: String::new(),
    })
}

// === Connectivity Tests ===

#[derive(serde::Serialize)]
struct TestResult {
    ok: bool,
    message: String,
}

#[tauri::command]
async fn test_mcp() -> TestResult {
    let binary = match find_mcp_binary() {
        Some(p) => p,
        None => return TestResult { ok: false, message: "MCP 바이너리를 찾을 수 없습니다".to_string() },
    };

    // Send JSON-RPC initialize request via stdin, read response from stdout
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"nexus-test","version":"0.0.1"}}}"#;

    let result = tokio::task::spawn_blocking(move || {
        use std::io::{Write, BufRead, BufReader};
        let mut child = std::process::Command::new(&binary)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        // Send request then drop stdin to signal EOF to the server
        if let Some(mut stdin) = child.stdin.take() {
            writeln!(stdin, "{}", request)?;
        }

        // Read first non-empty line from stdout (MCP responds line-by-line)
        let stdout = child.stdout.take().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::Other, "no stdout")
        })?;

        let mut reader = BufReader::new(stdout);
        let mut response = String::new();
        let start = std::time::Instant::now();

        loop {
            if start.elapsed().as_secs() >= 5 {
                let _ = child.kill();
                return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
            }
            response.clear();
            match reader.read_line(&mut response) {
                Ok(0) => std::thread::sleep(std::time::Duration::from_millis(50)),
                Ok(_) if response.trim().is_empty() => continue,
                Ok(_) => {
                    let _ = child.kill();
                    return Ok(response);
                }
                Err(e) => return Err(e),
            }
        }
    }).await;

    match result {
        Ok(Ok(stdout)) if stdout.contains("\"result\"") => {
            TestResult { ok: true, message: "MCP 서버 응답 정상".to_string() }
        }
        Ok(Ok(stdout)) if stdout.contains("\"error\"") => {
            TestResult { ok: false, message: format!("MCP 서버 오류 응답: {}", &stdout[..stdout.len().min(120)]) }
        }
        Ok(Ok(_)) => TestResult { ok: false, message: "MCP 서버 응답 없음".to_string() },
        Ok(Err(e)) if e.kind() == std::io::ErrorKind::TimedOut => {
            TestResult { ok: false, message: "MCP 서버 응답 시간 초과 (3초)".to_string() }
        }
        Ok(Err(e)) => TestResult { ok: false, message: format!("실행 오류: {}", e) },
        Err(e) => TestResult { ok: false, message: format!("내부 오류: {}", e) },
    }
}

#[tauri::command]
async fn test_cli(cli: String) -> TestResult {
    let cli_name = cli.clone();
    // Use find_cli_path_pub which applies is_executable_script filtering,
    // ensuring we never try to run a binary with a broken shebang interpreter.
    // Fall back to bare name only if nothing else found (e.g., native binary in PATH).
    let binary_path = find_sidecar(&cli)
        .or_else(|| {
            cli_detector::find_cli_path_pub(&cli)
                .map(|p| p.to_string_lossy().to_string())
        })
        .unwrap_or(cli.clone());

    let result = tokio::task::spawn_blocking(move || {
        // Prepend the binary's parent dir to PATH so that nvm/volta node scripts
        // (#!/usr/bin/env node) can resolve their interpreter from the same bin/ dir.
        let enriched_path = std::path::Path::new(&binary_path)
            .parent()
            .map(|p| {
                let cur = std::env::var("PATH").unwrap_or_default();
                format!("{}:{}", p.display(), cur)
            })
            .unwrap_or_else(|| std::env::var("PATH").unwrap_or_default());
        std::process::Command::new(&binary_path)
            .arg("--version")
            .env("PATH", enriched_path)
            .output()
    }).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if output.status.success() && !stdout.is_empty() {
                TestResult { ok: true, message: stdout }
            } else if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let detail = if !stderr.is_empty() { stderr } else { stdout };
                TestResult { ok: false, message: format!("{} 실행 실패: {}", cli_name, detail) }
            } else {
                TestResult { ok: false, message: format!("{} 응답 없음", cli_name) }
            }
        }
        Ok(Err(e)) => TestResult { ok: false, message: format!("실행 오류: {}", e) },
        Err(e) => TestResult { ok: false, message: format!("내부 오류: {}", e) },
    }
}

// === Agent Commands ===

#[tauri::command]
async fn detect_cli_agents() -> Result<Vec<DetectedAgent>, String> {
    // Run blocking CLI detection on a separate thread to avoid UI freeze
    tokio::task::spawn_blocking(|| cli_detector::detect_agents())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_new_session(
    state: State<AppState>,
    cli: String,
    model: String,
    project_id: String,
    name: Option<String>,
) -> Result<SessionMeta, String> {
    let cli_type = match cli.as_str() {
        "claude" => nexus_agent::cli_detector::CliType::Claude,
        "gemini" => nexus_agent::cli_detector::CliType::Gemini,
        _ => return Err(format!("Unknown CLI type: {}", cli)),
    };

    state
        .session_manager
        .create_session(cli_type, &model, &project_id, name.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_list_sessions(state: State<AppState>) -> Result<Vec<SessionMeta>, String> {
    state.session_manager.list_sessions().map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_delete_session(state: State<AppState>, session_id: String) -> Result<(), String> {
    state
        .session_manager
        .delete_session(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_build_prompt(
    state: State<AppState>,
    project_name: String,
    project_path: String,
    doc_count: u64,
    top_tags: Vec<String>,
) -> Result<String, String> {
    let context = PromptContext {
        project_name,
        project_path,
        doc_count,
        top_tags,
    };

    state
        .prompt_loader
        .build_system_prompt("librarian", &context)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn chat_start_session(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    model: String,
    project_name: String,
    project_path: String,
    doc_count: u64,
    top_tags: Vec<String>,
) -> Result<(), String> {
    // Ensure sidecar is running
    state.sidecar.ensure_running().map_err(|e| e.to_string())?;

    // Spawn background reader once (atomic flag ensures single spawn)
    if !state.reader_started.swap(true, Ordering::SeqCst) {
        spawn_background_reader(
            state.sidecar.clone(),
            app.clone(),
            state.pending_messages.clone(),
            state.session_manager.clone(),
        );
    }

    // Build system prompt
    let context = PromptContext {
        project_name,
        project_path,
        doc_count,
        top_tags,
    };
    let system_prompt = state
        .prompt_loader
        .build_system_prompt("librarian", &context)
        .map_err(|e| e.to_string())?;

    let mcp_server_path = which_nexus_mcp();

    // Resolve cliType and cliPath from session metadata
    let session = state
        .session_manager
        .get_session(&session_id)
        .map_err(|e| e.to_string())?;
    let cli_type = session.cli.to_string();
    let cli_path = cli_detector::find_cli_path_pub(&cli_type)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| cli_type.clone());

    let start_req = serde_json::json!({
        "type": "start",
        "sessionId": session_id,
        "cliType": cli_type,
        "cliPath": cli_path,
        "model": model,
        "systemPrompt": system_prompt,
        "mcpServers": {
            "nexus": {
                "command": mcp_server_path.to_string_lossy()
            }
        }
    });

    state.sidecar.send_request(&start_req).map_err(|e| e.to_string())?;
    // init ack is consumed by background reader and skipped (no frontend noise)
    Ok(())
}

#[tauri::command]
async fn chat_send_message(
    state: State<'_, AppState>,
    session_id: String,
    message: String,
) -> Result<(), String> {
    state.sidecar.ensure_running().map_err(|e| e.to_string())?;

    // Register a oneshot channel — background reader will fire it on result/error/cancelled
    let (tx, rx) = oneshot::channel::<bool>();
    {
        let mut pending = state.pending_messages.lock().unwrap();
        pending.insert(session_id.clone(), tx);
    }

    let msg_req = serde_json::json!({
        "type": "message",
        "sessionId": session_id,
        "content": message
    });

    if let Err(e) = state.sidecar.send_request(&msg_req) {
        // Clean up the dangling sender before returning
        state.pending_messages.lock().unwrap().remove(&session_id);
        return Err(e.to_string());
    }

    // Await terminal event — background reader fires this
    let _ = rx.await;
    Ok(())
}

#[tauri::command]
fn chat_rename_session(state: State<AppState>, session_id: String, name: String) -> Result<(), String> {
    state.session_manager.update_session_name(&session_id, &name).map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_cancel(state: State<AppState>, session_id: String) -> Result<(), String> {
    let req = serde_json::json!({ "type": "cancel", "sessionId": session_id });
    state.sidecar.send_request(&req).map_err(|e| e.to_string())
}

#[tauri::command]
fn chat_close_session(state: State<AppState>, session_id: String) -> Result<(), String> {
    // Try to send close to sidecar (best-effort, ignore if not running)
    if state.sidecar.is_running() {
        let req = serde_json::json!({ "type": "close", "sessionId": session_id });
        let _ = state.sidecar.send_request(&req);
    }
    // Always delete session metadata
    state.session_manager.delete_session(&session_id).map_err(|e| e.to_string())
}

fn find_sidecar_script() -> std::path::PathBuf {
    // Try multiple candidate paths
    let mut candidates = vec![
        // Dev: Tauri runs from src-tauri, cwd is project root
        std::env::current_dir()
            .unwrap_or_default()
            .join("apps/desktop/sidecar/agent-bridge.mjs"),
        // Dev: relative from src-tauri
        std::path::PathBuf::from("../sidecar/agent-bridge.mjs"),
        // Dev: relative from workspace root
        std::path::PathBuf::from("apps/desktop/sidecar/agent-bridge.mjs"),
        // Absolute fallback (project specific)
        std::path::PathBuf::from(
            std::env::var("NEXUS_SIDECAR_PATH")
                .unwrap_or_default(),
        ),
    ];

    // Bundled resource (production): in Resources/sidecar/ (macOS app bundle)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(macos_dir) = exe.parent() {
            // MacOS/../Resources/sidecar/agent-bridge.mjs
            let resources = macos_dir.parent()
                .map(|p| p.join("Resources/sidecar/agent-bridge.mjs"));
            if let Some(path) = resources {
                candidates.insert(0, path);
            }
        }
    }

    for path in &candidates {
        if path.exists() {
            return path
                .canonicalize()
                .unwrap_or_else(|_| path.clone());
        }
    }

    // Last resort — will fail at runtime with clear error
    eprintln!("WARNING: sidecar script not found in any candidate path");
    candidates[0].clone()
}

fn which_nexus_mcp() -> std::path::PathBuf {
    // Try common paths
    let home = dirs::home_dir().unwrap_or_default();
    let local_bin = home.join(".local").join("bin").join("nexus-mcp-server");
    if local_bin.exists() {
        return local_bin;
    }

    // Fallback: try which
    if let Ok(output) = std::process::Command::new("which")
        .arg("nexus-mcp-server")
        .output()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return std::path::PathBuf::from(path);
        }
    }

    // Last resort
    local_bin
}

fn main() {
    // Auto-setup on first launch
    ensure_obsidian();
    install_cli_symlinks();
    register_mcp_server();

    let pool = sqlite::create_pool().expect("Failed to create database pool");
    sqlite::run_migrations(&pool).expect("Failed to run migrations");

    // Initialize agent subsystem
    let session_manager = SessionManager::new().expect("Failed to initialize session manager");
    let prompt_loader = PromptLoader::new().expect("Failed to initialize prompt loader");
    prompt_loader
        .ensure_defaults()
        .expect("Failed to initialize default prompts");

    // Sidecar script path resolution
    let sidecar_script = find_sidecar_script();
    eprintln!("Sidecar script: {}", sidecar_script.display());
    let sidecar = Arc::new(SidecarManager::new(sidecar_script));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            pool,
            session_manager: Arc::new(session_manager),
            prompt_loader,
            sidecar,
            pending_messages: Arc::new(Mutex::new(HashMap::new())),
            reader_started: Arc::new(AtomicBool::new(false)),
        })
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
            detect_cli_agents,
            system_status,
            open_url,
            test_mcp,
            test_cli,
            diagnose_cli,
            chat_new_session,
            chat_list_sessions,
            chat_delete_session,
            chat_build_prompt,
            chat_start_session,
            chat_send_message,
            chat_rename_session,
            chat_cancel,
            chat_close_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
