//! Background cleanup worker for memory garbage collection.
//!
//! Periodically removes expired memories based on their TTL and performs
//! maintenance tasks on the memory store.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::memory::core::errors::MemoryResult;
use crate::memory::core::ids::MemoryId;
use crate::memory::storage::vector_store::VectorMemoryStore;

/// Configuration for background cleanup.
#[derive(Debug, Clone)]
pub struct CleanupConfig {
    /// Interval between cleanup runs (in seconds).
    pub interval_seconds: u64,
    /// Whether background cleanup is enabled.
    pub enabled: bool,
    /// Maximum memories per session (soft limit for pruning).
    pub max_memories_per_session: usize,
}

impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            interval_seconds: 3600, // 1 hour
            enabled: true,
            max_memories_per_session: 500,
        }
    }
}

/// Statistics from a cleanup run.
#[derive(Debug, Clone, Default)]
pub struct CleanupStats {
    /// Number of expired items deleted.
    pub expired_deleted: usize,
    /// Number of items pruned due to session limit.
    pub pruned_count: usize,
    /// Total cleanup duration in milliseconds.
    pub duration_ms: u64,
}

/// Background cleanup worker for memory maintenance.
pub struct BackgroundCleanup<S> {
    store: Arc<S>,
    config: CleanupConfig,
    shutdown: Arc<Notify>,
}

impl<S: VectorMemoryStore + 'static> BackgroundCleanup<S> {
    /// Create a new background cleanup worker.
    #[must_use]
    pub fn new(store: Arc<S>, config: CleanupConfig) -> Self {
        Self {
            store,
            config,
            shutdown: Arc::new(Notify::new()),
        }
    }

    /// Get a shutdown notifier to stop the cleanup worker.
    #[must_use]
    pub fn shutdown_notifier(&self) -> Arc<Notify> {
        Arc::clone(&self.shutdown)
    }

    /// Spawn the background cleanup worker as a tokio task.
    ///
    /// Returns a `JoinHandle` that can be used to await completion.
    #[must_use]
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            self.run().await;
        })
    }

    /// Run the cleanup loop until shutdown is signaled.
    async fn run(&self) {
        if !self.config.enabled {
            info!("Background cleanup is disabled");
            return;
        }

        let interval = Duration::from_secs(self.config.interval_seconds);
        info!(?interval, "Starting background cleanup worker");

        loop {
            tokio::select! {
                () = tokio::time::sleep(interval) => {
                    match self.run_cleanup().await {
                        Ok(stats) => {
                            if stats.expired_deleted > 0 || stats.pruned_count > 0 {
                                info!(
                                    expired = stats.expired_deleted,
                                    pruned = stats.pruned_count,
                                    duration_ms = stats.duration_ms,
                                    "Cleanup completed"
                                );
                            } else {
                                debug!("Cleanup completed with no items to remove");
                            }
                        }
                        Err(err) => {
                            warn!(?err, "Cleanup failed");
                        }
                    }
                }
                () = self.shutdown.notified() => {
                    info!("Background cleanup worker shutting down");
                    break;
                }
            }
        }
    }

    /// Run a single cleanup cycle.
    ///
    /// # Errors
    /// Returns an error if store operations fail.
    pub async fn run_cleanup(&self) -> MemoryResult<CleanupStats> {
        let start = std::time::Instant::now();
        let mut stats = CleanupStats::default();

        // Find and delete expired items
        let expired_ids = self.find_expired_items().await?;
        if !expired_ids.is_empty() {
            stats.expired_deleted = expired_ids.len();
            self.store.delete_by_ids(expired_ids).await?;
        }

        #[allow(clippy::cast_possible_truncation)]
        {
            stats.duration_ms = start.elapsed().as_millis() as u64;
        }
        Ok(stats)
    }

    /// Find items that have exceeded their TTL.
    ///
    /// Note: This is a placeholder implementation. In production, the
    /// `VectorMemoryStore` trait should be extended with a `find_expired`
    /// method that queries: `WHERE created_at + ttl_seconds < now`.
    #[allow(clippy::unused_async)]
    async fn find_expired_items(&self) -> MemoryResult<Vec<MemoryId>> {
        // Placeholder: returns empty list until VectorMemoryStore is extended
        let _ = Utc::now();
        Ok(Vec::new())
    }
}

/// Builder for cleanup configuration.
#[derive(Debug, Clone, Default)]
pub struct CleanupConfigBuilder {
    interval_seconds: Option<u64>,
    enabled: Option<bool>,
    max_memories_per_session: Option<usize>,
}

impl CleanupConfigBuilder {
    /// Create a new builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the cleanup interval in seconds.
    #[must_use]
    pub const fn interval_seconds(mut self, seconds: u64) -> Self {
        self.interval_seconds = Some(seconds);
        self
    }

    /// Enable or disable background cleanup.
    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    /// Set the maximum memories per session.
    #[must_use]
    pub const fn max_memories_per_session(mut self, max: usize) -> Self {
        self.max_memories_per_session = Some(max);
        self
    }

    /// Build the cleanup configuration.
    #[must_use]
    pub fn build(self) -> CleanupConfig {
        let default = CleanupConfig::default();
        CleanupConfig {
            interval_seconds: self.interval_seconds.unwrap_or(default.interval_seconds),
            enabled: self.enabled.unwrap_or(default.enabled),
            max_memories_per_session: self
                .max_memories_per_session
                .unwrap_or(default.max_memories_per_session),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = CleanupConfig::default();
        assert_eq!(config.interval_seconds, 3600);
        assert!(config.enabled);
        assert_eq!(config.max_memories_per_session, 500);
    }

    #[test]
    fn test_config_builder() {
        let config = CleanupConfigBuilder::new()
            .interval_seconds(1800)
            .enabled(false)
            .max_memories_per_session(1000)
            .build();

        assert_eq!(config.interval_seconds, 1800);
        assert!(!config.enabled);
        assert_eq!(config.max_memories_per_session, 1000);
    }

    #[test]
    fn test_stats_default() {
        let stats = CleanupStats::default();
        assert_eq!(stats.expired_deleted, 0);
        assert_eq!(stats.pruned_count, 0);
        assert_eq!(stats.duration_ms, 0);
    }
}
