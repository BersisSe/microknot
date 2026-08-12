use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("JSON error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("SQLite error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid workflow: {0}")]
    InvalidWorkflow(String),
    #[error("knot execution failed: {0}")]
    Knot(String),
    #[error("workflow with id '{id}' not found")]
    NotFound { id: String },
    #[error("workflow with id '{id}' already exists")]
    Duplicate { id: String },
}
