//! Image search functionality.

use scraper::{Html, Selector};

use crate::scraping::config::ScrapingConfig;
use crate::scraping::error::ScrapingError;
use crate::scraping::types::ImageResult;

/// Search for images using DuckDuckGo Images.
///
/// # Errors
/// Returns an error if the search fails.
pub async fn search_images(
    client: &reqwest::Client,
    query: &str,
    count: usize,
    _config: &ScrapingConfig,
) -> Result<Vec<ImageResult>, ScrapingError> {
    // Use DuckDuckGo image search (HTML version)
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "https://duckduckgo.com/?q={}&t=h_&iax=images&ia=images",
        encoded_query
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "Image search returned status: {}",
            response.status()
        )));
    }

    let html = response.text().await?;

    // DuckDuckGo images are loaded via JavaScript, so we need to extract the vqd token
    // and make an API call
    let vqd = extract_vqd(&html)?;

    // Make the actual image API call
    let api_url = format!(
        "https://duckduckgo.com/i.js?l=us-en&o=json&q={}&vqd={}&f=,,,,,&p=1",
        encoded_query, vqd
    );

    let api_response = client.get(&api_url).send().await?;

    if !api_response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "Image API returned status: {}",
            api_response.status()
        )));
    }

    let json: serde_json::Value = api_response.json().await?;
    parse_ddg_images(&json, count)
}

/// Extract the vqd token from DuckDuckGo HTML.
fn extract_vqd(html: &str) -> Result<String, ScrapingError> {
    // Look for vqd in the HTML
    // Pattern: vqd='...' or vqd="..."
    let patterns = [
        r#"vqd='"#,
        r#"vqd=""#,
        r#"vqd=("#,
    ];

    for pattern in patterns {
        if let Some(start) = html.find(pattern) {
            let start = start + pattern.len();
            let end_char = if pattern.ends_with('\'') {
                '\''
            } else if pattern.ends_with('"') {
                '"'
            } else {
                ')'
            };

            if let Some(end) = html[start..].find(end_char) {
                return Ok(html[start..start + end].to_string());
            }
        }
    }

    // Try regex as fallback
    let re = regex::Regex::new(r#"vqd[=:]['"]?([^'"&\)]+)"#)?;
    if let Some(caps) = re.captures(html) {
        if let Some(m) = caps.get(1) {
            return Ok(m.as_str().to_string());
        }
    }

    Err(ScrapingError::ExtractionFailed(
        "Could not extract vqd token from DuckDuckGo".to_string(),
    ))
}

/// Parse DuckDuckGo image results.
fn parse_ddg_images(json: &serde_json::Value, count: usize) -> Result<Vec<ImageResult>, ScrapingError> {
    let mut results = Vec::new();

    if let Some(images) = json.get("results").and_then(|r| r.as_array()) {
        for img in images.iter().take(count) {
            let url = img
                .get("image")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if url.is_empty() {
                continue;
            }

            let thumbnail_url = img
                .get("thumbnail")
                .and_then(|v| v.as_str())
                .map(String::from);

            let title = img
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let source_url = img
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let source_domain = img
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let width = img
                .get("width")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok());

            let height = img
                .get("height")
                .and_then(|v| v.as_u64())
                .and_then(|v| u32::try_from(v).ok());

            results.push(ImageResult {
                url,
                thumbnail_url,
                title,
                source_url,
                source_domain,
                width,
                height,
                format: None,
                size: None,
            });
        }
    }

    Ok(results)
}

/// Search for images using Brave Image Search API.
///
/// # Errors
/// Returns an error if the API key is missing or the request fails.
#[allow(dead_code)]
pub async fn search_images_brave(
    client: &reqwest::Client,
    query: &str,
    count: usize,
    config: &ScrapingConfig,
) -> Result<Vec<ImageResult>, ScrapingError> {
    let api_key = config
        .api_keys
        .brave
        .as_ref()
        .ok_or_else(|| ScrapingError::ApiKeyRequired("Brave Search".to_string()))?;

    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "https://api.search.brave.com/res/v1/images/search?q={}&count={}",
        encoded_query, count
    );

    let response = client
        .get(&url)
        .header("X-Subscription-Token", api_key)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "Brave image search returned status: {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response.json().await?;
    parse_brave_images(&json, count)
}

/// Parse Brave image results.
fn parse_brave_images(json: &serde_json::Value, count: usize) -> Result<Vec<ImageResult>, ScrapingError> {
    let mut results = Vec::new();

    if let Some(images) = json.get("results").and_then(|r| r.as_array()) {
        for img in images.iter().take(count) {
            let url = img
                .get("properties")
                .and_then(|p| p.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if url.is_empty() {
                continue;
            }

            let thumbnail_url = img
                .get("thumbnail")
                .and_then(|t| t.get("src"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let title = img
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let source_url = img
                .get("url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let source_domain = img
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            results.push(ImageResult {
                url,
                thumbnail_url,
                title,
                source_url,
                source_domain,
                width: None,
                height: None,
                format: None,
                size: None,
            });
        }
    }

    Ok(results)
}

/// Fallback: scrape Google Images HTML (for when APIs aren't available).
#[allow(dead_code)]
pub async fn scrape_google_images(
    client: &reqwest::Client,
    query: &str,
    count: usize,
) -> Result<Vec<ImageResult>, ScrapingError> {
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "https://www.google.com/search?q={}&tbm=isch&hl=en",
        encoded_query
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "Google Images returned status: {}",
            response.status()
        )));
    }

    let html = response.text().await?;
    parse_google_images_html(&html, count)
}

/// Parse Google Images HTML results.
fn parse_google_images_html(html: &str, count: usize) -> Result<Vec<ImageResult>, ScrapingError> {
    let document = Html::parse_document(html);
    let mut results = Vec::new();

    // Google Images uses data attributes for image URLs
    if let Ok(selector) = Selector::parse("img[data-src]") {
        for element in document.select(&selector).take(count) {
            let url = element
                .value()
                .attr("data-src")
                .or_else(|| element.value().attr("src"))
                .unwrap_or_default()
                .to_string();

            if url.is_empty() || url.starts_with("data:") {
                continue;
            }

            let alt = element
                .value()
                .attr("alt")
                .unwrap_or_default()
                .to_string();

            results.push(ImageResult {
                url: url.clone(),
                thumbnail_url: Some(url),
                title: alt,
                source_url: String::new(),
                source_domain: String::new(),
                width: None,
                height: None,
                format: None,
                size: None,
            });
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ddg_images() {
        let json = serde_json::json!({
            "results": [
                {
                    "image": "https://example.com/image1.jpg",
                    "thumbnail": "https://example.com/thumb1.jpg",
                    "title": "Test Image 1",
                    "url": "https://example.com/page1",
                    "source": "example.com",
                    "width": 800,
                    "height": 600
                },
                {
                    "image": "https://example.com/image2.jpg",
                    "title": "Test Image 2",
                    "url": "https://example.com/page2",
                    "source": "example.com"
                }
            ]
        });

        let results = parse_ddg_images(&json, 10).ok();
        assert!(results.is_some());
        let results = results.unwrap_or_default();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].url, "https://example.com/image1.jpg");
        assert_eq!(results[0].width, Some(800));
    }
}
