//! Tauri commands for conversation management.

use std::str::FromStr;

use tauri::State;
use tracing::{debug, info};

use halldyll_agent::llm::ollama_starter_ministral::OllamaMinistral;
use halldyll_agent::memory::core::ids::SessionId;

use super::store::ConversationStore;
use crate::state::AppState;

use super::types::{ConversationMessage, ConversationMeta};

/// Keep-alive duration for Ollama.
const KEEP_ALIVE: &str = "1h";

/// Model name for title generation.
const MODEL_NAME: &str = "ministral-3:8b-instruct-2512-q8_0";

/// List all conversations.
#[tauri::command]
pub async fn list_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationMeta>, String> {
    let store = state.conversation_store.clone();
    store
        .list_all()
        .await
        .map_err(|e| format!("Failed to list conversations: {e}"))
}

/// Create a new conversation.
#[tauri::command]
pub async fn create_conversation(state: State<'_, AppState>) -> Result<ConversationMeta, String> {
    let new_id = SessionId::new();
    let now_ms = chrono::Utc::now().timestamp_millis();

    let store = state.conversation_store.clone();
    let meta = store
        .create(new_id, now_ms)
        .await
        .map_err(|e| format!("Failed to create conversation: {e}"))?;

    // Set as active conversation
    state.set_active_session(new_id).await;

    info!("Created new conversation: {}", new_id);
    Ok(meta)
}

/// Switch to a different conversation.
#[tauri::command]
pub async fn switch_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = SessionId::from_str(&conversation_id)
        .map_err(|e| format!("Invalid conversation ID: {e}"))?;

    // Verify it exists
    let store = state.conversation_store.clone();
    let exists = store
        .exists(session_id)
        .await
        .map_err(|e| format!("Failed to check conversation: {e}"))?;

    if !exists {
        return Err("Conversation not found".to_string());
    }

    state.set_active_session(session_id).await;
    debug!("Switched to conversation: {}", session_id);
    Ok(())
}

/// Delete a conversation.
#[tauri::command]
pub async fn delete_conversation(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = SessionId::from_str(&conversation_id)
        .map_err(|e| format!("Invalid conversation ID: {e}"))?;

    // Archive the conversation (soft delete)
    let store = state.conversation_store.clone();
    store
        .archive(session_id)
        .await
        .map_err(|e| format!("Failed to delete conversation: {e}"))?;

    // If this was the active conversation, clear it
    if state.get_active_session().await.ok() == Some(session_id) {
        state.clear_active_session().await;
    }

    info!("Deleted conversation: {}", session_id);
    Ok(())
}

/// Rename a conversation.
#[tauri::command]
pub async fn rename_conversation(
    conversation_id: String,
    title: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let session_id = SessionId::from_str(&conversation_id)
        .map_err(|e| format!("Invalid conversation ID: {e}"))?;

    let store = state.conversation_store.clone();
    store
        .update_title(session_id, &title)
        .await
        .map_err(|e| format!("Failed to rename conversation: {e}"))?;

    debug!("Renamed conversation {} to: {}", session_id, title);
    Ok(())
}

/// Load messages for a conversation.
#[tauri::command]
pub async fn load_conversation_messages(
    conversation_id: String,
    limit: u32,
    state: State<'_, AppState>,
) -> Result<Vec<ConversationMessage>, String> {
    let session_id = SessionId::from_str(&conversation_id)
        .map_err(|e| format!("Invalid conversation ID: {e}"))?;

    let engine = state.engine.read().await;
    let events = engine
        .load_transcript_events(session_id, limit as usize)
        .await
        .map_err(|e| format!("Failed to load messages: {e}"))?;

    let messages: Vec<ConversationMessage> = events
        .into_iter()
        .filter(|e| e.role.as_str() == "user" || e.role.as_str() == "assistant")
        .map(|e| ConversationMessage {
            role: e.role.to_string(),
            content: e.content,
            timestamp: e.timestamp.timestamp_millis(),
        })
        .collect();

    Ok(messages)
}

/// Get the active conversation ID.
#[tauri::command]
pub async fn get_active_conversation(state: State<'_, AppState>) -> Result<Option<String>, String> {
    match state.get_active_session().await {
        Ok(id) => Ok(Some(id.to_string())),
        Err(_) => Ok(None),
    }
}

/// Generate a title for a conversation using the LLM.
#[tauri::command]
pub async fn generate_conversation_title(
    conversation_id: String,
    first_user_message: String,
    first_assistant_message: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let session_id = SessionId::from_str(&conversation_id)
        .map_err(|e| format!("Invalid conversation ID: {e}"))?;

    // Build prompt for title generation
    let prompt = format!(
        r#"Generate a very short title (3-6 words maximum) for this conversation.
Output ONLY the title, nothing else. No quotes, no punctuation at the end.
Do not use asterisks or any special formatting.

User: {}
Assistant: {}

Title:"#,
        first_user_message.chars().take(200).collect::<String>(),
        first_assistant_message
            .chars()
            .take(200)
            .collect::<String>()
    );

    // Generate title using LLM
    let title = tauri::async_runtime::spawn_blocking(move || {
        let client = OllamaMinistral::new_default().map_err(|e| e.to_string())?;
        client
            .generate_8192(MODEL_NAME, &prompt, KEEP_ALIVE)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Clean up the title
    let title = title
        .trim()
        .trim_matches('"')
        .trim_matches('*')
        .trim()
        .to_string();

    // Limit to reasonable length
    let title: String = title.chars().take(50).collect();

    // Save the title
    let store = state.conversation_store.clone();
    store
        .update_title(session_id, &title)
        .await
        .map_err(|e| format!("Failed to save title: {e}"))?;

    debug!("Generated title for {}: {}", session_id, title);
    Ok(title)
}
