use std::path::PathBuf;
use thiserror::Error;

/// Main error type for aidot
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum AidotError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration parse error: {0}")]
    ConfigParse(String),

    #[error("Template parse error: {0}")]
    TemplateParse(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Repository not found: {0}")]
    RepositoryNotFound(String),

    #[error("Template directory already exists: {0}")]
    TemplateAlreadyExists(PathBuf),

    #[error("Invalid template structure: {0}")]
    InvalidTemplate(String),

    #[error("Tool not detected: {0}")]
    ToolNotDetected(String),

    #[error("Merge conflict: {0}")]
    MergeConflict(String),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type alias for aidot operations
pub type Result<T> = std::result::Result<T, AidotError>;
