//! Pruning and deduplication utilities.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::memory::core::ids::MemoryId;
use crate::memory::core::item::MemoryItem;

/// Apply TTL filtering and return kept items plus expired ids.
#[must_use]
pub fn apply_ttl(items: Vec<MemoryItem>, now: DateTime<Utc>) -> (Vec<MemoryItem>, Vec<MemoryId>) {
    let mut kept = Vec::new();
    let mut expired = Vec::new();

    for item in items {
        if let Some(ttl) = item.metadata.ttl_seconds {
            let age = now
                .signed_duration_since(item.metadata.created_at)
                .num_seconds();
            if let Ok(age_seconds) = u64::try_from(age)
                && age_seconds >= ttl
            {
                expired.push(item.id);
                continue;
            }
        }
        kept.push(item);
    }

    (kept, expired)
}

/// Keep at most `max_items` items, preferring most recent updates.
#[must_use]
pub fn prune_by_count(mut items: Vec<MemoryItem>, max_items: usize) -> Vec<MemoryItem> {
    if items.len() <= max_items {
        return items;
    }

    items.sort_by(|a, b| b.metadata.updated_at.cmp(&a.metadata.updated_at));
    items.truncate(max_items);
    items
}

/// Merge duplicates by content hash, keeping the most salient item.
#[must_use]
pub fn merge_duplicates(items: Vec<MemoryItem>) -> Vec<MemoryItem> {
    let mut map: HashMap<String, MemoryItem> = HashMap::new();

    for item in items {
        match map.get(&item.content_hash) {
            Some(existing) => {
                let keep_new = item.metadata.salience >= existing.metadata.salience
                    && item.metadata.updated_at >= existing.metadata.updated_at;
                if keep_new {
                    map.insert(item.content_hash.clone(), item);
                }
            }
            None => {
                map.insert(item.content_hash.clone(), item);
            }
        }
    }

    map.into_values().collect()
}
