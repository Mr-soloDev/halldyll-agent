//! Adapter modules for external integrations.

pub mod rig_adapter;

pub use rig_adapter::{init_tracing, run_with_memory};
