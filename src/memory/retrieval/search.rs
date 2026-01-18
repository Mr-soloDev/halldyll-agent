//! Retrieval helpers for long-term memory.

use crate::memory::core::errors::MemoryResult;
use crate::memory::core::ids::SessionId;
use crate::memory::embedding::embedder::{EmbedFuture, Embedder};
use crate::memory::ingest::transcript_event::{TranscriptEvent, TranscriptRole};
use crate::memory::storage::vector_store::{StoreFuture, VectorMemoryStore, VectorSearchResult};

/// Build a query string from the user message and recent turns.
#[must_use]
pub fn build_query_text(user_message: &str, turns: &[TranscriptEvent]) -> String {
    let mut query = String::new();
    for event in turns {
        let role = match event.role {
            TranscriptRole::User => "user",
            TranscriptRole::Assistant => "assistant",
            TranscriptRole::Tool => "tool",
            TranscriptRole::System => "system",
        };
        query.push_str(role);
        query.push_str(": ");
        query.push_str(&event.content);
        query.push('\n');
    }
    query.push_str("user: ");
    query.push_str(user_message);
    query
}

/// Build an embedding for a query string.
pub fn build_query_embedding<'a>(
    embedder: &'a dyn Embedder,
    query: &'a str,
) -> EmbedFuture<'a, MemoryResult<rig::embeddings::Embedding>> {
    embedder.embed_text(query)
}

/// Fetch top-k raw results from the vector store.
pub fn fetch_top_k_raw<'a>(
    store: &'a dyn VectorMemoryStore,
    session_id: SessionId,
    query: &'a str,
    top_k: usize,
    min_similarity: f64,
) -> StoreFuture<'a, MemoryResult<Vec<VectorSearchResult>>> {
    store.query(session_id, query, top_k, min_similarity)
}
