use thiserror::Error;

#[derive(Error, Debug)]
pub enum NexusError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("R2D2 pool error: {0}")]
    Pool(#[from] r2d2::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Project already exists: {0}")]
    ProjectAlreadyExists(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Path does not exist: {0}")]
    PathNotFound(String),

    #[error("Indexing error: {0}")]
    Indexing(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Watcher error: {0}")]
    Watcher(String),
}

pub type Result<T> = std::result::Result<T, NexusError>;
