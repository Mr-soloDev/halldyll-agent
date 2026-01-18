//! Core types for scraping results.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A search query with parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The search query string.
    pub query: String,
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Language filter (e.g., "en", "fr").
    pub language: Option<String>,
    /// Region filter (e.g., "us", "fr").
    pub region: Option<String>,
    /// Time filter for results.
    pub time_filter: Option<TimeFilter>,
    /// Safe search setting.
    pub safe_search: SafeSearch,
    /// Type of results to search for.
    pub result_type: SearchResultType,
}

impl SearchQuery {
    /// Create a new search query with default settings.
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            max_results: 10,
            language: None,
            region: None,
            time_filter: None,
            safe_search: SafeSearch::Moderate,
            result_type: SearchResultType::Web,
        }
    }

    /// Set max results.
    #[must_use]
    pub const fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Set language filter.
    #[must_use]
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Set region filter.
    #[must_use]
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set time filter.
    #[must_use]
    pub const fn with_time_filter(mut self, filter: TimeFilter) -> Self {
        self.time_filter = Some(filter);
        self
    }

    /// Set safe search level.
    #[must_use]
    pub const fn with_safe_search(mut self, safe: SafeSearch) -> Self {
        self.safe_search = safe;
        self
    }

    /// Set result type.
    #[must_use]
    pub const fn with_result_type(mut self, result_type: SearchResultType) -> Self {
        self.result_type = result_type;
        self
    }

    /// Generate a cache key for this query.
    #[must_use]
    pub fn cache_key(&self) -> String {
        format!(
            "search:{}:{}:{}:{}:{:?}:{:?}",
            self.query,
            self.max_results,
            self.language.as_deref().unwrap_or("any"),
            self.region.as_deref().unwrap_or("any"),
            self.time_filter,
            self.result_type
        )
    }
}

/// Time filter for search results.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum TimeFilter {
    /// Results from the past day.
    Day,
    /// Results from the past week.
    Week,
    /// Results from the past month.
    Month,
    /// Results from the past year.
    Year,
    /// All time (no filter).
    AllTime,
}

/// Safe search levels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SafeSearch {
    /// No filtering.
    Off,
    /// Moderate filtering (default).
    #[default]
    Moderate,
    /// Strict filtering.
    Strict,
}

/// Type of search results.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SearchResultType {
    /// Web pages (default).
    #[default]
    Web,
    /// Images.
    Images,
    /// Videos.
    Videos,
    /// News articles.
    News,
    /// Code/repositories.
    Code,
}

/// A single search result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    /// Title of the result.
    pub title: String,
    /// URL of the result.
    pub url: String,
    /// Description or snippet.
    pub description: String,
    /// Source domain.
    pub domain: String,
    /// When the result was published (if available).
    pub published_at: Option<DateTime<Utc>>,
    /// Result type.
    pub result_type: SearchResultType,
    /// Relevance score (0.0 - 1.0).
    pub relevance: f32,
    /// Additional metadata.
    pub metadata: SearchResultMetadata,
}

/// Additional metadata for search results.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchResultMetadata {
    /// Favicon URL.
    pub favicon: Option<String>,
    /// Thumbnail URL (for images/videos).
    pub thumbnail: Option<String>,
    /// Author name.
    pub author: Option<String>,
    /// Reading time estimate in minutes.
    pub reading_time: Option<u32>,
    /// Word count.
    pub word_count: Option<u32>,
    /// Language of the content.
    pub language: Option<String>,
}

/// Scraped web content.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebContent {
    /// Original URL.
    pub url: String,
    /// Final URL after redirects.
    pub final_url: String,
    /// Page title.
    pub title: String,
    /// Main text content.
    pub text: String,
    /// HTML content (if preserved).
    pub html: Option<String>,
    /// Meta description.
    pub description: Option<String>,
    /// Author.
    pub author: Option<String>,
    /// Publication date.
    pub published_at: Option<DateTime<Utc>>,
    /// Language.
    pub language: Option<String>,
    /// Images found on the page.
    pub images: Vec<ImageInfo>,
    /// Links found on the page.
    pub links: Vec<LinkInfo>,
    /// When the content was scraped.
    pub scraped_at: DateTime<Utc>,
    /// Content type.
    pub content_type: String,
    /// Word count.
    pub word_count: usize,
}

impl WebContent {
    /// Create a new empty web content.
    #[must_use]
    pub fn empty(url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            url: url.clone(),
            final_url: url,
            title: String::new(),
            text: String::new(),
            html: None,
            description: None,
            author: None,
            published_at: None,
            language: None,
            images: Vec::new(),
            links: Vec::new(),
            scraped_at: Utc::now(),
            content_type: String::from("text/html"),
            word_count: 0,
        }
    }

    /// Get a summary of the content (first N characters).
    #[must_use]
    pub fn summary(&self, max_chars: usize) -> String {
        if self.text.len() <= max_chars {
            self.text.clone()
        } else {
            let mut summary = self.text.chars().take(max_chars).collect::<String>();
            summary.push_str("...");
            summary
        }
    }
}

/// Information about an image found on a page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageInfo {
    /// Image URL.
    pub url: String,
    /// Alt text.
    pub alt: Option<String>,
    /// Width in pixels.
    pub width: Option<u32>,
    /// Height in pixels.
    pub height: Option<u32>,
}

/// Information about a link found on a page.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinkInfo {
    /// Link URL.
    pub url: String,
    /// Link text.
    pub text: String,
    /// Is external link.
    pub is_external: bool,
}

/// Image search result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageResult {
    /// Image URL.
    pub url: String,
    /// Thumbnail URL.
    pub thumbnail_url: Option<String>,
    /// Image title/description.
    pub title: String,
    /// Source page URL.
    pub source_url: String,
    /// Source domain.
    pub source_domain: String,
    /// Image width.
    pub width: Option<u32>,
    /// Image height.
    pub height: Option<u32>,
    /// Image format (jpg, png, etc.).
    pub format: Option<String>,
    /// File size in bytes.
    pub size: Option<u64>,
}

/// Video search result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoResult {
    /// Video ID (platform-specific).
    pub id: String,
    /// Video title.
    pub title: String,
    /// Video description.
    pub description: String,
    /// Video URL.
    pub url: String,
    /// Embed URL.
    pub embed_url: Option<String>,
    /// Thumbnail URL.
    pub thumbnail_url: Option<String>,
    /// Channel/uploader name.
    pub channel: String,
    /// Channel URL.
    pub channel_url: Option<String>,
    /// Duration in seconds.
    pub duration: Option<u32>,
    /// View count.
    pub views: Option<u64>,
    /// Like count.
    pub likes: Option<u64>,
    /// Upload date.
    pub published_at: Option<DateTime<Utc>>,
    /// Video platform.
    pub platform: VideoPlatform,
}

/// Video hosting platform.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum VideoPlatform {
    /// YouTube.
    #[default]
    YouTube,
    /// Vimeo.
    Vimeo,
    /// Dailymotion.
    Dailymotion,
    /// Other platform.
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_builder() {
        let query = SearchQuery::new("rust programming")
            .with_max_results(20)
            .with_language("en")
            .with_region("us")
            .with_time_filter(TimeFilter::Week)
            .with_safe_search(SafeSearch::Strict);

        assert_eq!(query.query, "rust programming");
        assert_eq!(query.max_results, 20);
        assert_eq!(query.language, Some("en".to_string()));
        assert_eq!(query.region, Some("us".to_string()));
        assert_eq!(query.time_filter, Some(TimeFilter::Week));
        assert_eq!(query.safe_search, SafeSearch::Strict);
    }

    #[test]
    fn test_cache_key_generation() {
        let query = SearchQuery::new("test query");
        let key = query.cache_key();
        assert!(key.contains("test query"));
        assert!(key.starts_with("search:"));
    }

    #[test]
    fn test_web_content_summary() {
        let mut content = WebContent::empty("https://example.com");
        content.text = "This is a long text that should be truncated for the summary.".to_string();

        let summary = content.summary(20);
        assert_eq!(summary, "This is a long text ...");
    }
}
