//! Application state shared across all request handlers.

use std::sync::Arc;

use crate::llm::ollama_starter_ministral::OllamaMinistral;

/// Default model name.
const DEFAULT_MODEL: &str = "ministral-3:8b-instruct-2512-q8_0";

/// Shared application state.
pub struct AppState {
    /// Ollama client for LLM operations.
    pub ollama: OllamaMinistral,
    /// Model name to use.
    pub model_name: String,
}

impl AppState {
    /// Create a new application state.
    ///
    /// # Errors
    /// Returns an error if Ollama client cannot be created.
    pub fn new() -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let ollama = OllamaMinistral::new_default()
            .map_err(|e| format!("Failed to create Ollama client: {e}"))?;

        let model_name = std::env::var("HALLDYLL_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Ok(Arc::new(Self { ollama, model_name }))
    }
}
