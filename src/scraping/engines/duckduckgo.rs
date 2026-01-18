//! DuckDuckGo search engine implementation.
//!
//! Uses DuckDuckGo HTML search (no API key required).

use scraper::{Html, Selector};
use url::Url;

use crate::scraping::config::ScrapingConfig;
use crate::scraping::error::ScrapingError;
use crate::scraping::types::{
    SafeSearch, SearchQuery, SearchResult, SearchResultMetadata, SearchResultType, TimeFilter,
};

/// Base URL for DuckDuckGo HTML search.
const DDG_HTML_URL: &str = "https://html.duckduckgo.com/html/";

/// Perform a search on DuckDuckGo.
///
/// # Errors
/// Returns an error if the search request fails or parsing fails.
pub async fn search(
    client: &reqwest::Client,
    query: &SearchQuery,
    _config: &ScrapingConfig,
) -> Result<Vec<SearchResult>, ScrapingError> {
    let params = build_params(query);

    let response = client
        .post(DDG_HTML_URL)
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "DuckDuckGo returned status: {}",
            response.status()
        )));
    }

    let html = response.text().await?;
    parse_results(&html, query.max_results)
}

/// Build form parameters for DuckDuckGo search.
fn build_params(query: &SearchQuery) -> Vec<(&'static str, String)> {
    let mut params = vec![("q", query.query.clone()), ("b", String::new())];

    // Safe search
    let kp = match query.safe_search {
        SafeSearch::Off => "-2",
        SafeSearch::Moderate => "-1",
        SafeSearch::Strict => "1",
    };
    params.push(("kp", kp.to_string()));

    // Time filter
    if let Some(time) = &query.time_filter {
        let df = match time {
            TimeFilter::Day => "d",
            TimeFilter::Week => "w",
            TimeFilter::Month => "m",
            TimeFilter::Year => "y",
            TimeFilter::AllTime => "",
        };
        if !df.is_empty() {
            params.push(("df", df.to_string()));
        }
    }

    // Region
    if let Some(region) = &query.region {
        params.push(("kl", format!("{region}-{region}")));
    }

    params
}

/// Parse DuckDuckGo HTML results.
fn parse_results(html: &str, max_results: usize) -> Result<Vec<SearchResult>, ScrapingError> {
    let document = Html::parse_document(html);

    // DuckDuckGo result selectors
    let result_selector = Selector::parse(".result")
        .map_err(|e| ScrapingError::HtmlParse(format!("Invalid selector: {e:?}")))?;
    let title_selector = Selector::parse(".result__a")
        .map_err(|e| ScrapingError::HtmlParse(format!("Invalid selector: {e:?}")))?;
    let snippet_selector = Selector::parse(".result__snippet")
        .map_err(|e| ScrapingError::HtmlParse(format!("Invalid selector: {e:?}")))?;
    let url_selector = Selector::parse(".result__url")
        .map_err(|e| ScrapingError::HtmlParse(format!("Invalid selector: {e:?}")))?;

    let mut results = Vec::new();
    let mut position = 0;

    for element in document.select(&result_selector) {
        if results.len() >= max_results {
            break;
        }

        // Get title
        let title = element
            .select(&title_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        if title.is_empty() {
            continue;
        }

        // Get URL from href attribute
        let url = element
            .select(&title_selector)
            .next()
            .and_then(|e| e.value().attr("href"))
            .map(|href| extract_url_from_ddg_redirect(href))
            .unwrap_or_default();

        if url.is_empty() {
            continue;
        }

        // Get snippet
        let description = element
            .select(&snippet_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        // Get display URL for domain
        let display_url = element
            .select(&url_selector)
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let domain = extract_domain(&url).unwrap_or_else(|| display_url.clone());

        position += 1;
        // Relevance decreases with position
        let relevance = 1.0 - (position as f32 * 0.05).min(0.9);

        results.push(SearchResult {
            title,
            url,
            description,
            domain,
            published_at: None,
            result_type: SearchResultType::Web,
            relevance,
            metadata: SearchResultMetadata::default(),
        });
    }

    if results.is_empty() {
        tracing::warn!("No results found in DuckDuckGo HTML response");
    }

    Ok(results)
}

/// Extract the actual URL from DuckDuckGo's redirect URL.
fn extract_url_from_ddg_redirect(href: &str) -> String {
    // DuckDuckGo uses redirect URLs like:
    // //duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com&rut=...
    if let Some(uddg_start) = href.find("uddg=") {
        let start = uddg_start + 5;
        let end = href[start..].find('&').map_or(href.len(), |i| start + i);
        let encoded = &href[start..end];
        urlencoding::decode(encoded)
            .map(|s| s.into_owned())
            .unwrap_or_else(|_| encoded.to_string())
    } else if href.starts_with("http") {
        href.to_string()
    } else if href.starts_with("//") {
        format!("https:{href}")
    } else {
        href.to_string()
    }
}

/// Extract domain from URL.
fn extract_domain(url: &str) -> Option<String> {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_params() {
        let query = SearchQuery::new("rust programming")
            .with_safe_search(SafeSearch::Strict)
            .with_time_filter(TimeFilter::Week);

        let params = build_params(&query);

        assert!(params.iter().any(|(k, v)| *k == "q" && v == "rust programming"));
        assert!(params.iter().any(|(k, v)| *k == "kp" && v == "1"));
        assert!(params.iter().any(|(k, v)| *k == "df" && v == "w"));
    }

    #[test]
    fn test_extract_url_from_ddg_redirect() {
        let redirect = "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpage&rut=123";
        let url = extract_url_from_ddg_redirect(redirect);
        assert_eq!(url, "https://example.com/page");
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(
            extract_domain("https://www.example.com/page"),
            Some("www.example.com".to_string())
        );
        assert_eq!(extract_domain("invalid"), None);
    }
}
