//! Startup helpers for the Halldyll agent server.
//!
//! Cloud-only mode: connects to remote Ollama on `RunPod`.

use std::future::Future;
use std::process::ExitCode;
use std::sync::Arc;

use crate::server::{self, AppState};

/// Run the server (used by `halldyll-server` binary on `RunPod`).
///
/// # Returns
/// `ExitCode::SUCCESS` on graceful shutdown, `1` on failure.
#[must_use]
pub fn run() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting Halldyll Agent v{}", env!("CARGO_PKG_VERSION"));

    let ollama_url = std::env::var("HALLDYLL_OLLAMA_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    tracing::info!("Ollama endpoint: {ollama_url}");

    let state = match AppState::new() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to create state: {e}");
            return ExitCode::from(1);
        }
    };

    let port = get_port();

    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("Failed to create runtime: {e}");
            return ExitCode::from(1);
        }
    };

    if let Err(e) = rt.block_on(server::run_server(state, port)) {
        tracing::error!("Server error: {e}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

/// Initialize application state without starting the server.
///
/// # Errors
/// Returns an error if state creation fails.
pub fn initialize() -> Result<Arc<AppState>, Box<dyn std::error::Error + Send + Sync>> {
    let ollama_url = std::env::var("HALLDYLL_OLLAMA_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    tracing::info!("Ollama endpoint: {ollama_url}");

    AppState::new().map_err(|e| format!("Failed to create state: {e}").into())
}

/// Run server with graceful shutdown.
///
/// # Errors
/// Returns an error if the server fails.
pub async fn run_server_with_shutdown<F>(
    state: Arc<AppState>,
    port: u16,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    F: Future<Output = ()> + Send + 'static,
{
    server::run_server_with_shutdown(state, port, shutdown_signal).await
}

/// Get configured server port.
#[must_use]
pub fn get_port() -> u16 {
    std::env::var("HALLDYLL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(server::DEFAULT_PORT)
}
