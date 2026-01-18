//! Ollama starter for `ministral-3:8b-instruct-2512-q8_0`, designed to satisfy a strict lint policy.
//!
//! Goals:
//! - No `unsafe`.
//! - Blocking HTTP for deterministic startup and generation.
//! - Keep the startup path allocation-light; generation returns owned text.
//!
//! Behaviour:
//! - Check whether Ollama is reachable via `GET /api/version`.
//! - If not reachable, spawn `ollama serve` with `OLLAMA_CONTEXT_LENGTH=8192`.
//! - Preload (warm-up) the model via `POST /api/generate` with runtime options.

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// The model name as installed in Ollama.
const MINISTRAL_MODEL: &str = "mistral:7b-instruct-q8_0";

/// Default Ollama API host.
const DEFAULT_OLLAMA_HOST: &str = "127.0.0.1";
/// Default Ollama API port.
const DEFAULT_OLLAMA_PORT: u16 = 11_434;

/// Environment variable for custom Ollama URL (e.g., "http://213.173.108.10:19212")
const OLLAMA_URL_ENV: &str = "HALLDYLL_OLLAMA_URL";

/// Get Ollama base URL from environment or use default localhost.
fn get_ollama_base_url() -> String {
    std::env::var(OLLAMA_URL_ENV).unwrap_or_else(|_| {
        format!("http://{}:{}", DEFAULT_OLLAMA_HOST, DEFAULT_OLLAMA_PORT)
    })
}

/// Get Ollama host and port from environment or use defaults.
fn get_ollama_host_port() -> (String, u16) {
    if let Ok(url) = std::env::var(OLLAMA_URL_ENV) {
        // Parse URL like "http://213.173.108.10:19212"
        if let Some(stripped) = url.strip_prefix("http://") {
            let parts: Vec<&str> = stripped.split(':').collect();
            if parts.len() == 2 {
                if let Ok(port) = parts[1].parse::<u16>() {
                    return (parts[0].to_string(), port);
                }
            }
        }
    }
    (DEFAULT_OLLAMA_HOST.to_string(), DEFAULT_OLLAMA_PORT)
}

/// Target context length (tokens).
const CONTEXT_LENGTH: u32 = 8_192;

/// Keep the model loaded in memory for a reasonable duration.
const KEEP_ALIVE: &str = "1h";

/// Warm-up prompt: minimal non-empty prompt.
const WARMUP_PROMPT: &str = " ";

/// Startup wait settings.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const STARTUP_RETRY: Duration = Duration::from_millis(250);

/// HTTP I/O timeouts.
const IO_TIMEOUT: Duration = Duration::from_secs(5);
/// HTTP client timeout for long-running generations.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(120);

/// Conservative default batch for 8K context to reduce OOM risk.
const NUM_BATCH: u32 = 256;
/// Default token budget for generation.
const DEFAULT_NUM_PREDICT: u32 = 512;
/// Warm-up token budget.
const WARMUP_NUM_PREDICT: u32 = 1;

/// Default thread count if `available_parallelism()` is unavailable.
const DEFAULT_NUM_THREAD: u32 = 8;

/// Errors produced by the Ollama starter.
#[derive(Debug)]
pub enum OllamaStarterError {
    /// Failed to spawn or talk to Ollama due to an OS I/O error.
    Io(std::io::Error),
    /// Ollama did not become ready in time.
    StartupTimeout,
    /// HTTP response was not a success.
    HttpStatusNotOk(u16),
    /// The response could not be parsed sufficiently to extract a status code.
    HttpMalformedResponse,
    /// Internal fixed buffer was too small.
    BufferTooSmall,
    /// HTTP client error when using the blocking client.
    HttpClient(reqwest::Error),
}

impl From<std::io::Error> for OllamaStarterError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<reqwest::Error> for OllamaStarterError {
    fn from(value: reqwest::Error) -> Self {
        Self::HttpClient(value)
    }
}

impl fmt::Display for OllamaStarterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::StartupTimeout => write!(f, "ollama startup timed out"),
            Self::HttpStatusNotOk(status) => write!(f, "ollama http status not ok: {status}"),
            Self::HttpMalformedResponse => write!(f, "ollama http response malformed"),
            Self::BufferTooSmall => write!(f, "internal buffer too small"),
            Self::HttpClient(err) => write!(f, "http client error: {err}"),
        }
    }
}

impl std::error::Error for OllamaStarterError {}

#[derive(Serialize)]
struct GenerateOptions {
    num_ctx: u32,
    num_predict: u32,
    num_batch: u32,
    num_thread: u32,
    f16_kv: bool,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    keep_alive: &'a str,
    options: GenerateOptions,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: Option<String>,
}

/// Blocking Ollama client for ensuring server readiness and generating text.
pub struct OllamaMinistral {
    client: Client,
}

impl OllamaMinistral {
    /// Create a default client configured for the local Ollama server.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built.
    pub fn new_default() -> Result<Self, OllamaStarterError> {
        let client = Client::builder()
            .connect_timeout(IO_TIMEOUT)
            .timeout(CLIENT_TIMEOUT)
            .build()?;
        Ok(Self { client })
    }

    /// Ensure Ollama is running with a context length of 8192.
    ///
    /// # Errors
    /// Returns an error if Ollama cannot be reached or started.
    pub fn ensure_server_running_8192(&self, ollama_bin: &str) -> Result<(), OllamaStarterError> {
        if self.is_ready()? {
            return Ok(());
        }

        spawn_ollama_serve(ollama_bin)?;
        self.wait_until_ready()
    }

    /// Preload `MINISTRAL_MODEL` and keep it resident in memory.
    ///
    /// # Errors
    /// Returns an error if the warm-up request fails.
    pub fn preload_ministral_8192(&self, keep_alive: &str) -> Result<(), OllamaStarterError> {
        self.post_generate(
            MINISTRAL_MODEL,
            WARMUP_PROMPT,
            keep_alive,
            WARMUP_NUM_PREDICT,
        )?;
        Ok(())
    }

    /// Generate a response with `num_ctx=8192` and return the raw model output.
    ///
    /// # Errors
    /// Returns an error if the request fails or the response is malformed.
    pub fn generate_8192(
        &self,
        model: &str,
        prompt: &str,
        keep_alive: &str,
    ) -> Result<String, OllamaStarterError> {
        let response = self.post_generate(model, prompt, keep_alive, DEFAULT_NUM_PREDICT)?;
        response
            .response
            .ok_or(OllamaStarterError::HttpMalformedResponse)
    }

    fn is_ready(&self) -> Result<bool, OllamaStarterError> {
        let url = format!("{}/api/version", get_ollama_base_url());
        let response = self.client.get(&url).send()?;
        Ok(response.status().is_success())
    }

    fn wait_until_ready(&self) -> Result<(), OllamaStarterError> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;

        while Instant::now() < deadline {
            if self.is_ready()? {
                return Ok(());
            }
            sleep(STARTUP_RETRY);
        }

        Err(OllamaStarterError::StartupTimeout)
    }

    fn post_generate(
        &self,
        model: &str,
        prompt: &str,
        keep_alive: &str,
        num_predict: u32,
    ) -> Result<GenerateResponse, OllamaStarterError> {
        let options = GenerateOptions {
            num_ctx: CONTEXT_LENGTH,
            num_predict,
            num_batch: NUM_BATCH,
            num_thread: detect_num_thread(),
            f16_kv: true,
        };
        let request = GenerateRequest {
            model,
            prompt,
            stream: false,
            keep_alive,
            options,
        };

        let url = format!("{}/api/generate", get_ollama_base_url());
        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()?;
        let status = response.status();
        if !status.is_success() {
            return Err(OllamaStarterError::HttpStatusNotOk(status.as_u16()));
        }

        response
            .json::<GenerateResponse>()
            .map_err(OllamaStarterError::from)
    }
}

/// Start Ollama if needed and preload `MINISTRAL_MODEL` with a runtime `num_ctx=8192`.
///
/// This function is intentionally minimal: it only ensures the server is up and
/// warms up the model. Your agent should still pass `options` per request (or create a Modelfile).
///
/// # Errors
/// Returns an error if the server cannot be reached or started, or if the warm-up request fails.
pub fn ensure_ollama_and_preload_ministral() -> Result<(), OllamaStarterError> {
    if is_ollama_ready()? {
        preload_ministral()?;
        return Ok(());
    }

    spawn_ollama_serve("ollama")?;
    wait_until_ready()?;
    preload_ministral()?;
    Ok(())
}

fn is_ollama_ready() -> Result<bool, OllamaStarterError> {
    let mut stream = connect_ollama()?;
    send_get_version(&mut stream)?;
    let status = read_http_status(&mut stream)?;
    Ok(status == 200)
}

fn spawn_ollama_serve(ollama_bin: &str) -> Result<(), OllamaStarterError> {
    // `ollama serve` will keep running after this process drops the handle.
    // We silence stdout/stderr to avoid printing in production.
    let _child = Command::new(ollama_bin)
        .arg("serve")
        .env("OLLAMA_CONTEXT_LENGTH", "8192")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}

fn wait_until_ready() -> Result<(), OllamaStarterError> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;

    while Instant::now() < deadline {
        if is_ollama_ready()? {
            return Ok(());
        }
        sleep(STARTUP_RETRY);
    }

    Err(OllamaStarterError::StartupTimeout)
}

fn preload_ministral() -> Result<(), OllamaStarterError> {
    let num_thread = detect_num_thread();

    let mut body_buf = [0_u8; 768];
    let body_len = build_generate_warmup_body(&mut body_buf, num_thread)?;

    let mut header_buf = [0_u8; 512];
    let header_len = build_post_header(&mut header_buf, body_len)?;

    let mut stream = connect_ollama()?;
    stream.write_all(&header_buf[..header_len])?;
    stream.write_all(&body_buf[..body_len])?;
    stream.flush()?;

    let status = read_http_status(&mut stream)?;
    if status != 200 {
        return Err(OllamaStarterError::HttpStatusNotOk(status));
    }

    Ok(())
}

fn detect_num_thread() -> u32 {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .map_or(DEFAULT_NUM_THREAD, |v| u32::try_from(v).unwrap_or(u32::MAX))
}

fn connect_ollama() -> Result<TcpStream, OllamaStarterError> {
    let (host, port) = get_ollama_host_port();
    let addr = (host.as_str(), port)
        .to_socket_addrs()?
        .next()
        .ok_or(OllamaStarterError::HttpMalformedResponse)?;

    let stream = TcpStream::connect(addr)?;
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    Ok(stream)
}

fn send_get_version(stream: &mut TcpStream) -> Result<(), OllamaStarterError> {
    // GET /api/version
    // Host header is required for HTTP/1.1.
    const REQ: &[u8] = b"GET /api/version HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    stream.write_all(REQ)?;
    stream.flush()?;
    Ok(())
}

fn build_post_header(out: &mut [u8], content_len: usize) -> Result<usize, OllamaStarterError> {
    // POST /api/generate
    // Content-Length must be exact.
    let mut i = 0_usize;

    i = push_bytes(out, i, b"POST /api/generate HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: ")?;
    i = push_usize_as_dec(out, i, content_len)?;
    i = push_bytes(out, i, b"\r\nConnection: close\r\n\r\n")?;

    Ok(i)
}

fn build_generate_warmup_body(
    out: &mut [u8],
    num_thread: u32,
) -> Result<usize, OllamaStarterError> {
    // JSON body:
    // {
    //   "model":"...","prompt":" ","stream":false,"keep_alive":"-1",
    //   "options":{"num_ctx":8192,"num_predict":1,"num_batch":256,"num_thread":<n>,"f16_kv":true}
    // }
    //
    // Keep it small and deterministic: the warm-up response is minimal.
    let mut i = 0_usize;

    i = push_bytes(out, i, b"{\"model\":\"")?;
    i = push_str(out, i, MINISTRAL_MODEL)?;
    i = push_bytes(out, i, b"\",\"prompt\":\"")?;
    i = push_str(out, i, WARMUP_PROMPT)?;
    i = push_bytes(out, i, b"\",\"stream\":false,\"keep_alive\":\"")?;
    i = push_str(out, i, KEEP_ALIVE)?;
    i = push_bytes(out, i, b"\",\"options\":{\"num_ctx\":")?;
    i = push_u32_as_dec(out, i, CONTEXT_LENGTH)?;
    i = push_bytes(out, i, b",\"num_predict\":")?;
    i = push_u32_as_dec(out, i, WARMUP_NUM_PREDICT)?;
    i = push_bytes(out, i, b",\"num_batch\":")?;
    i = push_u32_as_dec(out, i, NUM_BATCH)?;
    i = push_bytes(out, i, b",\"num_thread\":")?;
    i = push_u32_as_dec(out, i, num_thread)?;
    i = push_bytes(out, i, b",\"f16_kv\":true}}")?;
    Ok(i)
}

fn read_http_status(stream: &mut TcpStream) -> Result<u16, OllamaStarterError> {
    let mut buf = [0_u8; 512];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Err(OllamaStarterError::HttpMalformedResponse);
    }

    parse_http_status(&buf[..n]).ok_or(OllamaStarterError::HttpMalformedResponse)
}

fn parse_http_status(buf: &[u8]) -> Option<u16> {
    // Expect: "HTTP/1.1 200 ..."
    // Find first space, then parse the next 3 digits.
    let mut idx = 0_usize;
    while idx < buf.len() {
        if buf[idx] == b' ' {
            break;
        }
        idx += 1;
    }
    if idx >= buf.len() {
        return None;
    }

    // Skip spaces
    while idx < buf.len() && buf[idx] == b' ' {
        idx += 1;
    }

    if idx + 2 >= buf.len() {
        return None;
    }

    let d0 = buf[idx];
    let d1 = buf[idx + 1];
    let d2 = buf[idx + 2];

    if !is_ascii_digit(d0) || !is_ascii_digit(d1) || !is_ascii_digit(d2) {
        return None;
    }

    let code = (u16::from(d0 - b'0') * 100) + (u16::from(d1 - b'0') * 10) + u16::from(d2 - b'0');
    Some(code)
}

const fn is_ascii_digit(b: u8) -> bool {
    b'0' <= b && b <= b'9'
}

fn push_bytes(out: &mut [u8], mut i: usize, src: &[u8]) -> Result<usize, OllamaStarterError> {
    if out.len().saturating_sub(i) < src.len() {
        return Err(OllamaStarterError::BufferTooSmall);
    }

    for &b in src {
        out[i] = b;
        i += 1;
    }

    Ok(i)
}

fn push_str(out: &mut [u8], i: usize, s: &str) -> Result<usize, OllamaStarterError> {
    push_bytes(out, i, s.as_bytes())
}

fn push_u32_as_dec(out: &mut [u8], i: usize, v: u32) -> Result<usize, OllamaStarterError> {
    push_usize_as_dec(out, i, v as usize)
}

fn push_usize_as_dec(
    out: &mut [u8],
    mut i: usize,
    mut v: usize,
) -> Result<usize, OllamaStarterError> {
    // Write decimal digits without allocation:
    // store digits reversed in a small stack buffer then copy back.
    let mut tmp = [0_u8; 20];
    let mut t = 0_usize;

    if v == 0 {
        if i >= out.len() {
            return Err(OllamaStarterError::BufferTooSmall);
        }
        out[i] = b'0';
        return Ok(i + 1);
    }

    while v != 0 {
        let digit = u8::try_from(v % 10).unwrap_or(0);
        tmp[t] = b'0' + digit;
        t += 1;
        v /= 10;
    }

    if out.len().saturating_sub(i) < t {
        return Err(OllamaStarterError::BufferTooSmall);
    }

    while t != 0 {
        t -= 1;
        out[i] = tmp[t];
        i += 1;
    }

    Ok(i)
}
