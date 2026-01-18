//! Web scraping and search system for the Halldyll agent.
//!
//! This module provides comprehensive web scraping capabilities:
//! - Search engines (DuckDuckGo, Brave, Google)
//! - HTML content extraction
//! - Image search
//! - Video search (YouTube)
//! - Code search (GitHub)
//! - Caching with TTL
//! - Memory integration

pub mod cache;
pub mod code;
pub mod config;
pub mod content;
pub mod engines;
pub mod error;
pub mod images;
pub mod types;
pub mod videos;

pub use cache::ScrapingCache;
pub use config::ScrapingConfig;
pub use content::ContentScraper;
pub use error::ScrapingError;
pub use types::{
    ImageResult, SearchQuery, SearchResult, SearchResultType, VideoResult, WebContent,
};

use std::sync::Arc;

use crate::memory::core::metadata::{MemorySource, Modality};

/// Main scraping service that coordinates all scraping operations.
pub struct ScrapingService {
    config: ScrapingConfig,
    cache: Arc<ScrapingCache>,
    client: reqwest::Client,
}

impl ScrapingService {
    /// Create a new scraping service with the given configuration.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(config: ScrapingConfig) -> Result<Self, ScrapingError> {
        let client = Self::build_client(&config)?;
        let cache = Arc::new(ScrapingCache::new(config.cache_config.clone()));

        Ok(Self {
            config,
            cache,
            client,
        })
    }

    /// Create a new scraping service with default configuration.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be created.
    pub fn with_defaults() -> Result<Self, ScrapingError> {
        Self::new(ScrapingConfig::default())
    }

    /// Build an HTTP client with appropriate headers and settings.
    fn build_client(config: &ScrapingConfig) -> Result<reqwest::Client, ScrapingError> {
        use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, USER_AGENT};

        let mut headers = HeaderMap::new();

        // Rotate user agents to avoid detection
        let ua = config.random_user_agent();
        if let Ok(ua_value) = HeaderValue::from_str(&ua) {
            headers.insert(USER_AGENT, ua_value);
        }

        if let Ok(accept) = HeaderValue::from_str(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
        ) {
            headers.insert(ACCEPT, accept);
        }

        if let Ok(lang) = HeaderValue::from_str("en-US,en;q=0.5,fr;q=0.3") {
            headers.insert(ACCEPT_LANGUAGE, lang);
        }

        reqwest::Client::builder()
            .default_headers(headers)
            .timeout(config.request_timeout)
            .connect_timeout(config.connect_timeout)
            .cookie_store(true)
            .gzip(true)
            .brotli(true)
            .deflate(true)
            .build()
            .map_err(|e| ScrapingError::HttpClient(e.to_string()))
    }

    /// Perform a web search using the configured search engine.
    ///
    /// # Errors
    /// Returns an error if the search fails.
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, ScrapingError> {
        // Check cache first
        let cache_key = query.cache_key();
        if let Some(cached) = self.cache.get_search(&cache_key) {
            tracing::debug!("Cache hit for search: {}", query.query);
            return Ok(cached);
        }

        // Perform search based on configured engine
        let results = match self.config.default_engine {
            engines::SearchEngine::DuckDuckGo => {
                engines::duckduckgo::search(&self.client, query, &self.config).await?
            }
            engines::SearchEngine::Brave => {
                engines::brave::search(&self.client, query, &self.config).await?
            }
            engines::SearchEngine::Google => {
                engines::google::search(&self.client, query, &self.config).await?
            }
        };

        // Cache results
        self.cache.set_search(&cache_key, &results);

        Ok(results)
    }

    /// Scrape content from a URL.
    ///
    /// # Errors
    /// Returns an error if scraping fails.
    pub async fn scrape_url(&self, url: &str) -> Result<WebContent, ScrapingError> {
        // Check cache first
        if let Some(cached) = self.cache.get_content(url) {
            tracing::debug!("Cache hit for URL: {url}");
            return Ok(cached);
        }

        let content = content::scrape_page(&self.client, url, &self.config).await?;

        // Cache content
        self.cache.set_content(url, &content);

        Ok(content)
    }

    /// Search for images.
    ///
    /// # Errors
    /// Returns an error if the image search fails.
    pub async fn search_images(
        &self,
        query: &str,
        count: usize,
    ) -> Result<Vec<ImageResult>, ScrapingError> {
        let cache_key = format!("img:{query}:{count}");
        if let Some(cached) = self.cache.get_images(&cache_key) {
            return Ok(cached);
        }

        let results = images::search_images(&self.client, query, count, &self.config).await?;

        self.cache.set_images(&cache_key, &results);

        Ok(results)
    }

    /// Search for videos on YouTube.
    ///
    /// # Errors
    /// Returns an error if the video search fails.
    pub async fn search_videos(
        &self,
        query: &str,
        count: usize,
    ) -> Result<Vec<VideoResult>, ScrapingError> {
        let cache_key = format!("vid:{query}:{count}");
        if let Some(cached) = self.cache.get_videos(&cache_key) {
            return Ok(cached);
        }

        let results = videos::search_youtube(&self.client, query, count, &self.config).await?;

        self.cache.set_videos(&cache_key, &results);

        Ok(results)
    }

    /// Get memory source for web-scraped content.
    #[must_use]
    pub const fn memory_source() -> MemorySource {
        MemorySource::Tool
    }

    /// Get modality for web content.
    #[must_use]
    pub const fn web_modality() -> Modality {
        Modality::Text
    }

    /// Get modality for image content.
    #[must_use]
    pub const fn image_modality() -> Modality {
        Modality::Image
    }

    /// Get modality for video content.
    #[must_use]
    pub const fn video_modality() -> Modality {
        Modality::Video
    }

    /// Clear all caches.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = ScrapingService::with_defaults();
        assert!(service.is_ok());
    }

    #[test]
    fn test_modalities() {
        assert_eq!(ScrapingService::web_modality(), Modality::Text);
        assert_eq!(ScrapingService::image_modality(), Modality::Image);
        assert_eq!(ScrapingService::video_modality(), Modality::Video);
    }
}
