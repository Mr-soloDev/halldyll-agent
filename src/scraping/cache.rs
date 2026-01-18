//! Caching system for scraping results.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;

use crate::scraping::config::CacheConfig;
use crate::scraping::types::{ImageResult, SearchResult, VideoResult, WebContent};

/// Cache entry with TTL.
#[derive(Clone)]
struct CacheEntry<T> {
    data: T,
    expires_at: Instant,
}

impl<T: Clone> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Thread-safe cache for scraping results.
pub struct ScrapingCache {
    config: CacheConfig,
    search_cache: Arc<DashMap<String, CacheEntry<Vec<SearchResult>>>>,
    content_cache: Arc<DashMap<String, CacheEntry<WebContent>>>,
    image_cache: Arc<DashMap<String, CacheEntry<Vec<ImageResult>>>>,
    video_cache: Arc<DashMap<String, CacheEntry<Vec<VideoResult>>>>,
}

impl ScrapingCache {
    /// Create a new cache with the given configuration.
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            search_cache: Arc::new(DashMap::new()),
            content_cache: Arc::new(DashMap::new()),
            image_cache: Arc::new(DashMap::new()),
            video_cache: Arc::new(DashMap::new()),
        }
    }

    /// Get cached search results.
    #[must_use]
    pub fn get_search(&self, key: &str) -> Option<Vec<SearchResult>> {
        if !self.config.enabled {
            return None;
        }

        self.search_cache.get(key).and_then(|entry| {
            if entry.is_expired() {
                drop(entry);
                self.search_cache.remove(key);
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    /// Cache search results.
    pub fn set_search(&self, key: &str, results: &[SearchResult]) {
        if !self.config.enabled {
            return;
        }

        self.enforce_max_entries(&self.search_cache);

        let ttl = Duration::from_secs(self.config.search_ttl_seconds);
        self.search_cache
            .insert(key.to_string(), CacheEntry::new(results.to_vec(), ttl));
    }

    /// Get cached web content.
    #[must_use]
    pub fn get_content(&self, url: &str) -> Option<WebContent> {
        if !self.config.enabled {
            return None;
        }

        self.content_cache.get(url).and_then(|entry| {
            if entry.is_expired() {
                drop(entry);
                self.content_cache.remove(url);
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    /// Cache web content.
    pub fn set_content(&self, url: &str, content: &WebContent) {
        if !self.config.enabled {
            return;
        }

        self.enforce_max_entries(&self.content_cache);

        let ttl = Duration::from_secs(self.config.content_ttl_seconds);
        self.content_cache
            .insert(url.to_string(), CacheEntry::new(content.clone(), ttl));
    }

    /// Get cached image results.
    #[must_use]
    pub fn get_images(&self, key: &str) -> Option<Vec<ImageResult>> {
        if !self.config.enabled {
            return None;
        }

        self.image_cache.get(key).and_then(|entry| {
            if entry.is_expired() {
                drop(entry);
                self.image_cache.remove(key);
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    /// Cache image results.
    pub fn set_images(&self, key: &str, results: &[ImageResult]) {
        if !self.config.enabled {
            return;
        }

        self.enforce_max_entries(&self.image_cache);

        let ttl = Duration::from_secs(self.config.image_ttl_seconds);
        self.image_cache
            .insert(key.to_string(), CacheEntry::new(results.to_vec(), ttl));
    }

    /// Get cached video results.
    #[must_use]
    pub fn get_videos(&self, key: &str) -> Option<Vec<VideoResult>> {
        if !self.config.enabled {
            return None;
        }

        self.video_cache.get(key).and_then(|entry| {
            if entry.is_expired() {
                drop(entry);
                self.video_cache.remove(key);
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    /// Cache video results.
    pub fn set_videos(&self, key: &str, results: &[VideoResult]) {
        if !self.config.enabled {
            return;
        }

        self.enforce_max_entries(&self.video_cache);

        let ttl = Duration::from_secs(self.config.video_ttl_seconds);
        self.video_cache
            .insert(key.to_string(), CacheEntry::new(results.to_vec(), ttl));
    }

    /// Clear all caches.
    pub fn clear(&self) {
        self.search_cache.clear();
        self.content_cache.clear();
        self.image_cache.clear();
        self.video_cache.clear();
    }

    /// Get cache statistics.
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            search_entries: self.search_cache.len(),
            content_entries: self.content_cache.len(),
            image_entries: self.image_cache.len(),
            video_entries: self.video_cache.len(),
        }
    }

    /// Remove expired entries from all caches.
    pub fn cleanup_expired(&self) {
        self.cleanup_expired_from(&self.search_cache);
        self.cleanup_expired_from(&self.content_cache);
        self.cleanup_expired_from(&self.image_cache);
        self.cleanup_expired_from(&self.video_cache);
    }

    /// Remove expired entries from a specific cache.
    fn cleanup_expired_from<T: Clone>(&self, cache: &DashMap<String, CacheEntry<T>>) {
        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|entry| entry.is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            cache.remove(&key);
        }
    }

    /// Enforce maximum entries limit by removing oldest entries.
    fn enforce_max_entries<T: Clone>(&self, cache: &DashMap<String, CacheEntry<T>>) {
        let max_per_cache = self.config.max_entries / 4;
        if cache.len() >= max_per_cache {
            // Remove expired entries first
            self.cleanup_expired_from(cache);

            // If still over limit, remove oldest entries
            if cache.len() >= max_per_cache {
                let to_remove = cache.len() - max_per_cache + 1;
                let keys: Vec<String> = cache
                    .iter()
                    .take(to_remove)
                    .map(|entry| entry.key().clone())
                    .collect();
                for key in keys {
                    cache.remove(&key);
                }
            }
        }
    }
}

/// Cache statistics.
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    /// Number of search result entries.
    pub search_entries: usize,
    /// Number of content entries.
    pub content_entries: usize,
    /// Number of image result entries.
    pub image_entries: usize,
    /// Number of video result entries.
    pub video_entries: usize,
}

impl CacheStats {
    /// Total number of entries across all caches.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.search_entries + self.content_entries + self.image_entries + self.video_entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_search_results() {
        let config = CacheConfig::default();
        let cache = ScrapingCache::new(config);

        let results = vec![SearchResult {
            title: "Test".to_string(),
            url: "https://test.com".to_string(),
            description: "Test description".to_string(),
            domain: "test.com".to_string(),
            published_at: None,
            result_type: crate::scraping::types::SearchResultType::Web,
            relevance: 1.0,
            metadata: crate::scraping::types::SearchResultMetadata::default(),
        }];

        cache.set_search("test_key", &results);

        let cached = cache.get_search("test_key");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap_or_default().len(), 1);
    }

    #[test]
    fn test_cache_stats() {
        let config = CacheConfig::default();
        let cache = ScrapingCache::new(config);

        let stats = cache.stats();
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_cache_disabled() {
        let mut config = CacheConfig::default();
        config.enabled = false;
        let cache = ScrapingCache::new(config);

        let results: Vec<SearchResult> = vec![];
        cache.set_search("test_key", &results);

        let cached = cache.get_search("test_key");
        assert!(cached.is_none());
    }
}
