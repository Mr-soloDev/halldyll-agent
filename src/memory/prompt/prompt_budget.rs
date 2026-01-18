//! Prompt budget enforcement utilities.

use crate::memory::core::item::MemoryItem;
use crate::memory::ingest::transcript_event::TranscriptEvent;
use crate::memory::prompt::prompt_builder::build_prompt_block;

/// Prompt parts before formatting.
#[derive(Clone, Debug)]
pub struct PromptParts {
    /// Optional session summary.
    pub summary: Option<String>,
    /// Retrieved long-term memories.
    pub memories: Vec<MemoryItem>,
    /// Recent transcript turns.
    pub turns: Vec<TranscriptEvent>,
    /// Current user message.
    pub user_message: String,
}

impl PromptParts {
    /// Approximate the character count of the prompt.
    #[must_use]
    pub fn estimate_len(&self) -> usize {
        estimate_len(self)
    }
}

/// Enforce the prompt budget by trimming memories and turns.
#[must_use]
pub fn enforce_budget(mut parts: PromptParts, max_chars: usize) -> PromptParts {
    loop {
        let actual_len = build_prompt_block(&parts).len();
        if actual_len <= max_chars {
            break;
        }

        if !parts.memories.is_empty() {
            parts.memories.pop();
            continue;
        }

        if !parts.turns.is_empty() {
            parts.turns.remove(0);
            continue;
        }

        let remaining = max_chars.saturating_sub(estimate_len_without_summary(&parts));
        if let Some(summary) = parts.summary.as_mut() {
            if remaining == 0 {
                parts.summary = None;
            } else if summary.chars().count() > remaining {
                let truncated: String = summary.chars().take(remaining).collect();
                *summary = truncated.trim_end().to_string();
            } else {
                break;
            }
            continue;
        }

        break;
    }

    parts
}

fn estimate_len(parts: &PromptParts) -> usize {
    let mut total = 0;
    total += "[MEMORY_SUMMARY]\n".len();
    if let Some(summary) = &parts.summary {
        total += summary.len() + 1;
    }
    total += "[MEMORY_RELEVANT]\n".len();
    for item in &parts.memories {
        total += item.content.len() + 32;
        total += item.metadata.tags.iter().map(String::len).sum::<usize>();
    }
    total += "[SHORT_TERM]\n".len();
    for event in &parts.turns {
        total += event.content.len() + 12;
    }
    total += "[USER_MESSAGE]\n".len();
    total += parts.user_message.len() + 1;
    total
}

fn estimate_len_without_summary(parts: &PromptParts) -> usize {
    let mut clone = parts.clone();
    clone.summary = None;
    estimate_len(&clone)
}
