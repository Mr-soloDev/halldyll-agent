//! Video search functionality (YouTube, etc.).

use chrono::{DateTime, Utc};
use scraper::{Html, Selector};

use crate::scraping::config::ScrapingConfig;
use crate::scraping::error::ScrapingError;
use crate::scraping::types::{VideoPlatform, VideoResult};

/// Search for videos on YouTube.
///
/// # Errors
/// Returns an error if the search fails.
pub async fn search_youtube(
    client: &reqwest::Client,
    query: &str,
    count: usize,
    config: &ScrapingConfig,
) -> Result<Vec<VideoResult>, ScrapingError> {
    // If we have a YouTube API key, use the API
    if let Some(api_key) = &config.api_keys.youtube {
        return search_youtube_api(client, query, count, api_key).await;
    }

    // Otherwise, scrape the HTML
    search_youtube_html(client, query, count).await
}

/// Search YouTube using the Data API.
async fn search_youtube_api(
    client: &reqwest::Client,
    query: &str,
    count: usize,
    api_key: &str,
) -> Result<Vec<VideoResult>, ScrapingError> {
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "https://www.googleapis.com/youtube/v3/search?part=snippet&type=video&q={}&maxResults={}&key={}",
        encoded_query, count, api_key
    );

    let response = client.get(&url).send().await?;

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ScrapingError::AccessDenied(
            "YouTube API quota exceeded or invalid key".to_string(),
        ));
    }

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "YouTube API returned status: {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response.json().await?;
    parse_youtube_api_response(&json)
}

/// Parse YouTube API response.
fn parse_youtube_api_response(json: &serde_json::Value) -> Result<Vec<VideoResult>, ScrapingError> {
    let mut results = Vec::new();

    if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
        for item in items {
            let id = item
                .get("id")
                .and_then(|i| i.get("videoId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            if id.is_empty() {
                continue;
            }

            let snippet = item.get("snippet");

            let title = snippet
                .and_then(|s| s.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let description = snippet
                .and_then(|s| s.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let channel = snippet
                .and_then(|s| s.get("channelTitle"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let channel_id = snippet
                .and_then(|s| s.get("channelId"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let thumbnail_url = snippet
                .and_then(|s| s.get("thumbnails"))
                .and_then(|t| t.get("high").or_else(|| t.get("default")))
                .and_then(|t| t.get("url"))
                .and_then(|v| v.as_str())
                .map(String::from);

            let published_at = snippet
                .and_then(|s| s.get("publishedAt"))
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc));

            results.push(VideoResult {
                id: id.clone(),
                title,
                description,
                url: format!("https://www.youtube.com/watch?v={id}"),
                embed_url: Some(format!("https://www.youtube.com/embed/{id}")),
                thumbnail_url,
                channel,
                channel_url: Some(format!("https://www.youtube.com/channel/{channel_id}")),
                duration: None, // Would need additional API call for this
                views: None,
                likes: None,
                published_at,
                platform: VideoPlatform::YouTube,
            });
        }
    }

    Ok(results)
}

/// Search YouTube by scraping HTML (fallback when no API key).
async fn search_youtube_html(
    client: &reqwest::Client,
    query: &str,
    count: usize,
) -> Result<Vec<VideoResult>, ScrapingError> {
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "https://www.youtube.com/results?search_query={}",
        encoded_query
    );

    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "YouTube returned status: {}",
            response.status()
        )));
    }

    let html = response.text().await?;
    parse_youtube_html(&html, count)
}

/// Parse YouTube HTML search results.
fn parse_youtube_html(html: &str, count: usize) -> Result<Vec<VideoResult>, ScrapingError> {
    let mut results = Vec::new();

    // YouTube embeds video data in JSON within the HTML
    // Look for ytInitialData
    if let Some(start) = html.find("var ytInitialData = ") {
        let start = start + 20;
        if let Some(end) = html[start..].find(";</script>") {
            let json_str = &html[start..start + end];
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                return parse_youtube_initial_data(&json, count);
            }
        }
    }

    // Fallback: try to parse from HTML structure
    let document = Html::parse_document(html);

    if let Ok(selector) = Selector::parse("a#video-title") {
        for element in document.select(&selector).take(count) {
            let href = element.value().attr("href").unwrap_or_default();

            // Extract video ID from href
            let id = if let Some(pos) = href.find("v=") {
                let start = pos + 2;
                let end = href[start..]
                    .find('&')
                    .map_or(href.len(), |i| start + i);
                href[start..end].to_string()
            } else if href.starts_with("/watch?v=") {
                href[9..].split('&').next().unwrap_or_default().to_string()
            } else {
                continue;
            };

            if id.is_empty() {
                continue;
            }

            let title = element.text().collect::<String>().trim().to_string();

            results.push(VideoResult {
                id: id.clone(),
                title,
                description: String::new(),
                url: format!("https://www.youtube.com/watch?v={id}"),
                embed_url: Some(format!("https://www.youtube.com/embed/{id}")),
                thumbnail_url: Some(format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg")),
                channel: String::new(),
                channel_url: None,
                duration: None,
                views: None,
                likes: None,
                published_at: None,
                platform: VideoPlatform::YouTube,
            });
        }
    }

    Ok(results)
}

/// Parse YouTube's initial data JSON.
fn parse_youtube_initial_data(
    json: &serde_json::Value,
    count: usize,
) -> Result<Vec<VideoResult>, ScrapingError> {
    let mut results = Vec::new();

    // Navigate the complex YouTube JSON structure
    let contents = json
        .get("contents")
        .and_then(|c| c.get("twoColumnSearchResultsRenderer"))
        .and_then(|t| t.get("primaryContents"))
        .and_then(|p| p.get("sectionListRenderer"))
        .and_then(|s| s.get("contents"))
        .and_then(|c| c.as_array());

    if let Some(sections) = contents {
        for section in sections {
            let items = section
                .get("itemSectionRenderer")
                .and_then(|i| i.get("contents"))
                .and_then(|c| c.as_array());

            if let Some(items) = items {
                for item in items {
                    if results.len() >= count {
                        break;
                    }

                    if let Some(video) = item.get("videoRenderer") {
                        if let Some(result) = parse_video_renderer(video) {
                            results.push(result);
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}

/// Parse a single video renderer from YouTube JSON.
fn parse_video_renderer(video: &serde_json::Value) -> Option<VideoResult> {
    let id = video.get("videoId")?.as_str()?.to_string();

    let title = video
        .get("title")
        .and_then(|t| t.get("runs"))
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .and_then(|r| r.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_string();

    let description = video
        .get("descriptionSnippet")
        .and_then(|d| d.get("runs"))
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|r| r.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    let channel = video
        .get("ownerText")
        .and_then(|o| o.get("runs"))
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .and_then(|r| r.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_string();

    let channel_url = video
        .get("ownerText")
        .and_then(|o| o.get("runs"))
        .and_then(|r| r.as_array())
        .and_then(|arr| arr.first())
        .and_then(|r| r.get("navigationEndpoint"))
        .and_then(|n| n.get("browseEndpoint"))
        .and_then(|b| b.get("canonicalBaseUrl"))
        .and_then(|u| u.as_str())
        .map(|u| format!("https://www.youtube.com{u}"));

    let thumbnail_url = video
        .get("thumbnail")
        .and_then(|t| t.get("thumbnails"))
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.last())
        .and_then(|t| t.get("url"))
        .and_then(|u| u.as_str())
        .map(String::from);

    let duration = video
        .get("lengthText")
        .and_then(|l| l.get("simpleText"))
        .and_then(|t| t.as_str())
        .and_then(parse_duration);

    let views = video
        .get("viewCountText")
        .and_then(|v| v.get("simpleText"))
        .and_then(|t| t.as_str())
        .and_then(parse_view_count);

    Some(VideoResult {
        id: id.clone(),
        title,
        description,
        url: format!("https://www.youtube.com/watch?v={id}"),
        embed_url: Some(format!("https://www.youtube.com/embed/{id}")),
        thumbnail_url,
        channel,
        channel_url,
        duration,
        views,
        likes: None,
        published_at: None,
        platform: VideoPlatform::YouTube,
    })
}

/// Parse duration string like "10:30" or "1:23:45" to seconds.
fn parse_duration(duration_str: &str) -> Option<u32> {
    let parts: Vec<&str> = duration_str.split(':').collect();
    match parts.len() {
        2 => {
            let minutes: u32 = parts[0].parse().ok()?;
            let seconds: u32 = parts[1].parse().ok()?;
            Some(minutes * 60 + seconds)
        }
        3 => {
            let hours: u32 = parts[0].parse().ok()?;
            let minutes: u32 = parts[1].parse().ok()?;
            let seconds: u32 = parts[2].parse().ok()?;
            Some(hours * 3600 + minutes * 60 + seconds)
        }
        _ => None,
    }
}

/// Parse view count string like "1,234,567 views" to number.
fn parse_view_count(view_str: &str) -> Option<u64> {
    let cleaned: String = view_str
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    cleaned.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("10:30"), Some(630));
        assert_eq!(parse_duration("1:23:45"), Some(5025));
        assert_eq!(parse_duration("0:45"), Some(45));
        assert_eq!(parse_duration("invalid"), None);
    }

    #[test]
    fn test_parse_view_count() {
        assert_eq!(parse_view_count("1,234,567 views"), Some(1_234_567));
        assert_eq!(parse_view_count("100 views"), Some(100));
        assert_eq!(parse_view_count("no numbers"), None);
    }
}
