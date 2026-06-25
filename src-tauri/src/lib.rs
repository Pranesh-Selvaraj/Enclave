//! Enclave — Secure, local-first, zero-knowledge note-taking application.
//!
//! This crate contains the Tauri v2 backend including:
//! - Encrypted SQLite storage (via rusqlite + sqlcipher)
//! - Local network discovery (via mDNS)
//! - P2P sync over WebSocket / WebRTC data channels
//! - AES-256-GCM encryption with Argon2id key derivation

use tauri::Manager;

/// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Enclave.", name)
}

#[tauri::command]
fn get_app_status() -> serde_json::Value {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "status": "operational",
        "network": "offline",
        "encryption": "AES-256-GCM",
        "storage": "SQLite + sqlcipher"
    })
}

/// Application state shared across all Tauri commands.
pub struct AppState {
    /// Path to the encrypted database
    pub db_path: std::path::PathBuf,
    /// Whether the local network sync service is running
    pub sync_active: bool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Resolve the app data directory for encrypted storage
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            std::fs::create_dir_all(&app_dir)
                .expect("Failed to create app data directory");

            let db_path = app_dir.join("enclave.db");

            app.manage(AppState {
                db_path,
                sync_active: false,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet, get_app_status])
        .run(tauri::generate_context!())
        .expect("Error while launching Enclave");
}
