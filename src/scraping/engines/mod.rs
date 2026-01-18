//! Search engine implementations.

pub mod brave;
pub mod duckduckgo;
pub mod google;

use serde::{Deserialize, Serialize};

/// Available search engines.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum SearchEngine {
    /// DuckDuckGo (default, no API key required).
    #[default]
    DuckDuckGo,
    /// Brave Search (API key required for best results).
    Brave,
    /// Google Custom Search (API key required).
    Google,
}

impl SearchEngine {
    /// Get the display name of the search engine.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::DuckDuckGo => "DuckDuckGo",
            Self::Brave => "Brave Search",
            Self::Google => "Google",
        }
    }

    /// Check if this engine requires an API key.
    #[must_use]
    pub const fn requires_api_key(&self) -> bool {
        matches!(self, Self::Brave | Self::Google)
    }
}
