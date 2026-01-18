//! Core memory types and identifiers.

pub mod config;
pub mod errors;
pub mod ids;
pub mod item;
pub mod kinds;
pub mod metadata;
pub mod user_profile;

pub use config::{
    EmbeddingConfig, ExtractorConfig, ExtractorMode, LlmConfig, MemoryConfig, PromptConfig,
    RetentionConfig, RetrievalConfig, ScoringConfig, ShortTermConfig, StorageConfig, SummaryConfig,
};
pub use errors::{MemoryError, MemoryResult};
pub use ids::{MemoryId, SessionId, TurnId, UserId};
pub use item::MemoryItem;
pub use kinds::{MemoryKind, MemoryKindParseError};
pub use metadata::{MemoryMetadata, MemorySource, Modality};
pub use user_profile::{ProfileMemory, UserProfile, PROMOTABLE_KINDS};
