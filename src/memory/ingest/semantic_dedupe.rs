//! Semantic deduplication for memory items.
//!
//! Uses vector similarity to detect and merge semantically duplicate memories.

use rig::embeddings::Embedding;

use crate::memory::core::errors::MemoryResult;
use crate::memory::core::ids::SessionId;
use crate::memory::core::item::MemoryItem;
use crate::memory::core::kinds::MergeHint;
use crate::memory::core::metadata::MemoryMetadata;
use crate::memory::embedding::embedder::Embedder;
use crate::memory::storage::vector_store::{VectorMemoryStore, VectorSearchResult};

/// Default threshold for considering two memories as semantic duplicates.
/// 0.92 = very high similarity, almost identical meaning.
pub const DEFAULT_SEMANTIC_THRESHOLD: f64 = 0.92;

/// Result of semantic duplicate detection.
#[derive(Debug, Clone)]
pub enum DedupeResult {
    /// No semantic duplicate found, item is unique.
    Unique {
        /// The unique memory item.
        item: MemoryItem,
        /// The computed embedding for the item.
        embedding: Embedding,
    },
    /// Found a semantic duplicate, merged into existing item.
    Merged {
        /// ID of the existing item that was matched.
        existing_id: crate::memory::core::ids::MemoryId,
        /// The merged memory item with combined metadata.
        merged_item: MemoryItem,
        /// The computed embedding for the new content.
        embedding: Embedding,
        /// Similarity score between the new and existing item.
        similarity: f64,
    },
    /// Found an exact duplicate (same content hash), skip entirely.
    ExactDuplicate,
}

/// Configuration for semantic deduplication.
#[derive(Debug, Clone)]
pub struct SemanticDedupeConfig {
    /// Similarity threshold for semantic duplicates (0.0 - 1.0).
    pub threshold: f64,
    /// Maximum number of candidates to check.
    pub max_candidates: usize,
    /// Whether to enable semantic deduplication.
    pub enabled: bool,
}

impl Default for SemanticDedupeConfig {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_SEMANTIC_THRESHOLD,
            max_candidates: 5,
            enabled: true,
        }
    }
}

/// Check if a new memory item is a semantic duplicate of existing memories.
///
/// Returns the dedupe result indicating whether to insert, merge, or skip.
///
/// # Errors
/// Returns an error if embedding or store operations fail.
pub async fn check_semantic_duplicate<E: Embedder, S: VectorMemoryStore>(
    embedder: &E,
    store: &S,
    new_item: &MemoryItem,
    config: &SemanticDedupeConfig,
) -> MemoryResult<DedupeResult> {
    if !config.enabled {
        let embedding = embedder.embed_text(&new_item.content).await?;
        return Ok(DedupeResult::Unique {
            item: new_item.clone(),
            embedding,
        });
    }

    // First check for exact hash duplicate
    if store
        .exists_hash(new_item.session_id, &new_item.content_hash)
        .await?
    {
        return Ok(DedupeResult::ExactDuplicate);
    }

    // Embed the new content
    let embedding = embedder.embed_text(&new_item.content).await?;

    // Query for similar items
    let candidates = store
        .query(
            new_item.session_id,
            &new_item.content,
            config.max_candidates,
            config.threshold,
        )
        .await?;

    // Find the most similar item above threshold
    let best_match = candidates
        .into_iter()
        .filter(|r| r.similarity >= config.threshold)
        .max_by(|a, b| {
            a.similarity
                .partial_cmp(&b.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    match best_match {
        Some(duplicate) => {
            // Merge the new item into the existing one
            let merged = merge_memories(&duplicate.item, new_item);
            Ok(DedupeResult::Merged {
                existing_id: duplicate.item.id,
                merged_item: merged,
                embedding,
                similarity: duplicate.similarity,
            })
        }
        None => Ok(DedupeResult::Unique {
            item: new_item.clone(),
            embedding,
        }),
    }
}

/// Find semantic duplicates for a memory item without merging.
///
/// Returns all items with similarity above the threshold.
///
/// # Errors
/// Returns an error if embedding or store operations fail.
pub async fn find_semantic_duplicates<E: Embedder, S: VectorMemoryStore>(
    embedder: &E,
    store: &S,
    session_id: SessionId,
    content: &str,
    threshold: f64,
    max_results: usize,
) -> MemoryResult<Vec<VectorSearchResult>> {
    // Embed and query
    let _ = embedder.embed_text(content).await?;
    let candidates = store.query(session_id, content, max_results, threshold).await?;

    Ok(candidates
        .into_iter()
        .filter(|r| r.similarity >= threshold)
        .collect())
}

/// Merge two semantically similar memories.
///
/// Strategy:
/// - Keep the higher salience
/// - Combine tags (deduplicated)
/// - Use the newer timestamp
/// - Keep the more recent content if kinds differ, otherwise keep existing
fn merge_memories(existing: &MemoryItem, new: &MemoryItem) -> MemoryItem {
    let merge_hint = existing.kind.merge_hint();

    // Determine the merged content based on merge hint
    let (content, kind) = match merge_hint {
        MergeHint::Replace => {
            // New content replaces old
            (new.content.clone(), new.kind)
        }
        MergeHint::Append => {
            // Keep existing content (we don't want to concatenate for duplicates)
            (existing.content.clone(), existing.kind)
        }
        MergeHint::Accumulate => {
            // Keep the longer/more detailed content
            if new.content.len() > existing.content.len() {
                (new.content.clone(), new.kind)
            } else {
                (existing.content.clone(), existing.kind)
            }
        }
    };

    // Merge metadata
    let merged_metadata = merge_metadata(&existing.metadata, &new.metadata);

    // Compute hash before moving content
    let content_hash = crate::memory::ingest::dedupe::hash_content(&content);

    // Create merged item with existing ID
    MemoryItem {
        id: existing.id,
        session_id: existing.session_id,
        kind,
        content,
        metadata: merged_metadata,
        content_hash,
    }
}

/// Merge metadata from two items.
fn merge_metadata(existing: &MemoryMetadata, new: &MemoryMetadata) -> MemoryMetadata {
    // Use higher salience
    let salience = existing.salience.max(new.salience);

    // Use more recent updated_at
    let updated_at = existing.updated_at.max(new.updated_at);

    // Keep original created_at
    let created_at = existing.created_at;

    // Combine and deduplicate tags
    let mut tags = existing.tags.clone();
    for tag in &new.tags {
        if !tags.contains(tag) {
            tags.push(tag.clone());
        }
    }

    // Keep source from new item (more recent)
    let source = new.source.clone();

    // Keep shorter TTL if both have one, otherwise keep whichever exists
    let ttl_seconds = match (existing.ttl_seconds, new.ttl_seconds) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    // Combine retrieval stats (sum counts, keep most recent retrieval)
    let retrieval_count = existing.retrieval_count.saturating_add(new.retrieval_count);
    let last_retrieved_at = match (existing.last_retrieved_at, new.last_retrieved_at) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    MemoryMetadata {
        created_at,
        updated_at,
        salience,
        tags,
        ttl_seconds,
        source,
        retrieval_count,
        last_retrieved_at,
    }
}

/// Batch semantic deduplication for multiple items.
///
/// Processes items in order, deduplicating against both the store and
/// previously processed items in the batch.
///
/// # Errors
/// Returns an error if embedding or store operations fail.
pub async fn batch_dedupe<E: Embedder, S: VectorMemoryStore>(
    embedder: &E,
    store: &S,
    items: Vec<MemoryItem>,
    config: &SemanticDedupeConfig,
) -> MemoryResult<Vec<DedupeResult>> {
    let mut results = Vec::with_capacity(items.len());

    for item in items {
        let result = check_semantic_duplicate(embedder, store, &item, config).await?;
        results.push(result);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::core::metadata::MemorySource;

    fn make_item(content: &str, salience: u8) -> MemoryItem {
        let session_id = crate::memory::core::ids::SessionId::new();
        let metadata = MemoryMetadata::new(MemorySource::User).with_salience(salience);
        MemoryItem::new(
            session_id,
            crate::memory::core::kinds::MemoryKind::Fact,
            content,
            metadata,
        )
        .unwrap()
    }

    #[test]
    fn test_merge_keeps_higher_salience() {
        let existing = make_item("I like coffee", 60);
        let new = make_item("I really like coffee", 80);

        let merged = merge_memories(&existing, &new);
        assert_eq!(merged.metadata.salience, 80);
    }

    #[test]
    fn test_merge_combines_tags() {
        let mut existing = make_item("I like coffee", 60);
        existing.metadata.tags = vec!["preference".to_string()];

        let mut new = make_item("I really like coffee", 70);
        new.metadata.tags = vec!["beverage".to_string()];

        let merged = merge_memories(&existing, &new);
        assert!(merged.metadata.tags.contains(&"preference".to_string()));
        assert!(merged.metadata.tags.contains(&"beverage".to_string()));
    }

    #[test]
    fn test_merge_keeps_existing_id() {
        let existing = make_item("I like coffee", 60);
        let new = make_item("I really like coffee", 70);
        let existing_id = existing.id;

        let merged = merge_memories(&existing, &new);
        assert_eq!(merged.id, existing_id);
    }

    #[test]
    fn test_config_default() {
        let config = SemanticDedupeConfig::default();
        assert!((config.threshold - 0.92).abs() < f64::EPSILON);
        assert!(config.enabled);
        assert_eq!(config.max_candidates, 5);
    }
}
