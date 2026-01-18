//! Memory subsystem for the Halldyll agent.
//!
//! This module provides a complete memory system for stateless LLMs, organized into:
//! - `core`: Configuration, errors, IDs, kinds, items, and metadata
//! - `ingest`: Transcript events, stores, extractors, deduplication, and pruning
//! - `embedding`: Embedding model abstraction and Ollama implementation
//! - `storage`: Vector and summary stores with SQLite backends
//! - `retrieval`: Query building, ranking, and search helpers
//! - `prompt`: Budget enforcement and prompt block construction
//! - `summarization`: LLM-based intelligent summarization
//! - `maintenance`: Background cleanup and garbage collection
//! - `engine`: Main orchestration of the memory system
//! - `adapters`: Integration adapters (e.g., Rig)

pub mod adapters;
pub mod core;
pub mod embedding;
pub mod engine;
pub mod ingest;
pub mod maintenance;
pub mod prompt;
pub mod retrieval;
pub mod storage;
pub mod summarization;

// Re-export commonly used types for convenience
pub use adapters::{init_tracing, run_with_memory};
pub use core::{
    EmbeddingConfig, ExtractorConfig, ExtractorMode, LlmConfig, MemoryConfig, MemoryError,
    MemoryId, MemoryItem, MemoryKind, MemoryMetadata, MemoryResult, MemorySource, PromptConfig,
    RetentionConfig, RetrievalConfig, ScoringConfig, SessionId, ShortTermConfig, StorageConfig,
    SummaryConfig, TurnId, UserId,
};
pub use embedding::{EmbedFuture, Embedder, OllamaEmbedder};
pub use engine::{MemoryBackends, MemoryEngine, PreparedContext};
pub use ingest::{
    HeuristicExtractor, LlmExtractor, SqliteTranscriptStore, TranscriptEvent, TranscriptRole,
    TranscriptStore,
};
pub use prompt::{PromptParts, build_prompt_block, enforce_budget};
pub use retrieval::{
    RankedMemory, build_query_embedding, build_query_text, fetch_top_k_raw, rank_results,
};
pub use storage::{
    SqliteSummaryStore, SqliteVectorMemoryStore, SummaryRecord, SummaryStore, VectorMemoryStore,
    VectorSearchResult, init_sqlite_vec_extension,
};
pub use maintenance::{BackgroundCleanup, CleanupConfig, CleanupStats};
