//! Memory item model with validation helpers.

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::core::ids::{MemoryId, SessionId};
use crate::memory::core::kinds::MemoryKind;
use crate::memory::core::metadata::MemoryMetadata;
use crate::memory::ingest::dedupe;

/// A persisted memory item with metadata and a stable hash.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique memory identifier.
    pub id: MemoryId,
    /// Session the memory belongs to.
    pub session_id: SessionId,
    /// Semantic category.
    pub kind: MemoryKind,
    /// Memory content.
    pub content: String,
    /// Metadata (timestamps, salience, tags).
    pub metadata: MemoryMetadata,
    /// Hash of normalized content for dedupe.
    pub content_hash: String,
}

impl MemoryItem {
    /// Create a new memory item with a computed hash.
    ///
    /// # Errors
    /// Returns an error if content is empty after trimming.
    pub fn new(
        session_id: SessionId,
        kind: MemoryKind,
        content: impl Into<String>,
        metadata: MemoryMetadata,
    ) -> MemoryResult<Self> {
        let content = content.into();
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(MemoryError::InvalidMemoryItem(
                "content is empty".to_string(),
            ));
        }

        let content_hash = dedupe::hash_content(trimmed);
        Ok(Self {
            id: MemoryId::new(),
            session_id,
            kind,
            content: trimmed.to_string(),
            metadata,
            content_hash,
        })
    }

    /// Normalize content for hashing.
    #[must_use]
    pub fn normalize_for_hash(content: &str) -> String {
        dedupe::normalize_text(content)
    }

    /// Ensure the content does not exceed the given character budget.
    #[must_use]
    pub fn truncate_to_budget(mut self, max_chars: usize) -> Self {
        if self.content.chars().count() > max_chars {
            let truncated: String = self.content.chars().take(max_chars).collect();
            self.content = truncated.trim_end().to_string();
        }

        self.content_hash = dedupe::hash_content(&self.content);
        self
    }

    /// Validate the memory item content and metadata.
    ///
    /// # Errors
    /// Returns an error if the content is empty, too long, or appears sensitive.
    pub fn validate(&self, max_chars: usize) -> MemoryResult<()> {
        if self.content.trim().is_empty() {
            return Err(MemoryError::InvalidMemoryItem(
                "content is empty".to_string(),
            ));
        }

        if self.content.chars().count() > max_chars {
            return Err(MemoryError::InvalidMemoryItem(format!(
                "content exceeds max chars ({max_chars})"
            )));
        }

        if self.metadata.salience > 100 {
            return Err(MemoryError::InvalidMemoryItem(
                "salience must be in 0..=100".to_string(),
            ));
        }

        if contains_sensitive(&self.content)? {
            return Err(MemoryError::InvalidMemoryItem(
                "content looks like a secret".to_string(),
            ));
        }

        Ok(())
    }
}

fn contains_sensitive(text: &str) -> MemoryResult<bool> {
    let pattern = r"(?i)(api[_-]?key|secret|password|token|bearer\s+[a-z0-9\-_]+|sk-[a-z0-9]{10,})";
    let regex = Regex::new(pattern)
        .map_err(|err| MemoryError::InvalidConfig(format!("invalid regex: {err}")))?;
    Ok(regex.is_match(text))
}
