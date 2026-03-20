use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedAgent {
    pub cli: CliType,
    pub path: PathBuf,
    pub version: String,
    pub authenticated: bool,
    pub models: Vec<String>,
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

    if let Some(agent) = detect_claude() {
        agents.push(agent);
    }

    if let Some(agent) = detect_gemini() {
        agents.push(agent);
    }

    agents
}

fn detect_claude() -> Option<DetectedAgent> {
    let path = find_cli_path("claude")?;
    let version = get_cli_version(&path, &["--version"])?;

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
    })
}

fn detect_gemini() -> Option<DetectedAgent> {
    let path = find_cli_path("gemini")?;
    let version = get_cli_version(&path, &["--version"])?;

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
    })
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
        } else if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

fn try_which(name: &str) -> Option<PathBuf> {
    let output = Command::new("which").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path_str.is_empty() {
        return None;
    }
    Some(PathBuf::from(path_str))
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
        .filter(|p| p.exists())
        .collect();
    found.sort();
    found.into_iter().last() // pick highest version
}

fn get_cli_version(path: &PathBuf, args: &[&str]) -> Option<String> {
    let output = Command::new(path).args(args).output().ok()?;

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

    if !creds_path.exists() {
        return false;
    }

    // Parse as JSON to check key existence without keeping token values in memory
    std::fs::read_to_string(&creds_path)
        .ok()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .and_then(|v| v.get("claudeAiOauthTokenData").cloned())
        .is_some()
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
}
