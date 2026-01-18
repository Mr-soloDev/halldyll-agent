//! Metadata associated with each memory item.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Source of a memory item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    /// User provided content.
    User,
    /// Assistant response.
    Assistant,
    /// Tool output.
    Tool,
    /// System generated content.
    System,
    /// Speech-to-text transcription.
    Stt {
        /// Model used for transcription.
        model: String,
        /// Confidence score (0.0 - 1.0).
        confidence: f32,
    },
    /// Text-to-speech generation.
    Tts {
        /// Model used for synthesis.
        model: String,
    },
    /// Image generation or analysis.
    ImageGen {
        /// Model used for image generation.
        model: String,
        /// Original prompt used.
        prompt: String,
    },
    /// Image analysis/vision.
    Vision {
        /// Model used for vision.
        model: String,
    },
}

impl MemorySource {
    /// Get the model name if this source has one.
    #[must_use]
    pub fn model_name(&self) -> Option<&str> {
        match self {
            Self::Stt { model, .. }
            | Self::Tts { model }
            | Self::ImageGen { model, .. }
            | Self::Vision { model } => Some(model),
            Self::User | Self::Assistant | Self::Tool | Self::System => None,
        }
    }

    /// Check if this source is from a multi-modal model.
    #[must_use]
    pub const fn is_multimodal(&self) -> bool {
        matches!(
            self,
            Self::Stt { .. } | Self::Tts { .. } | Self::ImageGen { .. } | Self::Vision { .. }
        )
    }
}

/// Content modality for multi-modal memory tracking.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    /// Plain text content.
    Text,
    /// Audio content (speech, music, sound).
    Audio,
    /// Image content (photos, graphics, screenshots).
    Image,
    /// Video content.
    Video,
    /// Source code or structured data.
    Code,
    /// Mixed/multiple modalities.
    Multimodal,
}

impl Modality {
    /// Stable tag string for this modality.
    #[must_use]
    pub const fn as_tag(self) -> &'static str {
        match self {
            Self::Text => "modality:text",
            Self::Audio => "modality:audio",
            Self::Image => "modality:image",
            Self::Video => "modality:video",
            Self::Code => "modality:code",
            Self::Multimodal => "modality:multimodal",
        }
    }

    /// Parse from a tag string.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "modality:text" => Some(Self::Text),
            "modality:audio" => Some(Self::Audio),
            "modality:image" => Some(Self::Image),
            "modality:video" => Some(Self::Video),
            "modality:code" => Some(Self::Code),
            "modality:multimodal" => Some(Self::Multimodal),
            _ => None,
        }
    }
}

/// Metadata for a memory item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryMetadata {
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Salience score (0-100).
    pub salience: u8,
    /// Optional tags to aid retrieval.
    pub tags: Vec<String>,
    /// Optional time-to-live in seconds.
    pub ttl_seconds: Option<u64>,
    /// Source of the memory.
    pub source: MemorySource,
    /// Number of times this memory has been retrieved.
    #[serde(default)]
    pub retrieval_count: u32,
    /// Timestamp of last retrieval.
    #[serde(default)]
    pub last_retrieved_at: Option<DateTime<Utc>>,
}

impl MemoryMetadata {
    /// Create metadata with defaults and a given source.
    #[must_use]
    pub fn new(source: MemorySource) -> Self {
        Self {
            created_at: Utc::now(),
            updated_at: Utc::now(),
            salience: 50,
            tags: Vec::new(),
            ttl_seconds: None,
            source,
            retrieval_count: 0,
            last_retrieved_at: None,
        }
    }

    /// Update the salience score.
    #[must_use]
    pub const fn with_salience(mut self, salience: u8) -> Self {
        self.salience = salience;
        self
    }

    /// Attach tags to the metadata.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Attach a TTL in seconds.
    #[must_use]
    pub const fn with_ttl(mut self, ttl_seconds: Option<u64>) -> Self {
        self.ttl_seconds = ttl_seconds;
        self
    }

    /// Record that this memory was retrieved.
    ///
    /// Increments the retrieval count and updates the last retrieved timestamp.
    pub fn record_retrieval(&mut self) {
        self.retrieval_count = self.retrieval_count.saturating_add(1);
        self.last_retrieved_at = Some(Utc::now());
    }

    /// Compute dynamic salience based on usage patterns.
    ///
    /// The dynamic salience boosts the base salience based on:
    /// - How often the memory has been retrieved (usage boost)
    /// - How recently it was retrieved (recency factor)
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Precision loss acceptable for time calculations
    pub fn dynamic_salience(&self) -> u8 {
        let base = f64::from(self.salience);

        // Usage boost: logarithmic growth based on retrieval count
        let usage_boost = f64::from(self.retrieval_count).ln_1p() * 5.0;

        // Recency factor: how recently was this memory retrieved
        let recency_factor = self.last_retrieved_at.map_or(0.5, |last| {
            let seconds_ago = Utc::now()
                .signed_duration_since(last)
                .num_seconds()
                .max(0) as f64;
            let days_ago = seconds_ago / 86400.0;
            1.0 / (1.0 + days_ago / 7.0)
        });

        let dynamic = base + usage_boost * recency_factor;

        // Clamp to valid range
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let clamped = dynamic.clamp(0.0, 100.0) as u8;
        clamped
    }

    /// Check if this memory is frequently used.
    ///
    /// A memory is considered frequently used if it has been retrieved
    /// at least 3 times in the last 7 days.
    #[must_use]
    pub fn is_frequently_used(&self) -> bool {
        if self.retrieval_count < 3 {
            return false;
        }

        self.last_retrieved_at.is_some_and(|last| {
            let days_ago = Utc::now()
                .signed_duration_since(last)
                .num_days();
            days_ago <= 7
        })
    }

    /// Add a modality tag to this metadata.
    pub fn add_modality(&mut self, modality: Modality) {
        let tag = modality.as_tag().to_string();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Get the primary modality from tags, if any.
    #[must_use]
    pub fn primary_modality(&self) -> Option<Modality> {
        self.tags.iter().find_map(|t| Modality::from_tag(t))
    }

    /// Get all modalities from tags.
    #[must_use]
    pub fn modalities(&self) -> Vec<Modality> {
        self.tags.iter().filter_map(|t| Modality::from_tag(t)).collect()
    }

    /// Check if this memory has a specific modality.
    #[must_use]
    pub fn has_modality(&self, modality: Modality) -> bool {
        self.tags.contains(&modality.as_tag().to_string())
    }

    /// Add a model tag (e.g., "model:whisper", "model:ministral").
    pub fn add_model_tag(&mut self, model_name: &str) {
        let tag = format!("model:{model_name}");
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Get the model name from tags, if present.
    #[must_use]
    pub fn model_from_tags(&self) -> Option<&str> {
        self.tags
            .iter()
            .find(|t| t.starts_with("model:"))
            .map(|t| &t[6..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modality_tag_roundtrip() {
        for modality in [
            Modality::Text,
            Modality::Audio,
            Modality::Image,
            Modality::Video,
            Modality::Code,
            Modality::Multimodal,
        ] {
            let tag = modality.as_tag();
            assert_eq!(Modality::from_tag(tag), Some(modality));
        }
    }

    #[test]
    fn test_add_modality() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        meta.add_modality(Modality::Audio);
        meta.add_modality(Modality::Text);

        assert!(meta.has_modality(Modality::Audio));
        assert!(meta.has_modality(Modality::Text));
        assert!(!meta.has_modality(Modality::Image));
    }

    #[test]
    fn test_primary_modality() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        meta.add_modality(Modality::Audio);
        meta.add_modality(Modality::Text);

        // First added modality is primary
        assert_eq!(meta.primary_modality(), Some(Modality::Audio));
    }

    #[test]
    fn test_modalities_list() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        meta.add_modality(Modality::Audio);
        meta.add_modality(Modality::Text);
        meta.add_modality(Modality::Code);

        let modalities = meta.modalities();
        assert_eq!(modalities.len(), 3);
        assert!(modalities.contains(&Modality::Audio));
        assert!(modalities.contains(&Modality::Text));
        assert!(modalities.contains(&Modality::Code));
    }

    #[test]
    fn test_model_tag() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        meta.add_model_tag("whisper-large-v3");

        assert_eq!(meta.model_from_tags(), Some("whisper-large-v3"));
    }

    #[test]
    fn test_stt_source() {
        let source = MemorySource::Stt {
            model: "whisper".to_string(),
            confidence: 0.95,
        };
        assert!(source.is_multimodal());
        assert_eq!(source.model_name(), Some("whisper"));
    }

    #[test]
    fn test_no_duplicate_modality_tags() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        meta.add_modality(Modality::Audio);
        meta.add_modality(Modality::Audio);
        meta.add_modality(Modality::Audio);

        let count = meta.tags.iter().filter(|t| *t == "modality:audio").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_record_retrieval() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        assert_eq!(meta.retrieval_count, 0);
        assert!(meta.last_retrieved_at.is_none());

        meta.record_retrieval();
        assert_eq!(meta.retrieval_count, 1);
        assert!(meta.last_retrieved_at.is_some());

        meta.record_retrieval();
        assert_eq!(meta.retrieval_count, 2);
    }

    #[test]
    fn test_dynamic_salience_no_retrievals() {
        let meta = MemoryMetadata::new(MemorySource::User);
        // Base salience is 50, with no retrievals the boost is minimal
        let dynamic = meta.dynamic_salience();
        assert!(dynamic >= 50);
        assert!(dynamic <= 55); // Small boost from ln(1) * 5 * 0.5
    }

    #[test]
    fn test_dynamic_salience_with_retrievals() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        meta.retrieval_count = 10;
        meta.last_retrieved_at = Some(Utc::now());

        let dynamic = meta.dynamic_salience();
        // Should be boosted above base salience
        assert!(dynamic > 50);
    }

    #[test]
    fn test_dynamic_salience_capped_at_100() {
        let mut meta = MemoryMetadata::new(MemorySource::User).with_salience(95);
        meta.retrieval_count = 100;
        meta.last_retrieved_at = Some(Utc::now());

        let dynamic = meta.dynamic_salience();
        assert!(dynamic <= 100);
    }

    #[test]
    fn test_is_frequently_used() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        assert!(!meta.is_frequently_used());

        meta.retrieval_count = 3;
        meta.last_retrieved_at = Some(Utc::now());
        assert!(meta.is_frequently_used());
    }

    #[test]
    fn test_is_frequently_used_old_retrieval() {
        let mut meta = MemoryMetadata::new(MemorySource::User);
        meta.retrieval_count = 10;
        // 30 days ago
        meta.last_retrieved_at = Some(Utc::now() - chrono::Duration::days(30));
        assert!(!meta.is_frequently_used());
    }
}
