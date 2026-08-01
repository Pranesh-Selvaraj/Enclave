//! Enclave — Secure, local-first, zero-knowledge knowledge base.
//!
//! Tauri v2 backend: IPC commands for vault lifecycle (init / unlock / lock),
//! document CRUD, and block CRUD.

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

const DB_FILENAME: &str = "enclave.db";

// ── App State ───────────────────────────────────────────────────────────────

pub struct AppState {
    pub app_dir: PathBuf,
    /// None when locked; Some when unlocked.
    pub db: Mutex<Option<rusqlite::Connection>>,
    pub network: core_network::NetworkState,
}

fn db_path(app_dir: &std::path::Path) -> PathBuf {
    app_dir.join(DB_FILENAME)
}

// wrap a fn that needs Connection, returning a "vault locked" error if None
fn with_db<T>(
    state: &AppState,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state.db.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(conn) => f(conn),
        None => Err("Vault is locked".to_string()),
    }
}

// ── Vault Lifecycle Commands ────────────────────────────────────────────────

#[tauri::command(async)]
fn is_vault_initialized(state: tauri::State<AppState>) -> bool {
    core_db::vault_exists(&db_path(&state.app_dir))
}

#[tauri::command(async)]
fn init_vault(state: tauri::State<AppState>, key: Vec<u8>) -> Result<(), String> {
    let path = db_path(&state.app_dir);
    if core_db::vault_exists(&path) {
        return Err("Vault already exists".to_string());
    }
    let conn = core_db::init_vault(&path, &key)?;
    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    *guard = Some(conn);
    Ok(())
}

#[tauri::command(async)]
fn unlock_vault(state: tauri::State<AppState>, key: Vec<u8>) -> Result<(), String> {
    let path = db_path(&state.app_dir);
    let conn = core_db::open_vault(&path, &key)?;
    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    *guard = Some(conn);
    Ok(())
}

#[tauri::command(async)]
fn lock_vault(state: tauri::State<AppState>) -> Result<(), String> {
    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    *guard = None;
    Ok(())
}

/// Delete vault + key file. Only safe when no user data exists (used when
/// vault creation fails partway and would otherwise lock the user out).
#[tauri::command(async)]
fn reset_vault(state: tauri::State<AppState>) -> Result<(), String> {
    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    *guard = None;
    for f in [DB_FILENAME, "vault.key"] {
        let _ = std::fs::remove_file(state.app_dir.join(f));
    }
    Ok(())
}

// ── Document Commands ───────────────────────────────────────────────────────

#[tauri::command(async)]
fn get_document_list(state: tauri::State<AppState>) -> Result<Vec<core_db::Document>, String> {
    with_db(&state, |db| core_db::query_documents(db).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn get_document(state: tauri::State<AppState>, id: String) -> Result<core_db::Document, String> {
    with_db(&state, |db| core_db::query_document(db, &id).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn create_document(state: tauri::State<AppState>, title: String) -> Result<core_db::Document, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        let doc = core_db::Document {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            created_at: now.clone(),
            updated_at: now.clone(),
            is_favorite: false,
            is_archived: false,
        };
        core_db::insert_document(db, &doc).map_err(|e| e.to_string())?;

        let block = core_db::Block {
            id: uuid::Uuid::new_v4().to_string(),
            document_id: doc.id.clone(),
            block_type: "paragraph".into(),
            content: serde_json::json!({}),
            sort_order: 1.0,
            created_at: now.clone(),
            updated_at: now,
        };
        core_db::insert_block(db, &block).map_err(|e| e.to_string())?;

        Ok(doc)
    })
}

#[tauri::command(async)]
fn delete_document(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    with_db(&state, |db| core_db::delete_document(db, &id).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn update_document_title(
    state: tauri::State<AppState>,
    id: String,
    title: String,
) -> Result<core_db::Document, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::update_document_title(db, &id, &title, &now).map_err(|e| e.to_string())?;
        core_db::query_document(db, &id).map_err(|e| e.to_string())
    })
}

// ── Block Commands ──────────────────────────────────────────────────────────

#[tauri::command(async)]
fn get_blocks(
    state: tauri::State<AppState>,
    document_id: String,
) -> Result<Vec<core_db::Block>, String> {
    with_db(&state, |db| core_db::query_blocks(db, &document_id).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn upsert_block(
    state: tauri::State<AppState>,
    id: String,
    document_id: String,
    block_type: String,
    content: serde_json::Value,
    sort_order: f64,
) -> Result<core_db::Block, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();

        // Plain upsert: OR REPLACE would delete+reinsert (FK cascade landmine
        // and it silently swallows the ON CONFLICT clause). created_at is
        // preserved by not touching it in DO UPDATE.
        db.execute(
            "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
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
            "SELECT b.id, b.document_id, b.content, b.type, b.sort_order, b.created_at, b.updated_at FROM blocks b WHERE b.id = ?1",
            rusqlite::params![id],
            core_db::row_to_block,
        )
        .map_err(|e| e.to_string())
    })
}

#[tauri::command(async)]
fn delete_block(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    with_db(&state, |db| core_db::delete_block(db, &id).map_err(|e| e.to_string()))
}

// ── Markdown Import / Export ────────────────────────────────────────────────

/// Write arbitrary bytes (markdown text or PNG) into the exports dir.
#[tauri::command(async)]
fn export_file(state: tauri::State<AppState>, filename: String, data: Vec<u8>) -> Result<String, String> {
    let exports_dir = state.app_dir.join("exports");
    std::fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;
    let path = exports_dir.join(sanitize_filename(&filename));
    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command(async)]
fn import_markdown(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))
}

/// Write bytes to a user-chosen path (markdown vault export, PNG saves).
#[tauri::command(async)]
fn write_file(path: String, data: Vec<u8>) -> Result<(), String> {
    std::fs::write(&path, &data).map_err(|e| format!("Failed to write file: {e}"))
}

fn sanitize_filename(name: &str) -> String {
    // is_alphanumeric is unicode-aware so CJK/accents survive; only path
    // separators and control chars are replaced.
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' { c } else { '_' })
        .collect();
    let trimmed = safe.trim();
    if trimmed.is_empty() { "untitled".into() } else { trimmed.into() }
}

// ── Backlinks ────────────────────────────────────────────────────────────────

#[tauri::command(async)]
fn get_backlinks(state: tauri::State<AppState>, title: String) -> Result<Vec<core_db::Backlink>, String> {
    with_db(&state, |db| core_db::query_backlinks(db, &title).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn get_page_list(state: tauri::State<AppState>) -> Result<Vec<core_db::PageInfo>, String> {
    with_db(&state, |db| core_db::query_all_page_titles(db).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn get_all_tags(state: tauri::State<AppState>) -> Result<Vec<core_db::TagInfo>, String> {
    with_db(&state, |db| core_db::query_all_tags(db).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn search_all(state: tauri::State<AppState>, query: String) -> Result<Vec<core_db::SearchResult>, String> {
    with_db(&state, |db| core_db::search_all(db, &query).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn find_or_create_document(state: tauri::State<AppState>, title: String) -> Result<core_db::Document, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::find_or_create_document(db, &title, &now).map_err(|e| e.to_string())
    })
}

// ── Vault Key File (encrypted seed phrase for password-based login) ──────────

#[tauri::command(async)]
fn store_vault_key(state: tauri::State<AppState>, key_data: Vec<u8>) -> Result<(), String> {
    let path = state.app_dir.join("vault.key");
    std::fs::write(&path, &key_data).map_err(|e| e.to_string())
}

#[tauri::command(async)]
fn load_vault_key(state: tauri::State<AppState>) -> Result<Vec<u8>, String> {
    let path = state.app_dir.join("vault.key");
    std::fs::read(&path).map_err(|_| "No password set".to_string())
}

// ── Favorites ────────────────────────────────────────────────────────────────

#[tauri::command(async)]
fn toggle_favorite(state: tauri::State<AppState>, id: String) -> Result<core_db::Document, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::toggle_document_favorite(db, &id, &now).map_err(|e| e.to_string())?;
        core_db::query_document(db, &id).map_err(|e| e.to_string())
    })
}

#[tauri::command(async)]
fn duplicate_document(state: tauri::State<AppState>, id: String) -> Result<core_db::Document, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::duplicate_document(db, &id, &now).map_err(|e| e.to_string())
    })
}

// ── Attachments (images etc.) ────────────────────────────────────────────────

/// Writes an attachment under <app_data>/attachments/<document_id>/ and
/// returns the absolute path (frontend serves it via the asset protocol).
#[tauri::command(async)]
fn save_attachment(
    state: tauri::State<AppState>,
    document_id: String,
    filename: String,
    data: Vec<u8>,
) -> Result<String, String> {
    let dir = state.app_dir.join("attachments").join(&document_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let base = sanitize_filename(&filename);
    let mut path = dir.join(&base);
    let mut i = 1;
    while path.exists() {
        // ponytail: naive "name (2)" dedupe, fine for local usage
        let stem = base.rsplit_once('.').map(|(s, _)| s).unwrap_or(&base);
        let ext = base.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
        path = dir.join(format!("{stem} ({i}).{ext}"));
        i += 1;
    }
    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

// ── Network Commands ────────────────────────────────────────────────────────

#[tauri::command(async)]
async fn start_network(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.network.start().await
}

#[tauri::command(async)]
async fn stop_network(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.network.stop().await
}

#[tauri::command(async)]
async fn network_status(state: tauri::State<'_, AppState>) -> Result<core_network::NetworkStatus, String> {
    Ok(state.network.status().await)
}

// ── App Entry Point ─────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            std::fs::create_dir_all(&app_dir)
                .expect("Failed to create app data directory");

            // DB starts locked — user must call init_vault or unlock_vault
            app.manage(AppState {
                app_dir: app_dir.clone(),
                db: Mutex::new(None),
                network: core_network::NetworkState::new(),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // vault lifecycle            is_vault_initialized,
            init_vault,
            unlock_vault,
            lock_vault,
            reset_vault,
            // documents
            get_document_list,
            get_document,
            create_document,
            delete_document,
            update_document_title,
            // blocks
            get_blocks,
            upsert_block,
            delete_block,
            // markdown import/export
            export_file,
            write_file,
            import_markdown,
            // vault key
            store_vault_key,
            load_vault_key,
            // backlinks
            get_backlinks,
            get_page_list,
            get_all_tags,
            search_all,
            save_attachment,
            find_or_create_document,
            // favorites & duplicates
            toggle_favorite,
            duplicate_document,
            // network
            start_network,
            stop_network,
            network_status,
        ])
        .run(tauri::generate_context!())
        .expect("Error while launching Enclave");
}
