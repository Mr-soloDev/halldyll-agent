//! Binary entrypoint that launches the Halldyll agent bootstrap.

use std::process::ExitCode;

use halldyll_agent::start_halldyll_agent;

/// Start the agent by ensuring Ollama is running and the Ministral model is preloaded.
fn main() -> ExitCode {
    start_halldyll_agent::run()
}
