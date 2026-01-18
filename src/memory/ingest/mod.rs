//! Ingestion and extraction modules for memory processing.

pub mod dedupe;
pub mod entity_extractor;
pub mod extractor_heuristic;
pub mod extractor_llm;
pub mod pruning;
pub mod semantic_dedupe;
pub mod transcript_event;
pub mod transcript_store;

pub use dedupe::{compute_hash, hash_content, normalize_text};
pub use entity_extractor::{EntityExtractor, EntityType, ExtractedEntity};
pub use extractor_heuristic::HeuristicExtractor;
pub use extractor_llm::LlmExtractor;
pub use pruning::{apply_ttl, merge_duplicates, prune_by_count};
pub use semantic_dedupe::{
    batch_dedupe, check_semantic_duplicate, find_semantic_duplicates, DedupeResult,
    SemanticDedupeConfig, DEFAULT_SEMANTIC_THRESHOLD,
};
pub use transcript_event::{TranscriptEvent, TranscriptRole};
pub use transcript_store::{SqliteTranscriptStore, StoreFuture, TranscriptStore};
