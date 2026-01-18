//! Error types for the memory subsystem.

use thiserror::Error;

/// Memory subsystem error type.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Invalid configuration or unsupported values.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    /// Invalid or unsafe memory item content.
    #[error("invalid memory item: {0}")]
    InvalidMemoryItem(String),
    /// `SQLite` storage error (sync).
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// `SQLite` storage error (async).
    #[error("tokio-rusqlite error: {0}")]
    TokioSqlite(#[from] tokio_rusqlite::Error),
    /// Vector store error.
    #[error("vector store error: {0}")]
    VectorStore(#[from] rig::vector_store::VectorStoreError),
    /// Embedding error.
    #[error("embedding error: {0}")]
    Embedding(#[from] rig::embeddings::EmbeddingError),
    /// HTTP client error from Rig.
    #[error("http client error: {0}")]
    HttpClient(#[from] rig::http_client::Error),
    /// Completion error.
    #[error("completion error: {0}")]
    Completion(#[from] rig::completion::CompletionError),
    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// URL parse error.
    #[error("url parse error: {0}")]
    Url(#[from] url::ParseError),
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Required `SQLite` extension not available.
    #[error("sqlite-vec extension is not available; load it before initializing the vector store")]
    SqliteVecUnavailable,
}

/// Convenience result alias for memory operations.
pub type MemoryResult<T> = Result<T, MemoryError>;
