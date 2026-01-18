//! Maintenance and background cleanup for the memory system.

pub mod background_cleanup;

pub use background_cleanup::{BackgroundCleanup, CleanupConfig, CleanupStats};
