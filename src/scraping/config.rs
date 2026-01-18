//! Configuration for the scraping module.

use std::time::Duration;

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::engines::SearchEngine;

/// Configuration for the scraping service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScrapingConfig {
    /// Default search engine to use.
    pub default_engine: SearchEngine,
    /// Request timeout.
    #[serde(with = "duration_serde")]
    pub request_timeout: Duration,
    /// Connection timeout.
    #[serde(with = "duration_serde")]
    pub connect_timeout: Duration,
    /// Maximum retries for failed requests.
    pub max_retries: u32,
    /// Delay between retries in milliseconds.
    pub retry_delay_ms: u64,
    /// Rate limit: max requests per second.
    pub rate_limit_per_second: f32,
    /// User agents to rotate.
    pub user_agents: Vec<String>,
    /// Cache configuration.
    pub cache_config: CacheConfig,
    /// Maximum content length to download (bytes).
    pub max_content_length: usize,
    /// Whether to follow redirects.
    pub follow_redirects: bool,
    /// Maximum number of redirects to follow.
    pub max_redirects: usize,
    /// API keys for various services.
    pub api_keys: ApiKeys,
}

impl Default for ScrapingConfig {
    fn default() -> Self {
        Self {
            default_engine: SearchEngine::DuckDuckGo,
            request_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            max_retries: 3,
            retry_delay_ms: 1000,
            rate_limit_per_second: 2.0,
            user_agents: default_user_agents(),
            cache_config: CacheConfig::default(),
            max_content_length: 10 * 1024 * 1024, // 10 MB
            follow_redirects: true,
            max_redirects: 10,
            api_keys: ApiKeys::default(),
        }
    }
}

impl ScrapingConfig {
    /// Create a new config with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default search engine.
    #[must_use]
    pub const fn with_engine(mut self, engine: SearchEngine) -> Self {
        self.default_engine = engine;
        self
    }

    /// Set request timeout.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set Brave API key.
    #[must_use]
    pub fn with_brave_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_keys.brave = Some(key.into());
        self
    }

    /// Set Google API key and search engine ID.
    #[must_use]
    pub fn with_google_api(
        mut self,
        api_key: impl Into<String>,
        cx: impl Into<String>,
    ) -> Self {
        self.api_keys.google_api_key = Some(api_key.into());
        self.api_keys.google_cx = Some(cx.into());
        self
    }

    /// Get a random user agent from the rotation list.
    #[must_use]
    pub fn random_user_agent(&self) -> String {
        if self.user_agents.is_empty() {
            return default_user_agents()[0].clone();
        }
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..self.user_agents.len());
        self.user_agents[idx].clone()
    }
}

/// Cache configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled.
    pub enabled: bool,
    /// TTL for search results (seconds).
    pub search_ttl_seconds: u64,
    /// TTL for scraped content (seconds).
    pub content_ttl_seconds: u64,
    /// TTL for image results (seconds).
    pub image_ttl_seconds: u64,
    /// TTL for video results (seconds).
    pub video_ttl_seconds: u64,
    /// Maximum cache size (number of entries).
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            search_ttl_seconds: 3600,           // 1 hour
            content_ttl_seconds: 86400,         // 24 hours
            image_ttl_seconds: 86400,           // 24 hours
            video_ttl_seconds: 3600,            // 1 hour
            max_entries: 1000,
        }
    }
}

/// API keys for various services.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ApiKeys {
    /// Brave Search API key.
    pub brave: Option<String>,
    /// Google Custom Search API key.
    pub google_api_key: Option<String>,
    /// Google Custom Search Engine ID.
    pub google_cx: Option<String>,
    /// YouTube Data API key.
    pub youtube: Option<String>,
    /// GitHub API token.
    pub github: Option<String>,
}

/// Default user agents for rotation.
fn default_user_agents() -> Vec<String> {
    vec![
        // Chrome on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        // Chrome on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        // Firefox on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
        // Firefox on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
        // Safari on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15".to_string(),
        // Edge on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0".to_string(),
        // Chrome on Linux
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
        // Firefox on Linux
        "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
    ]
}

/// Serde module for Duration serialization.
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScrapingConfig::default();
        assert_eq!(config.default_engine, SearchEngine::DuckDuckGo);
        assert_eq!(config.max_retries, 3);
        assert!(config.cache_config.enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = ScrapingConfig::new()
            .with_engine(SearchEngine::Brave)
            .with_timeout(Duration::from_secs(60))
            .with_brave_api_key("test-key");

        assert_eq!(config.default_engine, SearchEngine::Brave);
        assert_eq!(config.request_timeout, Duration::from_secs(60));
        assert_eq!(config.api_keys.brave, Some("test-key".to_string()));
    }

    #[test]
    fn test_random_user_agent() {
        let config = ScrapingConfig::default();
        let ua = config.random_user_agent();
        assert!(!ua.is_empty());
        assert!(ua.contains("Mozilla"));
    }
}
