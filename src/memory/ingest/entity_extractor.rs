//! Named entity extraction for memory enrichment.
//!
//! Extracts structured entities from text: names, locations, emails, URLs, file paths, etc.

use regex::Regex;
use std::collections::HashSet;

/// Types of entities that can be extracted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    /// A person's name
    Person,
    /// A geographic location (city, country, address)
    Location,
    /// An organization or company name
    Organization,
    /// An email address
    Email,
    /// A URL or web address
    Url,
    /// A file system path
    FilePath,
    /// A programming language or technology
    Technology,
    /// A date or time reference
    DateTime,
    /// A numeric value with unit (e.g., "5 hours", "100MB")
    Quantity,
}

impl EntityType {
    /// String representation for storage.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Location => "location",
            Self::Organization => "organization",
            Self::Email => "email",
            Self::Url => "url",
            Self::FilePath => "file_path",
            Self::Technology => "technology",
            Self::DateTime => "datetime",
            Self::Quantity => "quantity",
        }
    }
}

/// An extracted entity with its context.
#[derive(Debug, Clone)]
pub struct ExtractedEntity {
    /// The type of entity.
    pub entity_type: EntityType,
    /// The raw extracted value.
    pub value: String,
    /// Surrounding context (the sentence or phrase containing the entity).
    pub context: String,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
}

/// Pattern-based named entity extractor.
#[allow(clippy::struct_field_names)]
pub struct EntityExtractor {
    email_pattern: Regex,
    url_pattern: Regex,
    file_path_pattern: Regex,
    technology_pattern: Regex,
    datetime_pattern: Regex,
    quantity_pattern: Regex,
    person_pattern: Regex,
    location_pattern: Regex,
}

impl EntityExtractor {
    /// Create a new entity extractor.
    ///
    /// # Errors
    /// Returns an error if any regex pattern is invalid.
    pub fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            // Email: standard format
            email_pattern: Regex::new(
                r"(?i)[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
            )?,

            // URL: http(s), ftp protocols
            url_pattern: Regex::new(
                r"(?i)(https?|ftp)://[^\s<>]+",
            )?,

            // File paths: common extensions
            file_path_pattern: Regex::new(
                r"[\w./-]+\.(rs|py|ts|tsx|js|jsx|go|java|cpp|c|h|hpp|css|html|json|yaml|yml|toml|sql|md|txt)",
            )?,

            // Technologies and programming languages
            technology_pattern: Regex::new(
                r"(?i)\b(Rust|Python|JavaScript|TypeScript|Go|Java|C\+\+|Ruby|PHP|Swift|Kotlin|React|Vue|Angular|Nodejs|Node|Django|Flask|FastAPI|Spring|Rails|Docker|Kubernetes|AWS|Azure|GCP|PostgreSQL|MySQL|MongoDB|Redis|GraphQL|REST|gRPC|Tauri|Electron|SQLite|Git|GitHub|GitLab)\b",
            )?,

            // DateTime patterns (simplified)
            datetime_pattern: Regex::new(
                r"(?i)\b(\d{1,2}/\d{1,2}/\d{2,4}|\d{4}-\d{1,2}-\d{1,2}|today|yesterday|tomorrow|last week|next week|this month)\b",
            )?,

            // Quantities with units
            quantity_pattern: Regex::new(
                r"(?i)\b(\d+(?:\.\d+)?)\s*(kb|mb|gb|tb|bytes?|seconds?|minutes?|hours?|days?|weeks?|months?|years?|ms|px|em|rem|%)\b",
            )?,

            // Person names (preceded by name indicators)
            person_pattern: Regex::new(
                r"(?i)(?:my name is|call me|i am|created by|author)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
            )?,

            // Locations (preceded by location indicators)
            location_pattern: Regex::new(
                r"(?i)(?:in|from|at|based in|located in|live in)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)",
            )?,
        })
    }

    /// Extract all entities from the given text.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn extract(&self, text: &str) -> Vec<ExtractedEntity> {
        let mut entities = Vec::new();
        let mut seen: HashSet<(EntityType, String)> = HashSet::new();

        self.extract_emails(text, &mut entities, &mut seen);
        self.extract_urls(text, &mut entities, &mut seen);
        self.extract_file_paths(text, &mut entities, &mut seen);
        self.extract_technologies(text, &mut entities, &mut seen);
        self.extract_datetimes(text, &mut entities, &mut seen);
        self.extract_quantities(text, &mut entities, &mut seen);
        self.extract_persons(text, &mut entities, &mut seen);
        self.extract_locations(text, &mut entities, &mut seen);

        entities
    }

    fn extract_emails(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashSet<(EntityType, String)>,
    ) {
        for cap in self.email_pattern.find_iter(text) {
            let value = cap.as_str().to_string();
            let key = (EntityType::Email, value.clone());
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Email,
                    value,
                    context: extract_context(text, cap.start(), cap.end()),
                    confidence: 0.95,
                });
            }
        }
    }

    fn extract_urls(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashSet<(EntityType, String)>,
    ) {
        for cap in self.url_pattern.find_iter(text) {
            let value = cap.as_str().to_string();
            let key = (EntityType::Url, value.clone());
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Url,
                    value,
                    context: extract_context(text, cap.start(), cap.end()),
                    confidence: 0.95,
                });
            }
        }
    }

    fn extract_file_paths(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashSet<(EntityType, String)>,
    ) {
        for cap in self.file_path_pattern.find_iter(text) {
            let value = cap.as_str().to_string();
            // Filter out false positives
            if !value.starts_with("http") && value.len() > 3 {
                let key = (EntityType::FilePath, value.clone());
                if seen.insert(key) {
                    entities.push(ExtractedEntity {
                        entity_type: EntityType::FilePath,
                        value,
                        context: extract_context(text, cap.start(), cap.end()),
                        confidence: 0.85,
                    });
                }
            }
        }
    }

    fn extract_technologies(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashSet<(EntityType, String)>,
    ) {
        for cap in self.technology_pattern.find_iter(text) {
            let value = cap.as_str().to_string();
            let key = (EntityType::Technology, value.to_lowercase());
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Technology,
                    value,
                    context: extract_context(text, cap.start(), cap.end()),
                    confidence: 0.90,
                });
            }
        }
    }

    fn extract_datetimes(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashSet<(EntityType, String)>,
    ) {
        for cap in self.datetime_pattern.find_iter(text) {
            let value = cap.as_str().to_string();
            let key = (EntityType::DateTime, value.to_lowercase());
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::DateTime,
                    value,
                    context: extract_context(text, cap.start(), cap.end()),
                    confidence: 0.80,
                });
            }
        }
    }

    fn extract_quantities(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashSet<(EntityType, String)>,
    ) {
        for cap in self.quantity_pattern.find_iter(text) {
            let value = cap.as_str().to_string();
            let key = (EntityType::Quantity, value.to_lowercase());
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Quantity,
                    value,
                    context: extract_context(text, cap.start(), cap.end()),
                    confidence: 0.85,
                });
            }
        }
    }

    fn extract_persons(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashSet<(EntityType, String)>,
    ) {
        for cap in self.person_pattern.captures_iter(text) {
            let Some(name_match) = cap.get(1) else {
                continue;
            };
            let Some(full_match) = cap.get(0) else {
                continue;
            };
            let value = name_match.as_str().to_string();
            let key = (EntityType::Person, value.clone());
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Person,
                    value,
                    context: extract_context(text, full_match.start(), full_match.end()),
                    confidence: 0.75,
                });
            }
        }
    }

    fn extract_locations(
        &self,
        text: &str,
        entities: &mut Vec<ExtractedEntity>,
        seen: &mut HashSet<(EntityType, String)>,
    ) {
        for cap in self.location_pattern.captures_iter(text) {
            let Some(loc_match) = cap.get(1) else {
                continue;
            };
            let Some(full_match) = cap.get(0) else {
                continue;
            };
            let value = loc_match.as_str().to_string();
            let key = (EntityType::Location, value.clone());
            if seen.insert(key) {
                entities.push(ExtractedEntity {
                    entity_type: EntityType::Location,
                    value,
                    context: extract_context(text, full_match.start(), full_match.end()),
                    confidence: 0.70,
                });
            }
        }
    }

    /// Extract entities and return them as tags for memory metadata.
    #[must_use]
    pub fn extract_as_tags(&self, text: &str) -> Vec<String> {
        self.extract(text)
            .into_iter()
            .filter(|e| e.confidence >= 0.75)
            .map(|e| format!("{}:{}", e.entity_type.as_str(), e.value))
            .collect()
    }

    /// Extract only high-confidence entities of a specific type.
    #[must_use]
    pub fn extract_by_type(&self, text: &str, entity_type: EntityType) -> Vec<ExtractedEntity> {
        self.extract(text)
            .into_iter()
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }
}

impl Default for EntityExtractor {
    /// Creates a default entity extractor.
    ///
    /// # Panics
    /// Panics if the default regex patterns are invalid (should never happen).
    #[allow(clippy::expect_used)]
    fn default() -> Self {
        Self::new().expect("Default entity extractor patterns should be valid")
    }
}

/// Extract surrounding context for an entity match.
fn extract_context(text: &str, start: usize, end: usize) -> String {
    const CONTEXT_CHARS: usize = 50;

    let ctx_start = start.saturating_sub(CONTEXT_CHARS);
    let ctx_end = (end + CONTEXT_CHARS).min(text.len());

    // Find word boundaries
    let actual_start = text[..start]
        .rfind(|c: char| c.is_whitespace() || c == '.' || c == '!' || c == '?')
        .map_or(ctx_start, |i| (i + 1).max(ctx_start));

    let actual_end = text[end..]
        .find(['.', '!', '?', '\n'])
        .map_or(ctx_end, |i| (end + i + 1).min(ctx_end));

    text[actual_start..actual_end].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_email() {
        let extractor = EntityExtractor::new().unwrap();
        let text = "Contact me at john.doe@example.com for more info.";
        let entities = extractor.extract(text);

        assert!(entities.iter().any(|e| {
            e.entity_type == EntityType::Email && e.value == "john.doe@example.com"
        }));
    }

    #[test]
    fn test_extract_url() {
        let extractor = EntityExtractor::new().unwrap();
        let text = "Check out https://github.com/user/repo for the code.";
        let entities = extractor.extract(text);

        assert!(entities.iter().any(|e| {
            e.entity_type == EntityType::Url && e.value.contains("github.com")
        }));
    }

    #[test]
    fn test_extract_technology() {
        let extractor = EntityExtractor::new().unwrap();
        let text = "I am building this with Rust and Tauri for the desktop app.";
        let entities = extractor.extract(text);

        assert!(entities.iter().any(|e| {
            e.entity_type == EntityType::Technology && e.value == "Rust"
        }));
        assert!(entities.iter().any(|e| {
            e.entity_type == EntityType::Technology && e.value == "Tauri"
        }));
    }

    #[test]
    fn test_extract_person_name() {
        let extractor = EntityExtractor::new().unwrap();
        let text = "My name is Roy Geryan and I created this project.";
        let entities = extractor.extract(text);

        assert!(entities.iter().any(|e| {
            e.entity_type == EntityType::Person && e.value.contains("Roy")
        }));
    }

    #[test]
    fn test_extract_location() {
        let extractor = EntityExtractor::new().unwrap();
        let text = "I live in Paris and work from home.";
        let entities = extractor.extract(text);

        assert!(entities.iter().any(|e| {
            e.entity_type == EntityType::Location && e.value.contains("Paris")
        }));
    }

    #[test]
    fn test_extract_quantity() {
        let extractor = EntityExtractor::new().unwrap();
        let text = "The file is 50 MB and takes 2 hours to process.";
        let entities = extractor.extract(text);

        assert!(entities.iter().any(|e| {
            e.entity_type == EntityType::Quantity && e.value.contains("50")
        }));
    }

    #[test]
    fn test_no_duplicates() {
        let extractor = EntityExtractor::new().unwrap();
        let text = "Use Rust for Rust projects. Rust is great!";
        let entities = extractor.extract(text);

        let rust_count = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Technology && e.value == "Rust")
            .count();

        assert_eq!(rust_count, 1);
    }
}
