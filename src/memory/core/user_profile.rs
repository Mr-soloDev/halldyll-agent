//! User profile for cross-session memory persistence.
//!
//! Stores stable user information that persists across all sessions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::memory::core::ids::{SessionId, UserId};
use crate::memory::core::item::MemoryItem;
use crate::memory::core::kinds::MemoryKind;

/// Kinds of memories that should be promoted to user profile.
pub const PROMOTABLE_KINDS: &[MemoryKind] = &[
    MemoryKind::Identity,
    MemoryKind::Preference,
    MemoryKind::Aversion,
    MemoryKind::Constraint,
    MemoryKind::Policy,
];

/// User profile containing persistent cross-session information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserProfile {
    /// Unique user identifier.
    pub user_id: UserId,
    /// When the profile was created.
    pub created_at: DateTime<Utc>,
    /// When the profile was last updated.
    pub updated_at: DateTime<Utc>,
    /// Total number of sessions.
    pub total_sessions: u32,
    /// Total number of turns across all sessions.
    pub total_turns: u32,
    /// ID of the most recent session.
    pub last_session_id: Option<SessionId>,
    /// User's identity information (name, age, location, etc.).
    pub identity: Vec<ProfileMemory>,
    /// User's preferences (likes, favorites).
    pub preferences: Vec<ProfileMemory>,
    /// User's aversions (dislikes, allergies).
    pub aversions: Vec<ProfileMemory>,
    /// User's constraints (rules to follow).
    pub constraints: Vec<ProfileMemory>,
    /// Operating policies.
    pub policies: Vec<ProfileMemory>,
}

/// A memory item stored in the user profile.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileMemory {
    /// Content of the memory.
    pub content: String,
    /// Confidence/salience score (0-100).
    pub salience: u8,
    /// When this was first learned.
    pub learned_at: DateTime<Utc>,
    /// When this was last confirmed.
    pub confirmed_at: DateTime<Utc>,
    /// Number of times this was seen.
    pub seen_count: u32,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Source session ID.
    pub source_session: Option<SessionId>,
}

impl ProfileMemory {
    /// Create a new profile memory from a memory item.
    #[must_use]
    pub fn from_memory_item(item: &MemoryItem) -> Self {
        Self {
            content: item.content.clone(),
            salience: item.metadata.salience,
            learned_at: item.metadata.created_at,
            confirmed_at: item.metadata.updated_at,
            seen_count: 1,
            tags: item.metadata.tags.clone(),
            source_session: Some(item.session_id),
        }
    }

    /// Merge with another profile memory (update counts and confidence).
    pub fn merge_with(&mut self, other: &Self) {
        self.seen_count += other.seen_count;
        self.confirmed_at = self.confirmed_at.max(other.confirmed_at);
        // Increase salience with repeated confirmation (max 100)
        self.salience = (u16::from(self.salience) + 5).min(100) as u8;
        // Merge tags
        for tag in &other.tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }
    }
}

impl UserProfile {
    /// Create a new empty user profile.
    #[must_use]
    pub fn new(user_id: UserId) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            created_at: now,
            updated_at: now,
            total_sessions: 0,
            total_turns: 0,
            last_session_id: None,
            identity: Vec::new(),
            preferences: Vec::new(),
            aversions: Vec::new(),
            constraints: Vec::new(),
            policies: Vec::new(),
        }
    }

    /// Check if a memory kind should be promoted to user profile.
    #[must_use]
    pub fn should_promote(kind: MemoryKind) -> bool {
        PROMOTABLE_KINDS.contains(&kind)
    }

    /// Add or update a memory in the profile.
    pub fn add_memory(&mut self, item: &MemoryItem) {
        let new_mem = ProfileMemory::from_memory_item(item);
        let collection = self.collection_for_kind_mut(item.kind);

        // Check for similar content
        if let Some(existing) = collection
            .iter_mut()
            .find(|m| content_similarity(&m.content, &new_mem.content) > 0.85)
        {
            existing.merge_with(&new_mem);
        } else {
            collection.push(new_mem);
        }

        self.updated_at = Utc::now();
    }

    /// Get memories for a specific kind.
    #[must_use]
    pub fn get_memories(&self, kind: MemoryKind) -> &[ProfileMemory] {
        match kind {
            MemoryKind::Identity => &self.identity,
            MemoryKind::Preference => &self.preferences,
            MemoryKind::Aversion => &self.aversions,
            MemoryKind::Constraint => &self.constraints,
            MemoryKind::Policy => &self.policies,
            _ => &[],
        }
    }

    /// Get all profile memories as a formatted context string.
    #[must_use]
    pub fn to_context_string(&self) -> String {
        let mut parts = Vec::new();

        if !self.identity.is_empty() {
            let items: Vec<_> = self.identity.iter().map(|m| m.content.as_str()).collect();
            parts.push(format!("Identite: {}", items.join("; ")));
        }

        if !self.preferences.is_empty() {
            let items: Vec<_> = self.preferences.iter().map(|m| m.content.as_str()).collect();
            parts.push(format!("Preferences: {}", items.join("; ")));
        }

        if !self.aversions.is_empty() {
            let items: Vec<_> = self.aversions.iter().map(|m| m.content.as_str()).collect();
            parts.push(format!("Aversions: {}", items.join("; ")));
        }

        if !self.constraints.is_empty() {
            let items: Vec<_> = self.constraints.iter().map(|m| m.content.as_str()).collect();
            parts.push(format!("Contraintes: {}", items.join("; ")));
        }

        if !self.policies.is_empty() {
            let items: Vec<_> = self.policies.iter().map(|m| m.content.as_str()).collect();
            parts.push(format!("Politiques: {}", items.join("; ")));
        }

        parts.join("\n")
    }

    /// Update session tracking.
    pub fn record_session(&mut self, session_id: SessionId, turns: u32) {
        self.total_sessions += 1;
        self.total_turns += turns;
        self.last_session_id = Some(session_id);
        self.updated_at = Utc::now();
    }

    /// Get the total number of memories in the profile.
    #[must_use]
    pub const fn memory_count(&self) -> usize {
        self.identity.len()
            + self.preferences.len()
            + self.aversions.len()
            + self.constraints.len()
            + self.policies.len()
    }

    const fn collection_for_kind_mut(&mut self, kind: MemoryKind) -> &mut Vec<ProfileMemory> {
        match kind {
            MemoryKind::Identity => &mut self.identity,
            MemoryKind::Preference | MemoryKind::Fact | MemoryKind::Decision
            | MemoryKind::Goal | MemoryKind::Task | MemoryKind::Plan
            | MemoryKind::Procedure | MemoryKind::Episode | MemoryKind::Reflection
            | MemoryKind::Summary | MemoryKind::Feedback | MemoryKind::ToolResult
            | MemoryKind::CodeArtifact | MemoryKind::DocumentArtifact
            | MemoryKind::MediaArtifact | MemoryKind::Other | MemoryKind::Unknown => &mut self.preferences,
            MemoryKind::Aversion => &mut self.aversions,
            MemoryKind::Constraint => &mut self.constraints,
            MemoryKind::Policy => &mut self.policies,
        }
    }
}

/// Simple content similarity check (normalized Jaccard on words).
#[allow(clippy::cast_precision_loss)]
fn content_similarity(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let words_a: std::collections::HashSet<_> = a_lower.split_whitespace().collect();
    let words_b: std::collections::HashSet<_> = b_lower.split_whitespace().collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 {
        return 0.0;
    }

    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::core::metadata::{MemoryMetadata, MemorySource};

    fn make_item(kind: MemoryKind, content: &str) -> MemoryItem {
        let session_id = SessionId::new();
        let metadata = MemoryMetadata::new(MemorySource::User).with_salience(70);
        MemoryItem::new(session_id, kind, content, metadata).unwrap()
    }

    #[test]
    fn test_should_promote() {
        assert!(UserProfile::should_promote(MemoryKind::Identity));
        assert!(UserProfile::should_promote(MemoryKind::Preference));
        assert!(UserProfile::should_promote(MemoryKind::Aversion));
        assert!(!UserProfile::should_promote(MemoryKind::Fact));
        assert!(!UserProfile::should_promote(MemoryKind::Decision));
    }

    #[test]
    fn test_add_memory() {
        let user_id = UserId::new();
        let mut profile = UserProfile::new(user_id);

        let item = make_item(MemoryKind::Identity, "My name is Roy");
        profile.add_memory(&item);

        assert_eq!(profile.identity.len(), 1);
        assert_eq!(profile.identity[0].content, "My name is Roy");
    }

    #[test]
    fn test_merge_similar_memories() {
        let user_id = UserId::new();
        let mut profile = UserProfile::new(user_id);

        // Use identical content to test merge logic
        let item1 = make_item(MemoryKind::Preference, "I really enjoy drinking coffee");
        let item2 = make_item(MemoryKind::Preference, "I really enjoy drinking coffee");

        profile.add_memory(&item1);
        profile.add_memory(&item2);

        // Should merge because content is identical (Jaccard = 1.0)
        assert_eq!(profile.preferences.len(), 1);
        assert!(profile.preferences[0].seen_count >= 2);
    }

    #[test]
    fn test_no_merge_different_memories() {
        let user_id = UserId::new();
        let mut profile = UserProfile::new(user_id);

        // Use very different content
        let item1 = make_item(MemoryKind::Preference, "I like coffee");
        let item2 = make_item(MemoryKind::Preference, "I hate vegetables and fruits");

        profile.add_memory(&item1);
        profile.add_memory(&item2);

        // Should NOT merge because content is different
        assert_eq!(profile.preferences.len(), 2);
    }

    #[test]
    fn test_to_context_string() {
        let user_id = UserId::new();
        let mut profile = UserProfile::new(user_id);

        let item = make_item(MemoryKind::Identity, "My name is Roy");
        profile.add_memory(&item);

        let context = profile.to_context_string();
        assert!(context.contains("Identite"));
        assert!(context.contains("My name is Roy"));
    }

    #[test]
    fn test_content_similarity() {
        assert!(content_similarity("I like coffee", "I really like coffee") > 0.5);
        assert!(content_similarity("I like coffee", "I hate tea") < 0.3);
        assert!((content_similarity("hello world", "hello world") - 1.0).abs() < f64::EPSILON);
    }
}
