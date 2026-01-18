use tauri::State;
use tracing::debug;

use halldyll_agent::llm::ollama_starter_ministral::OllamaMinistral;

use crate::conversations::store::ConversationStore;
use crate::state::AppState;

const KEEP_ALIVE: &str = "1h";
const MODEL_NAME: &str = "ministral-3:8b-instruct-2512-q8_0";

/// System prompt for Halldyll.
const SYSTEM_PROMPT: &str = r#"Tu es Halldyll, un assistant IA local polyvalent.

Informations sur ton identite:
- Ton nom est Halldyll
- Tu as ete cree par Roy Geryan quand il avait 17 ans, pour s'amuser
- Tu fonctionnes entierement en local sur l'ordinateur de l'utilisateur

Regles de comportement:
- Sois amical, utile et concis
- N'utilise JAMAIS d'asterisques (*) pour l'emphase ou le formatage
- Reponds en francais par defaut, sauf si l'utilisateur parle une autre langue
- Si on te demande ton nom, reponds: "Je m'appelle Halldyll"
- Si on te demande qui t'a cree, reponds: "J'ai ete cree par Roy Geryan, il m'a cree quand il avait 17 ans pour s'amuser"
"#;

#[tauri::command]
pub async fn start_ollama_ministral() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let client = OllamaMinistral::new_default().map_err(|e| e.to_string())?;
        client
            .ensure_server_running_8192("ollama")
            .map_err(|e| e.to_string())?;
        client
            .preload_ministral_8192(KEEP_ALIVE)
            .map_err(|e| e.to_string())?;

        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn ollama_generate_8192(model: String, prompt: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let client = OllamaMinistral::new_default().map_err(|e| e.to_string())?;
        client
            .generate_8192(&model, &prompt, KEEP_ALIVE)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Chat with memory integration.
///
/// Prepares context from memory, calls the LLM, and records the turn.
#[tauri::command]
pub async fn chat_with_memory(
    user_message: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get or create active session
    let session_id = match state.get_active_session().await {
        Ok(id) => id,
        Err(_) => {
            // No active session, create one
            let new_id = halldyll_agent::memory::core::ids::SessionId::new();
            let now_ms = chrono::Utc::now().timestamp_millis();

            state
                .conversation_store
                .create(new_id, now_ms)
                .await
                .map_err(|e| format!("Failed to create conversation: {e}"))?;

            state.set_active_session(new_id).await;
            new_id
        }
    };

    let context = {
        let engine = state.engine.read().await;
        engine
            .prepare_context(session_id, &user_message, vec![])
            .await
            .map_err(|e| format!("Memory prepare failed: {e}"))?
    };

    debug!(
        "Prepared context with {} memories, {} short-term turns",
        context.memories.len(),
        context.short_term.len()
    );

    // Build prompt with system instructions
    let full_prompt = format!(
        "{}\n\n{}\n\nUser: {}\nAssistant:",
        SYSTEM_PROMPT, context.prompt_block, user_message
    );

    let response = tauri::async_runtime::spawn_blocking(move || {
        let client = OllamaMinistral::new_default().map_err(|e| e.to_string())?;
        client
            .generate_8192(MODEL_NAME, &full_prompt, KEEP_ALIVE)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Clean up asterisks from response
    let response = response.trim().replace("**", "").replace("*", "");

    {
        let engine = state.engine.read().await;
        engine
            .record_turn(session_id, &user_message, &response, None)
            .await
            .map_err(|e| format!("Memory record failed: {e}"))?;
    }

    // Update conversation timestamp
    let now_ms = chrono::Utc::now().timestamp_millis();
    let _ = state
        .conversation_store
        .touch_updated(session_id, now_ms)
        .await;

    debug!("Recorded turn for session {}", session_id);

    Ok(response)
}
