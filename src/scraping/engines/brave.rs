//! Brave Search API implementation.
//!
//! Uses the Brave Search API for high-quality results.
//! Requires an API key from https://brave.com/search/api/

use serde::Deserialize;

use crate::scraping::config::ScrapingConfig;
use crate::scraping::error::ScrapingError;
use crate::scraping::types::{
    SafeSearch, SearchQuery, SearchResult, SearchResultMetadata, SearchResultType, TimeFilter,
};

/// Brave Search API base URL.
const BRAVE_API_URL: &str = "https://api.search.brave.com/res/v1/web/search";

/// Perform a search using Brave Search API.
///
/// # Errors
/// Returns an error if the API key is missing or the request fails.
pub async fn search(
    client: &reqwest::Client,
    query: &SearchQuery,
    config: &ScrapingConfig,
) -> Result<Vec<SearchResult>, ScrapingError> {
    let api_key = config
        .api_keys
        .brave
        .as_ref()
        .ok_or_else(|| ScrapingError::ApiKeyRequired("Brave Search".to_string()))?;

    let url = build_url(query)?;

    let response = client
        .get(&url)
        .header("X-Subscription-Token", api_key)
        .header("Accept", "application/json")
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(ScrapingError::RateLimited(60));
    }

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ScrapingError::AccessDenied("Invalid Brave API key".to_string()));
    }

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "Brave Search returned status: {}",
            response.status()
        )));
    }

    let body: BraveResponse = response.json().await?;
    Ok(parse_response(body, query.max_results))
}

/// Build the API URL with query parameters.
fn build_url(query: &SearchQuery) -> Result<String, ScrapingError> {
    let mut url = url::Url::parse(BRAVE_API_URL)?;

    {
        let mut params = url.query_pairs_mut();
        params.append_pair("q", &query.query);
        params.append_pair("count", &query.max_results.to_string());

        // Safe search
        let safesearch = match query.safe_search {
            SafeSearch::Off => "off",
            SafeSearch::Moderate => "moderate",
            SafeSearch::Strict => "strict",
        };
        params.append_pair("safesearch", safesearch);

        // Freshness (time filter)
        if let Some(time) = &query.time_filter {
            let freshness = match time {
                TimeFilter::Day => "pd",
                TimeFilter::Week => "pw",
                TimeFilter::Month => "pm",
                TimeFilter::Year => "py",
                TimeFilter::AllTime => "",
            };
            if !freshness.is_empty() {
                params.append_pair("freshness", freshness);
            }
        }

        // Country
        if let Some(region) = &query.region {
            params.append_pair("country", region);
        }

        // Language
        if let Some(lang) = &query.language {
            params.append_pair("search_lang", lang);
        }
    }

    Ok(url.to_string())
}

/// Parse the Brave API response.
fn parse_response(response: BraveResponse, max_results: usize) -> Vec<SearchResult> {
    let mut results = Vec::new();

    if let Some(web) = response.web {
        for (i, result) in web.results.into_iter().take(max_results).enumerate() {
            let relevance = 1.0 - (i as f32 * 0.05).min(0.9);

            results.push(SearchResult {
                title: result.title,
                url: result.url.clone(),
                description: result.description.unwrap_or_default(),
                domain: extract_domain(&result.url),
                published_at: None,
                result_type: SearchResultType::Web,
                relevance,
                metadata: SearchResultMetadata {
                    favicon: result.meta_url.and_then(|m| m.favicon),
                    thumbnail: result.thumbnail.map(|t| t.src),
                    author: None,
                    reading_time: None,
                    word_count: None,
                    language: result.language,
                },
            });
        }
    }

    results
}

/// Extract domain from URL.
fn extract_domain(url: &str) -> String {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
        .unwrap_or_default()
}

// Brave API response structures

#[derive(Debug, Deserialize)]
struct BraveResponse {
    web: Option<WebResults>,
}

#[derive(Debug, Deserialize)]
struct WebResults {
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title: String,
    url: String,
    description: Option<String>,
    language: Option<String>,
    meta_url: Option<MetaUrl>,
    thumbnail: Option<Thumbnail>,
}

#[derive(Debug, Deserialize)]
struct MetaUrl {
    favicon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Thumbnail {
    src: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let query = SearchQuery::new("rust programming")
            .with_max_results(10)
            .with_safe_search(SafeSearch::Moderate)
            .with_region("us");

        let url = build_url(&query).ok();
        assert!(url.is_some());
        let url = url.as_deref().unwrap_or_default();
        assert!(url.contains("q=rust+programming") || url.contains("q=rust%20programming"));
        assert!(url.contains("count=10"));
        assert!(url.contains("safesearch=moderate"));
        assert!(url.contains("country=us"));
    }
}
