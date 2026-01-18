//! HTML content scraping and extraction.

use chrono::{DateTime, Utc};
use scraper::{Html, Selector};
use url::Url;

use crate::scraping::config::ScrapingConfig;
use crate::scraping::error::ScrapingError;
use crate::scraping::types::{ImageInfo, LinkInfo, WebContent};

/// Content scraper for web pages.
pub struct ContentScraper {
    /// Selectors for main content extraction.
    content_selectors: Vec<&'static str>,
}

impl Default for ContentScraper {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentScraper {
    /// Create a new content scraper with default selectors.
    #[must_use]
    pub fn new() -> Self {
        Self {
            content_selectors: vec![
                "article",
                "main",
                "[role='main']",
                ".post-content",
                ".article-content",
                ".entry-content",
                ".content",
                "#content",
                ".post",
                ".article",
            ],
        }
    }
}

/// Scrape a web page and extract its content.
///
/// # Errors
/// Returns an error if the request fails or content extraction fails.
pub async fn scrape_page(
    client: &reqwest::Client,
    url: &str,
    config: &ScrapingConfig,
) -> Result<WebContent, ScrapingError> {
    // Validate URL
    let parsed_url = Url::parse(url)?;

    // Make request
    let response = client.get(url).send().await?;

    // Check content length
    if let Some(len) = response.content_length() {
        if len as usize > config.max_content_length {
            return Err(ScrapingError::ExtractionFailed(format!(
                "Content too large: {len} bytes"
            )));
        }
    }

    // Get final URL after redirects
    let final_url = response.url().to_string();

    // Check content type
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/html")
        .to_string();

    if !content_type.contains("text/html") && !content_type.contains("text/plain") {
        return Err(ScrapingError::UnsupportedContentType(content_type));
    }

    let html = response.text().await?;
    extract_content(&html, url, &final_url, &content_type, &parsed_url)
}

/// Extract content from HTML.
fn extract_content(
    html: &str,
    original_url: &str,
    final_url: &str,
    content_type: &str,
    base_url: &Url,
) -> Result<WebContent, ScrapingError> {
    let document = Html::parse_document(html);
    let scraper = ContentScraper::new();

    // Extract title
    let title = extract_title(&document);

    // Extract meta description
    let description = extract_meta(&document, "description")
        .or_else(|| extract_meta(&document, "og:description"));

    // Extract author
    let author =
        extract_meta(&document, "author").or_else(|| extract_meta(&document, "article:author"));

    // Extract publish date
    let published_at = extract_publish_date(&document);

    // Extract language
    let language = extract_language(&document);

    // Extract main text content
    let text = extract_main_text(&document, &scraper);

    // Extract images
    let images = extract_images(&document, base_url);

    // Extract links
    let links = extract_links(&document, base_url);

    // Count words
    let word_count = text.split_whitespace().count();

    Ok(WebContent {
        url: original_url.to_string(),
        final_url: final_url.to_string(),
        title,
        text,
        html: Some(html.to_string()),
        description,
        author,
        published_at,
        language,
        images,
        links,
        scraped_at: Utc::now(),
        content_type: content_type.to_string(),
        word_count,
    })
}

/// Extract page title.
fn extract_title(document: &Html) -> String {
    // Try og:title first
    if let Some(og_title) = extract_meta(document, "og:title") {
        return og_title;
    }

    // Try title tag
    if let Ok(selector) = Selector::parse("title") {
        if let Some(element) = document.select(&selector).next() {
            let title = element.text().collect::<String>().trim().to_string();
            if !title.is_empty() {
                return title;
            }
        }
    }

    // Try h1
    if let Ok(selector) = Selector::parse("h1") {
        if let Some(element) = document.select(&selector).next() {
            return element.text().collect::<String>().trim().to_string();
        }
    }

    String::new()
}

/// Extract meta tag content.
fn extract_meta(document: &Html, name: &str) -> Option<String> {
    // Try name attribute
    let selector_str = format!("meta[name='{name}']");
    if let Ok(selector) = Selector::parse(&selector_str) {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                let content = content.trim();
                if !content.is_empty() {
                    return Some(content.to_string());
                }
            }
        }
    }

    // Try property attribute (for OpenGraph)
    let selector_str = format!("meta[property='{name}']");
    if let Ok(selector) = Selector::parse(&selector_str) {
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                let content = content.trim();
                if !content.is_empty() {
                    return Some(content.to_string());
                }
            }
        }
    }

    None
}

/// Extract publish date from various sources.
fn extract_publish_date(document: &Html) -> Option<DateTime<Utc>> {
    // Try various meta tags
    let date_metas = [
        "article:published_time",
        "og:published_time",
        "datePublished",
        "date",
        "pubdate",
    ];

    for meta_name in date_metas {
        if let Some(date_str) = extract_meta(document, meta_name) {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&date_str) {
                return Some(dt.with_timezone(&Utc));
            }
            // Try other formats
            if let Ok(dt) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                return Some(DateTime::from_naive_utc_and_offset(
                    dt.and_hms_opt(0, 0, 0).unwrap_or_default(),
                    Utc,
                ));
            }
        }
    }

    // Try time element with datetime attribute
    if let Ok(selector) = Selector::parse("time[datetime]") {
        if let Some(element) = document.select(&selector).next() {
            if let Some(datetime) = element.value().attr("datetime") {
                if let Ok(dt) = DateTime::parse_from_rfc3339(datetime) {
                    return Some(dt.with_timezone(&Utc));
                }
            }
        }
    }

    None
}

/// Extract language from HTML.
fn extract_language(document: &Html) -> Option<String> {
    // Try html lang attribute
    if let Ok(selector) = Selector::parse("html") {
        if let Some(element) = document.select(&selector).next() {
            if let Some(lang) = element.value().attr("lang") {
                let lang = lang.split('-').next().unwrap_or(lang);
                if !lang.is_empty() {
                    return Some(lang.to_string());
                }
            }
        }
    }

    // Try meta tag
    extract_meta(document, "language").or_else(|| extract_meta(document, "og:locale"))
}

/// Extract main text content from the page.
#[allow(clippy::cognitive_complexity)]
fn extract_main_text(document: &Html, scraper: &ContentScraper) -> String {
    // Try content selectors in order
    for selector_str in &scraper.content_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = clean_text(&element.text().collect::<String>());
                if text.split_whitespace().count() > 50 {
                    return text;
                }
            }
        }
    }

    // Fallback: extract from body, removing unwanted elements
    if let Ok(body_selector) = Selector::parse("body") {
        if let Some(body) = document.select(&body_selector).next() {
            // Get all text, excluding script/style
            let mut text = String::new();
            for node in body.text() {
                let trimmed = node.trim();
                if !trimmed.is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(trimmed);
                }
            }
            return clean_text(&text);
        }
    }

    String::new()
}

/// Clean extracted text.
fn clean_text(text: &str) -> String {
    // Normalize whitespace
    let text: String = text
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();

    // Collapse multiple spaces
    let mut result = String::with_capacity(text.len());
    let mut last_was_space = false;

    for c in text.chars() {
        if c == ' ' {
            if !last_was_space {
                result.push(c);
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }

    result.trim().to_string()
}

/// Extract images from the page.
fn extract_images(document: &Html, base_url: &Url) -> Vec<ImageInfo> {
    let mut images = Vec::new();

    if let Ok(selector) = Selector::parse("img[src]") {
        for element in document.select(&selector) {
            if let Some(src) = element.value().attr("src") {
                let url = resolve_url(src, base_url);
                if url.is_empty() {
                    continue;
                }

                let alt = element.value().attr("alt").map(String::from);
                let width = element
                    .value()
                    .attr("width")
                    .and_then(|w| w.parse().ok());
                let height = element
                    .value()
                    .attr("height")
                    .and_then(|h| h.parse().ok());

                images.push(ImageInfo {
                    url,
                    alt,
                    width,
                    height,
                });
            }
        }
    }

    images
}

/// Extract links from the page.
fn extract_links(document: &Html, base_url: &Url) -> Vec<LinkInfo> {
    let mut links = Vec::new();

    if let Ok(selector) = Selector::parse("a[href]") {
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                let url = resolve_url(href, base_url);
                if url.is_empty() {
                    continue;
                }

                let text = element.text().collect::<String>().trim().to_string();
                let is_external = Url::parse(&url)
                    .ok()
                    .map_or(false, |u| u.host() != base_url.host());

                links.push(LinkInfo {
                    url,
                    text,
                    is_external,
                });
            }
        }
    }

    links
}

/// Resolve a potentially relative URL against a base URL.
fn resolve_url(href: &str, base_url: &Url) -> String {
    // Skip javascript: and mailto: links
    if href.starts_with("javascript:") || href.starts_with("mailto:") || href.starts_with('#') {
        return String::new();
    }

    // Try to parse as absolute URL
    if let Ok(url) = Url::parse(href) {
        return url.to_string();
    }

    // Resolve relative URL
    base_url
        .join(href)
        .map(|u| u.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_text() {
        let text = "  Hello   world  \n\t  test  ";
        let cleaned = clean_text(text);
        assert_eq!(cleaned, "Hello world test");
    }

    #[test]
    fn test_resolve_url_absolute() {
        let base = Url::parse("https://example.com/page").ok().unwrap_or_else(|| {
            Url::parse("https://fallback.com").ok().unwrap_or_else(|| unreachable!())
        });
        let resolved = resolve_url("https://other.com/image.jpg", &base);
        assert_eq!(resolved, "https://other.com/image.jpg");
    }

    #[test]
    fn test_resolve_url_relative() {
        let base = Url::parse("https://example.com/page/").ok().unwrap_or_else(|| {
            Url::parse("https://fallback.com").ok().unwrap_or_else(|| unreachable!())
        });
        let resolved = resolve_url("image.jpg", &base);
        assert_eq!(resolved, "https://example.com/page/image.jpg");
    }

    #[test]
    fn test_resolve_url_skip_javascript() {
        let base = Url::parse("https://example.com").ok().unwrap_or_else(|| {
            Url::parse("https://fallback.com").ok().unwrap_or_else(|| unreachable!())
        });
        let resolved = resolve_url("javascript:void(0)", &base);
        assert!(resolved.is_empty());
    }
}
