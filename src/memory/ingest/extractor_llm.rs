//! Optional LLM-assisted memory extraction.

use reqwest::Client as ReqwestClient;
use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::message::AssistantContent;
use rig::providers::ollama;

use crate::memory::core::config::{ExtractorConfig, LlmConfig, PromptConfig};
use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::core::ids::SessionId;
use crate::memory::core::item::MemoryItem;
use crate::memory::core::kinds::MemoryKind;
use crate::memory::core::metadata::{MemoryMetadata, MemorySource};

/// LLM-assisted extractor for memory items.
pub struct LlmExtractor {
    model: ollama::CompletionModel,
    max_items: usize,
    max_memory_chars: usize,
}

impl LlmExtractor {
    /// Create a new extractor using the completion model config.
    ///
    /// # Errors
    /// Returns an error if the Ollama client cannot be built.
    pub fn new(
        llm: &LlmConfig,
        extractor: &ExtractorConfig,
        prompt: &PromptConfig,
    ) -> MemoryResult<Self> {
        let builder = ollama::Client::<ReqwestClient>::builder().api_key(rig::client::Nothing);
        let builder = if let Some(base_url) = &llm.base_url {
            builder.base_url(base_url)
        } else {
            builder
        };
        let client = builder.build().map_err(MemoryError::from)?;
        let model = client.completion_model(llm.model.clone());
        Ok(Self {
            model,
            max_items: extractor.llm_max_items,
            max_memory_chars: prompt.max_memory_chars,
        })
    }

    /// Extract memory items from a turn using the LLM.
    ///
    /// # Errors
    /// Returns an error if the completion call fails or JSON is invalid.
    pub async fn extract(
        &self,
        session_id: SessionId,
        user_message: &str,
        assistant_message: &str,
    ) -> MemoryResult<Vec<MemoryItem>> {
        let system_prompt = "You extract stable memories. Return strict JSON array of objects with fields: kind, content, salience, tags, ttl_seconds, source. Return [] if nothing should be stored.";
        let user_prompt = format!(
            "User message:\n{user_message}\n\nAssistant message:\n{assistant_message}\n\nOnly include durable facts, preferences, constraints, decisions, goals, tool results, or code artifacts."
        );

        let request = self
            .model
            .completion_request(user_prompt)
            .preamble(system_prompt.to_string())
            .temperature(0.0)
            .build();

        let response = self.model.completion(request).await?;
        let text = extract_text(&response.choice);
        let candidates: Vec<MemoryCandidate> = serde_json::from_str(&text)?;

        let mut items = Vec::new();
        for candidate in candidates.into_iter().take(self.max_items) {
            if candidate.content.trim().is_empty() {
                continue;
            }

            let mut metadata =
                MemoryMetadata::new(candidate.source.unwrap_or(MemorySource::System))
                    .with_salience(candidate.salience.unwrap_or(60))
                    .with_tags(candidate.tags.unwrap_or_default())
                    .with_ttl(candidate.ttl_seconds);

            metadata.updated_at = metadata.created_at;

            let item =
                match MemoryItem::new(session_id, candidate.kind, candidate.content, metadata) {
                    Ok(item) => item.truncate_to_budget(self.max_memory_chars),
                    Err(_) => continue,
                };

            if item.validate(self.max_memory_chars).is_err() {
                continue;
            }

            items.push(item);
        }

        Ok(items)
    }
}

#[derive(Debug, serde::Deserialize)]
struct MemoryCandidate {
    kind: MemoryKind,
    content: String,
    salience: Option<u8>,
    tags: Option<Vec<String>>,
    ttl_seconds: Option<u64>,
    source: Option<MemorySource>,
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
