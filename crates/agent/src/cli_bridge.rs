use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::AgentError;

/// Find the Node.js binary, searching common locations including nvm.
/// macOS GUI apps don't inherit shell PATH, so we must search explicitly.
fn find_node_binary() -> PathBuf {
    let system_candidates = [
        "/usr/local/bin/node",
        "/opt/homebrew/bin/node",
        "/usr/bin/node",
    ];
    for path in &system_candidates {
        if std::path::Path::new(path).exists() {
            return PathBuf::from(path);
        }
    }
    if let Some(home) = dirs::home_dir() {
        let nvm_dir = home.join(".nvm/versions/node");
        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            let mut versions: Vec<_> = entries.flatten().collect();
            versions.sort_by_key(|e| e.file_name());
            for entry in versions.iter().rev() {
                let candidate = entry.path().join("bin/node");
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from("node")
}

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
    #[serde(default)]
    pub cancelled: Option<bool>,
}

/// Manages a Node.js sidecar process for Claude Agent SDK communication.
///
/// stdin and reader use SEPARATE mutexes so send_request (e.g. cancel)
/// is never blocked by a concurrent blocking read_line in the reader loop.
pub struct SidecarManager {
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    /// Owned by the background reader thread after take_reader() is called.
    reader: Mutex<Option<BufReader<ChildStdout>>>,
    child: Mutex<Option<Child>>,
    sidecar_script: PathBuf,
}

impl SidecarManager {
    pub fn new(sidecar_script: PathBuf) -> Self {
        Self {
            stdin: Arc::new(Mutex::new(None)),
            reader: Mutex::new(None),
            child: Mutex::new(None),
            sidecar_script,
        }
    }

    /// Start the sidecar if not already running (or if it has crashed).
    pub fn ensure_running(&self) -> Result<(), AgentError> {
        let mut child_guard = self.child.lock()
            .map_err(|_| AgentError::ProcessCommFailed("Lock poisoned".to_string()))?;

        // Check if existing child is still alive
        if let Some(child) = child_guard.as_mut() {
            match child.try_wait() {
                Ok(None) => return Ok(()), // still running
                _ => {
                    // Exited or error — reset all state and restart
                    child_guard.take();
                    if let Ok(mut g) = self.stdin.lock() { g.take(); }
                    if let Ok(mut g) = self.reader.lock() { g.take(); }
                }
            }
        }

        info!("Starting sidecar: node {}", self.sidecar_script.display());
        let node_bin = find_node_binary();
        info!("Using node binary: {}", node_bin.display());

        let mut child = Command::new(&node_bin)
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

        *self.stdin.lock().map_err(|_| AgentError::ProcessCommFailed("Lock poisoned".to_string()))? = Some(stdin);
        *self.reader.lock().map_err(|_| AgentError::ProcessCommFailed("Lock poisoned".to_string()))? = Some(BufReader::new(stdout));
        *child_guard = Some(child);

        info!("Sidecar started");
        Ok(())
    }

    /// Take ownership of the reader for the background reader thread.
    /// Should be called once after ensure_running().
    pub fn take_reader(&self) -> Option<BufReader<ChildStdout>> {
        self.reader.lock().ok()?.take()
    }

    /// Send a JSONL request to the sidecar's stdin.
    /// Uses a separate lock from the reader — never blocks on read_line.
    pub fn send_request(&self, request: &serde_json::Value) -> Result<(), AgentError> {
        let mut stdin_guard = self.stdin.lock()
            .map_err(|_| AgentError::ProcessCommFailed("Lock poisoned".to_string()))?;
        let stdin = stdin_guard.as_mut().ok_or_else(|| {
            AgentError::ProcessCommFailed("Sidecar not running".to_string())
        })?;

        let line = serde_json::to_string(request)
            .map_err(|e| AgentError::ProcessCommFailed(e.to_string()))?;
        writeln!(stdin, "{}", line)
            .map_err(|e| AgentError::ProcessCommFailed(format!("stdin write failed: {}", e)))?;
        stdin.flush()
            .map_err(|e| AgentError::ProcessCommFailed(format!("stdin flush failed: {}", e)))?;

        debug!("Sent request: {}", request.get("type").and_then(|t| t.as_str()).unwrap_or("?"));
        Ok(())
    }

    /// Check if the sidecar process is alive.
    pub fn is_running(&self) -> bool {
        self.child.lock()
            .map(|mut g| {
                g.as_mut()
                    .map(|c| c.try_wait().ok().and_then(|s| s).is_none())
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    /// Shutdown the sidecar process.
    pub fn shutdown(&self) {
        if let Ok(mut g) = self.stdin.lock() { g.take(); }
        if let Ok(mut g) = self.child.lock() {
            if let Some(mut child) = g.take() {
                match child.try_wait() {
                    Ok(Some(_)) => info!("Sidecar exited cleanly"),
                    _ => {
                        let _ = child.kill();
                        let _ = child.wait();
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
    fn test_parse_bridge_response_cancelled() {
        let json = r#"{"type":"cancelled","sessionId":"abc"}"#;
        let resp: BridgeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.msg_type, "cancelled");
    }

    #[test]
    fn test_parse_bridge_response_error() {
        let json = r#"{"type":"error","sessionId":"abc","code":"auth_expired","message":"인증 만료","retryable":false}"#;
        let resp: BridgeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.msg_type, "error");
        assert_eq!(resp.retryable.unwrap(), false);
    }
}
