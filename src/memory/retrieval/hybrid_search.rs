//! Hybrid search combining BM25 keyword matching with vector similarity.
//!
//! Uses Reciprocal Rank Fusion (RRF) to combine results from both search methods.

use std::collections::HashMap;

use crate::memory::core::errors::MemoryResult;
use crate::memory::core::ids::{MemoryId, SessionId};
use crate::memory::embedding::embedder::Embedder;
use crate::memory::retrieval::ranking::RankedMemory;
use crate::memory::storage::vector_store::{VectorMemoryStore, VectorSearchResult};

/// Configuration for hybrid search.
#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    /// Weight for BM25/keyword results (0.0 - 1.0).
    pub bm25_weight: f32,
    /// Weight for vector/semantic results (0.0 - 1.0).
    pub vector_weight: f32,
    /// RRF constant k (typically 60).
    pub rrf_k: f32,
    /// Whether hybrid search is enabled.
    pub enabled: bool,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            bm25_weight: 0.3,
            vector_weight: 0.7,
            rrf_k: 60.0,
            enabled: true,
        }
    }
}

impl HybridSearchConfig {
    /// Create a config with equal weighting.
    #[must_use]
    pub const fn balanced() -> Self {
        Self {
            bm25_weight: 0.5,
            vector_weight: 0.5,
            rrf_k: 60.0,
            enabled: true,
        }
    }

    /// Create a config favoring vector search.
    #[must_use]
    pub const fn vector_heavy() -> Self {
        Self {
            bm25_weight: 0.2,
            vector_weight: 0.8,
            rrf_k: 60.0,
            enabled: true,
        }
    }

    /// Create a config favoring keyword search.
    #[must_use]
    pub const fn keyword_heavy() -> Self {
        Self {
            bm25_weight: 0.7,
            vector_weight: 0.3,
            rrf_k: 60.0,
            enabled: true,
        }
    }
}

/// Result from hybrid search with combined score.
#[derive(Debug, Clone)]
pub struct HybridSearchResult {
    /// The memory ID.
    pub memory_id: MemoryId,
    /// Combined score from RRF.
    pub combined_score: f64,
    /// Original vector similarity score (if found in vector results).
    pub vector_score: Option<f64>,
    /// BM25 score (if found in keyword results).
    pub bm25_score: Option<f64>,
    /// The ranked memory item.
    pub memory: RankedMemory,
}

/// Hybrid searcher combining keyword and vector search.
pub struct HybridSearcher<S> {
    store: S,
    config: HybridSearchConfig,
}

impl<S: VectorMemoryStore> HybridSearcher<S> {
    /// Create a new hybrid searcher.
    #[must_use]
    pub const fn new(store: S, config: HybridSearchConfig) -> Self {
        Self { store, config }
    }

    /// Perform a hybrid search combining vector and keyword results.
    ///
    /// # Errors
    /// Returns an error if the vector search fails.
    pub async fn search<E: Embedder>(
        &self,
        _embedder: &E,
        session_id: SessionId,
        query: &str,
        top_k: usize,
        min_similarity: f64,
    ) -> MemoryResult<Vec<HybridSearchResult>> {
        if !self.config.enabled {
            // Fall back to pure vector search
            return self
                .vector_only_search(session_id, query, top_k, min_similarity)
                .await;
        }

        // Get vector search results
        let vector_results = self
            .store
            .query(session_id, query, top_k * 2, min_similarity)
            .await?;

        // Get keyword search results (using vector store's FTS if available)
        // Note: For full BM25, the VectorMemoryStore trait should be extended
        // with a keyword_search method. For now, we simulate with vector results.
        let keyword_results = self.keyword_search(session_id, query, top_k * 2).await?;

        // Combine using RRF
        let combined = self.rrf_fusion(&vector_results, &keyword_results, top_k);

        Ok(combined)
    }

    /// Vector-only search fallback.
    async fn vector_only_search(
        &self,
        session_id: SessionId,
        query: &str,
        top_k: usize,
        min_similarity: f64,
    ) -> MemoryResult<Vec<HybridSearchResult>> {
        let results = self
            .store
            .query(session_id, query, top_k, min_similarity)
            .await?;

        Ok(results
            .into_iter()
            .map(|r| HybridSearchResult {
                memory_id: r.item.id,
                combined_score: r.similarity,
                vector_score: Some(r.similarity),
                bm25_score: None,
                memory: RankedMemory {
                    score: r.similarity,
                    similarity: r.similarity,
                    recency_score: 1.0,
                    salience_score: f64::from(r.item.metadata.salience) / 100.0,
                    item: r.item,
                },
            })
            .collect())
    }

    /// Keyword search using content matching.
    ///
    /// Note: This is a placeholder. In production, this should use `SQLite` FTS5
    /// with proper BM25 scoring. Currently returns empty results.
    #[allow(clippy::unused_async)]
    async fn keyword_search(
        &self,
        _session_id: SessionId,
        _query: &str,
        _top_k: usize,
    ) -> MemoryResult<Vec<KeywordResult>> {
        // Placeholder: Return empty results until FTS is implemented
        // In production, this would:
        // 1. Query the FTS5 virtual table
        // 2. Use bm25() function for scoring
        // 3. Return ranked keyword matches
        Ok(Vec::new())
    }

    /// Combine results using Reciprocal Rank Fusion.
    ///
    /// RRF score = sum(1 / (k + rank)) for each result list
    #[allow(clippy::cast_precision_loss)] // Rank values are small, precision loss is acceptable
    fn rrf_fusion(
        &self,
        vector_results: &[VectorSearchResult],
        keyword_results: &[KeywordResult],
        top_k: usize,
    ) -> Vec<HybridSearchResult> {
        let k = self.config.rrf_k;
        let mut scores: HashMap<MemoryId, RrfAccumulator> = HashMap::new();

        // Add vector results with their ranks
        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = self.config.vector_weight / (k + (rank + 1) as f32);
            let salience_score = f64::from(result.item.metadata.salience) / 100.0;
            let entry = scores.entry(result.item.id).or_insert_with(|| RrfAccumulator {
                combined_score: 0.0,
                vector_score: None,
                bm25_score: None,
                memory: RankedMemory {
                    score: result.similarity,
                    similarity: result.similarity,
                    recency_score: 1.0,
                    salience_score,
                    item: result.item.clone(),
                },
            });
            entry.combined_score += f64::from(rrf_score);
            entry.vector_score = Some(result.similarity);
        }

        // Add keyword results with their ranks
        for (rank, result) in keyword_results.iter().enumerate() {
            let rrf_score = self.config.bm25_weight / (k + (rank + 1) as f32);
            if let Some(entry) = scores.get_mut(&result.memory_id) {
                entry.combined_score += f64::from(rrf_score);
                entry.bm25_score = Some(result.bm25_score);
            }
            // Note: If the keyword result isn't in vector results, we skip it
            // because we don't have the full memory item. This could be enhanced.
        }

        // Convert to results and sort by combined score
        let mut results: Vec<HybridSearchResult> = scores
            .into_iter()
            .map(|(id, acc)| HybridSearchResult {
                memory_id: id,
                combined_score: acc.combined_score,
                vector_score: acc.vector_score,
                bm25_score: acc.bm25_score,
                memory: acc.memory,
            })
            .collect();

        results.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(top_k);
        results
    }
}

/// Accumulator for RRF scoring.
struct RrfAccumulator {
    combined_score: f64,
    vector_score: Option<f64>,
    bm25_score: Option<f64>,
    memory: RankedMemory,
}

/// Placeholder for keyword search results.
#[derive(Debug, Clone)]
struct KeywordResult {
    /// Memory ID.
    memory_id: MemoryId,
    /// BM25 score.
    bm25_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HybridSearchConfig::default();
        assert!((config.bm25_weight - 0.3).abs() < f32::EPSILON);
        assert!((config.vector_weight - 0.7).abs() < f32::EPSILON);
        assert!((config.rrf_k - 60.0).abs() < f32::EPSILON);
        assert!(config.enabled);
    }

    #[test]
    fn test_config_balanced() {
        let config = HybridSearchConfig::balanced();
        assert!((config.bm25_weight - 0.5).abs() < f32::EPSILON);
        assert!((config.vector_weight - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_config_vector_heavy() {
        let config = HybridSearchConfig::vector_heavy();
        assert!((config.bm25_weight - 0.2).abs() < f32::EPSILON);
        assert!((config.vector_weight - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_config_keyword_heavy() {
        let config = HybridSearchConfig::keyword_heavy();
        assert!((config.bm25_weight - 0.7).abs() < f32::EPSILON);
        assert!((config.vector_weight - 0.3).abs() < f32::EPSILON);
    }
}
