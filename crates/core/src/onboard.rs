use std::path::{Path, PathBuf};
use serde_json::{json, Value};

const CLAUDE_MD_SECTION: &str = include_str!("templates/claude_md_section.md");

const NEXUS_TOOLS: &[&str] = &[
    "mcp__nexus__nexus_search",
    "mcp__nexus__nexus_get_document",
    "mcp__nexus__nexus_get_section",
    "mcp__nexus__nexus_resolve_alias",
    "mcp__nexus__nexus_get_backlinks",
    "mcp__nexus__nexus_get_links",
    "mcp__nexus__nexus_get_metadata",
    "mcp__nexus__nexus_list_projects",
    "mcp__nexus__nexus_list_documents",
    "mcp__nexus__nexus_index_project",
    "mcp__nexus__nexus_status",
    "mcp__nexus__nexus_help",
];

const CLAUDE_MD_HEADING: &str = "## Obsidian Nexus - 문서 탐색 도구 우선순위";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Created,
    Skipped,
    Error,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OnboardStep {
    pub name: String,
    pub status: StepStatus,
    pub message: String,
}

impl OnboardStep {
    fn created(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), status: StepStatus::Created, message: message.into() }
    }
    fn skipped(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), status: StepStatus::Skipped, message: message.into() }
    }
    fn error(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), status: StepStatus::Error, message: message.into() }
    }
}

/// Run onboarding for a project: create .mcp.json, configure ~/.claude/settings.json,
/// and append Nexus section to CLAUDE.md.
pub fn onboard(project_path: Option<&str>, force: bool) -> crate::Result<Vec<OnboardStep>> {
    let project_path = if let Some(p) = project_path {
        PathBuf::from(p)
    } else {
        std::env::current_dir()?
    };

    if !project_path.is_dir() {
        return Err(crate::NexusError::PathNotFound(project_path.display().to_string()));
    }

    let mcp_bin_path = std::env::current_exe()
        .map_err(|e| crate::NexusError::Config(format!("Cannot determine binary path: {}", e)))?;
    let mcp_server_path = mcp_bin_path.parent()
        .unwrap_or(Path::new("."))
        .join("nexus-mcp-server");
    let mcp_bin_str = mcp_server_path.to_string_lossy();

    // settings.json: always in project-local .claude/ (created if not exists)
    let project_claude_dir = project_path.join(".claude");
    let settings_path = project_claude_dir.join("settings.json");
    let settings_scope = "project";

    let mut steps: Vec<OnboardStep> = Vec::new();

    // Step 1: .mcp.json
    let mcp_json_path = project_path.join(".mcp.json");
    steps.push(setup_mcp_json(&mcp_json_path, &mcp_bin_str, force));

    // Step 2: settings.json (project-local or global)
    steps.push(setup_settings_json(&settings_path, settings_scope, force));

    // Step 3: CLAUDE.md
    let claude_md_path = project_path.join("CLAUDE.md");
    steps.push(setup_claude_md(&claude_md_path, force));

    Ok(steps)
}

fn setup_mcp_json(path: &Path, mcp_bin_str: &str, force: bool) -> OnboardStep {
    let nexus_server_config = json!({
        "type": "stdio",
        "command": mcp_bin_str,
        "args": []
    });

    if path.exists() && !force {
        let existing = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return OnboardStep::error(".mcp.json", format!("파일 읽기 실패: {}", e)),
        };
        let mut config: Value = match serde_json::from_str(&existing) {
            Ok(v) => v,
            Err(e) => return OnboardStep::error(".mcp.json", format!("JSON 파싱 실패: {}", e)),
        };

        if config.get("mcpServers")
            .and_then(|s: &Value| s.get("nexus"))
            .is_some()
        {
            return OnboardStep::skipped(".mcp.json", "nexus 이미 등록됨");
        }

        let servers = config.get_mut("mcpServers")
            .and_then(|s: &mut Value| s.as_object_mut());
        if let Some(servers) = servers {
            servers.insert("nexus".to_string(), nexus_server_config);
        } else {
            config["mcpServers"] = json!({ "nexus": nexus_server_config });
        }
        let content = match serde_json::to_string_pretty(&config) {
            Ok(s) => s,
            Err(e) => return OnboardStep::error(".mcp.json", format!("직렬화 실패: {}", e)),
        };
        if let Err(e) = std::fs::write(path, format!("{}\n", content)) {
            return OnboardStep::error(".mcp.json", format!("파일 쓰기 실패: {}", e));
        }
        OnboardStep::created(".mcp.json", "nexus 서버 추가됨")
    } else {
        let config = json!({ "mcpServers": { "nexus": nexus_server_config } });
        let content = match serde_json::to_string_pretty(&config) {
            Ok(s) => s,
            Err(e) => return OnboardStep::error(".mcp.json", format!("직렬화 실패: {}", e)),
        };
        if let Err(e) = std::fs::write(path, format!("{}\n", content)) {
            return OnboardStep::error(".mcp.json", format!("파일 쓰기 실패: {}", e));
        }
        OnboardStep::created(".mcp.json", "생성됨")
    }
}

fn setup_settings_json(path: &Path, scope: &str, force: bool) -> OnboardStep {
    let nexus_tools: Vec<Value> = NEXUS_TOOLS.iter().map(|t| json!(*t)).collect();

    if path.exists() && !force {
        let existing = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return OnboardStep::error("settings.json", format!("파일 읽기 실패: {}", e)),
        };
        let mut config: Value = match serde_json::from_str(&existing) {
            Ok(v) => v,
            Err(e) => return OnboardStep::error("settings.json", format!("JSON 파싱 실패: {}", e)),
        };

        // Get or create permissions.allow array
        let allow = config
            .get_mut("permissions")
            .and_then(|p| p.get_mut("allow"))
            .and_then(|a| a.as_array_mut());

        if let Some(allow) = allow {
            let mut added = 0usize;
            for tool in &nexus_tools {
                if !allow.contains(tool) {
                    allow.push(tool.clone());
                    added += 1;
                }
            }
            if added == 0 {
                return OnboardStep::skipped("settings.json", format!("nexus 도구 이미 등록됨 ({})", scope));
            }
        } else {
            // Create permissions.allow
            if config.get("permissions").is_none() {
                config["permissions"] = json!({ "allow": nexus_tools });
            } else {
                config["permissions"]["allow"] = json!(nexus_tools);
            }
        }

        let content = match serde_json::to_string_pretty(&config) {
            Ok(s) => s,
            Err(e) => return OnboardStep::error("settings.json", format!("직렬화 실패: {}", e)),
        };
        if let Err(e) = write_file(path, &format!("{}\n", content)) {
            return OnboardStep::error("settings.json", format!("파일 쓰기 실패: {}", e));
        }
        OnboardStep::created("settings.json", format!("nexus 도구 권한 추가됨 ({})", scope))
    } else if force {
        let config = json!({ "permissions": { "allow": nexus_tools } });
        let content = match serde_json::to_string_pretty(&config) {
            Ok(s) => s,
            Err(e) => return OnboardStep::error("settings.json", format!("직렬화 실패: {}", e)),
        };
        if let Err(e) = write_file(path, &format!("{}\n", content)) {
            return OnboardStep::error("settings.json", format!("파일 쓰기 실패: {}", e));
        }
        OnboardStep::created("settings.json", format!("생성됨 ({}, force)", scope))
    } else {
        // New file
        let config = json!({ "permissions": { "allow": nexus_tools } });
        let content = match serde_json::to_string_pretty(&config) {
            Ok(s) => s,
            Err(e) => return OnboardStep::error("settings.json", format!("직렬화 실패: {}", e)),
        };
        if let Err(e) = write_file(path, &format!("{}\n", content)) {
            return OnboardStep::error("settings.json", format!("파일 쓰기 실패: {}", e));
        }
        OnboardStep::created("settings.json", format!("생성됨 ({})", scope))
    }
}

fn setup_claude_md(path: &Path, force: bool) -> OnboardStep {
    if path.exists() {
        let existing = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return OnboardStep::error("CLAUDE.md", format!("파일 읽기 실패: {}", e)),
        };

        if !force && existing.contains(CLAUDE_MD_HEADING) {
            return OnboardStep::skipped("CLAUDE.md", "Nexus 섹션 이미 존재함");
        }

        if force {
            // Remove existing Nexus section and re-append
            let cleaned = remove_nexus_section(&existing);
            let new_content = format!("{}{}", cleaned.trim_end(), CLAUDE_MD_SECTION);
            if let Err(e) = std::fs::write(path, new_content) {
                return OnboardStep::error("CLAUDE.md", format!("파일 쓰기 실패: {}", e));
            }
            return OnboardStep::created("CLAUDE.md", "Nexus 섹션 갱신됨 (force)");
        }

        // Append
        let new_content = format!("{}{}", existing.trim_end(), CLAUDE_MD_SECTION);
        if let Err(e) = std::fs::write(path, new_content) {
            return OnboardStep::error("CLAUDE.md", format!("파일 쓰기 실패: {}", e));
        }
        OnboardStep::created("CLAUDE.md", "Nexus 섹션 추가됨")
    } else {
        if let Err(e) = std::fs::write(path, CLAUDE_MD_SECTION) {
            return OnboardStep::error("CLAUDE.md", format!("파일 쓰기 실패: {}", e));
        }
        OnboardStep::created("CLAUDE.md", "생성됨")
    }
}

/// Remove the Nexus section from CLAUDE.md content (for force re-append)
fn remove_nexus_section(content: &str) -> String {
    if let Some(idx) = content.find(CLAUDE_MD_HEADING) {
        content[..idx].to_string()
    } else {
        content.to_string()
    }
}

/// Write file, creating parent directories if needed
fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_temp_project() -> TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    #[test]
    fn test_mcp_json_created_when_missing() {
        let dir = make_temp_project();
        let path = dir.path().join(".mcp.json");
        let step = setup_mcp_json(&path, "/usr/local/bin/nexus-mcp-server", false);
        assert!(matches!(step.status, StepStatus::Created));
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&content).unwrap();
        assert!(v["mcpServers"]["nexus"].is_object());
    }

    #[test]
    fn test_mcp_json_skipped_when_nexus_registered() {
        let dir = make_temp_project();
        let path = dir.path().join(".mcp.json");
        let existing = json!({ "mcpServers": { "nexus": { "type": "stdio", "command": "old", "args": [] } } });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();
        let step = setup_mcp_json(&path, "/usr/local/bin/nexus-mcp-server", false);
        assert!(matches!(step.status, StepStatus::Skipped));
    }

    #[test]
    fn test_mcp_json_merges_when_other_server_exists() {
        let dir = make_temp_project();
        let path = dir.path().join(".mcp.json");
        let existing = json!({ "mcpServers": { "other": { "type": "stdio", "command": "other", "args": [] } } });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();
        let step = setup_mcp_json(&path, "/usr/local/bin/nexus-mcp-server", false);
        assert!(matches!(step.status, StepStatus::Created));
        let content = std::fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&content).unwrap();
        assert!(v["mcpServers"]["nexus"].is_object());
        assert!(v["mcpServers"]["other"].is_object());
    }

    #[test]
    fn test_settings_json_created_when_missing() {
        let dir = make_temp_project();
        let path = dir.path().join("settings.json");
        let step = setup_settings_json(&path, "test", false);
        assert!(matches!(step.status, StepStatus::Created));
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&content).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        assert!(allow.contains(&json!("mcp__nexus__nexus_search")));
        assert_eq!(allow.len(), NEXUS_TOOLS.len());
    }

    #[test]
    fn test_settings_json_skipped_when_tools_present() {
        let dir = make_temp_project();
        let path = dir.path().join("settings.json");
        // All nexus tools already present
        let tools: Vec<Value> = NEXUS_TOOLS.iter().map(|t| json!(*t)).collect();
        let existing = json!({ "permissions": { "allow": tools } });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();
        let step = setup_settings_json(&path, "test", false);
        assert!(matches!(step.status, StepStatus::Skipped));
    }

    #[test]
    fn test_settings_json_merges_partial_tools() {
        let dir = make_temp_project();
        let path = dir.path().join("settings.json");
        let existing = json!({ "permissions": { "allow": ["mcp__nexus__nexus_search"] } });
        std::fs::write(&path, serde_json::to_string_pretty(&existing).unwrap()).unwrap();
        let step = setup_settings_json(&path, "test", false);
        assert!(matches!(step.status, StepStatus::Created));
        let content = std::fs::read_to_string(&path).unwrap();
        let v: Value = serde_json::from_str(&content).unwrap();
        let allow = v["permissions"]["allow"].as_array().unwrap();
        // Should have all tools without duplicates
        assert_eq!(allow.len(), NEXUS_TOOLS.len());
    }

    #[test]
    fn test_claude_md_created_when_missing() {
        let dir = make_temp_project();
        let path = dir.path().join("CLAUDE.md");
        let step = setup_claude_md(&path, false);
        assert!(matches!(step.status, StepStatus::Created));
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(CLAUDE_MD_HEADING));
    }

    #[test]
    fn test_claude_md_skipped_when_heading_exists() {
        let dir = make_temp_project();
        let path = dir.path().join("CLAUDE.md");
        let existing = format!("# My Project\n\n{}\n\nsome content", CLAUDE_MD_HEADING);
        std::fs::write(&path, &existing).unwrap();
        let step = setup_claude_md(&path, false);
        assert!(matches!(step.status, StepStatus::Skipped));
    }

    #[test]
    fn test_claude_md_appends_when_heading_missing() {
        let dir = make_temp_project();
        let path = dir.path().join("CLAUDE.md");
        std::fs::write(&path, "# My Project\n\nexisting content").unwrap();
        let step = setup_claude_md(&path, false);
        assert!(matches!(step.status, StepStatus::Created));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# My Project"));
        assert!(content.contains(CLAUDE_MD_HEADING));
    }

    #[test]
    fn test_claude_md_force_replaces_section() {
        let dir = make_temp_project();
        let path = dir.path().join("CLAUDE.md");
        let existing = format!("# Project\n\n{}\n\nold nexus content", CLAUDE_MD_HEADING);
        std::fs::write(&path, &existing).unwrap();
        let step = setup_claude_md(&path, true);
        assert!(matches!(step.status, StepStatus::Created));
        let content = std::fs::read_to_string(&path).unwrap();
        // Old section replaced with new
        assert!(content.contains(CLAUDE_MD_HEADING));
        assert!(!content.contains("old nexus content"));
    }

    #[test]
    fn test_onboard_returns_three_steps() {
        let dir = make_temp_project();
        // We can't easily test full onboard() due to home_dir dependency,
        // but we verify the step count via direct function calls
        let project = dir.path();
        let mcp_step = setup_mcp_json(&project.join(".mcp.json"), "/bin/nexus-mcp-server", false);
        let settings_step = setup_settings_json(&project.join("settings.json"), "test", false);
        let claude_step = setup_claude_md(&project.join("CLAUDE.md"), false);
        assert!(matches!(mcp_step.status, StepStatus::Created));
        assert!(matches!(settings_step.status, StepStatus::Created));
        assert!(matches!(claude_step.status, StepStatus::Created));
        assert_eq!(mcp_step.name, ".mcp.json");
        assert_eq!(settings_step.name, "settings.json");
        assert_eq!(claude_step.name, "CLAUDE.md");
    }

    #[test]
    fn test_remove_nexus_section() {
        let content = format!("# Header\n\ncontent before\n{}\n\nnexus content here", CLAUDE_MD_HEADING);
        let cleaned = remove_nexus_section(&content);
        assert!(!cleaned.contains(CLAUDE_MD_HEADING));
        assert!(cleaned.contains("content before"));
    }
}
