//! Error types for the scraping module.

use thiserror::Error;

/// Errors that can occur during scraping operations.
#[derive(Debug, Error)]
pub enum ScrapingError {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// HTTP client configuration error.
    #[error("HTTP client error: {0}")]
    HttpClient(String),

    /// URL parsing error.
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// HTML parsing error.
    #[error("HTML parsing error: {0}")]
    HtmlParse(String),

    /// JSON parsing error.
    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Content extraction failed.
    #[error("Content extraction failed: {0}")]
    ExtractionFailed(String),

    /// Search engine returned no results.
    #[error("No results found for query: {0}")]
    NoResults(String),

    /// Rate limit exceeded.
    #[error("Rate limit exceeded, retry after {0} seconds")]
    RateLimited(u64),

    /// Access denied or blocked.
    #[error("Access denied: {0}")]
    AccessDenied(String),

    /// Timeout waiting for response.
    #[error("Request timed out")]
    Timeout,

    /// Content type not supported.
    #[error("Unsupported content type: {0}")]
    UnsupportedContentType(String),

    /// Cache error.
    #[error("Cache error: {0}")]
    Cache(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Regex error.
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// API key required but not configured.
    #[error("API key required for {0}")]
    ApiKeyRequired(String),

    /// Generic error.
    #[error("{0}")]
    Other(String),
}

impl ScrapingError {
    /// Check if this error is retryable.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Timeout | Self::RateLimited(_) | Self::HttpRequest(_)
        )
    }

    /// Get retry delay in seconds if applicable.
    #[must_use]
    pub const fn retry_delay(&self) -> Option<u64> {
        match self {
            Self::RateLimited(seconds) => Some(*seconds),
            Self::Timeout => Some(5),
            Self::HttpRequest(_) => Some(2),
            _ => None,
        }
    }
}
