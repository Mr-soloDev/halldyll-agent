//! Embedding model wrapper for Rig + Ollama.

use std::future::Future;
use std::pin::Pin;

use reqwest::Client as ReqwestClient;
use rig::client::{EmbeddingsClient, Nothing};
use rig::embeddings::{Embedding, EmbeddingModel};
use rig::providers::ollama;

use crate::memory::core::config::EmbeddingConfig;
use crate::memory::core::errors::{MemoryError, MemoryResult};

/// Boxed future type for embedder operations.
pub type EmbedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait abstraction over embedding models.
pub trait Embedder: Send + Sync {
    /// Embed a single text string.
    ///
    /// # Errors
    /// Returns an error if the embedding request fails.
    fn embed_text(&self, text: &str) -> EmbedFuture<'_, MemoryResult<Embedding>>;
    /// Embed multiple texts.
    ///
    /// # Errors
    /// Returns an error if the embedding request fails.
    fn embed_texts(&self, texts: Vec<String>) -> EmbedFuture<'_, MemoryResult<Vec<Embedding>>>;
    /// Return embedding dimensionality.
    fn ndims(&self) -> usize;
}

type OllamaEmbeddingModel = ollama::EmbeddingModel<ReqwestClient>;

/// Ollama embedder using Rig provider.
#[derive(Clone)]
pub struct OllamaEmbedder {
    model: OllamaEmbeddingModel,
    ndims: usize,
}

impl OllamaEmbedder {
    /// Create a new Ollama embedder from config.
    ///
    /// # Errors
    /// Returns an error if the base URL is invalid or the client cannot be built.
    pub fn new(config: &EmbeddingConfig) -> MemoryResult<Self> {
        let builder = ollama::Client::<ReqwestClient>::builder().api_key(Nothing);
        let builder = if let Some(base_url) = &config.base_url {
            builder.base_url(base_url)
        } else {
            builder
        };
        let client = builder.build().map_err(MemoryError::from)?;
        let model = client.embedding_model_with_ndims(config.model.clone(), config.ndims);
        Ok(Self {
            model,
            ndims: config.ndims,
        })
    }
}

impl Embedder for OllamaEmbedder {
    fn embed_text(&self, text: &str) -> EmbedFuture<'_, MemoryResult<Embedding>> {
        let text = text.to_string();
        Box::pin(async move {
            self.model
                .embed_text(&text)
                .await
                .map_err(MemoryError::Embedding)
        })
    }

    fn embed_texts(&self, texts: Vec<String>) -> EmbedFuture<'_, MemoryResult<Vec<Embedding>>> {
        Box::pin(async move {
            self.model
                .embed_texts(texts)
                .await
                .map_err(MemoryError::Embedding)
        })
    }

    fn ndims(&self) -> usize {
        self.ndims
    }
}
