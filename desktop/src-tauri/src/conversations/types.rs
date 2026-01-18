//! Types for conversation management.

use serde::{Deserialize, Serialize};

use halldyll_agent::memory::core::ids::SessionId;

/// Metadata for a conversation displayed in the sidebar.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationMeta {
    /// Unique identifier (same as `SessionId`).
    pub id: String,
    /// Display title.
    pub title: String,
    /// Creation timestamp in milliseconds since Unix epoch.
    pub created_at: i64,
    /// Last activity timestamp in milliseconds since Unix epoch.
    pub updated_at: i64,
    /// Number of messages in the conversation.
    pub message_count: u32,
}

impl ConversationMeta {
    /// Create metadata from a session ID with default values.
    #[must_use]
    pub fn from_session(session_id: SessionId, now_ms: i64) -> Self {
        Self {
            id: session_id.to_string(),
            title: String::new(),
            created_at: now_ms,
            updated_at: now_ms,
            message_count: 0,
        }
    }
}

/// A message in a conversation for frontend display.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Role: "user" or "assistant".
    pub role: String,
    /// Message content.
    pub content: String,
    /// Timestamp in milliseconds since Unix epoch.
    pub timestamp: i64,
}
