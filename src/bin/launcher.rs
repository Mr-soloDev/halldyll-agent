//! Cloud launcher - deploys to `RunPod` and monitors services.
//!
//! Run with: `cargo run`
//!
//! This launcher:
//! 1. Deploys Ollama + server to `RunPod` via SSH
//! 2. Services continue running even when PC is off
//! 3. Ctrl+C stops all `RunPod` services

use std::io::{BufRead, BufReader};
use std::process::{Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// `RunPod` SSH configuration.
const RUNPOD_HOST: &str = "213.173.108.10";
const RUNPOD_PORT: &str = "11842";
const RUNPOD_USER: &str = "root";
const RUNPOD_URL: &str = "https://zcgzoso3znn9kl-3000.proxy.runpod.net";

/// Commands to deploy and start services on `RunPod`.
const STARTUP_COMMANDS: &str = r#"
cd ~/halldyll-agent && \
git pull 2>&1 && \
source ~/.cargo/env && \
cargo build --release --bin halldyll-server 2>&1 | tail -5 && \
pkill ollama 2>/dev/null || true; \
pkill halldyll-server 2>/dev/null || true; \
sleep 1 && \
nohup ollama serve > /tmp/ollama.log 2>&1 & \
sleep 3 && \
echo "=== Starting Halldyll Server ===" && \
HALLDYLL_PORT=3000 nohup ./target/release/halldyll-server > /tmp/halldyll.log 2>&1 & \
sleep 2 && \
echo "Server started" && \
echo "listening on 0.0.0.0:3000"
"#;

/// Commands to stop services on `RunPod`.
const CLEANUP_COMMANDS: &str = "pkill ollama 2>/dev/null || true; pkill halldyll-server 2>/dev/null || true";

/// Get SSH key path.
fn get_ssh_key() -> String {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    format!("{home}/.ssh/id_ed25519")
}

/// Build SSH command arguments.
fn ssh_args(ssh_key: &str) -> Vec<String> {
    vec![
        "-o".to_string(),
        "StrictHostKeyChecking=no".to_string(),
        "-o".to_string(),
        "ServerAliveInterval=30".to_string(),
        "-o".to_string(),
        "ServerAliveCountMax=3".to_string(),
        "-p".to_string(),
        RUNPOD_PORT.to_string(),
        "-i".to_string(),
        ssh_key.to_string(),
        format!("{RUNPOD_USER}@{RUNPOD_HOST}"),
    ]
}

/// Deploy to `RunPod` via SSH.
fn deploy_to_runpod(ssh_key: &str, shutdown_flag: &Arc<AtomicBool>) -> Result<(), String> {
    println!("  Connecting to {RUNPOD_HOST}:{RUNPOD_PORT}...");

    let mut args = ssh_args(ssh_key);
    args.push(STARTUP_COMMANDS.to_string());

    let mut child = Command::new("ssh")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("SSH failed: {e}"))?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if shutdown_flag.load(Ordering::Relaxed) {
                break;
            }
            if let Ok(line) = line {
                println!("  {line}");
                if line.contains("listening on") {
                    break;
                }
            }
        }
    }

    drop(child);
    Ok(())
}

/// Stop services on `RunPod`.
fn cleanup_runpod(ssh_key: &str) {
    println!("  Stopping services...");

    let mut args = ssh_args(ssh_key);
    args.push(CLEANUP_COMMANDS.to_string());

    let _ = Command::new("ssh")
        .args(&args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    println!("  Services stopped.");
}

/// Wait for Ctrl+C.
fn wait_for_interrupt(shutdown_flag: &Arc<AtomicBool>) {
    while !shutdown_flag.load(Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn main() -> ExitCode {
    println!();
    println!("  ╔═══════════════════════════════════════════╗");
    println!("  ║     Halldyll Agent - Cloud Launcher       ║");
    println!("  ╚═══════════════════════════════════════════╝");
    println!();

    let ssh_key = get_ssh_key();
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_handler = Arc::clone(&shutdown_flag);

    // Setup Ctrl+C handler
    if let Err(e) = ctrlc::set_handler(move || {
        println!();
        println!("  ─────────────────────────────────────────");
        println!("  Shutting down...");
        shutdown_flag_handler.store(true, Ordering::Relaxed);
    }) {
        eprintln!("  Warning: Ctrl+C handler failed: {e}");
    }

    // Deploy to RunPod
    println!("  Deploying to RunPod...");
    println!();

    if let Err(e) = deploy_to_runpod(&ssh_key, &shutdown_flag) {
        eprintln!("  Deploy failed: {e}");
        return ExitCode::from(1);
    }

    if shutdown_flag.load(Ordering::Relaxed) {
        cleanup_runpod(&ssh_key);
        return ExitCode::SUCCESS;
    }

    // Success
    println!();
    println!("  ═══════════════════════════════════════════");
    println!("  Server running at:");
    println!("  {RUNPOD_URL}");
    println!();
    println!("  Press Ctrl+C to stop");
    println!("  ═══════════════════════════════════════════");
    println!();

    // Wait for shutdown
    wait_for_interrupt(&shutdown_flag);

    // Cleanup
    println!();
    cleanup_runpod(&ssh_key);
    println!();

    ExitCode::SUCCESS
}
