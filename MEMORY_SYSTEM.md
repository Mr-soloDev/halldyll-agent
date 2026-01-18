# Memory System (Stateless LLM) - Overview and Integration Guide

This document explains the memory subsystem architecture, module organization,
data flows, and integration with the LLM via Tauri desktop application.

## Scope and Goals

- Stateless LLM (Ministral 3B via Ollama), memory stored externally.
- Three layers: short-term transcripts, session summaries, long-term vector store.
- Deterministic prompt injection and robust error handling.
- Integrated with Tauri desktop app for seamless user experience.

## Dependencies (Cargo.toml)

- `rig-core` (LLM + embeddings)
- `rig-sqlite` (vector store with sqlite-vec)
- `tokio`, `tokio-rusqlite` (async + SQLite)
- `rusqlite` (SQLite bindings)
- `serde`, `serde_json` (serialization)
- `uuid` (strongly-typed IDs)
- `chrono` (timestamps)
- `thiserror` (typed errors)
- `tracing` (observability)
- `lru`, `dashmap` (caches)
- `regex` (heuristic extraction)
- `reqwest` + rustls (HTTP for Rig)

## Module Structure

```
src/memory/
├── mod.rs                  # Module declarations and re-exports
├── core/                   # Core types and configuration
│   ├── mod.rs
│   ├── config.rs           # MemoryConfig + sub-configs
│   ├── errors.rs           # MemoryError + MemoryResult
│   ├── ids.rs              # SessionId, MemoryId, TurnId (strongly-typed UUIDs)
│   ├── kinds.rs            # MemoryKind enum
│   ├── metadata.rs         # MemoryMetadata + MemorySource
│   └── item.rs             # MemoryItem with validation, truncation, hash
├── ingest/                 # Data ingestion pipeline
│   ├── mod.rs
│   ├── transcript_event.rs # TranscriptEvent + TranscriptRole
│   ├── transcript_store.rs # Transcript storage trait + SQLite impl
│   ├── extractor_heuristic.rs # Regex-based memory extraction
│   ├── extractor_llm.rs    # LLM-assisted JSON extraction
│   ├── dedupe.rs           # Content normalization + hashing
│   └── pruning.rs          # TTL and duplicate handling
├── embedding/              # Embedding generation
│   ├── mod.rs
│   └── embedder.rs         # Embedder trait + OllamaEmbedder
├── storage/                # Persistent storage
│   ├── mod.rs
│   ├── vector_store.rs     # Vector store trait + SQLite impl
│   └── summary_store.rs    # Summary store trait + SQLite impl
├── retrieval/              # Memory retrieval
│   ├── mod.rs
│   ├── retrieval.rs        # Query building + vector fetch
│   └── ranking.rs          # Similarity + recency + salience scoring
├── prompt/                 # Prompt construction
│   ├── mod.rs
│   ├── prompt_budget.rs    # Budget enforcement and trimming
│   └── prompt_builder.rs   # Deterministic prompt block generation
├── engine/                 # Orchestration
│   ├── mod.rs
│   └── engine.rs           # MemoryEngine API
└── adapters/               # External integrations
    ├── mod.rs
    └── rig_adapter.rs      # Rig/Ollama adapter
```

## Public API

`MemoryEngine` is the primary entry point:

```rust
// Construction
MemoryEngine::new(config, backends) -> MemoryResult<MemoryEngine>
MemoryEngine::from_config(config) -> MemoryResult<MemoryEngine>

// Core operations
prepare_context(session_id, user_message, recent_turns) -> MemoryResult<PreparedContext>
record_turn(session_id, user_message, assistant_message, tool_events) -> MemoryResult<()>
```

`PreparedContext` contains:

- `summary`: Optional session summary
- `memories`: Ranked long-term memories
- `short_term`: Recent transcript events
- `user_message`: Current user input
- `prompt_block`: Ready-to-inject prompt string

## Data Flow

### Write Path (record_turn)

```
User Message + Assistant Response
        │
        ▼
┌───────────────────┐
│  TranscriptStore  │ ◄── Append events
└───────────────────┘
        │
        ▼
┌───────────────────┐
│ HeuristicExtractor│ ◄── Regex patterns
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   LlmExtractor    │ ◄── Optional JSON extraction
└───────────────────┘
        │
        ▼
┌───────────────────┐
│     Dedupe        │ ◄── LRU cache + content hash
└───────────────────┘
        │
        ▼
┌───────────────────┐
│    Embedder       │ ◄── nomic-embed-text
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   VectorStore     │ ◄── SQLite + sqlite-vec
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   SummaryStore    │ ◄── Updated every N turns
└───────────────────┘
```

### Read Path (prepare_context)

```
User Message
        │
        ▼
┌───────────────────┐
│  TranscriptStore  │ ◄── Load recent turns
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Query Builder    │ ◄── Combine context
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   VectorStore     │ ◄── Semantic search
└───────────────────┘
        │
        ▼
┌───────────────────┐
│     Ranking       │ ◄── similarity + recency + salience
└───────────────────┘
        │
        ▼
┌───────────────────┐
│    TTL Pruning    │ ◄── Remove expired items
└───────────────────┘
        │
        ▼
┌───────────────────┐
│   SummaryStore    │ ◄── Load session summary
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Prompt Budget    │ ◄── Trim to max_chars
└───────────────────┘
        │
        ▼
┌───────────────────┐
│  Prompt Builder   │ ◄── Format [MEMORY_*] blocks
└───────────────────┘
        │
        ▼
    PreparedContext
```

## Prompt Format

```
[MEMORY_SUMMARY]
Session context and prior conversation summary...

[MEMORY_RELEVANT]
* (fact) User prefers dark themes [salience: 0.8] [age: 3600s]
* (preference) User works with Rust [salience: 0.7] [age: 7200s]

[SHORT_TERM]
- User: Hello, can you help me?
- Assistant: Of course! What do you need?

[USER_MESSAGE]
Current user input...
```

## Tauri Desktop Integration

### State Management

```rust
// desktop/src-tauri/src/state.rs
pub struct AppState {
    pub engine: Arc<RwLock<MemoryEngine>>,
    pub session_id: SessionId,
}
```

### Initialization (lib.rs)

```rust
.setup(|app| {
    let app_data_dir = app.path().app_data_dir()?;
    let config = MemoryConfig {
        storage: StorageConfig {
            sqlite_path: app_data_dir.join("memory.sqlite"),
            ..Default::default()
        },
        ..Default::default()
    };
    let engine = MemoryEngine::from_config(config).await?;
    app.manage(AppState::new(engine, SessionId::new()));
    Ok(())
})
```

### Chat Command (commands.rs)

```rust
#[tauri::command]
pub async fn chat_with_memory(
    user_message: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // 1. Prepare context from memory
    let context = state.engine.read().await
        .prepare_context(state.session_id, &user_message, vec![]).await?;

    // 2. Call LLM with enriched prompt
    let response = ollama_generate(&context.prompt_block).await?;

    // 3. Record turn for future retrieval
    state.engine.read().await
        .record_turn(state.session_id, &user_message, &response, None).await?;

    Ok(response)
}
```

### Storage Location

- Windows: `%APPDATA%/halldyll-agent/memory.sqlite`
- Linux: `~/.local/share/halldyll-agent/memory.sqlite`
- macOS: `~/Library/Application Support/halldyll-agent/memory.sqlite`

## Configuration Reference

```rust
MemoryConfig {
    short_term: ShortTermConfig {
        window: 6,              // Recent turns to include
        cache_capacity: 256,    // LRU cache size
    },
    summary: SummaryConfig {
        interval_turns: 8,      // Turns between summary updates
        max_chars: 1200,        // Max summary length
    },
    retrieval: RetrievalConfig {
        top_k: 6,               // Memories to retrieve
        min_similarity: 0.2,    // Minimum similarity threshold
    },
    scoring: ScoringConfig {
        alpha_recency: 0.15,    // Recency weight
        beta_salience: 0.35,    // Salience weight
        recency_half_life_seconds: 604800, // 7 days
    },
    extractor: ExtractorConfig {
        mode: ExtractorMode::Heuristic,
        llm_every_n_turns: 6,
        llm_max_items: 6,
        min_content_chars: 10,
    },
    storage: StorageConfig {
        sqlite_path: PathBuf::from("memory.sqlite"),
        transcript_table: "memory_transcript",
        summary_table: "memory_summary",
        memory_table: "memory_items",
    },
    embedding: EmbeddingConfig {
        model: "nomic-embed-text",
        ndims: 768,
        base_url: None,
    },
    llm: LlmConfig {
        model: "ministral-3:8b-instruct-2512-q8_0",
        temperature: 0.4,
        max_tokens: None,
        base_url: None,
    },
    prompt: PromptConfig {
        max_chars: 3600,        // Total prompt budget
        max_memory_chars: 1200, // Per-item limit
    },
    retention: RetentionConfig {
        ttl_seconds_by_kind: HashMap::new(),
    },
}
```

## SQLite Tables

| Table | Purpose |
|-------|---------|
| `memory_transcript` | Short-term conversation events |
| `memory_summary` | Session summaries |
| `memory_items` | Long-term memory items |
| `memory_items_embeddings` | Vector embeddings (sqlite-vec) |

## Quick Wiring Checklist

1. Configure `sqlite_path` in `MemoryConfig.storage`
2. Initialize `MemoryEngine::from_config(config).await`
3. On each user turn:
   - `prepare_context(...)` → get prompt block
   - Call LLM with `context.prompt_block`
   - `record_turn(...)` → persist memory

