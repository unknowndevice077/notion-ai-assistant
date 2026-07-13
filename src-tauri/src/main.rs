#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod commands;
mod db;
mod model_registry;
mod models;
mod notion;
mod ollama;
mod unsplash;

use db::Db;
use rusqlite::Connection;
use std::sync::Mutex;

fn app_data_dir() -> std::path::PathBuf {
    let dir = dirs_next_data_dir();
    std::fs::create_dir_all(&dir).expect("failed to create app data directory");
    dir
}

fn dirs_next_data_dir() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    { let home = std::env::var("HOME").unwrap_or_else(|_| ".".into()); std::path::PathBuf::from(home).join("Library/Application Support/NotionAIAssistant") }
    #[cfg(target_os = "windows")]
    { let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into()); std::path::PathBuf::from(appdata).join("NotionAIAssistant") }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    { let home = std::env::var("HOME").unwrap_or_else(|_| ".".into()); std::path::PathBuf::from(home).join(".notion-ai-assistant") }
}

fn main() {
    let db_path = app_data_dir().join("notion-ai-assistant.sqlite3");
    let conn = Connection::open(db_path).expect("failed to open local database");
    db::init(&conn).expect("failed to initialize database schema");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Db(Mutex::new(conn)))
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_available_models,
            commands::get_ollama_status,
            commands::pull_ollama_model,
            commands::connect_notion,
            commands::test_notion_connection,
            commands::test_ai_connection,
            commands::get_presets,
            commands::generate_content,
            commands::run_preset,
            commands::discard_notion_push,
            commands::list_batches,
            commands::list_businesses,
            commands::edit_business,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Notion AI Assistant");
}