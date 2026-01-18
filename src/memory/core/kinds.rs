//! Memory item kinds.
//!
//! This module defines the semantic **category** of a memory item.
//!
//! Why this exists:
//! - Filtering: fetch only goals, only preferences, only artifacts, etc.
//! - Prioritization: some kinds should weigh more during retrieval.
//! - Retention intent: different kinds often have different lifetimes.
//! - Prompt formatting: emit explicit tags like `(fact)`.
//!
//! Notes:
//! - The kind is intentionally payload-agnostic: it classifies meaning, not modality.
//!   (Modality like text/image/audio belongs in metadata/content descriptors.)
//! - This enum uses stable `snake_case` identifiers for storage and interoperability.
//! - This module provides **priors** (importance/retention/merge) but does not implement
//!   ranking, deduplication, or TTL policies.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Version of the kind schema (useful for storage migrations / telemetry).
pub const KIND_SCHEMA_VERSION: u16 = 1;

/// Coarse retention class for a memory kind.
///
/// This does **not** define concrete durations (that belongs to policy/config).
/// It only communicates intent so downstream components can map it to TTL rules.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RetentionClass {
    /// Should never expire automatically.
    Permanent,
    /// Very long lived (months/years).
    Long,
    /// Medium lived (weeks/months).
    Medium,
    /// Short lived (hours/days).
    Short,
    /// Ephemeral (best-effort, likely to be dropped quickly).
    Ephemeral,
}

impl RetentionClass {
    /// Stable string representation (for logs/telemetry).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Permanent => "permanent",
            Self::Long => "long",
            Self::Medium => "medium",
            Self::Short => "short",
            Self::Ephemeral => "ephemeral",
        }
    }
}

impl fmt::Display for RetentionClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// High-level family used for broad filtering and policy mapping.
///
/// This is stable and intentionally small; you should not need to change it often.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MemoryFamily {
    /// Stable “truths” about the user/agent/project (identity, facts, preferences, rules).
    Semantic,
    /// Records of “what happened” (events/observations) and compressions of those events.
    Episodic,
    /// “How to do things” / action guidance / planning state.
    Procedural,
    /// References to external artifacts (code, docs, media).
    Artifact,
    /// Operational/meta items (tool outputs, feedback, etc.).
    Meta,
    /// Uncategorized (deliberate misc).
    Other,
    /// Forward-compat bucket.
    Unknown,
}

impl MemoryFamily {
    /// Stable string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Episodic => "episodic",
            Self::Procedural => "procedural",
            Self::Artifact => "artifact",
            Self::Meta => "meta",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for MemoryFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Merge hint for the *meaning* of updates.
///
/// This is **not** deduplication logic. It simply communicates how items of this kind
/// typically evolve over time.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MergeHint {
    /// New information should replace the old (e.g., identity, facts, constraints).
    Replace,
    /// Keep history (append new instances).
    Append,
    /// Keep multiple and/or fold into an aggregate representation.
    Accumulate,
}

impl MergeHint {
    /// Stable string representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Append => "append",
            Self::Accumulate => "accumulate",
        }
    }
}

impl fmt::Display for MergeHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The semantic category of a memory item.
///
/// Design guidance:
/// - **Semantic** memory: identity/facts/preferences/constraints/policies.
/// - **Episodic** memory: events/observations (what happened).
/// - **Procedural** memory: goals/tasks/plans/decisions/procedures (how to do / what to do).
///
/// IMPORTANT:
/// - `Other` means “we chose not to categorize further”.
/// - `Unknown` means “we received a kind we don't recognize” (forward compatibility).
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MemoryKind {
    /// User identity or stable profile anchors (name, identity-like constants).
    Identity,

    /// A stable fact about the user, agent, or project (semantic truth).
    Fact,

    /// A positive preference (likes, favorites, defaults).
    Preference,

    /// A negative preference / dislike / aversion.
    Aversion,

    /// A hard constraint or prohibition (must not / cannot).
    Constraint,

    /// Operating rule / policy / instruction that should persist across turns.
    /// (e.g., “no unsafe”, “answer in French”, “keep responses concise”.)
    Policy,

    /// A long(er)-term objective.
    Goal,

    /// A short-term actionable item (todo / next step).
    Task,

    /// A multi-step plan / approach (often derived from goals + constraints).
    Plan,

    /// A decision or chosen option (often time-sensitive).
    Decision,

    /// A procedural “how-to” / playbook / runbook / routine.
    /// This is distinct from `Policy` (rules) and `Plan` (a specific plan for a situation).
    Procedure,

    /// An observation / event / episode (“what happened”).
    Episode,

    /// An abstracted insight synthesized from episodes (reflection).
    Reflection,

    /// A summary chunk (conversation summary, episodic compression).
    Summary,

    /// Direct user feedback about the agent/system behavior (what to change).
    Feedback,

    /// A tool call result worth retaining (command output, retrieval result, etc.).
    ToolResult,

    /// A reference to code that matters (file path, symbol, patch id, commit).
    CodeArtifact,

    /// A reference to a non-code artifact (doc, dataset, spec, ticket, note).
    DocumentArtifact,

    /// A reference to a media artifact (image/audio/video) or generated asset.
    MediaArtifact,

    /// Uncategorized (deliberate "misc" bucket).
    #[default]
    Other,

    /// Forward-compatibility bucket for unknown/added-later kinds.
    #[serde(other)]
    Unknown,
}

/// Parse error for [`MemoryKind`].
#[derive(Debug, Clone)]
pub struct MemoryKindParseError {
    value: String,
}

impl MemoryKindParseError {
    /// The raw value that failed parsing.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for MemoryKindParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid memory kind: {}", self.value)
    }
}

impl std::error::Error for MemoryKindParseError {}

impl MemoryKind {
    /// All known kinds (excluding `Unknown`).
    pub const ALL: &'static [Self] = &[
        Self::Identity,
        Self::Fact,
        Self::Preference,
        Self::Aversion,
        Self::Constraint,
        Self::Policy,
        Self::Goal,
        Self::Task,
        Self::Plan,
        Self::Decision,
        Self::Procedure,
        Self::Episode,
        Self::Reflection,
        Self::Summary,
        Self::Feedback,
        Self::ToolResult,
        Self::CodeArtifact,
        Self::DocumentArtifact,
        Self::MediaArtifact,
        Self::Other,
    ];

    /// Stable storage identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Fact => "fact",
            Self::Preference => "preference",
            Self::Aversion => "aversion",
            Self::Constraint => "constraint",
            Self::Policy => "policy",
            Self::Goal => "goal",
            Self::Task => "task",
            Self::Plan => "plan",
            Self::Decision => "decision",
            Self::Procedure => "procedure",
            Self::Episode => "episode",
            Self::Reflection => "reflection",
            Self::Summary => "summary",
            Self::Feedback => "feedback",
            Self::ToolResult => "tool_result",
            Self::CodeArtifact => "code_artifact",
            Self::DocumentArtifact => "document_artifact",
            Self::MediaArtifact => "media_artifact",
            Self::Other => "other",
            Self::Unknown => "unknown",
        }
    }

    /// Small stable numeric code for compact storage and indexing.
    ///
    /// This is intentionally **not** derived from enum discriminants to preserve stability.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Identity => 1,
            Self::Fact => 2,
            Self::Preference => 3,
            Self::Aversion => 4,
            Self::Constraint => 5,
            Self::Policy => 6,
            Self::Goal => 7,
            Self::Task => 8,
            Self::Plan => 9,
            Self::Decision => 10,
            Self::Procedure => 11,
            Self::Episode => 12,
            Self::Reflection => 13,
            Self::Summary => 14,
            Self::Feedback => 15,
            Self::ToolResult => 16,
            Self::CodeArtifact => 17,
            Self::DocumentArtifact => 18,
            Self::MediaArtifact => 19,
            Self::Other => 20,
            Self::Unknown => 255,
        }
    }

    /// Convert a stored numeric code back into a [`MemoryKind`].
    #[must_use]
    pub const fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Identity,
            2 => Self::Fact,
            3 => Self::Preference,
            4 => Self::Aversion,
            5 => Self::Constraint,
            6 => Self::Policy,
            7 => Self::Goal,
            8 => Self::Task,
            9 => Self::Plan,
            10 => Self::Decision,
            11 => Self::Procedure,
            12 => Self::Episode,
            13 => Self::Reflection,
            14 => Self::Summary,
            15 => Self::Feedback,
            16 => Self::ToolResult,
            17 => Self::CodeArtifact,
            18 => Self::DocumentArtifact,
            19 => Self::MediaArtifact,
            20 => Self::Other,
            _ => Self::Unknown,
        }
    }

    /// Tag suitable for prompt injection formatting, e.g. `(fact)`.
    #[must_use]
    pub const fn prompt_tag(self) -> &'static str {
        match self {
            Self::Identity => "(identity)",
            Self::Fact => "(fact)",
            Self::Preference => "(preference)",
            Self::Aversion => "(aversion)",
            Self::Constraint => "(constraint)",
            Self::Policy => "(policy)",
            Self::Goal => "(goal)",
            Self::Task => "(task)",
            Self::Plan => "(plan)",
            Self::Decision => "(decision)",
            Self::Procedure => "(procedure)",
            Self::Episode => "(episode)",
            Self::Reflection => "(reflection)",
            Self::Summary => "(summary)",
            Self::Feedback => "(feedback)",
            Self::ToolResult => "(tool_result)",
            Self::CodeArtifact => "(code_artifact)",
            Self::DocumentArtifact => "(document_artifact)",
            Self::MediaArtifact => "(media_artifact)",
            Self::Other => "(other)",
            Self::Unknown => "(unknown)",
        }
    }

    /// Broad family classification.
    #[must_use]
    pub const fn family(self) -> MemoryFamily {
        match self {
            Self::Identity
            | Self::Fact
            | Self::Preference
            | Self::Aversion
            | Self::Constraint
            | Self::Policy => MemoryFamily::Semantic,

            Self::Episode | Self::Reflection | Self::Summary => MemoryFamily::Episodic,

            Self::Goal | Self::Task | Self::Plan | Self::Decision | Self::Procedure => {
                MemoryFamily::Procedural
            }

            Self::CodeArtifact | Self::DocumentArtifact | Self::MediaArtifact => {
                MemoryFamily::Artifact
            }

            Self::Feedback | Self::ToolResult => MemoryFamily::Meta,

            Self::Other => MemoryFamily::Other,
            Self::Unknown => MemoryFamily::Unknown,
        }
    }

    /// Default importance score in range `0..=100`.
    ///
    /// This is a prior hint for retrievers/rankers. Do not treat as an absolute truth.
    #[must_use]
    pub const fn default_importance(self) -> u8 {
        match self {
            Self::Identity => 100,
            Self::Constraint => 95,
            Self::Fact => 90,
            Self::Policy => 88,
            Self::Goal => 85,
            Self::Procedure => 82,
            Self::Preference | Self::Aversion => 80,
            Self::Decision | Self::Plan => 75,
            Self::Task | Self::Reflection | Self::Feedback => 70,
            Self::CodeArtifact => 65,
            Self::Summary => 60,
            Self::DocumentArtifact | Self::MediaArtifact => 55,
            Self::Episode => 45,
            Self::ToolResult => 35,
            Self::Other => 20,
            Self::Unknown => 10,
        }
    }

    /// Default retention class (policy/config decides concrete TTL).
    #[must_use]
    pub const fn default_retention(self) -> RetentionClass {
        match self {
            Self::Identity => RetentionClass::Permanent,

            Self::Fact
            | Self::Preference
            | Self::Aversion
            | Self::Constraint
            | Self::Policy
            | Self::Procedure
            | Self::CodeArtifact
            | Self::DocumentArtifact
            | Self::MediaArtifact => RetentionClass::Long,

            Self::Goal
            | Self::Reflection
            | Self::Summary
            | Self::Decision
            | Self::Plan
            | Self::Episode
            | Self::Feedback => RetentionClass::Medium,

            Self::Task | Self::Other => RetentionClass::Short,

            Self::ToolResult | Self::Unknown => RetentionClass::Ephemeral,
        }
    }

    /// Merge prior for typical evolution of this kind.
    #[must_use]
    pub const fn merge_hint(self) -> MergeHint {
        match self {
            Self::Identity
            | Self::Fact
            | Self::Preference
            | Self::Aversion
            | Self::Constraint
            | Self::Policy
            | Self::Procedure => MergeHint::Replace,

            Self::Goal
            | Self::Task
            | Self::Plan
            | Self::Decision
            | Self::Reflection
            | Self::Summary
            | Self::Feedback
            | Self::CodeArtifact
            | Self::DocumentArtifact
            | Self::MediaArtifact => MergeHint::Accumulate,

            Self::Episode | Self::ToolResult | Self::Other | Self::Unknown => MergeHint::Append,
        }
    }

    /// True if this kind is primarily “profile/semantic” memory about the user/agent.
    #[must_use]
    pub const fn is_profile_semantic(self) -> bool {
        matches!(self.family(), MemoryFamily::Semantic)
    }

    /// True if this kind is primarily planning/procedural memory.
    #[must_use]
    pub const fn is_planning(self) -> bool {
        matches!(self.family(), MemoryFamily::Procedural)
    }

    /// True if this kind is primarily episodic (“what happened”) memory.
    #[must_use]
    pub const fn is_episodic(self) -> bool {
        matches!(self.family(), MemoryFamily::Episodic)
    }

    /// True if this kind references an artifact rather than a pure statement.
    #[must_use]
    pub const fn is_artifact(self) -> bool {
        matches!(self.family(), MemoryFamily::Artifact)
    }

    /// Lossy parsing: returns `Unknown` instead of failing.
    #[must_use]
    pub fn parse_lossy(s: &str) -> Self {
        Self::from_str(s).unwrap_or(Self::Unknown)
    }
}

impl fmt::Display for MemoryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Private: classify ASCII separators.
const fn is_sep_byte(b: u8) -> bool {
    matches!(b, b'_' | b'-' | b' ' | b'.' | b'/' | b'\\' | b':')
}

/// Private: ASCII alnum check.
const fn is_ascii_alnum(b: u8) -> bool {
    (b >= b'A' && b <= b'Z') || (b >= b'a' && b <= b'z') || (b >= b'0' && b <= b'9')
}

/// Private: ASCII lowercase or digit check.
const fn is_ascii_lower_or_digit(b: u8) -> bool {
    (b >= b'a' && b <= b'z') || (b >= b'0' && b <= b'9')
}

/// Private: ASCII uppercase check.
const fn is_ascii_upper(b: u8) -> bool {
    b >= b'A' && b <= b'Z'
}

/// Private: ASCII lowercase mapping.
const fn to_ascii_lower(b: u8) -> u8 {
    if b >= b'A' && b <= b'Z' {
        b + (b'a' - b'A')
    } else {
        b
    }
}

/// Private iterator that normalizes an input kind string into a snake_case-like stream:
/// - ignores leading separators
/// - collapses repeated separators
/// - converts to lowercase
/// - treats CamelCase boundaries as separators (e.g., `ToolResult` -> `tool_result`)
struct KindNormIter<'a> {
    bytes: &'a [u8],
    idx: usize,
    pending_sep: bool,
    last_input_was_lower_or_digit: bool,
    emitted_any: bool,
    carry: Option<u8>,
}

impl<'a> KindNormIter<'a> {
    const fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            idx: 0,
            pending_sep: false,
            last_input_was_lower_or_digit: false,
            emitted_any: false,
            carry: None,
        }
    }

    fn next_norm_byte(&mut self) -> Option<u8> {
        if let Some(b) = self.carry.take() {
            return Some(b);
        }

        while self.idx < self.bytes.len() {
            let b = self.bytes[self.idx];
            self.idx += 1;

            if is_sep_byte(b) {
                self.pending_sep = true;
                self.last_input_was_lower_or_digit = false;
                continue;
            }

            if !is_ascii_alnum(b) {
                // Treat non-alnum as separator.
                self.pending_sep = true;
                self.last_input_was_lower_or_digit = false;
                continue;
            }

            let lower = to_ascii_lower(b);
            let camel_boundary = is_ascii_upper(b) && self.last_input_was_lower_or_digit;

            // Update last_input_was_lower_or_digit based on the raw input byte.
            self.last_input_was_lower_or_digit = is_ascii_lower_or_digit(b);

            // Emit '_' if we have a pending separator or CamelCase boundary and we have emitted
            // something already (avoid leading underscore).
            if self.emitted_any && (self.pending_sep || camel_boundary) {
                self.pending_sep = false;
                self.carry = Some(lower);
                return Some(b'_');
            }

            self.pending_sep = false;
            self.emitted_any = true;
            return Some(lower);
        }

        None
    }
}

/// Private: normalized match without allocation.
fn matches_canonical(input: &str, canonical: &str) -> bool {
    let mut it = KindNormIter::new(input);
    let mut cb = canonical.as_bytes().iter().copied();

    loop {
        match (it.next_norm_byte(), cb.next()) {
            (None, None) => return true,
            (Some(a), Some(b)) if a == b => {}
            _ => return false,
        }
    }
}

impl FromStr for MemoryKind {
    type Err = MemoryKindParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let raw = s.trim();

        // Short aliases (keep these explicit: they are not canonical identifiers).
        if raw.eq_ignore_ascii_case("pref") {
            return Ok(Self::Preference);
        }
        if raw.eq_ignore_ascii_case("todo") {
            return Ok(Self::Task);
        }
        if raw.eq_ignore_ascii_case("doc") {
            return Ok(Self::DocumentArtifact);
        }

        // Canonical identifiers (accept snake_case, kebab-case, spaces, CamelCase).
        let kind = if matches_canonical(raw, "identity") {
            Self::Identity
        } else if matches_canonical(raw, "fact") {
            Self::Fact
        } else if matches_canonical(raw, "preference") {
            Self::Preference
        } else if matches_canonical(raw, "aversion") || matches_canonical(raw, "dislike") {
            Self::Aversion
        } else if matches_canonical(raw, "constraint") || matches_canonical(raw, "rule") {
            Self::Constraint
        } else if matches_canonical(raw, "policy") || matches_canonical(raw, "instruction") {
            Self::Policy
        } else if matches_canonical(raw, "goal") || matches_canonical(raw, "objective") {
            Self::Goal
        } else if matches_canonical(raw, "task") {
            Self::Task
        } else if matches_canonical(raw, "plan") {
            Self::Plan
        } else if matches_canonical(raw, "decision") || matches_canonical(raw, "choice") {
            Self::Decision
        } else if matches_canonical(raw, "procedure")
            || matches_canonical(raw, "playbook")
            || matches_canonical(raw, "runbook")
            || matches_canonical(raw, "workflow")
        {
            Self::Procedure
        } else if matches_canonical(raw, "episode")
            || matches_canonical(raw, "event")
            || matches_canonical(raw, "observation")
        {
            Self::Episode
        } else if matches_canonical(raw, "reflection") || matches_canonical(raw, "insight") {
            Self::Reflection
        } else if matches_canonical(raw, "summary") {
            Self::Summary
        } else if matches_canonical(raw, "feedback") {
            Self::Feedback
        } else if matches_canonical(raw, "tool_result")
            || matches_canonical(raw, "tooloutput")
            || matches_canonical(raw, "tool_output")
            || matches_canonical(raw, "tool")
        {
            Self::ToolResult
        } else if matches_canonical(raw, "code_artifact") || matches_canonical(raw, "code") {
            Self::CodeArtifact
        } else if matches_canonical(raw, "document_artifact") || matches_canonical(raw, "document")
        {
            Self::DocumentArtifact
        } else if matches_canonical(raw, "media_artifact") || matches_canonical(raw, "media") {
            Self::MediaArtifact
        } else if matches_canonical(raw, "other") {
            Self::Other
        } else if matches_canonical(raw, "unknown") {
            Self::Unknown
        } else {
            return Err(MemoryKindParseError {
                value: raw.to_string(),
            });
        };

        Ok(kind)
    }
}
