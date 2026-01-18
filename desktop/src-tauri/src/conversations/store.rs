//! SQLite-backed conversation metadata store.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use rusqlite::OptionalExtension;
use tokio_rusqlite::Connection;

use halldyll_agent::memory::core::ids::SessionId;

use super::types::ConversationMeta;

/// Boxed future type for store operations.
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Error type for conversation store operations.
#[derive(Debug)]
pub struct StoreError(pub String);

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StoreError {}

impl From<tokio_rusqlite::Error> for StoreError {
    fn from(err: tokio_rusqlite::Error) -> Self {
        Self(err.to_string())
    }
}

/// Result type for conversation store operations.
pub type StoreResult<T> = Result<T, StoreError>;

/// Trait for conversation metadata storage.
#[allow(dead_code)]
pub trait ConversationStore: Send + Sync {
    /// List all non-archived conversations ordered by `updated_at` DESC.
    fn list_all(&self) -> StoreFuture<'_, StoreResult<Vec<ConversationMeta>>>;

    /// Get a conversation by ID.
    fn get_by_id(&self, id: SessionId) -> StoreFuture<'_, StoreResult<Option<ConversationMeta>>>;

    /// Create a new conversation record.
    fn create(&self, id: SessionId, now_ms: i64) -> StoreFuture<'_, StoreResult<ConversationMeta>>;

    /// Update the title of a conversation.
    fn update_title(&self, id: SessionId, title: &str) -> StoreFuture<'_, StoreResult<()>>;

    /// Touch the `updated_at` timestamp and increment message count.
    fn touch_updated(&self, id: SessionId, now_ms: i64) -> StoreFuture<'_, StoreResult<()>>;

    /// Archive a conversation (soft delete).
    fn archive(&self, id: SessionId) -> StoreFuture<'_, StoreResult<()>>;

    /// Permanently delete a conversation.
    fn delete_permanent(&self, id: SessionId) -> StoreFuture<'_, StoreResult<()>>;

    /// Check if a conversation exists.
    fn exists(&self, id: SessionId) -> StoreFuture<'_, StoreResult<bool>>;
}

/// SQLite implementation of conversation metadata store.
pub struct SqliteConversationStore {
    conn: Arc<Connection>,
    table: String,
}

impl SqliteConversationStore {
    /// Table name for conversations.
    pub const DEFAULT_TABLE: &'static str = "conversations";

    /// Initialize the store and create the table if it doesn't exist.
    ///
    /// # Errors
    /// Returns an error if database operations fail.
    pub async fn new(conn: Arc<Connection>) -> StoreResult<Self> {
        let table = Self::DEFAULT_TABLE.to_string();
        let table_name = table.clone();

        conn.call(move |conn| {
            conn.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table_name} (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL DEFAULT '',
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL,
                    message_count INTEGER NOT NULL DEFAULT 0,
                    is_archived INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_{table_name}_updated
                    ON {table_name} (updated_at DESC);
                CREATE INDEX IF NOT EXISTS idx_{table_name}_archived
                    ON {table_name} (is_archived, updated_at DESC);"
            ))?;
            Ok(())
        })
        .await?;

        Ok(Self { conn, table })
    }
}

impl ConversationStore for SqliteConversationStore {
    fn list_all(&self) -> StoreFuture<'_, StoreResult<Vec<ConversationMeta>>> {
        Box::pin(async move {
            let table = self.table.clone();
            let rows = self
                .conn
                .call(move |conn| {
                    let mut stmt = conn.prepare(&format!(
                        "SELECT id, title, created_at, updated_at, message_count
                         FROM {table}
                         WHERE is_archived = 0
                         ORDER BY updated_at DESC
                         LIMIT 100"
                    ))?;
                    let rows = stmt
                        .query_map([], |row| {
                            Ok(ConversationMeta {
                                id: row.get(0)?,
                                title: row.get(1)?,
                                created_at: row.get(2)?,
                                updated_at: row.get(3)?,
                                message_count: row.get(4)?,
                            })
                        })?
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(rows)
                })
                .await?;
            Ok(rows)
        })
    }

    fn get_by_id(&self, id: SessionId) -> StoreFuture<'_, StoreResult<Option<ConversationMeta>>> {
        Box::pin(async move {
            let table = self.table.clone();
            let id_str = id.to_string();
            let row = self
                .conn
                .call(move |conn| {
                    let mut stmt = conn.prepare(&format!(
                        "SELECT id, title, created_at, updated_at, message_count
                         FROM {table}
                         WHERE id = ?1 AND is_archived = 0"
                    ))?;
                    let row = stmt
                        .query_row([&id_str], |row| {
                            Ok(ConversationMeta {
                                id: row.get(0)?,
                                title: row.get(1)?,
                                created_at: row.get(2)?,
                                updated_at: row.get(3)?,
                                message_count: row.get(4)?,
                            })
                        })
                        .optional()?;
                    Ok(row)
                })
                .await?;
            Ok(row)
        })
    }

    fn create(&self, id: SessionId, now_ms: i64) -> StoreFuture<'_, StoreResult<ConversationMeta>> {
        Box::pin(async move {
            let table = self.table.clone();
            let id_str = id.to_string();
            let meta = ConversationMeta::from_session(id, now_ms);

            self.conn
                .call(move |conn| {
                    conn.execute(
                        &format!(
                            "INSERT INTO {table} (id, title, created_at, updated_at, message_count)
                             VALUES (?1, ?2, ?3, ?4, ?5)"
                        ),
                        rusqlite::params![id_str, "", now_ms, now_ms, 0],
                    )?;
                    Ok(())
                })
                .await?;

            Ok(meta)
        })
    }

    fn update_title(&self, id: SessionId, title: &str) -> StoreFuture<'_, StoreResult<()>> {
        let title = title.to_string();
        Box::pin(async move {
            let table = self.table.clone();
            let id_str = id.to_string();
            self.conn
                .call(move |conn| {
                    conn.execute(
                        &format!("UPDATE {table} SET title = ?1 WHERE id = ?2"),
                        rusqlite::params![title, id_str],
                    )?;
                    Ok(())
                })
                .await?;
            Ok(())
        })
    }

    fn touch_updated(&self, id: SessionId, now_ms: i64) -> StoreFuture<'_, StoreResult<()>> {
        Box::pin(async move {
            let table = self.table.clone();
            let id_str = id.to_string();
            self.conn
                .call(move |conn| {
                    conn.execute(
                        &format!(
                            "UPDATE {table}
                             SET updated_at = ?1, message_count = message_count + 1
                             WHERE id = ?2"
                        ),
                        rusqlite::params![now_ms, id_str],
                    )?;
                    Ok(())
                })
                .await?;
            Ok(())
        })
    }

    fn archive(&self, id: SessionId) -> StoreFuture<'_, StoreResult<()>> {
        Box::pin(async move {
            let table = self.table.clone();
            let id_str = id.to_string();
            self.conn
                .call(move |conn| {
                    conn.execute(
                        &format!("UPDATE {table} SET is_archived = 1 WHERE id = ?1"),
                        rusqlite::params![id_str],
                    )?;
                    Ok(())
                })
                .await?;
            Ok(())
        })
    }

    fn delete_permanent(&self, id: SessionId) -> StoreFuture<'_, StoreResult<()>> {
        Box::pin(async move {
            let table = self.table.clone();
            let id_str = id.to_string();
            self.conn
                .call(move |conn| {
                    conn.execute(
                        &format!("DELETE FROM {table} WHERE id = ?1"),
                        rusqlite::params![id_str],
                    )?;
                    Ok(())
                })
                .await?;
            Ok(())
        })
    }

    fn exists(&self, id: SessionId) -> StoreFuture<'_, StoreResult<bool>> {
        Box::pin(async move {
            let table = self.table.clone();
            let id_str = id.to_string();
            let exists = self
                .conn
                .call(move |conn| {
                    let count: i64 = conn.query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE id = ?1 AND is_archived = 0"),
                        rusqlite::params![id_str],
                        |row| row.get(0),
                    )?;
                    Ok(count > 0)
                })
                .await?;
            Ok(exists)
        })
    }
}
