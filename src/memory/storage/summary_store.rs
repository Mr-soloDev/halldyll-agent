//! Summary store for per-session rollups.

use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, TimeZone, Utc};
use rusqlite::OptionalExtension;
use tokio_rusqlite::Connection;

use crate::memory::core::config::StorageConfig;
use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::core::ids::SessionId;

/// Boxed future type for summary store operations.
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A stored session summary record.
#[derive(Clone, Debug)]
pub struct SummaryRecord {
    /// Session id for this summary.
    pub session_id: SessionId,
    /// Summary content.
    pub summary: String,
    /// Last update time.
    pub updated_at: DateTime<Utc>,
    /// Turn count at last update.
    pub turn_count: u64,
}

/// Summary store trait.
pub trait SummaryStore: Send + Sync {
    /// Get the summary for a session.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn get_summary(
        &self,
        session_id: SessionId,
    ) -> StoreFuture<'_, MemoryResult<Option<SummaryRecord>>>;
    /// Set the summary for a session.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn set_summary(&self, record: SummaryRecord) -> StoreFuture<'_, MemoryResult<()>>;
}

/// `SQLite` implementation of the summary store.
pub struct SqliteSummaryStore {
    conn: Connection,
    table: String,
}

impl SqliteSummaryStore {
    /// Initialize the summary store.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened.
    pub async fn new(config: &StorageConfig) -> MemoryResult<Self> {
        let conn = Connection::open(&config.sqlite_path).await?;
        let table = config.summary_table.clone();
        let table_name = table.clone();

        conn.call(move |conn| {
            conn.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table_name} (
                    session_id TEXT PRIMARY KEY,
                    summary TEXT NOT NULL,
                    updated_at INTEGER NOT NULL,
                    turn_count INTEGER NOT NULL
                )"
            ))?;
            Ok(())
        })
        .await?;

        Ok(Self { conn, table })
    }
}

impl SummaryStore for SqliteSummaryStore {
    fn get_summary(
        &self,
        session_id: SessionId,
    ) -> StoreFuture<'_, MemoryResult<Option<SummaryRecord>>> {
        Box::pin(async move {
            let table = self.table.clone();
            let session = session_id.to_string();
            let record = self
                .conn
                .call(move |conn| {
                    let row = conn
                        .query_row(
                            &format!(
                                "SELECT summary, updated_at, turn_count FROM {table} WHERE session_id = ?1"
                            ),
                            rusqlite::params![session],
                            |row| {
                                let summary: String = row.get(0)?;
                                let updated_at_ms: i64 = row.get(1)?;
                                let turn_count: i64 = row.get(2)?;
                                Ok((summary, updated_at_ms, turn_count))
                            },
                        )
                        .optional()?;
                    Ok(row)
                })
                .await?;

            let record = match record {
                Some((summary, updated_at_ms, turn_count)) => {
                    let updated_at = Utc
                        .timestamp_millis_opt(updated_at_ms)
                        .single()
                        .ok_or_else(|| {
                            MemoryError::InvalidMemoryItem(
                                "invalid updated_at timestamp".to_string(),
                            )
                        })?;
                    let turn_count = u64::try_from(turn_count).map_err(|_| {
                        MemoryError::InvalidMemoryItem("invalid turn count".to_string())
                    })?;
                    Some(SummaryRecord {
                        session_id,
                        summary,
                        updated_at,
                        turn_count,
                    })
                }
                None => None,
            };

            Ok(record)
        })
    }

    fn set_summary(&self, record: SummaryRecord) -> StoreFuture<'_, MemoryResult<()>> {
        Box::pin(async move {
            let table = self.table.clone();
            let session_id = record.session_id.to_string();
            let summary = record.summary.clone();
            let updated_at = record.updated_at.timestamp_millis();
            let turn_count = i64::try_from(record.turn_count)
                .map_err(|_| MemoryError::InvalidMemoryItem("invalid turn count".to_string()))?;

            self.conn
                .call(move |conn| {
                    conn.execute(
                        &format!(
                            "INSERT OR REPLACE INTO {table} (session_id, summary, updated_at, turn_count)
                             VALUES (?1, ?2, ?3, ?4)"
                        ),
                        rusqlite::params![session_id, summary, updated_at, turn_count],
                    )?;
                    Ok(())
                })
                .await?;
            Ok(())
        })
    }
}
