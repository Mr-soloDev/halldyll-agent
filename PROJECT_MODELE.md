# Halldyll project modèle (Rust core + Tauri desktop)

## 1) Overview

This project is a local-first chat client that talks to an Ollama model running on the same machine. It is split into:

- a Rust core crate (llm helpers, startup logic)
- a Tauri backend (Rust commands exposed to the UI)
- a Vite + React desktop UI

The desktop UI invokes Tauri commands. Those commands use the Rust core crate to ensure Ollama is running and to generate replies.

## 2) Repo map (high level)

- Cargo.toml
  - root Rust crate (halldyll_agent)
- src/
  - lib.rs: lint policy and public modules
  - main.rs: CLI entrypoint
  - start_halldyll_agent.rs: startup helper
  - llm/
    - mod.rs
    - ollama_starter_ministral.rs: Ollama client + startup logic
- desktop/
  - package.json, vite.config.ts, tsconfig*.json
  - index.html
  - src/
    - main.tsx
    - App.tsx
    - App.css
  - src-tauri/
    - Cargo.toml, build.rs
    - tauri.conf.json
    - capabilities/default.json
    - src/
      - main.rs
      - lib.rs
      - commands.rs
- cargo.md
  - Rust command checklist
- lunch.md
  - quick dev command list
- desktop/README.md
  - template notes for Tauri + React + TS

## 3) Commands (how to run)

### Desktop dev (Tauri)

```*
cd C:\Users\Mr_solo\Documents\halldyll_agent\desktop
npm install
npm run tauri dev
```

This starts Vite and then launches the Tauri app.

### Rust quality checks (from cargo.md)

```**
cargo check
cargo clippy
cargo clippy -- -D warnings
cargo fmt -- --check
cargo fmt
cargo check --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps
cargo test --all-features
cargo test --doc --all-features
cargo test
```

## 4) Runtime flow (chat request)

1. App startup triggers `start_ollama_ministral` from the UI.
2. Tauri command calls the Rust client in a blocking thread.
3. The Rust client checks `GET /api/version` on `127.0.0.1:11434`.
4. If not ready, it spawns `ollama serve` with `OLLAMA_CONTEXT_LENGTH=8192`.
5. It waits until the server is ready.
6. It sends a warmup `POST /api/generate` with a minimal prompt to preload the model.
7. When the user sends a message, the UI calls `ollama_generate_8192`.
8. Tauri calls `generate_8192`, which posts to `/api/generate` and returns the `response` field.
9. The UI appends the assistant reply to the chat log.

## 5) Rust core crate (halldyll_agent)

### src/lib.rs

- Defines a strict lint policy (warnings as errors, no unsafe, no unused items).
- Exposes two public modules: `llm` and `start_halldyll_agent`.

### src/llm/mod.rs

- Module boundary for Ollama related helpers.

### src/llm/ollama_starter_ministral.rs

Main responsibilities:

- Constants for Ollama host/port, model name, timeouts, context size, batch size, and keep alive.
- `OllamaStarterError` enum with IO and HTTP errors plus parsing errors.
- Two paths are implemented:
  - Low level TCP path used by `ensure_ollama_and_preload_ministral()`.
  - Blocking HTTP path used by `OllamaMinistral`.

Key elements:

- `MINISTRAL_MODEL = "ministral-3:8b-instruct-2512-q8_0"`
- `CONTEXT_LENGTH = 8192`
- `KEEP_ALIVE = "1h"` (duration string required by Ollama)
- `NUM_BATCH = 256`
- `DEFAULT_NUM_PREDICT = 512`
- `WARMUP_NUM_PREDICT = 1`

Low level startup path:

- `ensure_ollama_and_preload_ministral()`
  - Calls `is_ollama_ready()` via TCP `GET /api/version`.
  - If needed, spawns `ollama serve` and waits for readiness.
  - Calls `preload_ministral()` to warm up the model.

Blocking HTTP client path:

- `OllamaMinistral::new_default()`
  - Builds a `reqwest::blocking::Client` with connect and overall timeouts.
- `ensure_server_running_8192(ollama_bin)`
  - Calls the HTTP readiness probe and spawns Ollama if needed.
- `preload_ministral_8192(keep_alive)`
  - Sends a warmup `POST /api/generate` with small prediction budget.
- `generate_8192(model, prompt, keep_alive)`
  - Sends a request with `num_ctx = 8192` and `num_predict = 512`.
  - Returns the `response` string from JSON.

Request shape used for generation:

- `model`: string
- `prompt`: string
- `stream`: false
- `keep_alive`: duration string
- `options`:
  - `num_ctx`: 8192
  - `num_predict`: 1 (warmup) or 512 (chat)
  - `num_batch`: 256
  - `num_thread`: detected from CPU cores
  - `f16_kv`: true

### src/start_halldyll_agent.rs

- `run()` returns `ExitCode`.
- Calls `ensure_ollama_and_preload_ministral()`.
- Returns success on Ok, 1 on error.

### src/main.rs

- CLI entrypoint calling `start_halldyll_agent::run()`.

## 6) Desktop UI (Vite + React)

### desktop/index.html

- Standard Vite entry with `#root` and the `main.tsx` script.

### desktop/src/main.tsx

- Creates the React root and renders `App` inside `React.StrictMode`.
- Imports `App.css` for styling.

### desktop/src/App.tsx

State:

- `status`: "starting" | "ready" | "error"
- `statusText`: human readable status line
- `messages`: list of user/assistant messages
- `input`: current text input
- `busy`: true while generating

Refs:

- `logRef`: used to auto scroll chat log
- `inputRef`: focus management

Key functions:

- `start()`
  - Sets status to starting, invokes `start_ollama_ministral`.
  - On success, sets status to ready.
  - On error, sets status to error and shows error details.
- `onSend()`
  - Pushes the user message.
  - Calls `ollama_generate_8192` with the model constant and prompt.
  - Appends assistant reply or error text.
- `onNewChat()`
  - Clears chat history and input.

UI structure:

- Top bar with brand, model label, status pill, status text, and action buttons.
- Chat log with empty state or message list.
- Typing row shown while generation is in progress.
- Composer dock with a text area and a send button.

Tauri invokes:

- `start_ollama_ministral`
- `ollama_generate_8192`

### desktop/src/App.css

- Defines theme variables and layout rules.
- Styles for top bar, status pill, chat list, message turns, typing row, and composer.
- Minimal scrollbar styling for the chat area.

## 7) Tauri backend

### desktop/src-tauri/src/main.rs

- Entrypoint for the desktop app. Calls `halldyll_desktop_lib::run()`.

### desktop/src-tauri/src/lib.rs

- Builds the Tauri app and registers commands.
- Commands exposed:
  - `start_ollama_ministral`
  - `ollama_generate_8192`

### desktop/src-tauri/src/commands.rs

- Uses `OllamaMinistral` from the core crate.
- Runs blocking work inside `tauri::async_runtime::spawn_blocking`.
- Uses `KEEP_ALIVE = "1h"` for server warmup and chat generation.

### desktop/src-tauri/tauri.conf.json

- Dev flow: runs `npm run dev` and uses `http://localhost:5173`.
- Build flow: `npm run build` and `../dist` output.
- Window config: 1100x720, label `main`.

### desktop/src-tauri/capabilities/default.json

- Default capability with `core:default` permissions for main window.

### desktop/src-tauri/build.rs

- Calls `tauri_build::build()` for generated bindings.

### desktop/src-tauri/Cargo.toml

- Declares the Tauri package and depends on the root crate via `path = "../.."`.

## 8) Build and config files

### desktop/package.json

Scripts:

- `npm run dev`: Vite dev server
- `npm run build`: TypeScript check + Vite build
- `npm run tauri dev`: Tauri dev runner

### desktop/vite.config.ts

- Uses TAURI_DEV_HOST if set for HMR.
- Ensures the dev server runs on port 5173.

### desktop/tsconfig.json / tsconfig.node.json

- Bundler mode settings, strict type checking, and no emit.

## 9) Data and API details

- Ollama API base: `http://127.0.0.1:11434`
- Readiness: `GET /api/version`
- Generation: `POST /api/generate`
- Response JSON field used by the app: `response`

## 10) Troubleshooting

- If you see a 400 error about keep alive, ensure `KEEP_ALIVE` uses a duration with a unit (example: `1h`).
- If the app stays in starting state, check that Ollama is installed and on PATH.
- If the model is missing, run `ollama pull ministral-3:8b-instruct-2512-q8_0`.
- If port 11434 is busy, stop the other Ollama instance and restart.
- If the UI is empty, confirm Vite is running and the Tauri window loaded the dev URL.

## 11) Extending the project

- Change the model string in `desktop/src/App.tsx` and `src/llm/ollama_starter_ministral.rs`.
- Adjust `CONTEXT_LENGTH`, `NUM_BATCH`, or `DEFAULT_NUM_PREDICT` in the Rust module.
- Add more Tauri commands for streaming or system prompts.

## 12) Reference notes

- `cargo.md` is a strict Rust command checklist.
- `lunch.md` is a quick desktop dev command list.
- `desktop/README.md` is the base template note from Vite/Tauri.
