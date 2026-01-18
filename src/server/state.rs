//! Application state shared across all request handlers.

use std::sync::Arc;

use crate::llm::ollama_starter_ministral::OllamaMinistral;
use crate::scraping::ScrapingService;

/// Shared application state.
pub struct AppState {
    /// Ollama client for LLM operations.
    pub ollama: OllamaMinistral,
    /// Scraping service for web search.
    pub scraper: ScrapingService,
    /// Model name to use.
    pub model_name: String,
}

impl AppState {
    /// Create a new application state.
    ///
    /// # Errors
    /// Returns an error if Ollama client or scraper cannot be created.
    pub fn new() -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let ollama = OllamaMinistral::new_default()
            .map_err(|e| format!("Failed to create Ollama client: {e}"))?;

        let scraper = ScrapingService::with_defaults()
            .map_err(|e| format!("Failed to create scraping service: {e}"))?;

        let model_name = std::env::var("HALLDYLL_MODEL")
            .unwrap_or_else(|_| "mistral:7b-instruct-q8_0".to_string());

        Ok(Arc::new(Self {
            ollama,
            scraper,
            model_name,
        }))
    }
}
