//! Ranking utilities for retrieved memory items.

use chrono::{DateTime, Utc};

use crate::memory::core::config::ScoringConfig;
use crate::memory::core::item::MemoryItem;
use crate::memory::storage::vector_store::VectorSearchResult;

/// Ranked memory item with score breakdown.
#[derive(Clone, Debug)]
pub struct RankedMemory {
    /// Final combined score.
    pub score: f64,
    /// Vector similarity score.
    pub similarity: f64,
    /// Recency score component.
    pub recency_score: f64,
    /// Salience score component.
    pub salience_score: f64,
    /// Memory item.
    pub item: MemoryItem,
}

/// Compute ranked memories from raw vector search results.
#[must_use]
pub fn rank_results(
    raw: Vec<VectorSearchResult>,
    config: &ScoringConfig,
    now: DateTime<Utc>,
) -> Vec<RankedMemory> {
    let half_life = u32::try_from(config.recency_half_life_seconds).unwrap_or(u32::MAX);
    let half_life = f64::from(half_life.max(1));
    let mut ranked = Vec::with_capacity(raw.len());

    for result in raw {
        let age_seconds = now
            .signed_duration_since(result.item.metadata.updated_at)
            .num_seconds()
            .max(0);
        let age_seconds = u32::try_from(age_seconds).unwrap_or(u32::MAX);
        let age_seconds = f64::from(age_seconds);
        let recency_score = 1.0 / (1.0 + age_seconds / half_life);
        let salience_score = f64::from(result.item.metadata.salience) / 100.0;
        let score = config.beta_salience.mul_add(
            salience_score,
            config
                .alpha_recency
                .mul_add(recency_score, result.similarity),
        );

        ranked.push(RankedMemory {
            score,
            similarity: result.similarity,
            recency_score,
            salience_score,
            item: result.item,
        });
    }

    ranked.sort_by(|a, b| b.score.total_cmp(&a.score));
    ranked
}
