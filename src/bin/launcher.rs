//! Local launcher that connects to RunPod and starts the cloud server.
//! Run with: cargo run --bin halldyll

use std::io::{BufRead, BufReader};
use std::process::{Command, ExitCode, Stdio};

/// RunPod SSH configuration
const RUNPOD_HOST: &str = "213.173.108.10";
const RUNPOD_PORT: &str = "11842";
const RUNPOD_USER: &str = "root";
const RUNPOD_URL: &str = "https://zcgzoso3znn9kl-3000.proxy.runpod.net";

/// Commands to run on RunPod
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
./target/release/halldyll-server
"#;

fn main() -> ExitCode {
    println!();
    println!("  ╔════════════════════════════════════════╗");
    println!("  ║       Halldyll Agent - Cloud Mode      ║");
    println!("  ╚════════════════════════════════════════╝");
    println!();

    // Get SSH key path
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    let ssh_key = format!("{}/.ssh/id_ed25519", home);

    println!("  [1/2] Connecting to RunPod...");
    println!("        Host: {}:{}", RUNPOD_HOST, RUNPOD_PORT);
    println!();

    // Build SSH command
    let mut child = match Command::new("ssh")
        .args([
            "-o", "StrictHostKeyChecking=no",
            "-o", "ServerAliveInterval=30",
            "-o", "ServerAliveCountMax=3",
            "-p", RUNPOD_PORT,
            "-i", &ssh_key,
            &format!("{}@{}", RUNPOD_USER, RUNPOD_HOST),
            STARTUP_COMMANDS,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            eprintln!("  Failed to connect to RunPod: {}", e);
            eprintln!();
            eprintln!("  Make sure:");
            eprintln!("    1. SSH key exists at: {}", ssh_key);
            eprintln!("    2. RunPod pod is running");
            eprintln!("    3. You have internet connection");
            return ExitCode::from(1);
        }
    };

    println!("  [2/2] Starting services on RunPod...");
    println!();
    println!("  ─────────────────────────────────────────");

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("  {}", line);

                // Detect when server is ready
                if line.contains("listening on") {
                    println!("  ─────────────────────────────────────────");
                    println!();
                    println!("  ✓ Agent is running!");
                    println!();
                    println!("  URL: {}", RUNPOD_URL);
                    println!();
                    println!("  Press Ctrl+C to stop");
                    println!();
                }
            }
        }
    }

    // Wait for process to complete
    match child.wait() {
        Ok(status) => {
            if status.success() {
                ExitCode::SUCCESS
            } else {
                eprintln!();
                eprintln!("  Server stopped unexpectedly.");
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("  Error: {}", e);
            ExitCode::from(1)
        }
    }
}
