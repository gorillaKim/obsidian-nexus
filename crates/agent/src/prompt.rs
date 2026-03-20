use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::AgentError;

/// Default prompts embedded in the binary as fallback.
mod defaults {
    pub const SYSTEM: &str = include_str!("../resources/librarian/system.md");
    pub const SEARCH_STRATEGY: &str = include_str!("../resources/librarian/search-strategy.md");
    pub const DOC_MAINTENANCE: &str = include_str!("../resources/librarian/doc-maintenance.md");
    pub const APP_GUIDE: &str = include_str!("../resources/librarian/app-guide.md");
    pub const OUTPUT_RULES: &str = include_str!("../resources/librarian/output-rules.md");
    pub const CONFIG: &str = include_str!("../resources/config.json");
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agents: HashMap<String, AgentDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub prompts: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct PromptContext {
    pub project_name: String,
    pub project_path: String,
    pub doc_count: u64,
    pub top_tags: Vec<String>,
}

pub struct PromptLoader {
    agents_dir: PathBuf,
}

impl PromptLoader {
    pub fn new() -> Result<Self, AgentError> {
        let agents_dir = Self::agents_dir()?;
        Ok(Self { agents_dir })
    }

    fn agents_dir() -> Result<PathBuf, AgentError> {
        let home = dirs::home_dir().ok_or_else(|| {
            AgentError::ConfigLoadFailed("Could not determine home directory".to_string())
        })?;
        Ok(home.join(".obsidian-nexus").join("agents"))
    }

    /// Initialize the agents directory with default prompt files if missing.
    pub fn ensure_defaults(&self) -> Result<(), AgentError> {
        let librarian_dir = self.agents_dir.join("librarian");
        std::fs::create_dir_all(&librarian_dir).map_err(AgentError::Io)?;

        let files = [
            ("librarian/system.md", defaults::SYSTEM),
            ("librarian/search-strategy.md", defaults::SEARCH_STRATEGY),
            ("librarian/doc-maintenance.md", defaults::DOC_MAINTENANCE),
            ("librarian/app-guide.md", defaults::APP_GUIDE),
            ("librarian/output-rules.md", defaults::OUTPUT_RULES),
        ];

        for (path, content) in &files {
            let full_path = self.agents_dir.join(path);
            if !full_path.exists() {
                std::fs::write(&full_path, content).map_err(AgentError::Io)?;
                debug!("Created default prompt: {}", path);
            }
        }

        // Write config.json if missing
        let config_path = self.agents_dir.join("config.json");
        if !config_path.exists() {
            std::fs::write(&config_path, defaults::CONFIG).map_err(AgentError::Io)?;
            debug!("Created default config.json");
        }

        Ok(())
    }

    /// Load and validate the agent configuration.
    pub fn load_config(&self) -> Result<AgentConfig, AgentError> {
        let config_path = self.agents_dir.join("config.json");

        let content = if config_path.exists() {
            std::fs::read_to_string(&config_path).map_err(AgentError::Io)?
        } else {
            warn!("config.json not found, using built-in default");
            defaults::CONFIG.to_string()
        };

        serde_json::from_str(&content)
            .map_err(|e| AgentError::ConfigLoadFailed(format!("Invalid config.json: {}", e)))
    }

    /// Build the system prompt for a given agent with context.
    pub fn build_system_prompt(
        &self,
        agent_name: &str,
        context: &PromptContext,
    ) -> Result<String, AgentError> {
        let config = self.load_config()?;

        let agent_def = config.agents.get(agent_name).ok_or_else(|| {
            AgentError::PromptLoadFailed(format!("Agent '{}' not found in config", agent_name))
        })?;

        if !agent_def.enabled {
            return Err(AgentError::PromptLoadFailed(format!(
                "Agent '{}' is disabled",
                agent_name
            )));
        }

        let mut parts = Vec::new();

        for prompt_path in &agent_def.prompts {
            let content = self.load_prompt_file(prompt_path)?;
            let body = extract_body(&content);
            parts.push(body);
        }

        // Add project context section
        let context_section = format!(
            "\n## 프로젝트 컨텍스트\n- 현재 프로젝트: {}\n- 프로젝트 경로: {}\n- 문서 수: {}개\n- 주요 태그: {}",
            context.project_name,
            context.project_path,
            context.doc_count,
            context.top_tags.join(", "),
        );
        parts.push(context_section);

        let combined = parts.join("\n\n---\n\n");

        // Variable substitution
        let result = substitute_variables(&combined, context);

        // Validation
        self.validate_prompt(&result)?;

        Ok(result)
    }

    fn load_prompt_file(&self, relative_path: &str) -> Result<String, AgentError> {
        let full_path = self.agents_dir.join(relative_path);

        if full_path.exists() {
            // Path traversal guard
            let canonical = full_path
                .canonicalize()
                .map_err(|e| AgentError::PromptLoadFailed(format!("Invalid path: {}", e)))?;
            let base = self
                .agents_dir
                .canonicalize()
                .map_err(|e| AgentError::PromptLoadFailed(format!("Invalid base: {}", e)))?;
            if !canonical.starts_with(&base) {
                return Err(AgentError::PromptLoadFailed(
                    "Path traversal detected: path is outside agents directory".to_string(),
                ));
            }

            std::fs::read_to_string(&canonical).map_err(AgentError::Io)
        } else {
            // Fallback to built-in defaults
            warn!(
                "Prompt file not found: {}, using built-in default",
                relative_path
            );
            match relative_path {
                "librarian/system.md" => Ok(defaults::SYSTEM.to_string()),
                "librarian/search-strategy.md" => Ok(defaults::SEARCH_STRATEGY.to_string()),
                "librarian/doc-maintenance.md" => Ok(defaults::DOC_MAINTENANCE.to_string()),
                "librarian/app-guide.md" => Ok(defaults::APP_GUIDE.to_string()),
                "librarian/output-rules.md" => Ok(defaults::OUTPUT_RULES.to_string()),
                _ => Err(AgentError::PromptLoadFailed(format!(
                    "No fallback for: {}",
                    relative_path
                ))),
            }
        }
    }

    fn validate_prompt(&self, prompt: &str) -> Result<(), AgentError> {
        if prompt.trim().is_empty() {
            return Err(AgentError::PromptValidationFailed(
                "Prompt is empty".to_string(),
            ));
        }

        // Check for unsubstituted variables
        let unsubstituted: Vec<&str> = prompt
            .match_indices('{')
            .filter_map(|(i, _)| {
                let rest = &prompt[i..];
                if let Some(end) = rest.find('}') {
                    let var = &rest[..=end];
                    if var.len() > 2
                        && var.chars().nth(1).map_or(false, |c| c.is_alphabetic())
                    {
                        Some(var)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if !unsubstituted.is_empty() {
            warn!(
                "Unsubstituted variables in prompt: {:?}",
                unsubstituted
            );
        }

        Ok(())
    }

    /// Generate an MCP config JSON file for the CLI.
    pub fn generate_mcp_config(&self, nexus_mcp_path: &Path) -> Result<PathBuf, AgentError> {
        let config = serde_json::json!({
            "mcpServers": {
                "nexus": {
                    "command": nexus_mcp_path.to_string_lossy(),
                    "args": []
                }
            }
        });

        let config_path = self.agents_dir.join("nexus-mcp-config.json");
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&config_path, content).map_err(AgentError::Io)?;

        Ok(config_path)
    }
}

/// Extract the body from a markdown file, stripping frontmatter.
fn extract_body(content: &str) -> String {
    let trimmed = content.trim();

    if trimmed.starts_with("---") {
        // Find the closing ---
        if let Some(end_idx) = trimmed[3..].find("---") {
            let body_start = end_idx + 6; // Skip both --- markers
            return trimmed[body_start..].trim().to_string();
        }
    }

    trimmed.to_string()
}

/// Substitute context variables in the prompt text.
fn substitute_variables(text: &str, context: &PromptContext) -> String {
    text.replace("{project_name}", &context.project_name)
        .replace("{project_path}", &context.project_path)
        .replace("{doc_count}", &context.doc_count.to_string())
        .replace("{top_tags}", &context.top_tags.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_body_with_frontmatter() {
        let content = "---\nname: test\nversion: 1.0\n---\n\nHello world";
        assert_eq!(extract_body(content), "Hello world");
    }

    #[test]
    fn test_extract_body_without_frontmatter() {
        let content = "Hello world";
        assert_eq!(extract_body(content), "Hello world");
    }

    #[test]
    fn test_substitute_variables() {
        let context = PromptContext {
            project_name: "my-vault".to_string(),
            project_path: "/path/to/vault".to_string(),
            doc_count: 42,
            top_tags: vec!["#dev".to_string(), "#infra".to_string()],
        };

        let text = "Project: {project_name}, Docs: {doc_count}";
        let result = substitute_variables(text, &context);
        assert_eq!(result, "Project: my-vault, Docs: 42");
    }
}
