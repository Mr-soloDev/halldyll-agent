//! Code search functionality (GitHub, etc.).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::scraping::config::ScrapingConfig;
use crate::scraping::error::ScrapingError;

/// Search for code on GitHub.
///
/// # Errors
/// Returns an error if the search fails.
pub async fn search_github(
    client: &reqwest::Client,
    query: &str,
    count: usize,
    config: &ScrapingConfig,
) -> Result<Vec<CodeResult>, ScrapingError> {
    // GitHub API requires authentication for code search
    let token = config
        .api_keys
        .github
        .as_ref()
        .ok_or_else(|| ScrapingError::ApiKeyRequired("GitHub".to_string()))?;

    search_github_code(client, query, count, token).await
}

/// Search GitHub Code API.
async fn search_github_code(
    client: &reqwest::Client,
    query: &str,
    count: usize,
    token: &str,
) -> Result<Vec<CodeResult>, ScrapingError> {
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "https://api.github.com/search/code?q={}&per_page={}",
        encoded_query,
        count.min(100)
    );

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "Halldyll-Agent/1.0")
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ScrapingError::AccessDenied(
            "Invalid GitHub token".to_string(),
        ));
    }

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        // Check if it's rate limiting
        if let Some(remaining) = response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
        {
            if remaining == 0 {
                let reset = response
                    .headers()
                    .get("x-ratelimit-reset")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(60);
                return Err(ScrapingError::RateLimited(reset));
            }
        }
        return Err(ScrapingError::AccessDenied(
            "GitHub API access denied".to_string(),
        ));
    }

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "GitHub API returned status: {}",
            response.status()
        )));
    }

    let json: GitHubCodeResponse = response.json().await?;
    Ok(parse_github_code_response(json))
}

/// Search for GitHub repositories.
///
/// # Errors
/// Returns an error if the search fails.
pub async fn search_github_repos(
    client: &reqwest::Client,
    query: &str,
    count: usize,
    config: &ScrapingConfig,
) -> Result<Vec<RepoResult>, ScrapingError> {
    let encoded_query = urlencoding::encode(query);
    let url = format!(
        "https://api.github.com/search/repositories?q={}&per_page={}&sort=stars&order=desc",
        encoded_query,
        count.min(100)
    );

    let mut request = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "Halldyll-Agent/1.0");

    // Add token if available (not required for repo search, but helps with rate limits)
    if let Some(token) = &config.api_keys.github {
        request = request.header("Authorization", format!("Bearer {token}"));
    }

    let response = request.send().await?;

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ScrapingError::RateLimited(60));
    }

    if !response.status().is_success() {
        return Err(ScrapingError::HttpClient(format!(
            "GitHub API returned status: {}",
            response.status()
        )));
    }

    let json: GitHubRepoResponse = response.json().await?;
    Ok(parse_github_repo_response(json))
}

/// Parse GitHub code search response.
fn parse_github_code_response(response: GitHubCodeResponse) -> Vec<CodeResult> {
    response
        .items
        .into_iter()
        .map(|item| CodeResult {
            name: item.name,
            path: item.path,
            url: item.html_url,
            repository: item.repository.full_name,
            repository_url: item.repository.html_url,
            sha: item.sha,
            score: item.score,
        })
        .collect()
}

/// Parse GitHub repository search response.
fn parse_github_repo_response(response: GitHubRepoResponse) -> Vec<RepoResult> {
    response
        .items
        .into_iter()
        .map(|item| RepoResult {
            name: item.name,
            full_name: item.full_name,
            description: item.description,
            url: item.html_url,
            clone_url: item.clone_url,
            language: item.language,
            stars: item.stargazers_count,
            forks: item.forks_count,
            open_issues: item.open_issues_count,
            created_at: item.created_at,
            updated_at: item.updated_at,
            topics: item.topics.unwrap_or_default(),
            license: item.license.map(|l| l.name),
            owner: item.owner.login,
            owner_url: item.owner.html_url,
            owner_avatar: item.owner.avatar_url,
        })
        .collect()
}

/// Code search result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CodeResult {
    /// File name.
    pub name: String,
    /// File path in repository.
    pub path: String,
    /// URL to the file.
    pub url: String,
    /// Repository full name (owner/repo).
    pub repository: String,
    /// Repository URL.
    pub repository_url: String,
    /// Git SHA of the file.
    pub sha: String,
    /// Search relevance score.
    pub score: f64,
}

/// Repository search result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoResult {
    /// Repository name.
    pub name: String,
    /// Full name (owner/repo).
    pub full_name: String,
    /// Description.
    pub description: Option<String>,
    /// Repository URL.
    pub url: String,
    /// Clone URL.
    pub clone_url: String,
    /// Primary language.
    pub language: Option<String>,
    /// Star count.
    pub stars: u64,
    /// Fork count.
    pub forks: u64,
    /// Open issues count.
    pub open_issues: u64,
    /// Creation date.
    pub created_at: Option<DateTime<Utc>>,
    /// Last update date.
    pub updated_at: Option<DateTime<Utc>>,
    /// Topics/tags.
    pub topics: Vec<String>,
    /// License name.
    pub license: Option<String>,
    /// Owner username.
    pub owner: String,
    /// Owner profile URL.
    pub owner_url: String,
    /// Owner avatar URL.
    pub owner_avatar: String,
}

// GitHub API response structures

#[derive(Debug, Deserialize)]
struct GitHubCodeResponse {
    items: Vec<GitHubCodeItem>,
}

#[derive(Debug, Deserialize)]
struct GitHubCodeItem {
    name: String,
    path: String,
    sha: String,
    html_url: String,
    score: f64,
    repository: GitHubRepoRef,
}

#[derive(Debug, Deserialize)]
struct GitHubRepoRef {
    full_name: String,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRepoResponse {
    items: Vec<GitHubRepoItem>,
}

#[derive(Debug, Deserialize)]
struct GitHubRepoItem {
    name: String,
    full_name: String,
    description: Option<String>,
    html_url: String,
    clone_url: String,
    language: Option<String>,
    stargazers_count: u64,
    forks_count: u64,
    open_issues_count: u64,
    created_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    topics: Option<Vec<String>>,
    license: Option<GitHubLicense>,
    owner: GitHubOwner,
}

#[derive(Debug, Deserialize)]
struct GitHubLicense {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitHubOwner {
    login: String,
    html_url: String,
    avatar_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_result_serialization() {
        let result = CodeResult {
            name: "main.rs".to_string(),
            path: "src/main.rs".to_string(),
            url: "https://github.com/owner/repo/blob/main/src/main.rs".to_string(),
            repository: "owner/repo".to_string(),
            repository_url: "https://github.com/owner/repo".to_string(),
            sha: "abc123".to_string(),
            score: 0.95,
        };

        let json = serde_json::to_string(&result);
        assert!(json.is_ok());
    }

    #[test]
    fn test_repo_result_serialization() {
        let result = RepoResult {
            name: "awesome-repo".to_string(),
            full_name: "owner/awesome-repo".to_string(),
            description: Some("An awesome repository".to_string()),
            url: "https://github.com/owner/awesome-repo".to_string(),
            clone_url: "https://github.com/owner/awesome-repo.git".to_string(),
            language: Some("Rust".to_string()),
            stars: 1000,
            forks: 100,
            open_issues: 10,
            created_at: None,
            updated_at: None,
            topics: vec!["rust".to_string(), "cli".to_string()],
            license: Some("MIT".to_string()),
            owner: "owner".to_string(),
            owner_url: "https://github.com/owner".to_string(),
            owner_avatar: "https://avatars.githubusercontent.com/u/12345".to_string(),
        };

        let json = serde_json::to_string(&result);
        assert!(json.is_ok());
    }
}
