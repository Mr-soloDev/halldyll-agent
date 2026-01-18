//! Application state for Tauri with memory integration.

use std::sync::Arc;

use tokio::sync::RwLock;

use halldyll_agent::memory::core::ids::SessionId;
use halldyll_agent::memory::engine::MemoryEngine;

use crate::conversations::SqliteConversationStore;

/// Shared application state managed by Tauri.
pub struct AppState {
    /// Memory engine instance.
    pub engine: Arc<RwLock<MemoryEngine>>,
    /// Conversation metadata store.
    pub conversation_store: Arc<SqliteConversationStore>,
    /// Current active session identifier (mutable).
    active_session: Arc<RwLock<Option<SessionId>>>,
}

impl AppState {
    /// Create a new application state with the given engine and conversation store.
    #[must_use]
    pub fn new(engine: MemoryEngine, conversation_store: SqliteConversationStore) -> Self {
        Self {
            engine: Arc::new(RwLock::new(engine)),
            conversation_store: Arc::new(conversation_store),
            active_session: Arc::new(RwLock::new(None)),
        }
    }

    /// Get the active session ID.
    ///
    /// # Errors
    /// Returns an error if no session is active.
    pub async fn get_active_session(&self) -> Result<SessionId, String> {
        let guard = self.active_session.read().await;
        guard.ok_or_else(|| "No active conversation".to_string())
    }

    /// Set the active session ID.
    pub async fn set_active_session(&self, id: SessionId) {
        let mut guard = self.active_session.write().await;
        *guard = Some(id);
    }

    /// Clear the active session.
    pub async fn clear_active_session(&self) {
        let mut guard = self.active_session.write().await;
        *guard = None;
    }
}
