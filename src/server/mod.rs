//! HTTP server for the Halldyll agent API.
//!
//! Provides REST endpoints for:
//! - Chat completions (LLM)
//! - Web search
//! - Memory operations

pub mod routes;
pub mod state;

pub use routes::create_router;
pub use state::AppState;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Default server port.
pub const DEFAULT_PORT: u16 = 3000;

/// Start the HTTP server.
///
/// # Errors
/// Returns an error if the server fails to start.
pub async fn run_server(state: Arc<AppState>, port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app: Router = create_router(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Halldyll Agent server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
