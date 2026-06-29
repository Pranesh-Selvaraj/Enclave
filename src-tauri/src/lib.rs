//! Enclave — Secure, local-first, zero-knowledge note-taking application.
//!
//! Tauri v2 backend: IPC commands delegating to `core_db` (storage)
//! and `core_network` (P2P — future).

use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

// ── Document Commands ───────────────────────────────────────────────────────

#[tauri::command]
fn get_document_list(state: tauri::State<AppState>) -> Result<Vec<core_db::Document>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    core_db::query_documents(&db).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_document(state: tauri::State<AppState>, id: String) -> Result<core_db::Document, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    core_db::query_document(&db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_document(state: tauri::State<AppState>, title: String) -> Result<core_db::Document, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let doc = core_db::Document {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        created_at: now.clone(),
        updated_at: now.clone(),
        is_favorite: false,
        is_archived: false,
    };
    core_db::insert_document(&db, &doc).map_err(|e| e.to_string())?;

    let block = core_db::Block {
        id: uuid::Uuid::new_v4().to_string(),
        document_id: doc.id.clone(),
        block_type: "paragraph".into(),
        content: serde_json::json!({}),
        sort_order: 1.0,
        created_at: now.clone(),
        updated_at: now,
    };
    core_db::insert_block(&db, &block).map_err(|e| e.to_string())?;

    Ok(doc)
}

#[tauri::command]
fn delete_document(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    core_db::delete_document(&db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_document_title(
    state: tauri::State<AppState>,
    id: String,
    title: String,
) -> Result<core_db::Document, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    core_db::update_document_title(&db, &id, &title, &now).map_err(|e| e.to_string())?;
    core_db::query_document(&db, &id).map_err(|e| e.to_string())
}

// ── Block Commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn get_blocks(
    state: tauri::State<AppState>,
    document_id: String,
) -> Result<Vec<core_db::Block>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    core_db::query_blocks(&db, &document_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn upsert_block(
    state: tauri::State<AppState>,
    id: String,
    document_id: String,
    block_type: String,
    content: serde_json::Value,
    sort_order: f64,
) -> Result<core_db::Block, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    db.execute(
        "INSERT OR REPLACE INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6,
                 COALESCE((SELECT created_at FROM blocks WHERE id = ?1), ?6))
         ON CONFLICT(id) DO UPDATE SET
             document_id = excluded.document_id,
             type = excluded.type,
             content = excluded.content,
             sort_order = excluded.sort_order,
             updated_at = excluded.updated_at",
        rusqlite::params![id, document_id, block_type, content.to_string(), sort_order, now],
    )
    .map_err(|e| e.to_string())?;

    db.query_row(
        &format!("{} WHERE b.id = ?1", core_db::BLOCK_COLS),
        rusqlite::params![id],
        core_db::row_to_block,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_block(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    core_db::delete_block(&db, &id).map_err(|e| e.to_string())
}

// ── App Entry Point ─────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            std::fs::create_dir_all(&app_dir)
                .expect("Failed to create app data directory");

            let db_path = app_dir.join("enclave.db");
            let conn = core_db::init_db(&db_path);

            app.manage(AppState {
                db: Mutex::new(conn),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_document_list,
            get_document,
            create_document,
            delete_document,
            update_document_title,
            get_blocks,
            upsert_block,
            delete_block,
        ])
        .run(tauri::generate_context!())
        .expect("Error while launching Enclave");
}
