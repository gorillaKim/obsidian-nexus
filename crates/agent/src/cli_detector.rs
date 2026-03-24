use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Check if a file is an executable binary or a script with a valid shebang interpreter.
/// This prevents "bad interpreter" errors and zombie processes by validating the shebang
/// without spawning the process.
fn is_executable_script(path: &Path) -> bool {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    
    // Read enough to check the shebang line
    let mut buffer = [0u8; 128];
    let n = match file.read(&mut buffer) {
        Ok(n) => n,
        Err(_) => return false,
    };

    if n < 2 || &buffer[..2] != b"#!" {
        // Not a script with shebang, assume it's a native binary or kernel-handled
        return true;
    }

    let content = String::from_utf8_lossy(&buffer[..n]);
    let first_line = match content.lines().next() {
        Some(l) => l,
        None => return true,
    };
    
    let shebang = first_line.trim_start_matches("#!").trim();
    if shebang.is_empty() {
        return false;
    }

    // Split into interpreter and arguments (e.g., "#!/usr/bin/env node --flags")
    let parts: Vec<&str> = shebang.split_whitespace().collect();
    let interpreter = parts[0];

    if interpreter.starts_with('/') {
        let interp_path = Path::new(interpreter);
        if interp_path.exists() {
            return true;
        }

        // Special handling for /usr/bin/env to check for command availability
        if interpreter == "/usr/bin/env" && parts.len() > 1 {
            // Find the first part that's not a flag (e.g., node)
            if let Some(cmd) = parts.iter().skip(1).find(|s| !s.starts_with('-')) {
                // 1. Check same directory as script (common for NVM/Volta)
                if let Some(parent) = path.parent() {
                    if parent.join(cmd).exists() {
                        return true;
                    }
                }
                // 2. Check current PATH
                if let Ok(path_env) = std::env::var("PATH") {
                    for p in std::env::split_paths(&path_env) {
                        if p.join(cmd).exists() {
                            return true;
                        }
                    }
                }
            }
        }
        
        // Absolute interpreter path doesn't exist
        return false;
    }

    // Relative path in shebang is rare and non-standard, but we allow it as a fallback
    true
}

/// Run a Command with a wall-clock timeout.
/// Kills the child process if it exceeds the timeout — prevents orphan shell processes.
fn command_output_timeout(mut cmd: Command, timeout: Duration) -> Option<std::process::Output> {
    cmd.stdout(std::process::Stdio::piped())
       .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().ok()?;
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait(); // reap zombie
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedAgent {
    pub cli: CliType,
    pub path: PathBuf,
    pub version: String,
    pub authenticated: bool,
    pub models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CliType {
    Claude,
    Gemini,
}

impl std::fmt::Display for CliType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliType::Claude => write!(f, "claude"),
            CliType::Gemini => write!(f, "gemini"),
        }
    }
}

/// Minimum supported CLI versions
const MIN_CLAUDE_VERSION: &str = "2.0.0";
const MIN_GEMINI_VERSION: &str = "1.0.0";

/// Detect all available CLI agents on the system.
pub fn detect_agents() -> Vec<DetectedAgent> {
    let mut agents = Vec::new();

    match detect_claude() {
        Some(agent) => agents.push(agent),
        None => agents.push(DetectedAgent {
            cli: CliType::Claude,
            path: PathBuf::new(),
            version: String::new(),
            authenticated: false,
            models: vec![],
            failure_reason: Some(diagnose_cli_failure("claude")),
        }),
    }

    match detect_gemini() {
        Some(agent) => agents.push(agent),
        None => agents.push(DetectedAgent {
            cli: CliType::Gemini,
            path: PathBuf::new(),
            version: String::new(),
            authenticated: false,
            models: vec![],
            failure_reason: Some(diagnose_cli_failure("gemini")),
        }),
    }

    agents
}

fn diagnose_cli_failure(name: &str) -> String {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = Command::new(&shell);
    cmd.args(["-l", "-c", &format!("which {}", name)]);
    let which_output = command_output_timeout(cmd, Duration::from_secs(3));

    match which_output {
        Some(o) if o.status.success() => {
            let path = String::from_utf8_lossy(&o.stdout).trim().to_string();
            format!("감지됨 ({}) — 버전 확인 실패", path)
        }
        _ => format!("PATH에서 {} 를 찾을 수 없음. npm install -g 로 설치하세요.", name),
    }
}

fn detect_claude() -> Option<DetectedAgent> {
    let path = find_cli_path("claude")?;
    let version = get_cli_version(&path, &["--version"])
        .or_else(|| get_version_via_shell("claude", &["--version"]))?;

    if !check_min_version(&version, MIN_CLAUDE_VERSION) {
        warn!(
            "Claude CLI version {} is below minimum {}",
            version, MIN_CLAUDE_VERSION
        );
    }

    let authenticated = check_claude_auth();
    let models = get_claude_models();

    debug!(
        "Detected Claude CLI: path={}, version={}, auth={}",
        path.display(),
        version,
        authenticated
    );

    Some(DetectedAgent {
        cli: CliType::Claude,
        path,
        version,
        authenticated,
        models,
        failure_reason: None,
    })
}

fn detect_gemini() -> Option<DetectedAgent> {
    let path = find_cli_path("gemini")?;
    let version = get_cli_version(&path, &["--version"])
        .or_else(|| get_version_via_shell("gemini", &["--version"]))?;

    if !check_min_version(&version, MIN_GEMINI_VERSION) {
        warn!(
            "Gemini CLI version {} is below minimum {}",
            version, MIN_GEMINI_VERSION
        );
    }

    let authenticated = check_gemini_auth();
    let models = get_gemini_models();

    debug!(
        "Detected Gemini CLI: path={}, version={}, auth={}",
        path.display(),
        version,
        authenticated
    );

    Some(DetectedAgent {
        cli: CliType::Gemini,
        path,
        version,
        authenticated,
        models,
        failure_reason: None,
    })
}

pub fn find_cli_path_pub(name: &str) -> Option<PathBuf> {
    find_cli_path(name)
}

fn find_cli_path(name: &str) -> Option<PathBuf> {
    // Try `which` first (works in terminal context)
    if let Some(path) = try_which(name) {
        return Some(path);
    }

    // GUI apps (e.g. Tauri via Homebrew) don't inherit full shell PATH.
    // Fall back to well-known install locations.
    let home = dirs::home_dir().unwrap_or_default();
    let candidates: Vec<PathBuf> = vec![
        // npm global (default & nvm)
        home.join(".npm-global/bin").join(name),
        home.join(".npm/bin").join(name),
        // nvm common locations
        home.join(".nvm/versions/node").join("*/bin").join(name),
        // fnm / volta / n
        home.join(".local/share/fnm/node-versions/*/installation/bin").join(name),
        home.join(".volta/bin").join(name),
        home.join("n/bin").join(name),
        // Homebrew (Apple Silicon & Intel)
        PathBuf::from("/opt/homebrew/bin").join(name),
        PathBuf::from("/usr/local/bin").join(name),
        // ~/.local/bin (nexus installs symlinks here)
        home.join(".local/bin").join(name),
    ];

    for candidate in candidates {
        // Expand simple glob patterns (single '*' segment)
        if candidate.to_string_lossy().contains('*') {
            if let Some(found) = glob_first(&candidate) {
                return Some(found);
            }
        } else if candidate.exists() && is_executable_script(&candidate) {
            return Some(candidate);
        }
    }

    None
}

fn try_which(name: &str) -> Option<PathBuf> {
    // Run via login shell so ~/.zshrc / ~/.bashrc PATH is loaded (needed for GUI apps)
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let shell_cmd = format!("which -a {}", name);
    let mut cmd = Command::new(&shell);
    cmd.args(["-l", "-c", &shell_cmd]);
    let output = command_output_timeout(cmd, Duration::from_secs(5))?;
    if !output.status.success() {
        return None;
    }
    // Return the first existing path from `which -a` that has a valid interpreter.
    // We use is_executable_script to check shebangs without spawning, preventing
    // "bad interpreter" errors during detection (e.g. broken homebrew node shebang).
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| PathBuf::from(l.trim()))
        .find(|p| p.exists() && is_executable_script(p))
}

/// Resolve the first existing path that contains a `*` glob segment.
fn glob_first(pattern: &PathBuf) -> Option<PathBuf> {
    let pattern_str = pattern.to_string_lossy();
    let parts: Vec<&str> = pattern_str.split('/').collect();
    let star_idx = parts.iter().position(|p| *p == "*")?;

    let base: PathBuf = parts[..star_idx].iter().collect();
    let suffix: PathBuf = parts[star_idx + 1..].iter().collect();

    let entries = std::fs::read_dir(&base).ok()?;
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path().join(&suffix))
        .filter(|p| p.exists() && is_executable_script(p))
        .collect();
    found.sort();
    found.into_iter().last() // pick highest version
}

fn get_cli_version(path: &PathBuf, args: &[&str]) -> Option<String> {
    // Prepend the binary's parent directory to PATH before executing.
    // This is the root-cause fix for nvm/volta/fnm node scripts:
    // their shebang (#!/usr/bin/env node) needs `node` in PATH, and the
    // version manager places node in the same bin/ directory as the CLI.
    let output = {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let enriched_path = match path.parent() {
            Some(parent) => format!("{}:{}", parent.display(), current_path),
            None => current_path,
        };
        let mut cmd = Command::new(path);
        cmd.args(args).env("PATH", enriched_path);
        command_output_timeout(cmd, Duration::from_secs(5))
    };

    // If still fails (e.g. homebrew shim pointing to wrong node version),
    // fall back to login shell by NAME so nvm/volta initializes its own PATH.
    let output = match output {
        Some(o) if !String::from_utf8_lossy(&o.stdout).trim().is_empty() => o,
        _ => {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let shell_cmd = format!("{} {}", name, args.join(" "));
            let mut cmd = Command::new(&shell);
            cmd.args(["-l", "-c", &shell_cmd]);
            command_output_timeout(cmd, Duration::from_secs(5))?
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }

    // Extract version number (e.g., "2.1.78 (Claude Code)" -> "2.1.78")
    let version = stdout
        .split_whitespace()
        .find(|s| s.chars().next().map_or(false, |c| c.is_ascii_digit()))
        .unwrap_or(&stdout)
        .to_string();

    Some(version)
}

fn get_version_via_shell(name: &str, args: &[&str]) -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let shell_cmd = format!("{} {}", name, args.join(" "));
    let mut cmd = Command::new(&shell);
    cmd.args(["-l", "-c", &shell_cmd]);
    let output = command_output_timeout(cmd, Duration::from_secs(5))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return None;
    }
    let version = stdout
        .split_whitespace()
        .find(|s| s.chars().next().map_or(false, |c| c.is_ascii_digit()))
        .unwrap_or(&stdout)
        .to_string();
    Some(version)
}

fn check_min_version(current: &str, minimum: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|s| s.parse::<u32>().ok())
            .collect()
    };

    let current_parts = parse(current);
    let min_parts = parse(minimum);

    for i in 0..min_parts.len() {
        let c = current_parts.get(i).copied().unwrap_or(0);
        let m = min_parts[i];
        if c > m {
            return true;
        }
        if c < m {
            return false;
        }
    }
    true
}

fn check_claude_auth() -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };
    let creds_path = home.join(".claude").join(".credentials.json");

    // If credentials file exists, verify it has a token
    if creds_path.exists() {
        return std::fs::read_to_string(&creds_path)
            .ok()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
            .and_then(|v| v.get("claudeAiOauthTokenData").cloned())
            .is_some();
    }

    // Newer Claude Code stores auth in the system keychain — assume authenticated
    // if the CLI is installed (no explicit unauthenticated state detectable)
    true
}

fn check_gemini_auth() -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };
    let creds_path = home.join(".gemini").join("oauth_creds.json");

    if !creds_path.exists() {
        return false;
    }

    // Parse as JSON to check key existence without keeping token values in memory
    std::fs::read_to_string(&creds_path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|v| v.get("access_token").cloned())
        .is_some()
}

fn get_claude_models() -> Vec<String> {
    vec![
        "sonnet".to_string(),
        "opus".to_string(),
        "haiku".to_string(),
    ]
}

fn get_gemini_models() -> Vec<String> {
    vec![
        "gemini-2.5-pro".to_string(),
        "gemini-2.5-flash".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_min_version() {
        assert!(check_min_version("2.1.78", "2.0.0"));
        assert!(check_min_version("2.0.0", "2.0.0"));
        assert!(!check_min_version("1.9.9", "2.0.0"));
        assert!(check_min_version("3.0.0", "2.0.0"));
    }

    #[test]
    fn test_cli_type_display() {
        assert_eq!(CliType::Claude.to_string(), "claude");
        assert_eq!(CliType::Gemini.to_string(), "gemini");
    }

    /// Integration test: verify that claude detection works end-to-end.
    /// Requires claude to be installed in the test environment.
    #[test]
    #[ignore = "requires claude CLI installed"]
    fn test_detect_claude_integration() {
        let agents = detect_agents();
        let claude = agents.iter().find(|a| a.cli == CliType::Claude);
        assert!(claude.is_some(), "Claude should be detected");
        let c = claude.unwrap();
        assert!(c.failure_reason.is_none(), "No failure: {:?}", c.failure_reason);
        assert!(!c.version.is_empty(), "Version should not be empty");
        println!("Claude: path={:?} version={}", c.path, c.version);
    }

    /// Integration test: verify that gemini detection works end-to-end.
    #[test]
    #[ignore = "requires gemini CLI installed"]
    fn test_detect_gemini_integration() {
        let agents = detect_agents();
        let gemini = agents.iter().find(|a| a.cli == CliType::Gemini);
        assert!(gemini.is_some(), "Gemini should be detected");
        let g = gemini.unwrap();
        assert!(g.failure_reason.is_none(), "No failure: {:?}", g.failure_reason);
        assert!(!g.version.is_empty(), "Version should not be empty");
        println!("Gemini: path={:?} version={}", g.path, g.version);
    }

    /// Test that get_cli_version falls back to shell when direct execution fails.
    #[test]
    fn test_get_cli_version_shell_fallback() {
        // Use a known command that works via login shell
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
        // The shell binary itself should report a version
        let path = std::path::PathBuf::from(&shell);
        let version = get_cli_version(&path, &["--version"]);
        assert!(version.is_some(), "Shell version check should succeed");
    }

    /// Verify that try_which resolves binaries via login shell PATH.
    #[test]
    fn test_try_which_login_shell() {
        // 'ls' should always be findable
        let path = try_which("ls");
        assert!(path.is_some(), "ls should be found via which");
    }

    /// Verify version extraction from multi-word output.
    #[test]
    fn test_version_extraction() {
        // Simulate "2.1.78 (Claude Code)" → "2.1.78"
        let stdout = "2.1.78 (Claude Code)".to_string();
        let version = stdout
            .split_whitespace()
            .find(|s| s.chars().next().map_or(false, |c| c.is_ascii_digit()))
            .unwrap_or(&stdout)
            .to_string();
        assert_eq!(version, "2.1.78");
    }

    #[test]
    fn test_is_executable_script_validation() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        
        // 1. Script with missing absolute interpreter
        let bad_script_path = temp_dir.path().join("bad_script");
        {
            let mut f = File::create(&bad_script_path).unwrap();
            writeln!(f, "#!/non/existent/node").unwrap();
        }
        assert!(!is_executable_script(&bad_script_path));

        // 2. Script with existing absolute interpreter (using /bin/sh which is standard)
        let good_script_path = temp_dir.path().join("good_script");
        {
            let mut f = File::create(&good_script_path).unwrap();
            writeln!(f, "#!/bin/sh").unwrap();
        }
        assert!(is_executable_script(&good_script_path));

        // 3. NVM style: /usr/bin/env node with node in same dir
        let nvm_gemini_path = temp_dir.path().join("gemini");
        let nvm_node_path = temp_dir.path().join("node");
        {
            let mut f = File::create(&nvm_gemini_path).unwrap();
            writeln!(f, "#!/usr/bin/env node").unwrap();
            File::create(&nvm_node_path).unwrap();
        }
        assert!(is_executable_script(&nvm_gemini_path));

        // 4. No shebang (binary or simple script)
        let binary_path = temp_dir.path().join("binary");
        {
            let mut f = File::create(&binary_path).unwrap();
            f.write_all(&[0x7f, 0x45, 0x4c, 0x46]).unwrap(); // ELF header
        }
        assert!(is_executable_script(&binary_path));
    }
}
