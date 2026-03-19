use std::path::{Path, PathBuf};
use serde_json::{json, Value};

const LIBRARIAN_AGENT_TEMPLATE: &str = include_str!("templates/librarian_agent.md");
const LIBRARIAN_SKILL_TEMPLATE: &str = include_str!("templates/librarian_skill.md");

#[derive(Debug)]
pub struct OnboardResult {
    pub project_path: PathBuf,
    pub created: Vec<String>,
    pub skipped: Vec<String>,
}

impl OnboardResult {
    pub fn report(&self) -> String {
        let mut report = String::from("# Nexus Onboarding Complete\n\n");
        report.push_str(&format!("**Project**: {}\n\n", self.project_path.display()));

        if !self.created.is_empty() {
            report.push_str("## Created\n");
            for f in &self.created {
                report.push_str(&format!("- {}\n", f));
            }
            report.push('\n');
        }

        if !self.skipped.is_empty() {
            report.push_str("## Skipped (already exists)\n");
            for f in &self.skipped {
                report.push_str(&format!("- {}\n", f));
            }
            report.push('\n');
        }

        report.push_str("## Next Steps\n");
        report.push_str("1. **Restart Claude Code session** (MCP servers load at session start)\n");
        report.push_str("2. Use `/librarian <query>` to search documents\n");
        report.push_str("3. The librarian subagent will handle deep search and document management\n");

        report
    }
}

/// Run onboarding: create .mcp.json, librarian agent, and librarian skill in target project.
pub fn onboard(project_path: Option<&str>, force: bool) -> crate::Result<OnboardResult> {
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
    // MCP server binary is sibling named nexus-mcp-server
    let mcp_server_path = mcp_bin_path.parent()
        .unwrap_or(Path::new("."))
        .join("nexus-mcp-server");
    let mcp_bin_str = mcp_server_path.to_string_lossy();

    let mut created: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    // 1. Create/update .mcp.json
    let mcp_json_path = project_path.join(".mcp.json");
    setup_mcp_json(&mcp_json_path, &mcp_bin_str, force, &mut created, &mut skipped)?;

    // 2. Create .claude/agents/librarian.md
    let agent_path = project_path.join(".claude/agents/librarian.md");
    write_template(&agent_path, LIBRARIAN_AGENT_TEMPLATE, force, &mut created, &mut skipped)?;

    // 3. Create .claude/skills/librarian/SKILL.md
    let skill_path = project_path.join(".claude/skills/librarian/SKILL.md");
    write_template(&skill_path, LIBRARIAN_SKILL_TEMPLATE, force, &mut created, &mut skipped)?;

    Ok(OnboardResult {
        project_path,
        created,
        skipped,
    })
}

fn setup_mcp_json(
    path: &Path,
    mcp_bin_str: &str,
    force: bool,
    created: &mut Vec<String>,
    skipped: &mut Vec<String>,
) -> crate::Result<()> {
    let nexus_server_config = json!({
        "type": "stdio",
        "command": mcp_bin_str,
        "args": []
    });

    if path.exists() && !force {
        let existing = std::fs::read_to_string(path)?;
        let mut config: Value = serde_json::from_str(&existing)?;

        if config.get("mcpServers")
            .and_then(|s: &Value| s.get("nexus"))
            .is_some()
        {
            skipped.push(".mcp.json (nexus already registered)".to_string());
        } else {
            let servers = config.get_mut("mcpServers")
                .and_then(|s: &mut Value| s.as_object_mut());
            if let Some(servers) = servers {
                servers.insert("nexus".to_string(), nexus_server_config);
            } else {
                config["mcpServers"] = json!({ "nexus": nexus_server_config });
            }
            let content = serde_json::to_string_pretty(&config)?;
            std::fs::write(path, format!("{}\n", content))?;
            created.push(".mcp.json (nexus added)".to_string());
        }
    } else {
        let config = json!({ "mcpServers": { "nexus": nexus_server_config } });
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(path, format!("{}\n", content))?;
        created.push(".mcp.json".to_string());
    }

    Ok(())
}

fn write_template(
    path: &Path,
    content: &str,
    force: bool,
    created: &mut Vec<String>,
    skipped: &mut Vec<String>,
) -> crate::Result<()> {
    let display = path.to_string_lossy();

    if path.exists() && !force {
        skipped.push(format!("{} (use --force to overwrite)", display));
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, content)?;
    created.push(display.to_string());
    Ok(())
}
