use std::path::{Path, PathBuf};
use serde_json::{json, Value};

const CLAUDE_MD_SECTION: &str = include_str!("templates/claude_md_section.md");

const NEXUS_TOOLS: &[&str] = &[
    "mcp__nexus__nexus_search",
    "mcp__nexus__nexus_get_document",
    "mcp__nexus__nexus_get_documents",
    "mcp__nexus__nexus_get_section",
    "mcp__nexus__nexus_get_toc",
    "mcp__nexus__nexus_resolve_alias",
    "mcp__nexus__nexus_get_backlinks",
    "mcp__nexus__nexus_get_links",
    "mcp__nexus__nexus_get_cluster",
    "mcp__nexus__nexus_find_path",
    "mcp__nexus__nexus_find_related",
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
    Installed,
    Repaired,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct OnboardStep {
    pub name: String,
    pub status: StepStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl OnboardStep {
    fn created(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), status: StepStatus::Created, message: message.into(), path: None }
    }
    fn skipped(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), status: StepStatus::Skipped, message: message.into(), path: None }
    }
    fn error(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self { name: name.into(), status: StepStatus::Error, message: message.into(), path: None }
    }
    fn installed(name: impl Into<String>, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self { name: name.into(), status: StepStatus::Installed, message: message.into(), path: Some(path.into()) }
    }
    fn repaired(name: impl Into<String>, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self { name: name.into(), status: StepStatus::Repaired, message: message.into(), path: Some(path.into()) }
    }
}

/// 바이너리 탐색: ~/.local/bin → current_exe dir → PATH
/// 심링크 검증 포함. 반환값: Some(실제 실행 파일 경로)
fn resolve_binary(bin_name: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok().map(PathBuf::from);

    // 1. ~/.local/bin/<bin_name>
    if let Some(ref h) = home {
        let candidate = h.join(".local/bin").join(bin_name);
        if let Some(resolved) = resolve_symlink_or_file(&candidate) {
            return Some(resolved);
        }
    }

    // 2. /usr/local/bin/<bin_name>
    let candidate = PathBuf::from("/usr/local/bin").join(bin_name);
    if let Some(resolved) = resolve_symlink_or_file(&candidate) {
        return Some(resolved);
    }

    // 3. current_exe directory
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(bin_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // 4. PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = PathBuf::from(dir).join(bin_name);
            if let Some(resolved) = resolve_symlink_or_file(&candidate) {
                return Some(resolved);
            }
        }
    }

    None
}

/// 경로가 유효한 파일(또는 symlink 대상)인지 확인. 유효하면 대상 경로 반환.
fn resolve_symlink_or_file(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    // symlink인 경우 대상 존재 확인
    if path.is_symlink() {
        if let Ok(target) = std::fs::read_link(path) {
            let abs_target = if target.is_absolute() {
                target
            } else {
                path.parent().unwrap_or(Path::new(".")).join(target)
            };
            if abs_target.exists() {
                return Some(path.to_path_buf());
            }
            return None; // broken symlink
        }
        return None;
    }
    if path.is_file() { Some(path.to_path_buf()) } else { None }
}

/// broken symlink 감지 및 재생성 시도
fn repair_or_create_symlink(link_path: &Path, target: &Path) -> std::result::Result<bool, String> {
    // broken symlink 제거
    if link_path.exists() || link_path.is_symlink() {
        std::fs::remove_file(link_path).map_err(|e| e.to_string())?;
    }
    // 부모 디렉토리 생성
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path).map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    {
        return Err("symlink creation not supported on this platform".to_string());
    }
    Ok(true)
}

/// .claude 폴더를 project → HOME 방향으로 탐색, 가장 상위(HOME에 가까운) 발견 경로 반환
fn find_topmost_claude_dir(start: &Path) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok().map(PathBuf::from);
    let mut found: Option<PathBuf> = None;
    let mut current = start.to_path_buf();

    loop {
        let candidate = current.join(".claude");
        if candidate.is_dir() {
            found = Some(candidate);
        }
        // HOME에 도달하면 중단
        if let Some(ref h) = home {
            if &current == h {
                break;
            }
        }
        match current.parent() {
            Some(p) => current = p.to_path_buf(),
            None => break,
        }
    }

    found
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

    let mut steps: Vec<OnboardStep> = Vec::new();

    // Step 1: 바이너리 탐색 + 심링크 검증/수정
    let mcp_bin_str = resolve_mcp_server_binary(&mut steps);

    // Step 2: .claude 폴더 최상위 탐색
    let topmost_claude = find_topmost_claude_dir(&project_path);
    let (mcp_json_base, settings_base, claude_md_base) = if let Some(ref claude_dir) = topmost_claude {
        let base = claude_dir.parent().unwrap_or(&project_path).to_path_buf();
        (base.clone(), claude_dir.clone(), base)
    } else {
        // fallback: project_path에 생성
        (project_path.clone(), project_path.join(".claude"), project_path.clone())
    };

    // Step 3: .mcp.json
    let mcp_json_path = mcp_json_base.join(".mcp.json");
    steps.push(setup_mcp_json(&mcp_json_path, &mcp_bin_str, force));

    // Step 4: settings.json
    let settings_path = settings_base.join("settings.json");
    let settings_scope = if topmost_claude.as_ref()
        .and_then(|p| p.parent())
        .and_then(|p| std::env::var("HOME").ok().map(|h| p == Path::new(&h)))
        .unwrap_or(false)
    {
        "global"
    } else {
        "project"
    };
    steps.push(setup_settings_json(&settings_path, settings_scope, force));

    // Step 5: CLAUDE.md
    let claude_md_path = claude_md_base.join("CLAUDE.md");
    steps.push(setup_claude_md(&claude_md_path, force));

    Ok(steps)
}

/// nexus-mcp-server 바이너리를 탐색하고, 필요하면 심링크를 생성/수정합니다.
/// steps에 Installed/Repaired/Skipped 단계를 기록합니다.
fn resolve_mcp_server_binary(steps: &mut Vec<OnboardStep>) -> String {
    let bin_name = "nexus-mcp-server";
    let home = std::env::var("HOME").ok().map(PathBuf::from);

    // 이미 찾을 수 있는지 확인
    if let Some(found) = resolve_binary(bin_name) {
        let found_str = found.to_string_lossy().to_string();
        // broken symlink 체크 (이미 resolve_binary가 확인했으므로 valid)
        steps.push(OnboardStep::skipped(bin_name, format!("발견됨: {}", found_str)));
        return found_str;
    }

    // broken symlink 감지 후 수정
    let local_bin_link = home.as_ref().map(|h| h.join(".local/bin").join(bin_name));
    if let Some(ref link) = local_bin_link {
        if link.is_symlink() {
            // broken symlink — 현재 실행 파일로 대상 재생성 시도
            if let Ok(exe) = std::env::current_exe() {
                let target = exe.parent().unwrap_or(Path::new(".")).join(bin_name);
                if target.exists() {
                    match repair_or_create_symlink(link, &target) {
                        Ok(_) => {
                            let s = link.to_string_lossy().to_string();
                            steps.push(OnboardStep::repaired(bin_name, "broken symlink 수정됨", &s));
                            return s;
                        }
                        Err(e) => {
                            steps.push(OnboardStep::error(bin_name, format!("broken symlink 수정 실패: {}", e)));
                        }
                    }
                }
            }
        }
    }

    // 바이너리 미발견 — current_exe 기준으로 심링크 생성 시도
    if let Ok(exe) = std::env::current_exe() {
        let target = exe.parent().unwrap_or(Path::new(".")).join(bin_name);
        if target.exists() {
            if let Some(ref link) = local_bin_link {
                match repair_or_create_symlink(link, &target) {
                    Ok(_) => {
                        let s = link.to_string_lossy().to_string();
                        steps.push(OnboardStep::installed(bin_name, "~/.local/bin에 심링크 생성됨", &s));
                        return s;
                    }
                    Err(_) => {
                        // 심링크 실패 → 절대경로 직접 사용
                        let s = target.to_string_lossy().to_string();
                        steps.push(OnboardStep::installed(bin_name, "절대경로 사용 (심링크 생성 실패)", &s));
                        return s;
                    }
                }
            }
        }
        // fallback: current_exe 디렉토리의 nexus-mcp-server
        let fallback = exe.parent().unwrap_or(Path::new(".")).join(bin_name);
        let s = fallback.to_string_lossy().to_string();
        steps.push(OnboardStep::installed(bin_name, "경로 추정 (바이너리 미발견)", &s));
        return s;
    }

    // 최후 fallback
    let s = "/usr/local/bin/nexus-mcp-server".to_string();
    steps.push(OnboardStep::error(bin_name, "바이너리를 찾을 수 없어 기본 경로 사용"));
    s
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
