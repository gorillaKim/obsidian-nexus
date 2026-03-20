use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use uuid::Uuid;

use crate::cli_detector::CliType;
use crate::error::AgentError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub cli: CliType,
    pub model: String,
    pub name: String,
    pub project_id: String,
    #[serde(default)]
    pub message_count: u32,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
}

pub struct SessionManager {
    sessions_path: PathBuf,
}

impl SessionManager {
    pub fn new() -> Result<Self, AgentError> {
        let base_dir = Self::base_dir()?;
        std::fs::create_dir_all(&base_dir).map_err(AgentError::Io)?;

        let sessions_path = base_dir.join("sessions.json");

        Ok(Self { sessions_path })
    }

    fn base_dir() -> Result<PathBuf, AgentError> {
        let home = dirs::home_dir().ok_or_else(|| {
            AgentError::ConfigLoadFailed("Could not determine home directory".to_string())
        })?;
        Ok(home.join(".obsidian-nexus"))
    }

    pub fn create_session(
        &self,
        cli: CliType,
        model: &str,
        project_id: &str,
        name: Option<&str>,
    ) -> Result<SessionMeta, AgentError> {
        let session = SessionMeta {
            id: Uuid::new_v4().to_string(),
            cli,
            model: model.to_string(),
            name: name
                .unwrap_or("New Session")
                .to_string(),
            project_id: project_id.to_string(),
            message_count: 0,
            created_at: Utc::now(),
        };

        let mut sessions = self.load_sessions()?;
        sessions.push(session.clone());
        self.save_sessions(&sessions)?;

        debug!("Created session: {} ({})", session.id, session.name);

        Ok(session)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionMeta>, AgentError> {
        self.load_sessions()
    }

    pub fn get_session(&self, id: &str) -> Result<SessionMeta, AgentError> {
        let sessions = self.load_sessions()?;
        sessions
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| AgentError::SessionNotFound(id.to_string()))
    }

    pub fn delete_session(&self, id: &str) -> Result<(), AgentError> {
        let mut sessions = self.load_sessions()?;
        let len_before = sessions.len();
        sessions.retain(|s| s.id != id);

        if sessions.len() == len_before {
            return Err(AgentError::SessionNotFound(id.to_string()));
        }

        self.save_sessions(&sessions)?;
        debug!("Deleted session: {}", id);

        Ok(())
    }

    pub fn increment_message_count(&self, id: &str) -> Result<u32, AgentError> {
        let mut sessions = self.load_sessions()?;
        let session = sessions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| AgentError::SessionNotFound(id.to_string()))?;

        session.message_count += 1;
        let count = session.message_count;
        self.save_sessions(&sessions)?;

        Ok(count)
    }

    pub fn update_session_name(&self, id: &str, name: &str) -> Result<(), AgentError> {
        let mut sessions = self.load_sessions()?;
        let session = sessions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| AgentError::SessionNotFound(id.to_string()))?;

        session.name = name.to_string();
        self.save_sessions(&sessions)?;

        Ok(())
    }

    fn load_sessions(&self) -> Result<Vec<SessionMeta>, AgentError> {
        if !self.sessions_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&self.sessions_path).map_err(AgentError::Io)?;

        if content.trim().is_empty() {
            return Ok(Vec::new());
        }

        serde_json::from_str(&content).map_err(|e| {
            warn!("Failed to parse sessions.json, starting fresh: {}", e);
            AgentError::Json(e)
        })
    }

    fn save_sessions(&self, sessions: &[SessionMeta]) -> Result<(), AgentError> {
        let content = serde_json::to_string_pretty(sessions)?;
        std::fs::write(&self.sessions_path, content).map_err(AgentError::Io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_manager(dir: &TempDir) -> SessionManager {
        let sessions_path = dir.path().join("sessions.json");
        SessionManager { sessions_path }
    }

    #[test]
    fn test_create_and_list_sessions() {
        let dir = TempDir::new().unwrap();
        let mgr = test_manager(&dir);

        let session = mgr
            .create_session(CliType::Claude, "sonnet", "vault-1", Some("Test"))
            .unwrap();
        assert_eq!(session.model, "sonnet");
        assert_eq!(session.name, "Test");

        let sessions = mgr.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
    }

    #[test]
    fn test_delete_session() {
        let dir = TempDir::new().unwrap();
        let mgr = test_manager(&dir);

        let session = mgr
            .create_session(CliType::Claude, "sonnet", "vault-1", None)
            .unwrap();
        mgr.delete_session(&session.id).unwrap();

        let sessions = mgr.list_sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_delete_nonexistent_session() {
        let dir = TempDir::new().unwrap();
        let mgr = test_manager(&dir);

        let result = mgr.delete_session("nonexistent");
        assert!(result.is_err());
    }
}
