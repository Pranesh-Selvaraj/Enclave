//! Enclave — Secure, local-first, zero-knowledge knowledge base.
//!
//! Tauri v2 desktop/mobile shell. All vault behavior lives in `core-api`;
//! this crate only adapts it to Tauri: IPC commands, window/tray/widget
//! management, the updater plugin, and the Android foreground-sync bridge.
//! Keeping the shell thin is what lets the native Android app reuse the same
//! core (and therefore the same vault format and sync protocol).

use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;

// fastembed/ONNX Runtime ships no prebuilt binaries for x86_64-linux-android
// (emulator-only target), so local embeddings are compiled out there.
// arm64 phones + all desktops keep RAG. See the Cargo.toml gate.
#[cfg(not(all(target_os = "android", target_arch = "x86_64")))]
mod embed;

mod updater;

mod android_sync;

// ── App State ───────────────────────────────────────────────────────────────

pub struct AppState {
    /// Single source of truth for the vault, network and sync protocol.
    pub core: Arc<core_api::EnclaveCore>,
}

/// Map a core error onto the string contract the existing UI expects.
fn msg(e: core_api::CoreError) -> String {
    e.to_string()
}

// ── Vault Lifecycle Commands ────────────────────────────────────────────────

#[tauri::command(async)]
fn is_vault_initialized(state: tauri::State<AppState>) -> bool {
    state.core.is_vault_initialized()
}

/// True when the shared core is already unlocked — the editor island uses it
/// to skip the vault guard after the native shell unlocked the vault.
#[tauri::command(async)]
fn is_vault_unlocked(state: tauri::State<AppState>) -> bool {
    state.core.is_unlocked()
}

#[tauri::command(async)]
fn init_vault(state: tauri::State<AppState>, key: Vec<u8>) -> Result<(), String> {
    state.core.init_vault(&key).map_err(msg)
}

#[tauri::command(async)]
fn unlock_vault(state: tauri::State<AppState>, key: Vec<u8>) -> Result<(), String> {
    state.core.unlock_vault(&key).map_err(msg)
}

// Lock is async so it can stop the network (which holds the sync key).
#[tauri::command]
async fn lock_vault(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.core.lock_vault().await.map_err(msg)?;
    // Android: no vault key = no foreground sync service.
    let _ = android_sync::set_service(false);
    Ok(())
}

/// Delete vault + key file. Only safe when no user data exists (used when
/// vault creation fails partway and would otherwise lock the user out).
#[tauri::command(async)]
fn reset_vault(state: tauri::State<AppState>) -> Result<(), String> {
    state.core.reset_vault().map_err(msg)
}

// ── Document Commands ───────────────────────────────────────────────────────

#[tauri::command(async)]
fn get_document_list(state: tauri::State<AppState>) -> Result<Vec<core_api::Document>, String> {
    state.core.list_documents().map_err(msg)
}

#[tauri::command(async)]
fn get_document(state: tauri::State<AppState>, id: String) -> Result<core_api::Document, String> {
    state.core.get_document(&id).map_err(msg)
}

#[tauri::command(async)]
fn create_document(state: tauri::State<AppState>, title: String) -> Result<core_api::Document, String> {
    state.core.create_document(&title).map_err(msg)
}

#[tauri::command(async)]
fn delete_document(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.core.delete_document(&id).map_err(msg)
}

#[tauri::command(async)]
fn archive_document(state: tauri::State<AppState>, id: String) -> Result<core_api::Document, String> {
    state.core.archive_document(&id).map_err(msg)
}

#[tauri::command(async)]
fn restore_document(state: tauri::State<AppState>, id: String) -> Result<core_api::Document, String> {
    state.core.restore_document(&id).map_err(msg)
}

#[tauri::command(async)]
fn get_archived_documents(state: tauri::State<AppState>) -> Result<Vec<core_api::Document>, String> {
    state.core.list_archived_documents().map_err(msg)
}

#[tauri::command(async)]
fn update_document_title(
    state: tauri::State<AppState>,
    id: String,
    title: String,
) -> Result<core_api::Document, String> {
    state.core.update_document_title(&id, &title).map_err(msg)
}

#[tauri::command(async)]
fn find_or_create_document(state: tauri::State<AppState>, title: String) -> Result<core_api::Document, String> {
    state.core.find_or_create_document(&title).map_err(msg)
}

#[tauri::command(async)]
fn toggle_favorite(state: tauri::State<AppState>, id: String) -> Result<core_api::Document, String> {
    state.core.toggle_favorite(&id).map_err(msg)
}

#[tauri::command(async)]
fn duplicate_document(state: tauri::State<AppState>, id: String) -> Result<core_api::Document, String> {
    state.core.duplicate_document(&id).map_err(msg)
}

#[tauri::command(async)]
fn move_document(
    state: tauri::State<AppState>,
    id: String,
    folder_id: Option<String>,
) -> Result<core_api::Document, String> {
    state.core.move_document(&id, folder_id.as_deref()).map_err(msg)
}

// ── Folder Commands ─────────────────────────────────────────────────────────

#[tauri::command(async)]
fn get_folders(state: tauri::State<AppState>) -> Result<Vec<core_api::Folder>, String> {
    state.core.list_folders().map_err(msg)
}

#[tauri::command(async)]
fn create_folder(state: tauri::State<AppState>, name: String) -> Result<core_api::Folder, String> {
    state.core.create_folder(&name).map_err(msg)
}

#[tauri::command(async)]
fn rename_folder(state: tauri::State<AppState>, id: String, name: String) -> Result<(), String> {
    state.core.rename_folder(&id, &name).map_err(msg)
}

/// Deleting a folder never deletes its pages — they fall back to the root.
#[tauri::command(async)]
fn delete_folder(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.core.delete_folder(&id).map_err(msg)
}

// ── Block Commands ──────────────────────────────────────────────────────────

#[tauri::command(async)]
fn get_blocks(
    state: tauri::State<AppState>,
    document_id: String,
) -> Result<Vec<core_api::Block>, String> {
    state.core.get_blocks(&document_id).map_err(msg)
}

#[tauri::command(async)]
fn upsert_block(
    state: tauri::State<AppState>,
    id: String,
    document_id: String,
    block_type: String,
    content: serde_json::Value,
    sort_order: f64,
) -> Result<core_api::Block, String> {
    state
        .core
        .upsert_block(&id, &document_id, &block_type, content, sort_order)
        .map_err(msg)
}

#[tauri::command(async)]
fn delete_block(state: tauri::State<AppState>, id: String) -> Result<(), String> {
    state.core.delete_block(&id).map_err(msg)
}

// ── Embeddings (RAG storage) ────────────────────────────────────────────────

#[tauri::command(async)]
fn upsert_embedding(
    state: tauri::State<AppState>,
    block_id: String,
    document_id: String,
    text: String,
    vector: Vec<f64>,
) -> Result<(), String> {
    state
        .core
        .upsert_embedding(&block_id, &document_id, &text, &vector)
        .map_err(msg)
}

/// Ranked retrieval: ANN top-k via the in-DB vec0 index (exact-cosine
/// re-ranked), with an exact-scan fallback for unknown dimensions.
#[tauri::command(async)]
fn search_embeddings(
    state: tauri::State<AppState>,
    query: Vec<f64>,
    limit: usize,
) -> Result<Vec<core_api::Embedding>, String> {
    state.core.search_embeddings(&query, limit).map_err(msg)
}

/// Offline embedding via the built-in ONNX model (fastembed). Inference is
/// CPU-bound, so it runs on the blocking pool; the first call downloads the
/// model into the app data dir (needs internet once).
/// Not available on x86_64-android (emulator-only): ORT has no prebuilt
/// binaries for that target — see the Cargo.toml gate.
#[cfg(not(all(target_os = "android", target_arch = "x86_64")))]
#[tauri::command]
async fn embed_text(state: tauri::State<'_, AppState>, text: String) -> Result<Vec<f64>, String> {
    let cache_dir = state.core.app_dir().join("models");
    tauri::async_runtime::spawn_blocking(move || crate::embed::embed_text_blocking(&cache_dir, &text))
        .await
        .map_err(|e| e.to_string())?
}

// ── Vault-scoped settings (encrypted at rest; holds AI config + API keys) ───

#[tauri::command(async)]
fn get_setting(state: tauri::State<AppState>, key: String) -> Result<Option<String>, String> {
    state.core.get_setting(&key).map_err(msg)
}

#[tauri::command(async)]
fn set_setting(state: tauri::State<AppState>, key: String, value: String) -> Result<(), String> {
    state.core.set_setting(&key, &value).map_err(msg)
}

// ── Markdown Import / Export ────────────────────────────────────────────────

/// Write arbitrary bytes (markdown text or PNG) into the exports dir.
#[tauri::command(async)]
fn export_file(state: tauri::State<AppState>, filename: String, data: Vec<u8>) -> Result<String, String> {
    state.core.export_file(&filename, &data).map_err(msg)
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
    state.core.backup_vault().map_err(msg)
}

// ── Backlinks / tags / search ───────────────────────────────────────────────

#[tauri::command(async)]
fn get_backlinks(state: tauri::State<AppState>, title: String) -> Result<Vec<core_api::Backlink>, String> {
    state.core.get_backlinks(&title).map_err(msg)
}

#[tauri::command(async)]
fn find_relation_backlinks(state: tauri::State<AppState>, doc_id: String) -> Result<Vec<core_api::Backlink>, String> {
    state.core.find_relation_backlinks(&doc_id).map_err(msg)
}

#[tauri::command(async)]
fn get_page_list(state: tauri::State<AppState>) -> Result<Vec<core_api::PageInfo>, String> {
    state.core.get_page_list().map_err(msg)
}

#[tauri::command(async)]
fn get_all_tags(state: tauri::State<AppState>) -> Result<Vec<core_api::TagInfo>, String> {
    state.core.get_all_tags().map_err(msg)
}

#[tauri::command(async)]
fn search_all(state: tauri::State<AppState>, query: String) -> Result<Vec<core_api::SearchResult>, String> {
    state.core.search_all(&query).map_err(msg)
}

// ── Vault Key File (encrypted seed phrase for password-based login) ─────────

#[tauri::command(async)]
fn store_vault_key(state: tauri::State<AppState>, key_data: Vec<u8>) -> Result<(), String> {
    state.core.store_vault_key(&key_data).map_err(msg)
}

#[tauri::command(async)]
fn load_vault_key(state: tauri::State<AppState>) -> Result<Vec<u8>, String> {
    state.core.load_vault_key().map_err(msg)
}

// ── Attachments (images etc.) ───────────────────────────────────────────────

/// Writes an attachment under <app_data>/attachments/<document_id>/ and
/// returns the absolute path (frontend serves it via the asset protocol).
#[tauri::command(async)]
fn save_attachment(
    state: tauri::State<AppState>,
    document_id: String,
    filename: String,
    data: Vec<u8>,
) -> Result<String, String> {
    state
        .core
        .save_attachment(&document_id, &filename, &data)
        .map_err(msg)
}

// ── Network Commands ────────────────────────────────────────────────────────

#[tauri::command]
async fn start_network(state: tauri::State<'_, AppState>, name: Option<String>) -> Result<(), String> {
    state.core.start_network(name).await.map_err(msg)?;
    // Android: keep the stack alive while backgrounded (no-op elsewhere).
    let _ = android_sync::set_service(true);
    Ok(())
}

#[tauri::command]
async fn stop_network(state: tauri::State<'_, AppState>) -> Result<(), String> {
    state.core.stop_network().await.map_err(msg)?;
    let _ = android_sync::set_service(false);
    Ok(())
}

/// Manual peer connect for networks where mDNS discovery is blocked.
#[tauri::command]
async fn connect_peer(state: tauri::State<'_, AppState>, host: String, port: u16) -> Result<(), String> {
    state.core.connect_peer(&host, port).await.map_err(msg)
}

#[tauri::command]
async fn network_status(state: tauri::State<'_, AppState>) -> Result<core_api::NetworkStatus, String> {
    Ok(state.core.network_status().await)
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

            // DB starts locked — user must call init_vault or unlock_vault.
            // Shared process-wide core: the native shell may have created and
            // unlocked it before the editor island opened.
            let core = core_api::EnclaveCore::global(app_dir);
            app.manage(AppState { core: core.clone() });

            // The core owns the peer-message loop; events fan out to shells.
            // This keeps sync alive when the native shell is the only UI.
            let loop_core = core.clone();
            tauri::async_runtime::spawn(async move { loop_core.run_sync_loop().await });

            // Forward sync events to the web UI (toasts / peer failures).
            let app_handle = app.handle().clone();
            let mut events = core.subscribe_sync_events();
            tauri::async_runtime::spawn(async move {
                use tokio::sync::broadcast::error::RecvError;
                loop {
                    match events.recv().await {
                        Ok(core_api::SyncEvent::Done { peer, docs_changed, blocks_changed }) => {
                            let _ = app_handle.emit(
                                "sync-done",
                                serde_json::json!({
                                    "peer": peer,
                                    "docs_changed": docs_changed,
                                    "blocks_changed": blocks_changed,
                                }),
                            );
                        }
                        Ok(core_api::SyncEvent::PeerFailed { host, error }) => {
                            let _ = app_handle.emit(
                                "peer-connect-failed",
                                serde_json::json!({ "host": host, "error": error }),
                            );
                        }
                        Err(RecvError::Lagged(_)) => continue,
                        Err(RecvError::Closed) => break,
                    }
                }
            });

            #[cfg(desktop)]
            setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // vault lifecycle
            is_vault_initialized,
            is_vault_unlocked,
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
            connect_peer,
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
    use std::path::PathBuf;

    /// Regression: commands take State<AppState>, so AppState must be managed
    /// as a plain value — managing an Arc<AppState> instead makes every invoke
    /// fail with "state not managed" (Tauri 2 keys state by exact type).
    #[test]
    fn app_state_resolves_for_commands() {
        let app = tauri::test::mock_app();
        app.manage(AppState {
            core: Arc::new(core_api::EnclaveCore::new(PathBuf::from("/tmp/enclave-test"))),
        });
        let state = app.state::<AppState>();
        assert!(!is_vault_initialized(state), "no vault in the test dir");
    }
}
