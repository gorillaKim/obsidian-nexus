use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("CLI not found: {0}")]
    CliNotFound(String),

    #[error("CLI version check failed: {0}")]
    VersionCheckFailed(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already exists: {0}")]
    SessionAlreadyExists(String),

    #[error("Process spawn failed: {0}")]
    ProcessSpawnFailed(String),

    #[error("Process communication failed: {0}")]
    ProcessCommFailed(String),

    #[error("Prompt loading failed: {0}")]
    PromptLoadFailed(String),

    #[error("Prompt validation failed: {0}")]
    PromptValidationFailed(String),

    #[error("Config loading failed: {0}")]
    ConfigLoadFailed(String),

    #[error("Authentication expired for {0}")]
    AuthExpired(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
