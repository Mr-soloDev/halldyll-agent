//! Content normalization and deduplication helpers.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Normalize content for hashing (trim, lowercase, collapse whitespace).
#[must_use]
pub fn normalize_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut prev_space = false;

    for ch in text.trim().chars() {
        let is_space = ch.is_whitespace();
        if is_space {
            if !prev_space {
                normalized.push(' ');
                prev_space = true;
            }
        } else {
            for lower in ch.to_lowercase() {
                normalized.push(lower);
            }
            prev_space = false;
        }
    }

    normalized
}

/// Compute a stable hash for normalized content.
#[must_use]
pub fn compute_hash(normalized: &str) -> String {
    let mut hasher = DefaultHasher::new();
    normalized.hash(&mut hasher);
    let value = hasher.finish();
    format!("{value:016x}")
}

/// Compute a hash directly from raw content.
#[must_use]
pub fn hash_content(text: &str) -> String {
    compute_hash(&normalize_text(text))
}
