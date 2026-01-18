//! Transcript store for raw conversation events.

use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::OptionalExtension;
use tokio_rusqlite::Connection;

use crate::memory::core::config::StorageConfig;
use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::core::ids::SessionId;
use crate::memory::ingest::transcript_event::{TranscriptEvent, TranscriptRole};

/// Boxed future type for transcript store operations.
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Transcript store trait.
pub trait TranscriptStore: Send + Sync {
    /// Append a batch of transcript events.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn append_events(&self, events: Vec<TranscriptEvent>) -> StoreFuture<'_, MemoryResult<()>>;
    /// Load the most recent events for a session.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn load_recent(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> StoreFuture<'_, MemoryResult<Vec<TranscriptEvent>>>;
    /// Load events within a timestamp range.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn load_range(
        &self,
        session_id: SessionId,
        from_ts: DateTime<Utc>,
        to_ts: DateTime<Utc>,
    ) -> StoreFuture<'_, MemoryResult<Vec<TranscriptEvent>>>;
    /// Count distinct turns for a session.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn count_turns(&self, session_id: SessionId) -> StoreFuture<'_, MemoryResult<u64>>;
}

/// `SQLite` implementation of transcript storage.
pub struct SqliteTranscriptStore {
    conn: Connection,
    table: String,
}

impl SqliteTranscriptStore {
    /// Initialize the transcript store.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened.
    pub async fn new(config: &StorageConfig) -> MemoryResult<Self> {
        let conn = Connection::open(&config.sqlite_path).await?;
        let table = config.transcript_table.clone();
        let table_name = table.clone();

        conn.call(move |conn| {
            conn.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table_name} (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    session_id TEXT NOT NULL,
                    turn_id TEXT NOT NULL,
                    ts INTEGER NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    tool_name TEXT,
                    tool_payload TEXT
                );
                CREATE INDEX IF NOT EXISTS idx_{table_name}_session_ts
                    ON {table_name} (session_id, ts);
                CREATE INDEX IF NOT EXISTS idx_{table_name}_turn
                    ON {table_name} (session_id, turn_id);"
            ))?;
            Ok(())
        })
        .await?;

        Ok(Self { conn, table })
    }
}

impl TranscriptStore for SqliteTranscriptStore {
    fn append_events(&self, events: Vec<TranscriptEvent>) -> StoreFuture<'_, MemoryResult<()>> {
        Box::pin(async move {
            if events.is_empty() {
                return Ok(());
            }

            let table = self.table.clone();
            self.conn
                .call(move |conn| {
                    let tx = conn.transaction()?;
                    {
                        let mut stmt = tx.prepare(&format!(
                            "INSERT INTO {table}
                            (session_id, turn_id, ts, role, content, tool_name, tool_payload)
                            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
                        ))?;

                        for event in events {
                            let tool_payload = event
                                .tool_payload
                                .as_ref()
                                .map(serde_json::to_string)
                                .transpose()
                                .map_err(|err| tokio_rusqlite::Error::Other(Box::new(err)))?;
                            stmt.execute(rusqlite::params![
                                event.session_id.to_string(),
                                event.turn_id.to_string(),
                                event.timestamp.timestamp_millis(),
                                event.role.to_string(),
                                event.content,
                                event.tool_name,
                                tool_payload
                            ])?;
                        }
                    }

                    tx.commit()?;
                    Ok(())
                })
                .await?;
            Ok(())
        })
    }

    fn load_recent(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> StoreFuture<'_, MemoryResult<Vec<TranscriptEvent>>> {
        Box::pin(async move {
            let table = self.table.clone();
            let session = session_id.to_string();
            let limit = i64::try_from(limit)
                .map_err(|_| MemoryError::InvalidMemoryItem("limit exceeds i64".to_string()))?;
            let mut rows = self
                .conn
                .call(move |conn| {
                    let mut stmt = conn.prepare(&format!(
                        "SELECT turn_id, ts, role, content, tool_name, tool_payload
                         FROM {table}
                         WHERE session_id = ?1
                         ORDER BY ts DESC
                         LIMIT ?2"
                    ))?;
                    let rows = stmt
                        .query_map(rusqlite::params![session, limit], |row| {
                            let turn_id: String = row.get(0)?;
                            let ts: i64 = row.get(1)?;
                            let role: String = row.get(2)?;
                            let content: String = row.get(3)?;
                            let tool_name: Option<String> = row.get(4)?;
                            let tool_payload: Option<String> = row.get(5)?;
                            Ok((turn_id, ts, role, content, tool_name, tool_payload))
                        })?
                        .collect::<Result<Vec<_>, rusqlite::Error>>()?;
                    Ok(rows)
                })
                .await?;

            rows.reverse();
            let mut events = Vec::with_capacity(rows.len());
            for (turn_id, ts, role, content, tool_name, tool_payload) in rows {
                let timestamp = Utc.timestamp_millis_opt(ts).single().ok_or_else(|| {
                    MemoryError::InvalidMemoryItem("invalid timestamp".to_string())
                })?;
                let role = TranscriptRole::from_str(&role).map_err(|err| {
                    MemoryError::InvalidMemoryItem(format!("invalid role: {err}"))
                })?;
                let tool_payload = match tool_payload {
                    Some(payload) => Some(serde_json::from_str(&payload)?),
                    None => None,
                };
                events.push(TranscriptEvent {
                    turn_id: turn_id.parse().map_err(|err| {
                        MemoryError::InvalidMemoryItem(format!("invalid turn id: {err}"))
                    })?,
                    session_id,
                    timestamp,
                    role,
                    content,
                    tool_name,
                    tool_payload,
                });
            }

            Ok(events)
        })
    }

    fn load_range(
        &self,
        session_id: SessionId,
        from_ts: DateTime<Utc>,
        to_ts: DateTime<Utc>,
    ) -> StoreFuture<'_, MemoryResult<Vec<TranscriptEvent>>> {
        Box::pin(async move {
            let table = self.table.clone();
            let session = session_id.to_string();
            let from_millis = from_ts.timestamp_millis();
            let to_millis = to_ts.timestamp_millis();
            let rows = self
                .conn
                .call(move |conn| {
                    let mut stmt = conn.prepare(&format!(
                        "SELECT turn_id, ts, role, content, tool_name, tool_payload
                         FROM {table}
                         WHERE session_id = ?1 AND ts BETWEEN ?2 AND ?3
                         ORDER BY ts"
                    ))?;
                    let rows = stmt
                        .query_map(rusqlite::params![session, from_millis, to_millis], |row| {
                            let turn_id: String = row.get(0)?;
                            let ts: i64 = row.get(1)?;
                            let role: String = row.get(2)?;
                            let content: String = row.get(3)?;
                            let tool_name: Option<String> = row.get(4)?;
                            let tool_payload: Option<String> = row.get(5)?;
                            Ok((turn_id, ts, role, content, tool_name, tool_payload))
                        })?
                        .collect::<Result<Vec<_>, rusqlite::Error>>()?;
                    Ok(rows)
                })
                .await?;

            let mut events = Vec::with_capacity(rows.len());
            for (turn_id, ts, role, content, tool_name, tool_payload) in rows {
                let timestamp = Utc.timestamp_millis_opt(ts).single().ok_or_else(|| {
                    MemoryError::InvalidMemoryItem("invalid timestamp".to_string())
                })?;
                let role = TranscriptRole::from_str(&role).map_err(|err| {
                    MemoryError::InvalidMemoryItem(format!("invalid role: {err}"))
                })?;
                let tool_payload = match tool_payload {
                    Some(payload) => Some(serde_json::from_str(&payload)?),
                    None => None,
                };
                events.push(TranscriptEvent {
                    turn_id: turn_id.parse().map_err(|err| {
                        MemoryError::InvalidMemoryItem(format!("invalid turn id: {err}"))
                    })?,
                    session_id,
                    timestamp,
                    role,
                    content,
                    tool_name,
                    tool_payload,
                });
            }

            Ok(events)
        })
    }

    fn count_turns(&self, session_id: SessionId) -> StoreFuture<'_, MemoryResult<u64>> {
        Box::pin(async move {
            let table = self.table.clone();
            let session = session_id.to_string();
            let count = self
                .conn
                .call(move |conn| {
                    let row = conn
                        .query_row(
                            &format!(
                                "SELECT COUNT(DISTINCT turn_id) FROM {table} WHERE session_id = ?1"
                            ),
                            rusqlite::params![session],
                            |row| row.get::<_, i64>(0),
                        )
                        .optional()?;
                    Ok(row)
                })
                .await?;
            let count = count.unwrap_or(0);
            let count = u64::try_from(count)
                .map_err(|_| MemoryError::InvalidMemoryItem("invalid turn count".to_string()))?;
            Ok(count)
        })
    }
}
