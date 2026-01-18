//! Transcript event model for conversation logging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::memory::core::ids::{SessionId, TurnId};

/// Role of a transcript event.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptRole {
    /// User input.
    User,
    /// Assistant response.
    Assistant,
    /// Tool output.
    Tool,
    /// System message.
    System,
}

impl TranscriptRole {
    /// Stable string form for storage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
            Self::System => "system",
        }
    }
}

impl fmt::Display for TranscriptRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for TranscriptRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "tool" => Ok(Self::Tool),
            "system" => Ok(Self::System),
            _ => Err(value.to_string()),
        }
    }
}

/// A single transcript event tied to a turn.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TranscriptEvent {
    /// Turn identifier for grouping.
    pub turn_id: TurnId,
    /// Session identifier.
    pub session_id: SessionId,
    /// Timestamp for ordering.
    pub timestamp: DateTime<Utc>,
    /// Role of the event.
    pub role: TranscriptRole,
    /// Content payload.
    pub content: String,
    /// Optional tool name for tool events.
    pub tool_name: Option<String>,
    /// Optional tool payload for tool events.
    pub tool_payload: Option<serde_json::Value>,
}

impl TranscriptEvent {
    /// Build a user event for a turn.
    #[must_use]
    pub fn user(turn_id: TurnId, session_id: SessionId, content: impl Into<String>) -> Self {
        Self {
            turn_id,
            session_id,
            timestamp: Utc::now(),
            role: TranscriptRole::User,
            content: content.into(),
            tool_name: None,
            tool_payload: None,
        }
    }

    /// Build an assistant event for a turn.
    #[must_use]
    pub fn assistant(turn_id: TurnId, session_id: SessionId, content: impl Into<String>) -> Self {
        Self {
            turn_id,
            session_id,
            timestamp: Utc::now(),
            role: TranscriptRole::Assistant,
            content: content.into(),
            tool_name: None,
            tool_payload: None,
        }
    }

    /// Build a tool event for a turn.
    #[must_use]
    pub fn tool(
        turn_id: TurnId,
        session_id: SessionId,
        tool_name: impl Into<String>,
        content: impl Into<String>,
        payload: Option<serde_json::Value>,
    ) -> Self {
        Self {
            turn_id,
            session_id,
            timestamp: Utc::now(),
            role: TranscriptRole::Tool,
            content: content.into(),
            tool_name: Some(tool_name.into()),
            tool_payload: payload,
        }
    }
}
