//! Prompt builder for memory context.

use chrono::Utc;

use crate::memory::core::item::MemoryItem;
use crate::memory::ingest::transcript_event::{TranscriptEvent, TranscriptRole};
use crate::memory::prompt::prompt_budget::PromptParts;

/// Build a deterministic prompt block from prepared parts.
#[must_use]
pub fn build_prompt_block(parts: &PromptParts) -> String {
    let mut out = String::with_capacity(parts.estimate_len());

    out.push_str("[MEMORY_SUMMARY]\n");
    if let Some(summary) = &parts.summary {
        out.push_str(summary);
        out.push('\n');
    }

    out.push_str("[MEMORY_RELEVANT]\n");
    for item in &parts.memories {
        render_memory_item(&mut out, item);
    }

    out.push_str("[SHORT_TERM]\n");
    for event in &parts.turns {
        render_turn(&mut out, event);
    }

    out.push_str("[USER_MESSAGE]\n");
    out.push_str(&parts.user_message);
    out.push('\n');

    out
}

fn render_memory_item(out: &mut String, item: &MemoryItem) {
    let age_seconds = Utc::now()
        .signed_duration_since(item.metadata.created_at)
        .num_seconds();
    let tags = if item.metadata.tags.is_empty() {
        "none".to_string()
    } else {
        item.metadata.tags.join(",")
    };

    out.push_str("* (");
    out.push_str(item.kind.as_str());
    out.push_str(") ");
    out.push_str(&item.content);
    out.push_str(" [tags: ");
    out.push_str(&tags);
    out.push_str("] [salience: ");
    out.push_str(&item.metadata.salience.to_string());
    out.push_str("] [age_s: ");
    out.push_str(&age_seconds.to_string());
    out.push_str("]\n");
}

fn render_turn(out: &mut String, event: &TranscriptEvent) {
    let role = match event.role {
        TranscriptRole::User => "User",
        TranscriptRole::Assistant => "Assistant",
        TranscriptRole::Tool => "Tool",
        TranscriptRole::System => "System",
    };
    out.push_str("- ");
    out.push_str(role);
    out.push_str(": ");
    out.push_str(&event.content);
    out.push('\n');
}
