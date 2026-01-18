//! Memory engine orchestration.

use std::num::NonZeroUsize;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use dashmap::DashMap;
use lru::LruCache;
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::memory::core::config::{ExtractorMode, MemoryConfig};
use crate::memory::core::errors::{MemoryError, MemoryResult};
use crate::memory::core::ids::{SessionId, TurnId};
use crate::memory::core::item::MemoryItem;
use crate::memory::embedding::embedder::{Embedder, OllamaEmbedder};
use crate::memory::ingest::extractor_heuristic::HeuristicExtractor;
use crate::memory::ingest::extractor_llm::LlmExtractor;
use crate::memory::ingest::pruning;
use crate::memory::ingest::transcript_event::{TranscriptEvent, TranscriptRole};
use crate::memory::ingest::transcript_store::{SqliteTranscriptStore, TranscriptStore};
use crate::memory::prompt::prompt_budget::{PromptParts, enforce_budget};
use crate::memory::prompt::prompt_builder::build_prompt_block;
use crate::memory::retrieval::ranking::rank_results;
use crate::memory::retrieval::{build_query_text, fetch_top_k_raw};
use crate::memory::storage::summary_store::{SqliteSummaryStore, SummaryRecord, SummaryStore};
use crate::memory::storage::vector_store::{SqliteVectorMemoryStore, VectorMemoryStore};

/// Prepared memory context for LLM prompting.
#[derive(Clone, Debug)]
pub struct PreparedContext {
    /// Optional session summary.
    pub summary: Option<String>,
    /// Retrieved long-term memories.
    pub memories: Vec<MemoryItem>,
    /// Short-term turns used for context.
    pub short_term: Vec<TranscriptEvent>,
    /// Current user message.
    pub user_message: String,
    /// Prompt block ready to inject into the model.
    pub prompt_block: String,
}

/// Backend dependencies for the memory engine.
pub struct MemoryBackends {
    /// Transcript store implementation.
    pub transcript_store: Arc<dyn TranscriptStore>,
    /// Summary store implementation.
    pub summary_store: Arc<dyn SummaryStore>,
    /// Vector store implementation.
    pub vector_store: Arc<dyn VectorMemoryStore>,
    /// Embedding model wrapper.
    pub embedder: Arc<dyn Embedder>,
}

impl MemoryBackends {
    /// Build default `SQLite` backends from config.
    ///
    /// # Errors
    /// Returns an error if any backend cannot be initialized.
    pub async fn sqlite(config: &MemoryConfig) -> MemoryResult<Self> {
        let transcript_store = Arc::new(SqliteTranscriptStore::new(&config.storage).await?);
        let summary_store = Arc::new(SqliteSummaryStore::new(&config.storage).await?);
        let vector_store = Arc::new(SqliteVectorMemoryStore::new(config).await?);
        let embedder = Arc::new(OllamaEmbedder::new(&config.embedding)?);

        Ok(Self {
            transcript_store,
            summary_store,
            vector_store,
            embedder,
        })
    }
}

#[derive(Clone, Debug, Default)]
struct SessionState {
    turn_count: u64,
    last_summary_turn: u64,
    last_llm_turn: u64,
}

/// Memory engine with stateless model integration.
pub struct MemoryEngine {
    config: MemoryConfig,
    transcript_store: Arc<dyn TranscriptStore>,
    summary_store: Arc<dyn SummaryStore>,
    vector_store: Arc<dyn VectorMemoryStore>,
    embedder: Arc<dyn Embedder>,
    heuristic_extractor: HeuristicExtractor,
    llm_extractor: Option<LlmExtractor>,
    session_state: DashMap<SessionId, SessionState>,
    dedupe_cache: Mutex<LruCache<String, ()>>,
}

impl MemoryEngine {
    /// Create a new memory engine.
    ///
    /// # Errors
    /// Returns an error if configuration or extractors are invalid.
    pub fn new(config: MemoryConfig, backends: MemoryBackends) -> MemoryResult<Self> {
        config.validate()?;
        let heuristic_extractor = HeuristicExtractor::new(&config.extractor, &config.prompt)
            .map_err(|err| MemoryError::InvalidConfig(err.to_string()))?;
        let llm_extractor = match config.extractor.mode {
            ExtractorMode::Heuristic => None,
            ExtractorMode::Llm => Some(LlmExtractor::new(
                &config.llm,
                &config.extractor,
                &config.prompt,
            )?),
        };

        let capacity = NonZeroUsize::new(config.short_term.cache_capacity).ok_or_else(|| {
            MemoryError::InvalidConfig("short_term.cache_capacity must be > 0".to_string())
        })?;

        Ok(Self {
            config,
            transcript_store: backends.transcript_store,
            summary_store: backends.summary_store,
            vector_store: backends.vector_store,
            embedder: backends.embedder,
            heuristic_extractor,
            llm_extractor,
            session_state: DashMap::new(),
            dedupe_cache: Mutex::new(LruCache::new(capacity)),
        })
    }

    /// Create a new engine using `SQLite` backends.
    ///
    /// # Errors
    /// Returns an error if backends cannot be initialized.
    pub async fn from_config(config: MemoryConfig) -> MemoryResult<Self> {
        let backends = MemoryBackends::sqlite(&config).await?;
        Self::new(config, backends)
    }

    /// Prepare a memory context for a new user message.
    ///
    /// # Errors
    /// Returns an error if retrieval or storage access fails.
    pub async fn prepare_context(
        &self,
        session_id: SessionId,
        user_message: &str,
        recent_turns: Vec<TranscriptEvent>,
    ) -> MemoryResult<PreparedContext> {
        let short_term = if recent_turns.is_empty() {
            self.transcript_store
                .load_recent(session_id, self.config.short_term.window.saturating_mul(2))
                .await?
        } else {
            recent_turns
        };

        let query = build_query_text(user_message, &short_term);
        let raw = fetch_top_k_raw(
            self.vector_store.as_ref(),
            session_id,
            &query,
            self.config.retrieval.top_k,
            self.config.retrieval.min_similarity,
        )
        .await?;

        let ranked = rank_results(raw, &self.config.scoring, Utc::now());
        let memories: Vec<MemoryItem> = ranked.into_iter().map(|r| r.item).collect();
        let (memories, expired) = pruning::apply_ttl(memories, Utc::now());
        if !expired.is_empty() {
            self.vector_store.delete_by_ids(expired).await?;
        }

        let summary = self
            .summary_store
            .get_summary(session_id)
            .await?
            .map(|record| record.summary);

        let parts = PromptParts {
            summary,
            memories,
            turns: short_term,
            user_message: user_message.to_string(),
        };
        let parts = enforce_budget(parts, self.config.prompt.max_chars);
        let prompt_block = build_prompt_block(&parts);

        info!("Prepared memory context for session {}", session_id);

        Ok(PreparedContext {
            summary: parts.summary,
            memories: parts.memories,
            short_term: parts.turns,
            user_message: parts.user_message,
            prompt_block,
        })
    }

    /// Record a completed turn and update memory stores.
    ///
    /// # Errors
    /// Returns an error if persistence or embeddings fail.
    pub async fn record_turn(
        &self,
        session_id: SessionId,
        user_message: &str,
        assistant_message: &str,
        tool_events: Option<Vec<TranscriptEvent>>,
    ) -> MemoryResult<()> {
        let events =
            Self::build_turn_events(session_id, user_message, assistant_message, tool_events);
        self.transcript_store.append_events(events.clone()).await?;
        let state = self.advance_turn(session_id).await?;

        let extracted = self
            .extract_turn_memories(&events, session_id, user_message, assistant_message, &state)
            .await;
        let fresh = self.filter_new_items(session_id, extracted).await?;
        self.persist_fresh_items(session_id, fresh).await?;

        if self.should_update_summary(&state) {
            self.update_summary(session_id, state.turn_count).await?;
            self.mark_summary_updated(session_id, state.turn_count);
        }

        debug!("Recorded memory turn for session {}", session_id);

        Ok(())
    }

    fn build_turn_events(
        session_id: SessionId,
        user_message: &str,
        assistant_message: &str,
        tool_events: Option<Vec<TranscriptEvent>>,
    ) -> Vec<TranscriptEvent> {
        let turn_id = TurnId::new();
        let mut events = vec![
            TranscriptEvent::user(turn_id, session_id, user_message),
            TranscriptEvent::assistant(turn_id, session_id, assistant_message),
        ];

        if let Some(mut tool_events) = tool_events {
            for event in &mut tool_events {
                event.turn_id = turn_id;
                event.session_id = session_id;
            }
            events.extend(tool_events);
        }

        events
    }

    async fn extract_turn_memories(
        &self,
        events: &[TranscriptEvent],
        session_id: SessionId,
        user_message: &str,
        assistant_message: &str,
        state: &SessionState,
    ) -> Vec<MemoryItem> {
        let mut extracted = Vec::new();
        for event in events {
            extracted.extend(self.heuristic_extractor.extract_from_event(event));
        }

        if self.should_run_llm(state)
            && let Some(extractor) = &self.llm_extractor
        {
            match extractor
                .extract(session_id, user_message, assistant_message)
                .await
            {
                Ok(llm_items) => {
                    extracted.extend(llm_items);
                    self.mark_llm_run(session_id, state.turn_count);
                }
                Err(err) => {
                    debug!("LLM extractor failed: {err}");
                }
            }
        }

        extracted
    }

    async fn persist_fresh_items(
        &self,
        session_id: SessionId,
        items: Vec<MemoryItem>,
    ) -> MemoryResult<()> {
        for item in items {
            let embedding = self.embedder.embed_text(&item.content).await?;
            self.vector_store.upsert(item.clone(), embedding).await?;
            self.remember_hash(session_id, &item.content_hash).await;
        }

        Ok(())
    }

    async fn advance_turn(&self, session_id: SessionId) -> MemoryResult<SessionState> {
        if let Some(mut entry) = self.session_state.get_mut(&session_id) {
            entry.turn_count += 1;
            return Ok(entry.clone());
        }

        let count = self.transcript_store.count_turns(session_id).await?;
        let state = SessionState {
            turn_count: count,
            last_summary_turn: 0,
            last_llm_turn: 0,
        };
        self.session_state.insert(session_id, state.clone());
        Ok(state)
    }

    const fn should_run_llm(&self, state: &SessionState) -> bool {
        matches!(self.config.extractor.mode, ExtractorMode::Llm)
            && state.turn_count.saturating_sub(state.last_llm_turn)
                >= self.config.extractor.llm_every_n_turns
    }

    fn mark_llm_run(&self, session_id: SessionId, turn_count: u64) {
        if let Some(mut entry) = self.session_state.get_mut(&session_id) {
            entry.last_llm_turn = turn_count;
        }
    }

    const fn should_update_summary(&self, state: &SessionState) -> bool {
        state.turn_count.saturating_sub(state.last_summary_turn)
            >= self.config.summary.interval_turns
    }

    fn mark_summary_updated(&self, session_id: SessionId, turn_count: u64) {
        if let Some(mut entry) = self.session_state.get_mut(&session_id) {
            entry.last_summary_turn = turn_count;
        }
    }

    async fn filter_new_items(
        &self,
        session_id: SessionId,
        items: Vec<MemoryItem>,
    ) -> MemoryResult<Vec<MemoryItem>> {
        let mut fresh = Vec::new();
        for mut item in items {
            if item.metadata.ttl_seconds.is_none()
                && let Some(ttl) = self.config.retention.ttl_seconds_by_kind.get(&item.kind)
            {
                item.metadata.ttl_seconds = Some(*ttl);
            }

            if item.validate(self.config.prompt.max_memory_chars).is_err() {
                continue;
            }

            let content_hash = &item.content_hash;
            let key = format!("{session_id}:{content_hash}");
            if self.is_hash_seen(&key).await {
                continue;
            }

            if self
                .vector_store
                .exists_hash(session_id, &item.content_hash)
                .await?
            {
                self.remember_hash(session_id, &item.content_hash).await;
                continue;
            }

            fresh.push(item);
        }

        Ok(fresh)
    }

    async fn is_hash_seen(&self, key: &str) -> bool {
        let mut cache = self.dedupe_cache.lock().await;
        cache.get(key).is_some()
    }

    async fn remember_hash(&self, session_id: SessionId, content_hash: &str) {
        let key = format!("{session_id}:{content_hash}");
        let mut cache = self.dedupe_cache.lock().await;
        cache.put(key, ());
    }

    /// Load recent transcript events for a session.
    ///
    /// # Errors
    /// Returns an error if storage access fails.
    pub async fn load_transcript_events(
        &self,
        session_id: SessionId,
        limit: usize,
    ) -> MemoryResult<Vec<TranscriptEvent>> {
        self.transcript_store.load_recent(session_id, limit).await
    }

    async fn update_summary(&self, session_id: SessionId, turn_count: u64) -> MemoryResult<()> {
        let existing = self.summary_store.get_summary(session_id).await?;
        let mut summary = existing
            .as_ref()
            .map(|record| record.summary.clone())
            .unwrap_or_default();
        let from_ts = if let Some(record) = &existing {
            record.updated_at
        } else {
            Utc.timestamp_millis_opt(0).single().ok_or_else(|| {
                MemoryError::InvalidMemoryItem("invalid epoch timestamp".to_string())
            })?
        };
        let events = self
            .transcript_store
            .load_range(session_id, from_ts, Utc::now())
            .await?;

        for event in &events {
            summary.push_str(match event.role {
                TranscriptRole::User => "\nUser: ",
                TranscriptRole::Assistant => "\nAssistant: ",
                TranscriptRole::Tool => "\nTool: ",
                TranscriptRole::System => "\nSystem: ",
            });
            summary.push_str(&event.content);
        }

        if summary.chars().count() > self.config.summary.max_chars {
            summary = summary
                .chars()
                .rev()
                .take(self.config.summary.max_chars)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>();
        }

        let record = SummaryRecord {
            session_id,
            summary,
            updated_at: Utc::now(),
            turn_count,
        };
        self.summary_store.set_summary(record).await?;
        info!("Updated summary for session {}", session_id);
        Ok(())
    }
}
