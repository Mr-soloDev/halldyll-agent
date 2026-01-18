//! HTTP route handlers for the Halldyll agent API.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

use crate::scraping::{SearchQuery, SearchResult};

use super::state::AppState;

/// Create the API router with all routes.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/api/chat", post(chat_completion))
        .route("/api/search", post(web_search))
        .nest_service("/", ServeDir::new("static").fallback(ServeDir::new("static")))
        .with_state(state)
}

/// Health check endpoint.
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "halldyll-agent",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// Chat completion request.
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// The user's message.
    pub message: String,
    /// Optional system prompt.
    pub system_prompt: Option<String>,
}

/// Chat completion response.
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    /// The assistant's response.
    pub response: String,
    /// Model used.
    pub model: String,
}

/// Handle chat completion requests.
async fn chat_completion(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let prompt = if let Some(system) = &request.system_prompt {
        format!("{}\n\nUser: {}", system, request.message)
    } else {
        request.message.clone()
    };

    let response = state
        .ollama
        .generate_8192(&state.model_name, &prompt, "5m")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("LLM error: {e}")))?;

    Ok(Json(ChatResponse {
        response,
        model: state.model_name.clone(),
    }))
}

/// Web search request.
#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    /// The search query.
    pub query: String,
    /// Maximum number of results.
    pub max_results: Option<usize>,
}

/// Web search response.
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    /// Search results.
    pub results: Vec<SearchResultDto>,
    /// Number of results.
    pub count: usize,
}

/// Search result DTO.
#[derive(Debug, Serialize)]
pub struct SearchResultDto {
    /// Result title.
    pub title: String,
    /// Result URL.
    pub url: String,
    /// Result description/snippet.
    pub description: String,
}

impl From<SearchResult> for SearchResultDto {
    fn from(r: SearchResult) -> Self {
        Self {
            title: r.title,
            url: r.url,
            description: r.description,
        }
    }
}

/// Handle web search requests.
async fn web_search(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, String)> {
    let query = SearchQuery::new(&request.query)
        .with_max_results(request.max_results.unwrap_or(10));

    let results = state
        .scraper
        .search(&query)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Search error: {e}")))?;

    let dtos: Vec<SearchResultDto> = results.into_iter().map(SearchResultDto::from).collect();
    let count = dtos.len();

    Ok(Json(SearchResponse {
        results: dtos,
        count,
    }))
}
