//! Rig adapter example for memory-aware prompting with Ollama.

use reqwest::Client as ReqwestClient;
use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::message::AssistantContent;
use rig::providers::ollama;

use crate::memory::core::config::MemoryConfig;
use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::core::ids::SessionId;
use crate::memory::engine::MemoryEngine;
use crate::memory::ingest::transcript_event::TranscriptEvent;

/// Initialize tracing with a basic subscriber.
pub fn init_tracing() {
    let _ = tracing_subscriber::fmt().with_env_filter("info").try_init();
}

/// Execute a full memory-aware LLM call using Rig + Ollama.
///
/// # Errors
/// Returns an error if memory preparation, completion, or storage fails.
pub async fn run_with_memory(
    config: &MemoryConfig,
    engine: &MemoryEngine,
    session_id: SessionId,
    user_message: &str,
    recent_turns: Vec<TranscriptEvent>,
) -> MemoryResult<String> {
    let prepared = engine
        .prepare_context(session_id, user_message, recent_turns)
        .await?;

    let builder = ollama::Client::<ReqwestClient>::builder().api_key(rig::client::Nothing);
    let builder = if let Some(base_url) = &config.llm.base_url {
        builder.base_url(base_url)
    } else {
        builder
    };
    let client = builder.build().map_err(MemoryError::from)?;
    let model = client.completion_model(config.llm.model.clone());

    let request = model
        .completion_request(prepared.prompt_block.clone())
        .temperature(config.llm.temperature)
        .max_tokens_opt(config.llm.max_tokens)
        .build();

    let response = model.completion(request).await?;
    let assistant_text = extract_text(&response.choice);

    engine
        .record_turn(session_id, user_message, &assistant_text, None)
        .await?;

    Ok(assistant_text)
}

fn extract_text(choice: &rig::OneOrMany<AssistantContent>) -> String {
    let mut out = String::new();
    for content in choice.iter() {
        if let AssistantContent::Text(text) = content {
            out.push_str(&text.text);
        }
    }
    out
}
