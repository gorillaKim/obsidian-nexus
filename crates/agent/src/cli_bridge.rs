use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::AgentError;

/// A JSONL response from the sidecar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResponse {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: Option<Vec<String>>,
    #[serde(rename = "toolName", default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<serde_json::Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub done: Option<bool>,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub duration: Option<u64>,
    #[serde(default)]
    pub usage: Option<serde_json::Value>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub retryable: Option<bool>,
}

/// Manages a Node.js sidecar process for Claude Agent SDK communication.
pub struct SidecarManager {
    process: Arc<Mutex<Option<SidecarProcess>>>,
    sidecar_script: PathBuf,
}

struct SidecarProcess {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

fn lock_process(
    mutex: &Mutex<Option<SidecarProcess>>,
) -> Result<std::sync::MutexGuard<'_, Option<SidecarProcess>>, AgentError> {
    mutex
        .lock()
        .map_err(|_| AgentError::ProcessCommFailed("Lock poisoned".to_string()))
}

impl SidecarManager {
    pub fn new(sidecar_script: PathBuf) -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            sidecar_script,
        }
    }

    /// Start the Node.js sidecar process if not already running.
    pub fn ensure_running(&self) -> Result<(), AgentError> {
        let mut proc = lock_process(&self.process)?;
        if proc.is_some() {
            return Ok(());
        }

        info!("Starting sidecar: node {}", self.sidecar_script.display());

        let mut child = Command::new("node")
            .arg(&self.sidecar_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AgentError::ProcessSpawnFailed(format!("Failed to start sidecar: {}", e)))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            AgentError::ProcessSpawnFailed("Failed to capture sidecar stdin".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            AgentError::ProcessSpawnFailed("Failed to capture sidecar stdout".to_string())
        })?;

        let reader = BufReader::new(stdout);

        *proc = Some(SidecarProcess {
            child,
            stdin,
            reader,
        });

        info!("Sidecar started");
        Ok(())
    }

    /// Send a JSONL request to the sidecar's stdin.
    pub fn send_request(&self, request: &serde_json::Value) -> Result<(), AgentError> {
        let mut proc = lock_process(&self.process)?;
        let sidecar = proc.as_mut().ok_or_else(|| {
            AgentError::ProcessCommFailed("Sidecar not running".to_string())
        })?;

        let line = serde_json::to_string(request)
            .map_err(|e| AgentError::ProcessCommFailed(e.to_string()))?;

        writeln!(sidecar.stdin, "{}", line)
            .map_err(|e| AgentError::ProcessCommFailed(format!("stdin write failed: {}", e)))?;

        sidecar.stdin.flush()
            .map_err(|e| AgentError::ProcessCommFailed(format!("stdin flush failed: {}", e)))?;

        debug!("Sent request: {}", request.get("type").and_then(|t| t.as_str()).unwrap_or("?"));

        Ok(())
    }

    /// Read one JSONL response line from sidecar stdout.
    /// Returns None on EOF (sidecar exited).
    pub fn read_response(&self) -> Result<Option<BridgeResponse>, AgentError> {
        let mut proc = lock_process(&self.process)?;
        let sidecar = proc.as_mut().ok_or_else(|| {
            AgentError::ProcessCommFailed("Sidecar not running".to_string())
        })?;

        let mut line = String::new();
        match sidecar.reader.read_line(&mut line) {
            Ok(0) => {
                warn!("Sidecar EOF — process exited");
                Ok(None)
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return Ok(None);
                }
                let resp: BridgeResponse = serde_json::from_str(trimmed)
                    .map_err(|e| AgentError::ProcessCommFailed(format!("JSON parse error: {} (line: {})", e, trimmed)))?;
                Ok(Some(resp))
            }
            Err(e) => Err(AgentError::ProcessCommFailed(format!("stdout read failed: {}", e))),
        }
    }

    /// Check if the sidecar process is alive.
    pub fn is_running(&self) -> bool {
        lock_process(&self.process)
            .map(|proc| proc.is_some())
            .unwrap_or(false)
    }

    /// Shutdown the sidecar process.
    pub fn shutdown(&self) {
        if let Ok(mut proc) = lock_process(&self.process) {
            if let Some(mut sidecar) = proc.take() {
                // Close stdin to signal shutdown
                drop(sidecar.stdin);
                // Wait briefly then kill
                match sidecar.child.try_wait() {
                    Ok(Some(_)) => info!("Sidecar exited cleanly"),
                    _ => {
                        let _ = sidecar.child.kill();
                        let _ = sidecar.child.wait();
                        info!("Sidecar killed");
                    }
                }
            }
        }
    }
}

impl Drop for SidecarManager {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bridge_response_init() {
        let json = r#"{"type":"init","sessionId":"abc","model":"sonnet","mcpServers":["nexus"]}"#;
        let resp: BridgeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.msg_type, "init");
        assert_eq!(resp.session_id, "abc");
        assert_eq!(resp.model.unwrap(), "sonnet");
    }

    #[test]
    fn test_parse_bridge_response_text() {
        let json = r#"{"type":"text","sessionId":"abc","content":"안녕하세요","done":false}"#;
        let resp: BridgeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.msg_type, "text");
        assert_eq!(resp.content.unwrap(), "안녕하세요");
        assert_eq!(resp.done.unwrap(), false);
    }

    #[test]
    fn test_parse_bridge_response_tool_use() {
        let json = r#"{"type":"tool_use","sessionId":"abc","toolName":"nexus_search","input":{"query":"AWS"},"status":"running"}"#;
        let resp: BridgeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.msg_type, "tool_use");
        assert_eq!(resp.tool_name.unwrap(), "nexus_search");
        assert_eq!(resp.status.unwrap(), "running");
    }

    #[test]
    fn test_parse_bridge_response_result() {
        let json = r#"{"type":"result","sessionId":"abc","content":"최종 답변","cost":0.05,"duration":2100}"#;
        let resp: BridgeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.msg_type, "result");
        assert_eq!(resp.cost.unwrap(), 0.05);
    }

    #[test]
    fn test_parse_bridge_response_error() {
        let json = r#"{"type":"error","sessionId":"abc","code":"auth_expired","message":"인증 만료","retryable":false}"#;
        let resp: BridgeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.msg_type, "error");
        assert_eq!(resp.retryable.unwrap(), false);
    }
}
