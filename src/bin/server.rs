//! Halldyll server binary that runs on the cloud (RunPod).
//! Run with: cargo run --bin halldyll-server

use std::process::ExitCode;

use halldyll_agent::start_halldyll_agent;

fn main() -> ExitCode {
    start_halldyll_agent::run()
}
