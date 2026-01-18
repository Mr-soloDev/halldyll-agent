//! Halldyll Agent - Cloud-based LLM service.
//!
//! This crate provides a web server that connects to a remote Ollama instance
//! for LLM inference. Designed for cloud deployment on `RunPod`.

#![deny(warnings)]
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![deny(dead_code)]
#![deny(unused_imports)]
#![deny(unused_variables)]
#![deny(unused_must_use)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]

/// LLM client for Ollama API.
pub mod llm;
/// HTTP server and API routes.
pub mod server;
/// Entry helpers to start the Halldyll agent.
pub mod start_halldyll_agent;
