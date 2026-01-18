//! Google Custom Search API implementation.
//!
//! Uses the Google Custom Search JSON API.
//! Requires an API key and Custom Search Engine ID from Google Cloud Console.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::scraping::config::ScrapingConfig;
use crate::scraping::error::ScrapingError;
use crate::scraping::types::{
    SafeSearch, SearchQuery, SearchResult, SearchResultMetadata, SearchResultType, TimeFilter,
};

/// Google Custom Search API base URL.
const GOOGLE_API_URL: &str = "https://www.googleapis.com/customsearch/v1";

/// Perform a search using Google Custom Search API.
///
/// # Errors
/// Returns an error if the API keys are missing or the request fails.
pub async fn search(
    client: &reqwest::Client,
    query: &SearchQuery,
    config: &ScrapingConfig,
) -> Result<Vec<SearchResult>, ScrapingError> {
    let api_key = config
        .api_keys
        .google_api_key
        .as_ref()
        .ok_or_else(|| ScrapingError::ApiKeyRequired("Google API Key".to_string()))?;

    let cx = config
        .api_keys
        .google_cx
        .as_ref()
        .ok_or_else(|| ScrapingError::ApiKeyRequired("Google Custom Search Engine ID".to_string()))?;

    let url = build_url(query, api_key, cx)?;

    let response = client.get(&url).send().await?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(ScrapingError::RateLimited(60));
    }

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ScrapingError::AccessDenied(
            "Google API quota exceeded or invalid key".to_string(),
        ));
    }

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "Google Search returned status: {}",
            response.status()
        )));
    }

    let body: GoogleResponse = response.json().await?;
    Ok(parse_response(body, query.max_results))
}

/// Build the API URL with query parameters.
fn build_url(query: &SearchQuery, api_key: &str, cx: &str) -> Result<String, ScrapingError> {
    let mut url = url::Url::parse(GOOGLE_API_URL)?;

    {
        let mut params = url.query_pairs_mut();
        params.append_pair("key", api_key);
        params.append_pair("cx", cx);
        params.append_pair("q", &query.query);
        params.append_pair("num", &query.max_results.min(10).to_string()); // Google max is 10

        // Safe search
        let safe = match query.safe_search {
            SafeSearch::Off => "off",
            SafeSearch::Moderate => "medium",
            SafeSearch::Strict => "high",
        };
        params.append_pair("safe", safe);

        // Date restrict (time filter)
        if let Some(time) = &query.time_filter {
            let date_restrict = match time {
                TimeFilter::Day => "d1",
                TimeFilter::Week => "w1",
                TimeFilter::Month => "m1",
                TimeFilter::Year => "y1",
                TimeFilter::AllTime => "",
            };
            if !date_restrict.is_empty() {
                params.append_pair("dateRestrict", date_restrict);
            }
        }

        // Language
        if let Some(lang) = &query.language {
            params.append_pair("lr", &format!("lang_{lang}"));
        }

        // Region/Country
        if let Some(region) = &query.region {
            params.append_pair("gl", region);
        }
    }

    Ok(url.to_string())
}

/// Parse the Google API response.
fn parse_response(response: GoogleResponse, max_results: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    if let Some(items) = response.items {
        for (i, item) in items.into_iter().take(max_results).enumerate() {
            let relevance = 1.0 - (i as f32 * 0.05).min(0.9);

            let published_at = item
                .pagemap
                .as_ref()
                .and_then(|pm| pm.metatags.as_ref())
                .and_then(|tags| tags.first())
                .and_then(|tag| tag.article_published_time.as_ref())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            let thumbnail = item
                .pagemap
                .as_ref()
                .and_then(|pm| pm.cse_thumbnail.as_ref())
                .and_then(|thumbs| thumbs.first())
                .map(|t| t.src.clone());

            results.push(SearchResult {
                title: item.title,
                url: item.link.clone(),
                description: item.snippet.unwrap_or_default(),
                domain: item.display_link,
                published_at,
                result_type: SearchResultType::Web,
                relevance,
                metadata: SearchResultMetadata {
                    favicon: None,
                    thumbnail,
                    author: None,
                    reading_time: None,
                    word_count: None,
                    language: None,
                },
            });
        }
    }

    results
}

// Google API response structures

#[derive(Debug, Deserialize)]
struct GoogleResponse {
    items: Option<Vec<GoogleItem>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleItem {
    title: String,
    link: String,
    display_link: String,
    snippet: Option<String>,
    pagemap: Option<PageMap>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageMap {
    cse_thumbnail: Option<Vec<CseThumbnail>>,
    metatags: Option<Vec<MetaTags>>,
}

#[derive(Debug, Deserialize)]
struct CseThumbnail {
    src: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetaTags {
    #[serde(rename = "article:published_time")]
    article_published_time: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let query = SearchQuery::new("rust programming")
            .with_max_results(10)
            .with_safe_search(SafeSearch::Strict);

        let url = build_url(&query, "test-key", "test-cx").ok();
        assert!(url.is_some());
        let url = url.as_deref().unwrap_or_default();
        assert!(url.contains("key=test-key"));
        assert!(url.contains("cx=test-cx"));
        assert!(url.contains("q=rust+programming") || url.contains("q=rust%20programming"));
        assert!(url.contains("safe=high"));
    }
}
