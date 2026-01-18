//! User profile storage for cross-session memory.

use std::future::Future;
use std::pin::Pin;

use tokio_rusqlite::Connection;

use crate::memory::core::config::StorageConfig;
use crate::memory::core::errors::MemoryResult;
use crate::memory::core::ids::UserId;
use crate::memory::core::user_profile::UserProfile;

/// Boxed future type for user store operations.
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// User profile store trait.
pub trait UserStore: Send + Sync {
    /// Get a user profile by ID.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn get_profile(&self, user_id: UserId) -> StoreFuture<'_, MemoryResult<Option<UserProfile>>>;

    /// Save or update a user profile.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn save_profile(&self, profile: &UserProfile) -> StoreFuture<'_, MemoryResult<()>>;

    /// Delete a user profile.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn delete_profile(&self, user_id: UserId) -> StoreFuture<'_, MemoryResult<()>>;

    /// Check if a user profile exists.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    fn exists(&self, user_id: UserId) -> StoreFuture<'_, MemoryResult<bool>>;
}

/// `SQLite` implementation of the user store.
pub struct SqliteUserStore {
    conn: Connection,
    table: String,
}

impl SqliteUserStore {
    /// Initialize the user store.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened.
    pub async fn new(config: &StorageConfig) -> MemoryResult<Self> {
        let conn = Connection::open(&config.sqlite_path).await?;
        let table = "user_profiles".to_string();
        let table_name = table.clone();

        conn.call(move |conn| {
            conn.execute_batch(&format!(
                "CREATE TABLE IF NOT EXISTS {table_name} (
                    user_id TEXT PRIMARY KEY,
                    profile_json TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL
                )"
            ))?;
            Ok(())
        })
        .await?;

        Ok(Self { conn, table })
    }
}

impl UserStore for SqliteUserStore {
    fn get_profile(&self, user_id: UserId) -> StoreFuture<'_, MemoryResult<Option<UserProfile>>> {
        Box::pin(async move {
            let table = self.table.clone();
            let user_id_str = user_id.to_string();

            let result = self
                .conn
                .call(move |conn| {
                    let mut stmt = conn.prepare(&format!(
                        "SELECT profile_json FROM {table} WHERE user_id = ?1"
                    ))?;
                    let result: Option<String> = stmt
                        .query_row(rusqlite::params![user_id_str], |row| row.get(0))
                        .ok();
                    Ok(result)
                })
                .await?;

            match result {
                Some(json) => {
                    let profile: UserProfile = serde_json::from_str(&json)?;
                    Ok(Some(profile))
                }
                None => Ok(None),
            }
        })
    }

    fn save_profile(&self, profile: &UserProfile) -> StoreFuture<'_, MemoryResult<()>> {
        let profile = profile.clone();
        Box::pin(async move {
            let table = self.table.clone();
            let user_id_str = profile.user_id.to_string();
            let profile_json = serde_json::to_string(&profile)?;
            let created_at = profile.created_at.timestamp_millis();
            let updated_at = profile.updated_at.timestamp_millis();

            self.conn
                .call(move |conn| {
                    conn.execute(
                        &format!(
                            "INSERT OR REPLACE INTO {table} (user_id, profile_json, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4)"
                        ),
                        rusqlite::params![user_id_str, profile_json, created_at, updated_at],
                    )?;
                    Ok(())
                })
                .await?;

            Ok(())
        })
    }

    fn delete_profile(&self, user_id: UserId) -> StoreFuture<'_, MemoryResult<()>> {
        Box::pin(async move {
            let table = self.table.clone();
            let user_id_str = user_id.to_string();

            self.conn
                .call(move |conn| {
                    conn.execute(
                        &format!("DELETE FROM {table} WHERE user_id = ?1"),
                        rusqlite::params![user_id_str],
                    )?;
                    Ok(())
                })
                .await?;

            Ok(())
        })
    }

    fn exists(&self, user_id: UserId) -> StoreFuture<'_, MemoryResult<bool>> {
        Box::pin(async move {
            let table = self.table.clone();
            let user_id_str = user_id.to_string();

            let exists = self
                .conn
                .call(move |conn| {
                    let count: i64 = conn.query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE user_id = ?1"),
                        rusqlite::params![user_id_str],
                        |row| row.get(0),
                    )?;
                    Ok(count > 0)
                })
                .await?;

            Ok(exists)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_profile_serialization() {
        let user_id = UserId::new();
        let profile = UserProfile::new(user_id);

        let json = serde_json::to_string(&profile).unwrap();
        let restored: UserProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(profile.user_id, restored.user_id);
    }
}
