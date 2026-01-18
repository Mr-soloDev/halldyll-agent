//! Heuristic memory extraction logic.
//!
//! Enhanced extraction with comprehensive pattern matching for all memory kinds.

use regex::Regex;
use tracing::debug;

use crate::memory::core::config::{ExtractorConfig, PromptConfig};
use crate::memory::core::ids::SessionId;
use crate::memory::core::item::MemoryItem;
use crate::memory::core::kinds::MemoryKind;
use crate::memory::core::metadata::{MemoryMetadata, MemorySource};
use crate::memory::ingest::transcript_event::{TranscriptEvent, TranscriptRole};

/// A pattern rule mapping a regex to a memory kind.
struct PatternRule {
    pattern: Regex,
    kind: MemoryKind,
    priority: u8, // Higher = checked first
}

/// Heuristic extractor for memory items with enhanced pattern matching.
pub struct HeuristicExtractor {
    min_content_chars: usize,
    max_memory_chars: usize,
    rules: Vec<PatternRule>,
}

impl HeuristicExtractor {
    /// Create a heuristic extractor with comprehensive patterns.
    ///
    /// # Errors
    /// Returns an error if any regex pattern is invalid.
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::vec_init_then_push)]
    pub fn new(config: &ExtractorConfig, prompt: &PromptConfig) -> Result<Self, regex::Error> {
        let mut rules = Vec::new();

        // === IDENTITY patterns (priority 100) ===
        // Name patterns
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(my name is|i'm called|call me|je m'appelle|mon nom est)\s+\w+")?,
            kind: MemoryKind::Identity,
            priority: 100,
        });
        // Age patterns
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i am|i'm|j'ai)\s+\d+\s*(years? old|ans|yo)\b")?,
            kind: MemoryKind::Identity,
            priority: 100,
        });
        // Location patterns
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i live in|i'm from|based in|j'habite|je vis)\s+\w+")?,
            kind: MemoryKind::Identity,
            priority: 100,
        });
        // Profession patterns
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i work (at|for|as)|my job is|i'm a|je suis|je travaille)\s+\w+")?,
            kind: MemoryKind::Identity,
            priority: 100,
        });
        // Nationality/language
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i speak|i'm (fluent in|native)|my (native|first) language)\b")?,
            kind: MemoryKind::Identity,
            priority: 100,
        });

        // === CONSTRAINT patterns (priority 95) ===
        // Strong prohibitions
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(do not|don't|never|must not|cannot|can't|shouldn't|won't)\b")?,
            kind: MemoryKind::Constraint,
            priority: 95,
        });
        // Critical requirements
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(it's (important|critical|essential|crucial) that|always make sure)\b")?,
            kind: MemoryKind::Constraint,
            priority: 95,
        });
        // French constraints
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(ne (jamais|pas)|il faut (absolument|toujours))\b")?,
            kind: MemoryKind::Constraint,
            priority: 95,
        });

        // === AVERSION patterns (priority 90) ===
        // Dislikes
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i hate|i can't stand|i dislike|i despise|i loathe)\b")?,
            kind: MemoryKind::Aversion,
            priority: 90,
        });
        // Allergies/intolerances
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i'm allergic to|i'm intolerant|i can't (eat|have|use))\b")?,
            kind: MemoryKind::Aversion,
            priority: 90,
        });
        // Annoyances
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i'm (annoyed|frustrated|bothered) (by|when)|it annoys me)\b")?,
            kind: MemoryKind::Aversion,
            priority: 90,
        });
        // French aversions
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(je (deteste|n'aime pas|supporte pas)|j'ai horreur)\b")?,
            kind: MemoryKind::Aversion,
            priority: 90,
        });

        // === PREFERENCE patterns (priority 85) ===
        // Likes
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i|we)\s+(like|love|prefer|enjoy|adore)\b")?,
            kind: MemoryKind::Preference,
            priority: 85,
        });
        // Favorites
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(my favorite|i prefer|i always choose|i'm a fan of)\b")?,
            kind: MemoryKind::Preference,
            priority: 85,
        });
        // Habits
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i usually|i tend to|i often|i always)\b")?,
            kind: MemoryKind::Preference,
            priority: 85,
        });
        // French preferences
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(j'aime|je prefere|mon prefere|j'adore)\b")?,
            kind: MemoryKind::Preference,
            priority: 85,
        });

        // === POLICY patterns (priority 82) ===
        // Operating rules
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(always respond|always use|use .+ format|respond in|answer in)\b")?,
            kind: MemoryKind::Policy,
            priority: 82,
        });
        // Style instructions
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(be (concise|brief|detailed|formal|casual)|keep (it|things|responses))\b")?,
            kind: MemoryKind::Policy,
            priority: 82,
        });
        // Language policy
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(speak|write|reply|answer)\s+(in|only in)\s+\w+")?,
            kind: MemoryKind::Policy,
            priority: 82,
        });

        // === GOAL patterns (priority 80) ===
        // Wants and needs
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i|we)\s+(want|need|plan|aim|intend|hope)\s+to\b")?,
            kind: MemoryKind::Goal,
            priority: 80,
        });
        // Goals and objectives
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(my goal is|i'm trying to|i'm (working|learning|studying))\b")?,
            kind: MemoryKind::Goal,
            priority: 80,
        });
        // Future aspirations
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(one day i|someday i|i dream of|in the future)\b")?,
            kind: MemoryKind::Goal,
            priority: 80,
        });
        // French goals
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(je veux|j'aimerais|mon objectif|je compte)\b")?,
            kind: MemoryKind::Goal,
            priority: 80,
        });

        // === DECISION patterns (priority 75) ===
        // Decisions made
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i|we)\s+(decided|will|chose|picked|selected|went with)\b")?,
            kind: MemoryKind::Decision,
            priority: 75,
        });
        // Future commitments
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i'm going to|we're going to|let's (go with|use|do))\b")?,
            kind: MemoryKind::Decision,
            priority: 75,
        });

        // === TASK patterns (priority 72) ===
        // Todo items
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(todo|to-do|next step|action item|need to do)\b")?,
            kind: MemoryKind::Task,
            priority: 72,
        });
        // Reminders
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(remind me to|don't forget to|remember to|i should)\b")?,
            kind: MemoryKind::Task,
            priority: 72,
        });

        // === FEEDBACK patterns (priority 70) ===
        // Praise/criticism
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(good job|well done|that's (wrong|incorrect|right)|you (should|shouldn't))\b")?,
            kind: MemoryKind::Feedback,
            priority: 70,
        });
        // Corrections
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(actually|no,|that's not|incorrect|you made a mistake)\b")?,
            kind: MemoryKind::Feedback,
            priority: 70,
        });

        // === CODE ARTIFACT patterns (priority 65) ===
        // File references
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(the (file|function|class|module|method|variable) (is|called|named))\b")?,
            kind: MemoryKind::CodeArtifact,
            priority: 65,
        });
        // File extensions
        rules.push(PatternRule {
            pattern: Regex::new(r"\.(rs|py|ts|tsx|js|jsx|go|java|cpp|c|h|hpp|css|html|json|yaml|yml|toml|sql)\b")?,
            kind: MemoryKind::CodeArtifact,
            priority: 65,
        });
        // Git references
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(commit|branch|merge|pull request|pr|issue)\s*(#?\d+|[a-f0-9]{7,})")?,
            kind: MemoryKind::CodeArtifact,
            priority: 65,
        });
        // Code locations
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(in|at|see)\s+[a-zA-Z_][a-zA-Z0-9_]*::\w+")?,
            kind: MemoryKind::CodeArtifact,
            priority: 65,
        });

        // === PROCEDURE patterns (priority 62) ===
        // How-to
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(how to|step\s*\d+|first,?\s+(you|we)|then,?\s+(you|we))\b")?,
            kind: MemoryKind::Procedure,
            priority: 62,
        });
        // Instructions
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(to do this|the process is|follow these|here's how)\b")?,
            kind: MemoryKind::Procedure,
            priority: 62,
        });

        // === FACT patterns (priority 60) ===
        // Self-describing facts
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i am|i'm|i have|i've got|i own|my .+ is)\b")?,
            kind: MemoryKind::Fact,
            priority: 60,
        });
        // Knowledge statements
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(i know|i remember|i learned|i read|i heard)\b")?,
            kind: MemoryKind::Fact,
            priority: 60,
        });
        // Project facts
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(the project|this (app|application|system|code|codebase))\s+(is|uses|has)\b")?,
            kind: MemoryKind::Fact,
            priority: 60,
        });

        // === DOCUMENT ARTIFACT patterns (priority 55) ===
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(the (document|doc|spec|readme|wiki|guide|manual))\s+(is|says|mentions)\b")?,
            kind: MemoryKind::DocumentArtifact,
            priority: 55,
        });
        rules.push(PatternRule {
            pattern: Regex::new(r"\.(md|txt|pdf|docx?|xlsx?|pptx?)\b")?,
            kind: MemoryKind::DocumentArtifact,
            priority: 55,
        });

        // === MEDIA ARTIFACT patterns (priority 50) ===
        rules.push(PatternRule {
            pattern: Regex::new(r"\.(png|jpg|jpeg|gif|svg|mp3|wav|mp4|webm|ogg)\b")?,
            kind: MemoryKind::MediaArtifact,
            priority: 50,
        });
        rules.push(PatternRule {
            pattern: Regex::new(r"(?i)\b(the (image|picture|photo|audio|video|sound))\s+(shows|is|was)\b")?,
            kind: MemoryKind::MediaArtifact,
            priority: 50,
        });

        // Sort by priority descending
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        Ok(Self {
            min_content_chars: config.min_content_chars,
            max_memory_chars: prompt.max_memory_chars,
            rules,
        })
    }

    /// Extract memory items from a transcript event.
    #[must_use]
    pub fn extract_from_event(&self, event: &TranscriptEvent) -> Vec<MemoryItem> {
        let source = match event.role {
            TranscriptRole::User => MemorySource::User,
            TranscriptRole::Assistant => MemorySource::Assistant,
            TranscriptRole::Tool => MemorySource::Tool,
            TranscriptRole::System => MemorySource::System,
        };

        self.extract_from_text(event.session_id, source, &event.content)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn extract_from_text(
        &self,
        session_id: SessionId,
        source: MemorySource,
        text: &str,
    ) -> Vec<MemoryItem> {
        let mut items = Vec::new();
        for chunk in split_sentences(text) {
            let chunk = chunk.trim();
            if chunk.chars().count() < self.min_content_chars {
                continue;
            }

            // Find the first matching rule (highest priority wins)
            let kind = match self.rules.iter().find(|r| r.pattern.is_match(chunk)) {
                Some(rule) => rule.kind,
                None => continue,
            };

            let metadata = MemoryMetadata::new(source.clone()).with_salience(default_salience(kind));
            let candidate = match MemoryItem::new(session_id, kind, chunk.to_string(), metadata) {
                Ok(item) => item.truncate_to_budget(self.max_memory_chars),
                Err(_) => continue,
            };

            if let Err(err) = candidate.validate(self.max_memory_chars) {
                debug!("Skipping memory candidate: {err}");
                continue;
            }

            items.push(candidate);
        }

        items
    }
}

fn split_sentences(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    for (idx, ch) in text.char_indices() {
        if matches!(ch, '.' | '!' | '?' | '\n') {
            if start < idx {
                parts.push(&text[start..idx]);
            }
            start = idx + ch.len_utf8();
        }
    }
    if start < text.len() {
        parts.push(&text[start..]);
    }
    parts
}

const fn default_salience(kind: MemoryKind) -> u8 {
    match kind {
        MemoryKind::Identity => 90,
        MemoryKind::Constraint | MemoryKind::Policy => 80,
        MemoryKind::Decision => 75,
        MemoryKind::Preference | MemoryKind::Goal | MemoryKind::Aversion | MemoryKind::Feedback => {
            70
        }
        MemoryKind::ToolResult | MemoryKind::CodeArtifact => 65,
        MemoryKind::Fact | MemoryKind::Procedure | MemoryKind::Task | MemoryKind::Plan => 60,
        MemoryKind::Episode
        | MemoryKind::Reflection
        | MemoryKind::DocumentArtifact
        | MemoryKind::MediaArtifact => 55,
        MemoryKind::Summary | MemoryKind::Other | MemoryKind::Unknown => 50,
    }
}
