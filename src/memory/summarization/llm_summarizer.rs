//! LLM-based summarization for conversation transcripts.
//!
//! Produces intelligent, structured summaries of conversation history.

use reqwest::Client as ReqwestClient;
use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::message::AssistantContent;
use rig::providers::ollama;
use tracing::debug;

use crate::memory::core::config::LlmConfig;
use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::ingest::transcript_event::{TranscriptEvent, TranscriptRole};

/// System prompt for LLM summarization.
const SUMMARY_SYSTEM_PROMPT: &str = r"Tu es un assistant specialise dans le resume de conversations.
Produis un resume structure et concis en francais.

STRUCTURE OBLIGATOIRE:
## Faits cles
- Informations factuelles importantes sur l'utilisateur
- Identite, preferences, contraintes mentionnees

## Contexte
- Sujets principaux discutes
- Decisions prises ou conclusions

## A retenir
- Elements importants pour les conversations futures
- Preferences ou regles a respecter

REGLES:
- Maximum 400 mots
- Sois factuel et precis
- N'invente rien, base-toi uniquement sur la conversation
- Utilise des bullet points pour la clarte";

/// Summarization mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SummaryMode {
    /// Simple concatenation (no LLM).
    #[default]
    Simple,
    /// LLM-based intelligent summarization.
    Llm,
    /// Incremental update of existing summary.
    Incremental,
}

/// LLM-based summarizer for conversation transcripts.
pub struct LlmSummarizer {
    model: ollama::CompletionModel,
    max_input_chars: usize,
    max_output_chars: usize,
}

impl LlmSummarizer {
    /// Create a new LLM summarizer.
    ///
    /// # Errors
    /// Returns an error if the Ollama client cannot be built.
    pub fn new(config: &LlmConfig, max_output_chars: usize) -> MemoryResult<Self> {
        let builder = ollama::Client::<ReqwestClient>::builder().api_key(rig::client::Nothing);
        let builder = if let Some(base_url) = &config.base_url {
            builder.base_url(base_url)
        } else {
            builder
        };
        let client = builder.build().map_err(MemoryError::from)?;
        let model = client.completion_model(config.model.clone());

        Ok(Self {
            model,
            max_input_chars: 8000,
            max_output_chars,
        })
    }

    /// Summarize a list of transcript events.
    ///
    /// # Errors
    /// Returns an error if the LLM call fails.
    pub async fn summarize(&self, events: &[TranscriptEvent]) -> MemoryResult<String> {
        if events.is_empty() {
            return Ok(String::new());
        }

        let transcript = format_transcript(events, self.max_input_chars);
        let prompt = format!(
            "Resume la conversation suivante:\n\n{transcript}\n\nProduis un resume structure."
        );

        debug!("Summarizing {} events with LLM", events.len());

        let request = self
            .model
            .completion_request(prompt)
            .preamble(SUMMARY_SYSTEM_PROMPT.to_string())
            .temperature(0.3)
            .build();

        let response = self.model.completion(request).await?;
        let text = extract_text(&response.choice);
        let summary = truncate_to_chars(&text, self.max_output_chars);
        Ok(summary)
    }

    /// Update an existing summary with new events.
    ///
    /// # Errors
    /// Returns an error if the LLM call fails.
    pub async fn update_summary(
        &self,
        existing_summary: &str,
        new_events: &[TranscriptEvent],
    ) -> MemoryResult<String> {
        if new_events.is_empty() {
            return Ok(existing_summary.to_string());
        }

        let new_transcript = format_transcript(new_events, self.max_input_chars / 2);
        let prompt = format!(
            "Voici un resume existant:\n\n{existing_summary}\n\nEt voici de nouvelles interactions:\n\n{new_transcript}\n\nMets a jour le resume en integrant les nouvelles informations. Garde la meme structure."
        );

        debug!(
            "Updating summary with {} new events using LLM",
            new_events.len()
        );

        let request = self
            .model
            .completion_request(prompt)
            .preamble(SUMMARY_SYSTEM_PROMPT.to_string())
            .temperature(0.3)
            .build();

        let response = self.model.completion(request).await?;
        let text = extract_text(&response.choice);
        let summary = truncate_to_chars(&text, self.max_output_chars);
        Ok(summary)
    }

    /// Extract key facts from events for quick memory.
    ///
    /// # Errors
    /// Returns an error if the LLM call fails.
    pub async fn extract_key_facts(&self, events: &[TranscriptEvent]) -> MemoryResult<Vec<String>> {
        if events.is_empty() {
            return Ok(Vec::new());
        }

        let transcript = format_transcript(events, self.max_input_chars);
        let prompt = format!(
            "Extrais les faits cles de cette conversation sous forme de liste:\n\n{transcript}\n\nRetourne uniquement les faits importants, un par ligne, commencant par '- '."
        );

        let request = self
            .model
            .completion_request(prompt)
            .preamble("Tu extrais les faits cles d'une conversation. Sois concis et factuel.".to_string())
            .temperature(0.2)
            .build();

        let response = self.model.completion(request).await?;
        let text = extract_text(&response.choice);

        let facts: Vec<String> = text
            .lines()
            .filter(|line: &&str| line.trim().starts_with("- "))
            .map(|line: &str| line.trim().strip_prefix("- ").unwrap_or(line).to_string())
            .filter(|s: &String| !s.is_empty())
            .collect();

        Ok(facts)
    }
}

/// Extract text from assistant response.
fn extract_text(choice: &rig::OneOrMany<AssistantContent>) -> String {
    let mut out = String::new();
    for content in choice.iter() {
        if let AssistantContent::Text(text) = content {
            out.push_str(&text.text);
        }
    }
    out
}

/// Format transcript events into a readable string.
fn format_transcript(events: &[TranscriptEvent], max_chars: usize) -> String {
    let mut output = String::new();
    let mut char_count = 0;

    for event in events {
        let role_str = match event.role {
            TranscriptRole::User => "Utilisateur",
            TranscriptRole::Assistant => "Assistant",
            TranscriptRole::Tool => "Outil",
            TranscriptRole::System => "Systeme",
        };

        let line = format!("{}: {}\n", role_str, event.content);
        let line_len = line.chars().count();

        if char_count + line_len > max_chars {
            // Truncate and add ellipsis
            let remaining = max_chars.saturating_sub(char_count).saturating_sub(3);
            let truncated: String = line.chars().take(remaining).collect();
            output.push_str(&truncated);
            output.push_str("...");
            break;
        }

        output.push_str(&line);
        char_count += line_len;
    }

    output
}

/// Truncate a string to a maximum number of characters.
fn truncate_to_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        text.to_string()
    } else {
        text.chars().take(max_chars).collect()
    }
}

/// Create a simple concatenation summary (non-LLM fallback).
#[must_use]
pub fn simple_summary(events: &[TranscriptEvent], max_chars: usize) -> String {
    let mut summary = String::new();

    for event in events {
        let prefix = match event.role {
            TranscriptRole::User => "\nUser: ",
            TranscriptRole::Assistant => "\nAssistant: ",
            TranscriptRole::Tool => "\nTool: ",
            TranscriptRole::System => "\nSystem: ",
        };
        summary.push_str(prefix);
        summary.push_str(&event.content);
    }

    truncate_to_chars(&summary, max_chars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::core::ids::{SessionId, TurnId};

    fn make_event(role: TranscriptRole, content: &str) -> TranscriptEvent {
        TranscriptEvent {
            turn_id: TurnId::new(),
            session_id: SessionId::new(),
            timestamp: chrono::Utc::now(),
            role,
            content: content.to_string(),
            tool_name: None,
            tool_payload: None,
        }
    }

    #[test]
    fn test_format_transcript() {
        let events = vec![
            make_event(TranscriptRole::User, "Hello"),
            make_event(TranscriptRole::Assistant, "Hi there!"),
        ];

        let formatted = format_transcript(&events, 1000);
        assert!(formatted.contains("Utilisateur: Hello"));
        assert!(formatted.contains("Assistant: Hi there!"));
    }

    #[test]
    fn test_format_transcript_truncation() {
        let events = vec![
            make_event(TranscriptRole::User, "This is a very long message"),
            make_event(TranscriptRole::Assistant, "And another long response"),
        ];

        let formatted = format_transcript(&events, 30);
        assert!(formatted.len() <= 33); // 30 + "..."
        assert!(formatted.ends_with("..."));
    }

    #[test]
    fn test_simple_summary() {
        let events = vec![
            make_event(TranscriptRole::User, "Hello"),
            make_event(TranscriptRole::Assistant, "Hi!"),
        ];

        let summary = simple_summary(&events, 1000);
        assert!(summary.contains("User: Hello"));
        assert!(summary.contains("Assistant: Hi!"));
    }

    #[test]
    fn test_truncate_to_chars() {
        let text = "Hello, world!";
        assert_eq!(truncate_to_chars(text, 5), "Hello");
        assert_eq!(truncate_to_chars(text, 100), text);
    }
}
