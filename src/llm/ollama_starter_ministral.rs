//! Ollama client for cloud-based LLM inference.
//!
//! Connects to a remote Ollama server via HTTP API.
//! No local Ollama management - server runs on `RunPod`.

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

/// Environment variable for Ollama URL.
const OLLAMA_URL_ENV: &str = "HALLDYLL_OLLAMA_URL";

/// Default Ollama URL (localhost fallback).
const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434";

/// Context length (tokens).
const CONTEXT_LENGTH: u32 = 8_192;

/// Batch size for generation.
const NUM_BATCH: u32 = 256;

/// Default token budget.
const DEFAULT_NUM_PREDICT: u32 = 512;

/// Default thread count.
const DEFAULT_NUM_THREAD: u32 = 8;

/// HTTP timeouts.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Get Ollama base URL from environment.
fn get_ollama_url() -> String {
    std::env::var(OLLAMA_URL_ENV).unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string())
}

/// Errors from Ollama client.
#[derive(Debug)]
pub enum OllamaStarterError {
    /// HTTP client error.
    HttpClient(reqwest::Error),
    /// HTTP status error.
    HttpStatusNotOk(u16),
    /// Malformed response.
    HttpMalformedResponse,
}

impl From<reqwest::Error> for OllamaStarterError {
    fn from(value: reqwest::Error) -> Self {
        Self::HttpClient(value)
    }
}

impl fmt::Display for OllamaStarterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpClient(err) => write!(f, "http client error: {err}"),
            Self::HttpStatusNotOk(status) => write!(f, "ollama http status: {status}"),
            Self::HttpMalformedResponse => write!(f, "malformed response"),
        }
    }
}

impl std::error::Error for OllamaStarterError {}

#[derive(Serialize)]
struct GenerateOptions {
    num_ctx: u32,
    num_predict: u32,
    num_batch: u32,
    num_thread: u32,
    f16_kv: bool,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    keep_alive: &'a str,
    options: GenerateOptions,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: Option<String>,
}

/// Blocking Ollama client for text generation.
pub struct OllamaMinistral {
    client: Client,
    base_url: String,
}

impl OllamaMinistral {
    /// Create a new client.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built.
    pub fn new_default() -> Result<Self, OllamaStarterError> {
        let client = Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()?;

        Ok(Self {
            client,
            base_url: get_ollama_url(),
        })
    }

    /// Generate text with 8K context.
    ///
    /// # Errors
    /// Returns an error if the request fails.
    pub fn generate_8192(
        &self,
        model: &str,
        prompt: &str,
        keep_alive: &str,
    ) -> Result<String, OllamaStarterError> {
        let num_thread = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .map_or(DEFAULT_NUM_THREAD, |v| u32::try_from(v).unwrap_or(u32::MAX));

        let options = GenerateOptions {
            num_ctx: CONTEXT_LENGTH,
            num_predict: DEFAULT_NUM_PREDICT,
            num_batch: NUM_BATCH,
            num_thread,
            f16_kv: true,
        };

        let request = GenerateRequest {
            model,
            prompt,
            stream: false,
            keep_alive,
            options,
        };

        let url = format!("{}/api/generate", self.base_url);
        let response = self.client.post(&url).json(&request).send()?;

        let status = response.status();
        if !status.is_success() {
            return Err(OllamaStarterError::HttpStatusNotOk(status.as_u16()));
        }

        let body: GenerateResponse = response.json()?;
        body.response.ok_or(OllamaStarterError::HttpMalformedResponse)
    }
}

/// Placeholder for compatibility - does nothing in cloud mode.
///
/// # Errors
/// Always succeeds in cloud mode.
pub fn ensure_ollama_and_preload_ministral() -> Result<(), OllamaStarterError> {
    // In cloud mode, Ollama is already running on RunPod
    Ok(())
}
