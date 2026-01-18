//! Configuration for the memory subsystem.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::core::kinds::MemoryKind;

/// Top-level configuration for the memory engine.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Short-term memory settings.
    pub short_term: ShortTermConfig,
    /// Summary settings.
    pub summary: SummaryConfig,
    /// Retrieval settings.
    pub retrieval: RetrievalConfig,
    /// Ranking settings.
    pub scoring: ScoringConfig,
    /// Extraction settings.
    pub extractor: ExtractorConfig,
    /// Storage settings.
    pub storage: StorageConfig,
    /// Embedding model settings.
    pub embedding: EmbeddingConfig,
    /// Completion model settings.
    pub llm: LlmConfig,
    /// Prompt construction settings.
    pub prompt: PromptConfig,
    /// Retention and TTL settings.
    pub retention: RetentionConfig,
}

impl MemoryConfig {
    /// Validate configuration invariants.
    ///
    /// # Errors
    /// Returns an error if any values are out of range or invalid.
    pub fn validate(&self) -> MemoryResult<()> {
        if self.short_term.window == 0 {
            return Err(MemoryError::InvalidConfig(
                "short_term.window must be > 0".to_string(),
            ));
        }

        if self.short_term.cache_capacity == 0 {
            return Err(MemoryError::InvalidConfig(
                "short_term.cache_capacity must be > 0".to_string(),
            ));
        }

        if self.summary.interval_turns == 0 {
            return Err(MemoryError::InvalidConfig(
                "summary.interval_turns must be > 0".to_string(),
            ));
        }

        if self.retrieval.top_k == 0 {
            return Err(MemoryError::InvalidConfig(
                "retrieval.top_k must be > 0".to_string(),
            ));
        }

        if self.prompt.max_chars == 0 {
            return Err(MemoryError::InvalidConfig(
                "prompt.max_chars must be > 0".to_string(),
            ));
        }

        if self.embedding.ndims == 0 {
            return Err(MemoryError::InvalidConfig(
                "embedding.ndims must be > 0".to_string(),
            ));
        }

        for (kind, ttl) in &self.retention.ttl_seconds_by_kind {
            if *ttl == 0 {
                return Err(MemoryError::InvalidConfig(format!(
                    "ttl_seconds_by_kind for {kind} must be > 0"
                )));
            }
        }

        if let Some(base_url) = &self.embedding.base_url {
            Url::parse(base_url)?;
        }

        if let Some(base_url) = &self.llm.base_url {
            Url::parse(base_url)?;
        }

        Ok(())
    }
}

/// Short-term memory settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShortTermConfig {
    /// Number of recent turns to include.
    pub window: usize,
    /// LRU capacity for session caches.
    pub cache_capacity: usize,
}

impl Default for ShortTermConfig {
    fn default() -> Self {
        Self {
            window: 6,
            cache_capacity: 256,
        }
    }
}

/// Summary settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SummaryConfig {
    /// Number of turns between summary updates.
    pub interval_turns: u64,
    /// Max summary size in characters.
    pub max_chars: usize,
    /// Whether to use LLM for intelligent summarization.
    pub use_llm: bool,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        Self {
            interval_turns: 8,
            max_chars: 1200,
            use_llm: false,
        }
    }
}

/// Retrieval settings for long-term memory.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Number of memories to retrieve.
    pub top_k: usize,
    /// Minimum similarity threshold to keep.
    pub min_similarity: f64,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            top_k: 6,
            min_similarity: 0.2,
        }
    }
}

/// Ranking coefficients for retrieval results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Recency weight.
    pub alpha_recency: f64,
    /// Salience weight.
    pub beta_salience: f64,
    /// Half-life in seconds for recency decay.
    pub recency_half_life_seconds: u64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            alpha_recency: 0.15,
            beta_salience: 0.35,
            recency_half_life_seconds: 60 * 60 * 24 * 7,
        }
    }
}

/// Extraction mode selector.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractorMode {
    /// Heuristic-only extraction.
    Heuristic,
    /// LLM-assisted extraction.
    Llm,
}

/// Extraction settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractorConfig {
    /// Which extraction mode to use.
    pub mode: ExtractorMode,
    /// Use the LLM every N turns.
    pub llm_every_n_turns: u64,
    /// Max LLM-proposed items per call.
    pub llm_max_items: usize,
    /// Minimum content length to consider.
    pub min_content_chars: usize,
}

impl Default for ExtractorConfig {
    fn default() -> Self {
        Self {
            mode: ExtractorMode::Heuristic,
            llm_every_n_turns: 6,
            llm_max_items: 6,
            min_content_chars: 10,
        }
    }
}

/// Storage configuration for memory data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StorageConfig {
    /// `SQLite` database path.
    pub sqlite_path: PathBuf,
    /// Transcript table name.
    pub transcript_table: String,
    /// Summary table name.
    pub summary_table: String,
    /// Vector memory table name.
    pub memory_table: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            sqlite_path: PathBuf::from("memory.sqlite"),
            transcript_table: "memory_transcript".to_string(),
            summary_table: "memory_summary".to_string(),
            memory_table: "memory_items".to_string(),
        }
    }
}

/// Embedding model settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Ollama embedding model name.
    pub model: String,
    /// Embedding vector dimensions.
    pub ndims: usize,
    /// Optional custom base URL.
    pub base_url: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: "nomic-embed-text".to_string(),
            ndims: 768,
            base_url: None,
        }
    }
}

/// Completion model settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Ollama completion model name.
    pub model: String,
    /// Temperature for generation.
    pub temperature: f64,
    /// Optional max tokens.
    pub max_tokens: Option<u64>,
    /// Optional custom base URL.
    pub base_url: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "ministral-3:8b-instruct-2512-q8_0".to_string(),
            temperature: 0.4,
            max_tokens: None,
            base_url: None,
        }
    }
}

/// Prompt construction settings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PromptConfig {
    /// Maximum prompt size in characters.
    pub max_chars: usize,
    /// Maximum memory item size in characters.
    pub max_memory_chars: usize,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            max_chars: 3600,
            max_memory_chars: 1200,
        }
    }
}

/// Retention settings by memory kind.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Optional TTL overrides by memory kind.
    pub ttl_seconds_by_kind: HashMap<MemoryKind, u64>,
}
