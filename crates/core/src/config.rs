use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::{NexusError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub indexer: IndexerConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub watcher: WatcherConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_dimensions")]
    pub dimensions: usize,
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    #[serde(default)]
    pub openai: Option<OpenAIConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key_env: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default = "default_chunk_overlap")]
    pub chunk_overlap: usize,
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default = "default_limit")]
    pub default_limit: usize,
    #[serde(default = "default_hybrid_weight")]
    pub hybrid_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub file: bool,
}

// Defaults
fn default_provider() -> String { "ollama".into() }
fn default_ollama_url() -> String { "http://localhost:11434".into() }
fn default_model() -> String { "nomic-embed-text".into() }
fn default_dimensions() -> usize { 768 }
fn default_chunk_size() -> usize { 512 }
fn default_chunk_overlap() -> usize { 50 }
fn default_exclude_patterns() -> Vec<String> {
    vec![".obsidian".into(), ".trash".into(), "node_modules".into(), ".git".into()]
}
fn default_limit() -> usize { 20 }
fn default_hybrid_weight() -> f64 { 0.7 }
fn default_debounce_ms() -> u64 { 500 }
fn default_log_level() -> String { "info".into() }

impl Default for Config {
    fn default() -> Self {
        Self {
            embedding: EmbeddingConfig::default(),
            indexer: IndexerConfig::default(),
            search: SearchConfig::default(),
            watcher: WatcherConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            dimensions: default_dimensions(),
            ollama_url: default_ollama_url(),
            openai: None,
        }
    }
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            chunk_size: default_chunk_size(),
            chunk_overlap: default_chunk_overlap(),
            exclude_patterns: default_exclude_patterns(),
        }
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: default_limit(),
            hybrid_weight: default_hybrid_weight(),
        }
    }
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: default_debounce_ms(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: false,
        }
    }
}

impl Config {
    /// Load config from the nexus data directory, or create default
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to the nexus data directory
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| NexusError::Config(e.to_string()))?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get the nexus data directory (~/.nexus/)
    pub fn data_dir() -> PathBuf {
        dirs::home_dir()
            .expect("Could not find home directory")
            .join(".nexus")
    }

    /// Get the config file path
    pub fn config_path() -> PathBuf {
        Self::data_dir().join("config.toml")
    }

    /// Get the SQLite database path
    pub fn db_path() -> PathBuf {
        Self::data_dir().join("nexus.db")
    }

    /// Get the LanceDB directory for a project
    pub fn lance_dir(project_id: &str) -> PathBuf {
        Self::data_dir().join("lance").join(project_id)
    }

    /// Get the logs directory
    pub fn logs_dir() -> PathBuf {
        Self::data_dir().join("logs")
    }

    /// Get the models directory
    pub fn models_dir() -> PathBuf {
        Self::data_dir().join("models")
    }

    /// Ensure the data directory structure exists
    pub fn ensure_dirs() -> Result<()> {
        let data_dir = Self::data_dir();
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(data_dir.join("lance"))?;
        std::fs::create_dir_all(data_dir.join("models"))?;
        std::fs::create_dir_all(data_dir.join("logs"))?;
        Ok(())
    }

    /// Check if a path matches any exclude pattern
    pub fn is_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.indexer.exclude_patterns.iter().any(|pattern| {
            path_str.contains(pattern)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.embedding.provider, "ollama");
        assert_eq!(config.embedding.model, "nomic-embed-text");
        assert_eq!(config.embedding.dimensions, 768);
        assert_eq!(config.indexer.chunk_size, 512);
        assert_eq!(config.search.default_limit, 20);
    }

    #[test]
    fn test_exclude_patterns() {
        let config = Config::default();
        assert!(config.is_excluded(Path::new("vault/.obsidian/plugins")));
        assert!(config.is_excluded(Path::new("vault/.git/config")));
        assert!(!config.is_excluded(Path::new("vault/notes/hello.md")));
    }
}
