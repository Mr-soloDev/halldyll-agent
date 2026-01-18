//! Startup helpers that ensure Ollama is running and launches the agent server.

use std::process::ExitCode;

use crate::llm::ollama_starter_ministral;
use crate::server::{self, AppState};

/// Run the startup sequence and return an exit code suitable for a binary entrypoint.
///
/// This function:
/// 1. Initializes logging
/// 2. Ensures Ollama is running (locally or remotely via `HALLDYLL_OLLAMA_URL`)
/// 3. Preloads the LLM model
/// 4. Starts the HTTP API server
///
/// Returns `ExitCode::SUCCESS` on graceful shutdown.
/// Returns `1` on any failure.
#[must_use]
pub fn run() -> ExitCode {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Halldyll Agent v{}", env!("CARGO_PKG_VERSION"));

    // Check if we're using remote Ollama
    let ollama_url = std::env::var("HALLDYLL_OLLAMA_URL").ok();
    if let Some(ref url) = ollama_url {
        tracing::info!("Using remote Ollama at: {}", url);
    } else {
        tracing::info!("Using local Ollama at: http://127.0.0.1:11434");
        // Only try to start local Ollama if not using remote
        if let Err(e) = ollama_starter_ministral::ensure_ollama_and_preload_ministral() {
            tracing::error!("Failed to start Ollama: {}", e);
            return ExitCode::from(1);
        }
    }

    // Create application state
    let state = match AppState::new() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create application state: {}", e);
            return ExitCode::from(1);
        }
    };

    // Get port from environment or use default
    let port = std::env::var("HALLDYLL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(server::DEFAULT_PORT);

    // Run the async server
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("Failed to create Tokio runtime: {}", e);
            return ExitCode::from(1);
        }
    };

    if let Err(e) = rt.block_on(server::run_server(state, port)) {
        tracing::error!("Server error: {}", e);
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}
