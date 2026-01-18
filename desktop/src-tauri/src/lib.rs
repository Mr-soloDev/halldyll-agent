mod commands;
mod conversations;
mod state;

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;
use tokio_rusqlite::Connection;
use tracing::info;

use halldyll_agent::memory::core::config::{MemoryConfig, StorageConfig};
use halldyll_agent::memory::engine::MemoryEngine;
use halldyll_agent::memory::init_sqlite_vec_extension;

use crate::conversations::SqliteConversationStore;
use crate::state::AppState;

/// Build storage configuration with app data directory path.
fn build_storage_config(app_data_dir: PathBuf) -> StorageConfig {
    StorageConfig {
        sqlite_path: app_data_dir.join("memory.sqlite"),
        ..StorageConfig::default()
    }
}

/// Initialize memory engine with default configuration.
async fn init_memory_engine(app_data_dir: PathBuf) -> Result<MemoryEngine, String> {
    let storage = build_storage_config(app_data_dir);
    let config = MemoryConfig {
        storage,
        ..MemoryConfig::default()
    };

    MemoryEngine::from_config(config)
        .await
        .map_err(|e| format!("Failed to initialize memory engine: {e}"))
}

/// Initialize conversation store.
async fn init_conversation_store(app_data_dir: PathBuf) -> Result<SqliteConversationStore, String> {
    let db_path = app_data_dir.join("memory.sqlite");
    let conn = Connection::open(&db_path)
        .await
        .map_err(|e| format!("Failed to open database: {e}"))?;

    SqliteConversationStore::new(Arc::new(conn))
        .await
        .map_err(|e| format!("Failed to initialize conversation store: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize sqlite-vec extension BEFORE any SQLite connection
    init_sqlite_vec_extension();

    let app = tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");

            info!("Memory storage path: {:?}", app_data_dir);

            // Initialize memory engine
            let engine = tauri::async_runtime::block_on(init_memory_engine(app_data_dir.clone()))
                .expect("Failed to initialize memory engine");

            // Initialize conversation store
            let conversation_store =
                tauri::async_runtime::block_on(init_conversation_store(app_data_dir))
                    .expect("Failed to initialize conversation store");

            let state = AppState::new(engine, conversation_store);
            app.manage(state);

            info!("Application initialized");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Existing commands
            commands::start_ollama_ministral,
            commands::ollama_generate_8192,
            commands::chat_with_memory,
            // Conversation commands
            conversations::list_conversations,
            conversations::create_conversation,
            conversations::switch_conversation,
            conversations::delete_conversation,
            conversations::rename_conversation,
            conversations::load_conversation_messages,
            conversations::get_active_conversation,
            conversations::generate_conversation_title,
        ])
        .run(tauri::generate_context!());

    if app.is_err() {
        std::process::exit(1);
    }
}
