//! Vector store integration using Rig + `SQLite`.

use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::OnceLock;

use reqwest::Client as ReqwestClient;
use rig::OneOrMany;
use rig::client::{EmbeddingsClient, Nothing};
use rig::embeddings::Embedding;
use rig::providers::ollama;
use rig::vector_store::VectorStoreIndex;
use rig::vector_store::request::{SearchFilter, VectorSearchRequest};
use rig_sqlite::{
    Column, ColumnValue, SqliteSearchFilter, SqliteVectorStore, SqliteVectorStoreTable,
};
use serde::Deserialize;
use tokio_rusqlite::Connection;

use crate::memory::core::config::MemoryConfig;
use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::core::ids::{MemoryId, SessionId};
use crate::memory::core::item::MemoryItem;
use crate::memory::core::kinds::MemoryKind;
use crate::memory::core::metadata::MemoryMetadata;

type OllamaEmbeddingModel = ollama::EmbeddingModel<ReqwestClient>;

/// Boxed future type for vector store operations.
pub type StoreFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Vector search result with similarity score.
#[derive(Clone, Debug)]
pub struct VectorSearchResult {
    /// Similarity score in [0, 1].
    pub similarity: f64,
    /// Retrieved memory item.
    pub item: MemoryItem,
}

/// Vector store abstraction for memory items.
pub trait VectorMemoryStore: Send + Sync {
    /// Upsert a memory item with its embedding.
    ///
    /// # Errors
    /// Returns an error if the store cannot persist the item.
    fn upsert(&self, item: MemoryItem, embedding: Embedding) -> StoreFuture<'_, MemoryResult<()>>;
    /// Query memories for a session by text.
    ///
    /// # Errors
    /// Returns an error if the query cannot be executed.
    fn query(
        &self,
        session_id: SessionId,
        query: &str,
        top_k: usize,
        min_similarity: f64,
    ) -> StoreFuture<'_, MemoryResult<Vec<VectorSearchResult>>>;
    /// Check if a content hash already exists for a session.
    ///
    /// # Errors
    /// Returns an error if the store cannot be queried.
    fn exists_hash(
        &self,
        session_id: SessionId,
        content_hash: &str,
    ) -> StoreFuture<'_, MemoryResult<bool>>;
    /// Delete memories by id.
    ///
    /// # Errors
    /// Returns an error if deletion fails.
    fn delete_by_ids(&self, ids: Vec<MemoryId>) -> StoreFuture<'_, MemoryResult<()>>;
}

const DEFAULT_TABLE: &str = "memory_items";
static TABLE_NAME: OnceLock<&'static str> = OnceLock::new();

fn init_table_name(name: &str) -> MemoryResult<()> {
    if let Some(existing) = TABLE_NAME.get() {
        if *existing == name {
            return Ok(());
        }
        return Err(MemoryError::InvalidConfig(
            "memory table already initialized with a different name".to_string(),
        ));
    }

    let leaked = Box::leak(name.to_string().into_boxed_str());
    let _ = TABLE_NAME.set(leaked);
    Ok(())
}

fn table_name() -> &'static str {
    TABLE_NAME.get().copied().unwrap_or(DEFAULT_TABLE)
}

#[derive(Clone, Debug, Deserialize)]
struct MemoryDocument {
    id: String,
    session_id: String,
    kind: String,
    content: String,
    metadata_json: String,
    content_hash: String,
}

impl MemoryDocument {
    fn from_item(item: &MemoryItem) -> MemoryResult<Self> {
        let metadata_json = serde_json::to_string(&item.metadata)?;
        Ok(Self {
            id: item.id.to_string(),
            session_id: item.session_id.to_string(),
            kind: item.kind.to_string(),
            content: item.content.clone(),
            metadata_json,
            content_hash: item.content_hash.clone(),
        })
    }

    fn to_item(&self) -> MemoryResult<MemoryItem> {
        let id = MemoryId::from_str(&self.id)
            .map_err(|err| MemoryError::InvalidMemoryItem(format!("invalid memory id: {err}")))?;
        let session_id = SessionId::from_str(&self.session_id)
            .map_err(|err| MemoryError::InvalidMemoryItem(format!("invalid session id: {err}")))?;
        let kind = MemoryKind::from_str(&self.kind)
            .map_err(|err| MemoryError::InvalidMemoryItem(format!("invalid kind: {err}")))?;
        let metadata: MemoryMetadata = serde_json::from_str(&self.metadata_json)?;
        Ok(MemoryItem {
            id,
            session_id,
            kind,
            content: self.content.clone(),
            metadata,
            content_hash: self.content_hash.clone(),
        })
    }
}

impl SqliteVectorStoreTable for MemoryDocument {
    fn name() -> &'static str {
        table_name()
    }

    fn schema() -> Vec<Column> {
        vec![
            Column::new("id", "TEXT PRIMARY KEY"),
            Column::new("session_id", "TEXT").indexed(),
            Column::new("kind", "TEXT"),
            Column::new("content", "TEXT"),
            Column::new("metadata_json", "TEXT"),
            Column::new("content_hash", "TEXT").indexed(),
        ]
    }

    fn id(&self) -> String {
        self.id.clone()
    }

    fn column_values(&self) -> Vec<(&'static str, Box<dyn ColumnValue>)> {
        vec![
            ("id", Box::new(self.id.clone())),
            ("session_id", Box::new(self.session_id.clone())),
            ("kind", Box::new(self.kind.clone())),
            ("content", Box::new(self.content.clone())),
            ("metadata_json", Box::new(self.metadata_json.clone())),
            ("content_hash", Box::new(self.content_hash.clone())),
        ]
    }
}

/// SQLite-backed vector memory store.
pub struct SqliteVectorMemoryStore {
    conn: Connection,
    store: SqliteVectorStore<OllamaEmbeddingModel, MemoryDocument>,
    index: rig_sqlite::SqliteVectorIndex<OllamaEmbeddingModel, MemoryDocument>,
}

impl SqliteVectorMemoryStore {
    /// Initialize the `SQLite` vector store.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened or the sqlite-vec extension is missing.
    ///
    /// # Note
    /// You must call `init_sqlite_vec_extension()` before calling this function.
    pub async fn new(config: &MemoryConfig) -> MemoryResult<Self> {
        init_table_name(&config.storage.memory_table)?;
        let conn = Connection::open(&config.storage.sqlite_path).await?;

        let builder = ollama::Client::<ReqwestClient>::builder().api_key(Nothing);
        let builder = if let Some(base_url) = &config.embedding.base_url {
            builder.base_url(base_url)
        } else {
            builder
        };
        let client = builder.build().map_err(MemoryError::from)?;
        let model = client
            .embedding_model_with_ndims(config.embedding.model.clone(), config.embedding.ndims);

        let store = SqliteVectorStore::new(conn.clone(), &model).await?;
        let index = store.clone().index(model);

        Ok(Self { conn, store, index })
    }
}

impl VectorMemoryStore for SqliteVectorMemoryStore {
    fn upsert(&self, item: MemoryItem, embedding: Embedding) -> StoreFuture<'_, MemoryResult<()>> {
        Box::pin(async move {
            let doc = MemoryDocument::from_item(&item)?;
            let embeddings = OneOrMany::one(embedding);
            self.store.add_rows(vec![(doc, embeddings)]).await?;
            Ok(())
        })
    }

    fn query(
        &self,
        session_id: SessionId,
        query: &str,
        top_k: usize,
        min_similarity: f64,
    ) -> StoreFuture<'_, MemoryResult<Vec<VectorSearchResult>>> {
        let query_text = query.to_string();
        Box::pin(async move {
            let filter =
                SqliteSearchFilter::eq("session_id".to_string(), session_id.to_string().into());
            let request = VectorSearchRequest::builder()
                .query(query_text)
                .samples(top_k as u64)
                .threshold(min_similarity)
                .filter(filter)
                .build()
                .map_err(|err| MemoryError::InvalidConfig(err.to_string()))?;

            let raw = self.index.top_n::<MemoryDocument>(request).await?;
            let mut results = Vec::with_capacity(raw.len());
            for (score, _id, doc) in raw {
                if score < min_similarity {
                    continue;
                }
                let item = doc.to_item()?;
                results.push(VectorSearchResult {
                    similarity: score,
                    item,
                });
            }
            Ok(results)
        })
    }

    fn exists_hash(
        &self,
        session_id: SessionId,
        content_hash: &str,
    ) -> StoreFuture<'_, MemoryResult<bool>> {
        let content_hash = content_hash.to_string();
        Box::pin(async move {
            let session_id = session_id.to_string();
            let hash = content_hash;
            let table = table_name().to_string();
            let exists = self
                .conn
                .call(move |conn| {
                    let mut stmt = conn.prepare(&format!(
                        "SELECT 1 FROM {table} WHERE session_id = ?1 AND content_hash = ?2 LIMIT 1"
                    ))?;
                    let mut rows = stmt.query(rusqlite::params![session_id, hash])?;
                    Ok(rows.next()?.is_some())
                })
                .await?;
            Ok(exists)
        })
    }

    fn delete_by_ids(&self, ids: Vec<MemoryId>) -> StoreFuture<'_, MemoryResult<()>> {
        Box::pin(async move {
            if ids.is_empty() {
                return Ok(());
            }

            let table = table_name().to_string();
            let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
            let placeholders = (0..ids.len())
                .map(|i| format!("?{}", i + 1))
                .collect::<Vec<_>>()
                .join(", ");

            let ids_clone = ids.clone();
            self.conn
                .call(move |conn| {
                    let tx = conn.transaction()?;
                    let select_sql =
                        format!("SELECT rowid FROM {table} WHERE id IN ({placeholders})");
                    let rowids = {
                        let mut stmt = tx.prepare(&select_sql)?;
                        stmt.query_map(rusqlite::params_from_iter(ids_clone.iter()), |row| {
                            row.get::<_, i64>(0)
                        })?
                        .collect::<Result<Vec<_>, rusqlite::Error>>()?
                    };

                    let delete_doc_sql =
                        format!("DELETE FROM {table} WHERE id IN ({placeholders})");
                    tx.execute(&delete_doc_sql, rusqlite::params_from_iter(ids.iter()))?;

                    if !rowids.is_empty() {
                        let rowid_placeholders = (0..rowids.len())
                            .map(|i| format!("?{}", i + 1))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let delete_embed_sql = format!(
                            "DELETE FROM {table}_embeddings WHERE rowid IN ({rowid_placeholders})"
                        );
                        tx.execute(&delete_embed_sql, rusqlite::params_from_iter(rowids.iter()))?;
                    }

                    tx.commit()?;
                    Ok(())
                })
                .await?;

            Ok(())
        })
    }
}
