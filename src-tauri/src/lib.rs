//! Enclave — Secure, local-first, zero-knowledge knowledge base.
//!
//! Tauri v2 backend: IPC commands for vault lifecycle (init / unlock / lock),
//! document CRUD, and block CRUD.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tauri::Manager;

// fastembed/ONNX Runtime ships no prebuilt binaries for x86_64-linux-android
// (emulator-only target), so local embeddings are compiled out there.
// arm64 phones + all desktops keep RAG. See the Cargo.toml gate.
#[cfg(not(all(target_os = "android", target_arch = "x86_64")))]
mod embed;

mod updater;

const DB_FILENAME: &str = "enclave.db";

// ── App State ───────────────────────────────────────────────────────────────

pub struct AppState {
    pub app_dir: PathBuf,
    /// None when locked; Some when unlocked.
    pub db: Mutex<Option<rusqlite::Connection>>,
    /// Vault-derived sync PSK, present only while the vault is unlocked.
    /// Same owner = same seed phrase = same vault key = same sync key, so
    /// P2P sync authenticates and encrypts with it (see core-network/crypto.rs).
    pub sync_key: Mutex<Option<[u8; 32]>>,
    pub network: Arc<core_network::NetworkState>,
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
    *state.sync_key.lock().map_err(|e| e.to_string())? = Some(core_network::crypto::derive_sync_key(&key));
    Ok(())
}

#[tauri::command(async)]
fn unlock_vault(state: tauri::State<AppState>, key: Vec<u8>) -> Result<(), String> {
    let path = db_path(&state.app_dir);
    let conn = core_db::open_vault(&path, &key)?;
    let mut guard = state.db.lock().map_err(|e| e.to_string())?;
    *guard = Some(conn);
    *state.sync_key.lock().map_err(|e| e.to_string())? = Some(core_network::crypto::derive_sync_key(&key));
    Ok(())
}

// Lock is async so it can stop the network (which holds the sync key).
#[tauri::command]
async fn lock_vault(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Locked vault = no sync: drop the key and stop the network so no
    // session keeps running with key material after the user locks up.
    let _ = state.network.stop().await;
    *state.sync_key.lock().map_err(|e| e.to_string())? = None;
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

// ── Folder Commands ─────────────────────────────────────────────────────────

#[tauri::command(async)]
fn get_folders(state: tauri::State<AppState>) -> Result<Vec<core_db::Folder>, String> {
    with_db(&state, |db| core_db::query_folders(db).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn create_folder(state: tauri::State<AppState>, name: String) -> Result<core_db::Folder, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        let folder = core_db::Folder {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            created_at: now,
        };
        core_db::insert_folder(db, &folder).map_err(|e| e.to_string())?;
        Ok(folder)
    })
}

#[tauri::command(async)]
fn rename_folder(state: tauri::State<AppState>, id: String, name: String) -> Result<(), String> {
    with_db(&state, |db| core_db::rename_folder(db, &id, &name).map_err(|e| e.to_string()))
}

/// Deleting a folder never deletes its pages — they fall back to the root.
#[tauri::command(async)]
fn delete_folder(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    with_db(&state, |db| core_db::delete_folder(db, &id).map_err(|e| e.to_string()))
}

/// Move a page into a folder (folder_id null = root).
#[tauri::command(async)]
fn move_document(
    state: tauri::State<AppState>,
    id: String,
    folder_id: Option<String>,
) -> Result<core_db::Document, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::move_document(db, &id, folder_id.as_deref(), &now).map_err(|e| e.to_string())?;
        core_db::query_document(db, &id).map_err(|e| e.to_string())
    })
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
            rev: 0,
            deleted_at: None,
            folder_id: None,
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
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::delete_document(db, &id, &now).map_err(|e| e.to_string())
    })
}

#[tauri::command(async)]
fn archive_document(state: tauri::State<AppState>, id: String) -> Result<core_db::Document, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::archive_document(db, &id, &now).map_err(|e| e.to_string())?;
        core_db::query_document(db, &id).map_err(|e| e.to_string())
    })
}

#[tauri::command(async)]
fn restore_document(state: tauri::State<AppState>, id: String) -> Result<core_db::Document, String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::restore_document(db, &id, &now).map_err(|e| e.to_string())?;
        core_db::query_document(db, &id).map_err(|e| e.to_string())
    })
}

#[tauri::command(async)]
fn get_archived_documents(state: tauri::State<AppState>) -> Result<Vec<core_db::Document>, String> {
    with_db(&state, |db| core_db::query_archived_documents(db).map_err(|e| e.to_string()))
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
        let block = core_db::Block {
            id,
            document_id,
            block_type,
            content,
            sort_order,
            created_at: now.clone(),
            updated_at: now,
        };
        core_db::upsert_block(db, &block).map_err(|e| e.to_string())
    })
}

#[tauri::command(async)]
fn delete_block(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    with_db(&state, |db| core_db::delete_block(db, &id).map_err(|e| e.to_string()))
}

// ── Embeddings (RAG) ─────────────────────────────────────────────────────────

#[tauri::command(async)]
fn upsert_embedding(
    state: tauri::State<AppState>,
    block_id: String,
    document_id: String,
    text: String,
    vector: Vec<f64>,
) -> Result<(), String> {
    with_db(&state, |db| {
        let now = chrono::Utc::now().to_rfc3339();
        core_db::upsert_embedding(db, &block_id, &document_id, &text, &vector, &now)
            .map_err(|e| e.to_string())
    })
}

/// Ranked retrieval: ANN top-k via the in-DB vec0 index (exact-cosine
/// re-ranked), with an exact-scan fallback for unknown dimensions.
#[tauri::command(async)]
fn search_embeddings(
    state: tauri::State<AppState>,
    query: Vec<f64>,
    limit: usize,
) -> Result<Vec<core_db::Embedding>, String> {
    with_db(&state, |db| {
        core_db::query_embeddings_topk(db, &query, limit).map_err(|e| e.to_string())
    })
}

/// Offline embedding via the built-in ONNX model (fastembed). Inference is
/// CPU-bound, so it runs on the blocking pool; the first call downloads the
/// model into the app data dir (needs internet once).
/// Not available on x86_64-android (emulator-only): ORT has no prebuilt
/// binaries for that target — see the Cargo.toml gate.
#[cfg(not(all(target_os = "android", target_arch = "x86_64")))]
#[tauri::command]
async fn embed_text(state: tauri::State<'_, AppState>, text: String) -> Result<Vec<f64>, String> {
    let cache_dir = state.app_dir.join("models");
    tauri::async_runtime::spawn_blocking(move || crate::embed::embed_text_blocking(&cache_dir, &text))
        .await
        .map_err(|e| e.to_string())?
}

// ── Vault-scoped settings (encrypted at rest; holds AI config + API keys) ───

#[tauri::command(async)]
fn get_setting(state: tauri::State<AppState>, key: String) -> Result<Option<String>, String> {
    with_db(&state, |db| core_db::get_setting(db, &key).map_err(|e| e.to_string()))
}

#[tauri::command(async)]
fn set_setting(state: tauri::State<AppState>, key: String, value: String) -> Result<(), String> {
    with_db(&state, |db| core_db::set_setting(db, &key, &value).map_err(|e| e.to_string()))
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

/// Consistent encrypted snapshot of the vault via VACUUM INTO (SQLCipher-safe;
/// WAL-safe too, unlike a raw file copy). Restore = replace enclave.db while
/// the app is closed.
#[tauri::command(async)]
fn backup_vault(state: tauri::State<AppState>) -> Result<String, String> {
    let exports_dir = state.app_dir.join("exports");
    std::fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let dest = exports_dir.join(format!("enclave-backup-{stamp}.db"));
    let sql = format!("VACUUM INTO '{}'", dest.to_string_lossy().replace('\'', "''"));
    with_db(&state, |db| db.execute(&sql, []).map(|_| ()).map_err(|e| e.to_string()))?;
    Ok(dest.to_string_lossy().to_string())
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
fn find_relation_backlinks(state: tauri::State<AppState>, doc_id: String) -> Result<Vec<core_db::Backlink>, String> {
    with_db(&state, |db| core_db::find_relation_backlinks(db, &doc_id).map_err(|e| e.to_string()))
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
async fn start_network(state: tauri::State<'_, AppState>, name: Option<String>) -> Result<(), String> {
    let name = name.unwrap_or_else(|| "Enclave".to_string());
    let key = state
        .sync_key
        .lock()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Vault is locked — unlock before enabling sync".to_string())?;
    state.network.start(&name, key).await
}

#[tauri::command(async)]
async fn stop_network(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.network.stop().await
}

#[tauri::command(async)]
async fn network_status(state: tauri::State<'_, AppState>) -> Result<core_network::NetworkStatus, String> {
    Ok(state.network.status().await)
}

// ── Sync Message Handling (LAN v2) ──────────────────────────────────────────

/// Wire protocol: both sides send hello on connect; each side responds to a
/// hello with a full snapshot {kind:"snapshot", docs, blocks}; the receiver
/// merges it (doc-level LWW) and replies with an ack for the UI.
fn sync_snapshot(state: &AppState) -> Option<String> {
    let payload = with_db(state, |db| {
        let (docs, blocks) = core_db::query_sync_data(db).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "kind": "snapshot", "docs": docs, "blocks": blocks }).to_string())
    });
    payload.ok()
}

async fn handle_sync_message(
    app: &tauri::AppHandle,
    state: &AppState,
    net: &core_network::NetworkState,
    msg: core_network::PeerMessage,
) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg.payload) else {
        return;
    };
    match v["kind"].as_str() {
        Some("hello") => {
            let peer_id = v["peer_id"].as_str().unwrap_or(&msg.from_peer).to_string();
            if let Some(snapshot) = sync_snapshot(state) {
                net.send_to(&peer_id, snapshot).await;
            }
        }
        Some("snapshot") => {
            let peer_id = v["peer_id"].as_str().unwrap_or(&msg.from_peer).to_string();
            let docs: Vec<core_db::Document> = serde_json::from_value(v["docs"].clone()).unwrap_or_default();
            let blocks: Vec<core_db::Block> = serde_json::from_value(v["blocks"].clone()).unwrap_or_default();
            match with_db(state, |db| core_db::sync_merge(db, &docs, &blocks).map_err(|e| e.to_string())) {
                Ok(stats) => {
                    net.mark_synced().await;
                    let ack = serde_json::json!({
                        "kind": "ack",
                        "docs_changed": stats.docs_changed,
                        "blocks_changed": stats.blocks_changed,
                    })
                    .to_string();
                    net.send_to(&peer_id, ack).await;
                    let _ = app.emit(
                        "sync-done",
                        serde_json::json!({
                            "peer": &peer_id,
                            "docs_changed": stats.docs_changed,
                            "blocks_changed": stats.blocks_changed,
                        }),
                    );
                }
                Err(_) => { /* vault locked — ignore */ }
            }
        }
        Some("ack") => {
            net.mark_synced().await;
            let _ = app.emit(
                "sync-done",
                serde_json::json!({
                    "peer": v["peer_id"].as_str().unwrap_or(&msg.from_peer),
                    "docs_changed": v["docs_changed"].as_u64().unwrap_or(0),
                    "blocks_changed": v["blocks_changed"].as_u64().unwrap_or(0),
                }),
            );
        }
        _ => {}
    }
}

// ── App Entry Point ─────────────────────────────────────────────────────────

// Tray + quick-capture window are desktop-only (no system tray on Android).
#[cfg(desktop)]
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};

    let show = MenuItem::with_id(app, "show", "Show Enclave", true, None::<&str>)?;
    let capture = MenuItem::with_id(app, "capture", "Quick Capture", true, None::<&str>)?;
    let widget = MenuItem::with_id(app, "widget", "Wallpaper widget", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &capture, &widget, &quit])?;

    let icon = app
        .default_window_icon()
        .expect("bundle icons are configured")
        .clone();

    tauri::tray::TrayIconBuilder::with_id("enclave-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            "capture" => open_capture_window(app),
            "widget" => { let _ = toggle_widget(app.clone()); }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)
        .map(|_| ())
        // ponytail: tray is best-effort — a DE without tray support must not
        // block the app from starting.
        .or_else(|e| {
            eprintln!("tray setup failed (continuing without it): {e}");
            Ok(())
        })
}

#[cfg(desktop)]
fn open_capture_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("capture") {
        let _ = w.show();
        let _ = w.set_focus();
        return;
    }
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        "capture",
        tauri::WebviewUrl::App("/capture".into()),
    )
    .title("Enclave — Quick Capture")
    .inner_size(560.0, 300.0)
    .resizable(true)
    .build();
}

// ── Wallpaper widget (desktop) ──────────────────────────────────────────────
// A small frameless, transparent, always-on-top panel pinned to the corner of
// the primary monitor — the "desktop widget". The /widget route renders a
// mini dashboard (recents + quick capture) with a transparent background.
// ponytail: always_on_top (macOS-style floating widget) rather than a true
// desktop-level window (below app windows) — that needs per-platform window
// type hints; the floating panel is the portable v1.

#[cfg(desktop)]
fn widget_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("widget") {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    let window = tauri::WebviewWindowBuilder::new(app, "widget", tauri::WebviewUrl::App("/widget".into()))
        .title("Enclave — Widget")
        .inner_size(320.0, 460.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    // Dock it to the bottom-right of the primary monitor.
    if let Ok(Some(monitor)) = window.primary_monitor() {
        if let (Ok(scale), Ok(size)) = (window.scale_factor(), window.outer_size()) {
            let w = (size.width as f64 * scale) as i32;
            let h = (size.height as f64 * scale) as i32;
            let area = monitor.work_area();
            let _ = window.set_position(tauri::PhysicalPosition::new(
                area.position.x + area.size.width as i32 - w - 24,
                area.position.y + area.size.height as i32 - h - 24,
            ));
        }
    }
    let _ = window.show();
    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
fn toggle_widget(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("widget") {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    } else {
        widget_window(&app)?;
    }
    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
fn hide_widget(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("widget") {
        let _ = w.hide();
    }
    Ok(())
}

/// The widget's "open page" action: surface the main window and navigate it.
#[cfg(desktop)]
#[tauri::command]
fn open_doc_from_widget(app: tauri::AppHandle, id: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    app.emit("open-doc", id).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(updater::plugin())
        .setup(|app| {
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to resolve app data directory");

            std::fs::create_dir_all(&app_dir)
                .expect("Failed to create app data directory");

            // DB starts locked — user must call init_vault or unlock_vault
            let network = Arc::new(core_network::NetworkState::new());
            let sync_rx = network
                .message_rx
                .try_lock()
                .expect("network rx uncontended at setup")
                .take()
                .expect("sync receiver exists once");
            app.manage(AppState {
                app_dir: app_dir.clone(),
                db: Mutex::new(None),
                sync_key: Mutex::new(None),
                network: network.clone(),
            });

            // Consume peer messages for the lifetime of the app: hello →
            // send snapshot, snapshot → merge into vault, ack → notify UI.
            // State is looked up per message so the task doesn't need to own
            // an Arc (commands resolve State<AppState> by exact type).
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut rx = sync_rx;
                while let Some(msg) = rx.recv().await {
                    let st = app_handle.state::<AppState>();
                    let net = st.network.clone();
                    handle_sync_message(&app_handle, &st, &net, msg).await;
                }
            });

            #[cfg(desktop)]
            setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // vault lifecycle
            is_vault_initialized,
            init_vault,
            unlock_vault,
            lock_vault,
            reset_vault,
            // documents
            get_document_list,
            get_document,
            create_document,
            delete_document,
            archive_document,
            restore_document,
            get_archived_documents,
            update_document_title,
            // folders
            get_folders,
            create_folder,
            rename_folder,
            delete_folder,
            move_document,
            // blocks
            get_blocks,
            upsert_block,
            delete_block,
            // embeddings (RAG)
            upsert_embedding,
            search_embeddings,
            #[cfg(not(all(target_os = "android", target_arch = "x86_64")))]
            embed_text,
            // vault-scoped settings (AI config, API keys)
            get_setting,
            set_setting,
            // markdown import/export
            export_file,
            write_file,
            import_markdown,
            // vault key
            store_vault_key,
            load_vault_key,
            // backlinks
            get_backlinks,
            find_relation_backlinks,
            get_page_list,
            get_all_tags,
            search_all,
            save_attachment,
            find_or_create_document,
            // favorites & duplicates
            toggle_favorite,
            duplicate_document,
            // vault backup
            backup_vault,
            // network
            start_network,
            stop_network,
            network_status,
            // desktop wallpaper widget
            #[cfg(desktop)]
            toggle_widget,
            #[cfg(desktop)]
            hide_widget,
            #[cfg(desktop)]
            open_doc_from_widget,
            // self-update
            updater::app_version,
            updater::check_for_update,
            updater::download_update,
            updater::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("Error while launching Enclave");
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: commands take State<AppState>, so AppState must be managed
    /// as a plain value — managing an Arc<AppState> instead makes every invoke
    /// fail with "state not managed" (Tauri 2 keys state by exact type).
    #[test]
    fn app_state_resolves_for_commands() {
        let app = tauri::test::mock_app();
        app.manage(AppState {
            app_dir: PathBuf::from("/tmp/enclave-test"),
            db: Mutex::new(None),
            sync_key: Mutex::new(None),
            network: Arc::new(core_network::NetworkState::new()),
        });
        let state = app.state::<AppState>();
        assert!(!is_vault_initialized(state), "no vault in the test dir");
    }
}
