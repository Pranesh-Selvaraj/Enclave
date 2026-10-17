//! Enclave core API — the shell-agnostic surface over `core-db` and
//! `core-network`.
//!
//! Everything the app can do to the vault lives here: vault lifecycle,
//! document/folder/block CRUD, search, backlinks, settings, files and the P2P
//! sync protocol. Shells (the Tauri desktop app today, the native Android app
//! next) are thin adapters: they own windows, events and lifetime, never data
//! logic. That keeps desktop and mobile on one implementation and one wire
//! protocol — full interoperability by construction.
//!
//! Errors map 1:1 to the strings the existing UI already surfaces, so wiring a
//! new shell in does not change user-visible behavior.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use rusqlite::Connection;

pub mod crypto;
pub mod pairing;

// Re-exported so shells never need to depend on core-db directly.
pub use core_db::{
    Backlink, Block, DocIndexEntry, Document, Embedding, Folder, PageInfo, SearchResult,
    SyncStats, TagInfo,
};
pub use core_network::{NetworkState, NetworkStatus, Peer, PeerMessage};

// UniFFI surface for native shells (Android Kotlin). See `ffi.rs`.
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[cfg(feature = "uniffi")]
pub mod ffi;

const DB_FILENAME: &str = "enclave.db";
const KEY_FILENAME: &str = "vault.key";

/// Process-wide core. One process = one vault, one network stack, one sync
/// loop. Native Android and the Tauri editor island live in the same process,
/// so they must share this instead of opening two connections.
static GLOBAL_CORE: OnceLock<Arc<EnclaveCore>> = OnceLock::new();

// ── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CoreError {
    /// No unlocked database in this process.
    VaultLocked,
    /// `init_vault` on an existing database.
    VaultAlreadyExists,
    /// Storage-layer failure (rusqlite / core-db message).
    Database(String),
    /// Filesystem failure.
    Io(String),
    /// P2P network failure.
    Network(String),
    /// Caller passed something invalid (or the state forbids the call).
    InvalidInput(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // These strings are the existing user-facing contract.
            CoreError::VaultLocked => write!(f, "Vault is locked"),
            CoreError::VaultAlreadyExists => write!(f, "Vault already exists"),
            CoreError::Database(msg) | CoreError::Io(msg) | CoreError::Network(msg)
            | CoreError::InvalidInput(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CoreError {}

pub type CoreResult<T> = Result<T, CoreError>;

fn db_err(e: impl fmt::Display) -> CoreError {
    CoreError::Database(e.to_string())
}

fn io_err(e: impl fmt::Display) -> CoreError {
    CoreError::Io(e.to_string())
}

fn poisoned() -> CoreError {
    CoreError::Database("state lock poisoned".to_string())
}

// ── Sync events ─────────────────────────────────────────────────────────────

/// Shell-visible sync notifications. The Tauri shell forwards these to the web
/// UI as `sync-done` / `peer-connect-failed`; the Android shell will surface
/// them in the sync screen and notification.
#[derive(Debug, Clone, serde::Serialize)]
pub enum SyncEvent {
    Done {
        peer: String,
        docs_changed: u64,
        blocks_changed: u64,
    },
    PeerFailed {
        host: String,
        error: String,
    },
}

// ── Home-screen widget projection ───────────────────────────────────────────
// The Android widgets render from this projection instead of walking
// ProseMirror JSON in Kotlin (issue #122): one Rust implementation, unit
// tested here, so a document-schema change cannot silently break a widget.

/// One checklist row a widget shows.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChecklistRow {
    pub text: String,
    pub checked: bool,
}

/// A note shared to widgets, projected to exactly what they render.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WidgetNote {
    pub id: String,
    pub title: String,
    pub labels: Vec<String>,
    pub updated_at: String,
    pub is_favorite: bool,
    /// Body text as the pinned widget renders it (paragraphs joined with
    /// blank lines, hard breaks as newlines).
    pub text: String,
    pub checklist: Vec<ChecklistRow>,
}

fn json_opt(raw: &Option<String>) -> Option<serde_json::Value> {
    raw.as_deref().and_then(|s| serde_json::from_str(s).ok())
}

/// Recursively append `text` leaves; hard breaks become newlines and task
/// items are separated by a space (mirrors the editor's own text walk).
fn collect_text(node: &serde_json::Value, out: &mut String) {
    match node.get("type").and_then(|t| t.as_str()) {
        Some("text") => out.push_str(node.get("text").and_then(|t| t.as_str()).unwrap_or("")),
        Some("hardBreak") => out.push('\n'),
        Some("taskItem") => out.push(' '),
        _ => {}
    }
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            if let Some(s) = child.as_str() {
                out.push_str(s);
            } else {
                collect_text(child, out);
            }
        }
    }
}

/// Body text: top-level nodes except task lists, blank-line separated.
fn doc_body_text(doc: &serde_json::Value) -> String {
    let Some(nodes) = doc.get("content").and_then(|c| c.as_array()) else {
        return String::new();
    };
    let mut paragraphs = Vec::new();
    for node in nodes {
        if node.get("type").and_then(|t| t.as_str()) == Some("taskList") {
            continue;
        }
        let mut text = String::new();
        collect_text(node, &mut text);
        let text = text.trim();
        if !text.is_empty() {
            paragraphs.push(text.to_string());
        }
    }
    paragraphs.join("\n\n")
}

/// Checklist rows from the first top-level task list, if any.
fn doc_checklist(doc: &serde_json::Value) -> Vec<ChecklistRow> {
    let items = doc
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|nodes| {
            nodes
                .iter()
                .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("taskList"))
        })
        .and_then(|list| list.get("content").and_then(|c| c.as_array()));
    let Some(items) = items else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            if item.get("type").and_then(|t| t.as_str()) != Some("taskItem") {
                return None;
            }
            let mut text = String::new();
            collect_text(item, &mut text);
            Some(ChecklistRow {
                text: text.trim().to_string(),
                checked: item
                    .get("attrs")
                    .and_then(|a| a.get("checked"))
                    .and_then(|c| c.as_bool())
                    .unwrap_or(false),
            })
        })
        .collect()
}

/// Labels from a tags block JSON (`{"tags":[...]}`).
fn tags_of(tags: Option<&serde_json::Value>) -> Vec<String> {
    tags.and_then(|t| t.get("tags"))
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Project raw widget sources: only documents whose `widget` block is
/// enabled, with malformed/missing content degrading to empty fields.
/// Pure so the projection is unit-tested without a vault.
pub fn project_widget_notes(sources: Vec<core_db::WidgetSource>) -> Vec<WidgetNote> {
    sources
        .into_iter()
        .filter_map(|source| {
            let widget = json_opt(&source.widget_json)?;
            if !widget
                .get("enabled")
                .and_then(|e| e.as_bool())
                .unwrap_or(false)
            {
                return None;
            }
            let doc = json_opt(&source.doc_json).unwrap_or_else(|| serde_json::json!({}));
            Some(WidgetNote {
                id: source.document.id,
                title: source.document.title,
                labels: tags_of(json_opt(&source.tags_json).as_ref()),
                updated_at: source.document.updated_at,
                is_favorite: source.document.is_favorite,
                text: doc_body_text(&doc),
                checklist: doc_checklist(&doc),
            })
        })
        .collect()
}

/// Flip `attrs.checked` on the task item at `index` (first task list).
/// Returns false when the document has no such item.
fn toggle_task_at(content: &mut serde_json::Value, index: usize) -> bool {
    let Some(item) = content
        .get_mut("content")
        .and_then(|c| c.as_array_mut())
        .and_then(|nodes| {
            nodes
                .iter_mut()
                .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("taskList"))
        })
        .and_then(|list| list.get_mut("content"))
        .and_then(|c| c.as_array_mut())
        .and_then(|items| items.get_mut(index))
    else {
        return false;
    };
    if item.get("type").and_then(|t| t.as_str()) != Some("taskItem") {
        return false;
    }
    let Some(obj) = item.as_object_mut() else {
        return false;
    };
    let attrs = obj.entry("attrs").or_insert_with(|| serde_json::json!({}));
    let Some(attrs) = attrs.as_object_mut() else {
        return false;
    };
    let checked = attrs
        .get("checked")
        .and_then(|c| c.as_bool())
        .unwrap_or(false);
    attrs.insert("checked".to_string(), serde_json::Value::Bool(!checked));
    true
}

// ── Core context ────────────────────────────────────────────────────────────

/// Owns the vault connection, the vault-derived sync key and the P2P network.
/// One instance per process; the shell decides where `app_dir` points.
pub struct EnclaveCore {
    app_dir: PathBuf,
    /// None when locked; Some when unlocked.
    db: Mutex<Option<Connection>>,
    /// Vault-derived sync PSK, present only while unlocked.
    sync_key: Mutex<Option<[u8; 32]>>,
    network: Arc<NetworkState>,
    /// Fan-out for sync events: shells subscribe and surface them (toasts,
    /// notifications) while the loop itself lives here.
    sync_events: tokio::sync::broadcast::Sender<SyncEvent>,
    /// Cancels the active one-time pairing listener, if any.
    pairing_cancel: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// Monotonic counter bumped by data-changing operations, including sync
    /// merges. Shells poll it to refresh derived state (home-screen widgets)
    /// without an event bridge or a JNI hook (ADR 0006).
    mutation_revision: std::sync::atomic::AtomicU64,
}

impl EnclaveCore {
    pub fn new(app_dir: impl Into<PathBuf>) -> Self {
        let (sync_events, _) = tokio::sync::broadcast::channel(128);
        Self {
            app_dir: app_dir.into(),
            db: Mutex::new(None),
            sync_key: Mutex::new(None),
            network: Arc::new(NetworkState::new()),
            sync_events,
            pairing_cancel: Mutex::new(None),
            mutation_revision: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Monotonic count of data-changing operations. A shell polls it and
    /// refreshes derived state when it moves — the sync loop's merges happen
    /// inside the core, so the shell cannot observe them any other way.
    pub fn revision(&self) -> u64 {
        self.mutation_revision
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Record a data change. Called from the mutation paths (and from sync
    /// merges) so `revision` moves exactly when stored data did.
    pub fn bump_revision(&self) {
        self.mutation_revision
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Subscribe to sync events (sync-done / peer-connect-failed).
    pub fn subscribe_sync_events(&self) -> tokio::sync::broadcast::Receiver<SyncEvent> {
        self.sync_events.subscribe()
    }

    // ── Account pairing (one-time code, desktop → phone) ─────────────────────

    /// Open a one-time pairing listener that hands `seed` to a phone which
    /// proves the code. The seed comes from the shell (the frontend already
    /// holds it from unlock) so the core never has to keep it at rest.
    pub async fn pairing_start(&self, seed: String) -> CoreResult<pairing::PairingStart> {
        if !self.is_unlocked() {
            return Err(CoreError::VaultLocked);
        }
        self.pairing_cancel();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let start = pairing::serve(seed, rx).await?;
        if let Ok(mut slot) = self.pairing_cancel.lock() {
            *slot = Some(tx);
        }
        Ok(start)
    }

    /// Close the active pairing listener (user dismissed it, or it was used).
    pub fn pairing_cancel(&self) {
        if let Ok(mut slot) = self.pairing_cancel.lock() {
            if let Some(tx) = slot.take() {
                let _ = tx.send(());
            }
        }
    }

    /// Phone side: prove the code and decrypt the seed phrase.
    pub async fn pairing_redeem(&self, code: String, address: String) -> CoreResult<String> {
        pairing::redeem(&code, &address).await
    }

    /// The process-wide core, created on first use. The first caller's
    /// `app_dir` wins — shells must resolve the same directory.
    pub fn global(app_dir: impl Into<PathBuf>) -> Arc<Self> {
        GLOBAL_CORE.get_or_init(|| Arc::new(Self::new(app_dir))).clone()
    }

    /// The process-wide core if anything created it yet.
    pub fn global_if_init() -> Option<Arc<Self>> {
        GLOBAL_CORE.get().cloned()
    }

    /// True while the vault connection is open (password/seed entered).
    pub fn is_unlocked(&self) -> bool {
        self.db.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    pub fn app_dir(&self) -> &Path {
        &self.app_dir
    }

    pub fn network(&self) -> Arc<NetworkState> {
        self.network.clone()
    }

    /// Run `f` against the unlocked connection, or fail with `VaultLocked`.
    fn with_db<T>(&self, f: impl FnOnce(&Connection) -> CoreResult<T>) -> CoreResult<T> {
        let guard = self.db.lock().map_err(|_| poisoned())?;
        match guard.as_ref() {
            Some(conn) => f(conn),
            None => Err(CoreError::VaultLocked),
        }
    }

    // ── Vault lifecycle ─────────────────────────────────────────────────────

    pub fn is_vault_initialized(&self) -> bool {
        core_db::vault_exists(&self.app_dir.join(DB_FILENAME))
    }

    pub fn init_vault(&self, key: &[u8]) -> CoreResult<()> {
        let path = self.app_dir.join(DB_FILENAME);
        if core_db::vault_exists(&path) {
            return Err(CoreError::VaultAlreadyExists);
        }
        let conn = core_db::init_vault(&path, key).map_err(CoreError::Database)?;
        *self.db.lock().map_err(|_| poisoned())? = Some(conn);
        *self.sync_key.lock().map_err(|_| poisoned())? =
            Some(core_network::crypto::derive_sync_key(key));
        Ok(())
    }

    pub fn unlock_vault(&self, key: &[u8]) -> CoreResult<()> {
        let path = self.app_dir.join(DB_FILENAME);
        let conn = core_db::open_vault(&path, key).map_err(CoreError::Database)?;
        *self.db.lock().map_err(|_| poisoned())? = Some(conn);
        *self.sync_key.lock().map_err(|_| poisoned())? =
            Some(core_network::crypto::derive_sync_key(key));
        Ok(())
    }

    /// Lock = stop sync (it holds the PSK), drop the key, drop the connection.
    pub async fn lock_vault(&self) -> CoreResult<()> {
        let _ = self.network.stop().await;
        *self.sync_key.lock().map_err(|_| poisoned())? = None;
        *self.db.lock().map_err(|_| poisoned())? = None;
        Ok(())
    }

    /// Delete vault + key file. Only safe when no user data exists (used when
    /// vault creation fails partway and would otherwise lock the user out).
    pub fn reset_vault(&self) -> CoreResult<()> {
        *self.db.lock().map_err(|_| poisoned())? = None;
        for f in [DB_FILENAME, KEY_FILENAME] {
            let _ = std::fs::remove_file(self.app_dir.join(f));
        }
        Ok(())
    }

    // ── Password / mnemonic flows (native shell parity with VaultGuard) ──────

    /// Create a vault protected by `password`; returns the recovery mnemonic
    /// (shown once). Mirrors the web flow: init the encrypted DB, then store
    /// the password-encrypted phrase in vault.key. A partial failure resets
    /// the half-created vault so the user cannot be locked out.
    pub fn create_vault(&self, password: &str) -> CoreResult<String> {
        let mnemonic = crypto::generate_mnemonic()?;
        let key = crypto::derive_master_key(&mnemonic)?;
        if let Err(e) = self.init_vault(&key) {
            let _ = self.reset_vault();
            return Err(e);
        }
        let blob = crypto::encrypt_with_password(&mnemonic, password)?;
        if let Err(e) = self.store_vault_key(&blob) {
            let _ = self.reset_vault();
            return Err(e);
        }
        Ok(mnemonic)
    }

    /// Unlock using the password stored in vault.key.
    pub fn unlock_with_password(&self, password: &str) -> CoreResult<()> {
        let blob = self.load_vault_key()?;
        let mnemonic = crypto::decrypt_with_password(&blob, password)?;
        if !crypto::validate_mnemonic(&mnemonic) {
            return Err(CoreError::InvalidInput(
                "Invalid vault key — recovery phrase may be needed".into(),
            ));
        }
        self.unlock_with_mnemonic(&mnemonic)
    }

    /// Unlock with the 12-word recovery phrase.
    pub fn unlock_with_mnemonic(&self, mnemonic: &str) -> CoreResult<()> {
        let normalized = mnemonic.trim().to_lowercase();
        if !crypto::validate_mnemonic(&normalized) {
            return Err(CoreError::InvalidInput(
                "Invalid seed phrase. Check each word and try again.".into(),
            ));
        }
        let key = crypto::derive_master_key(&normalized)?;
        self.unlock_vault(&key)
    }

    /// Store (or replace) the password that protects the recovery phrase.
    pub fn set_vault_password(&self, mnemonic: &str, password: &str) -> CoreResult<()> {
        let blob = crypto::encrypt_with_password(mnemonic, password)?;
        self.store_vault_key(&blob)
    }

    // ── Documents ───────────────────────────────────────────────────────────

    pub fn list_documents(&self) -> CoreResult<Vec<Document>> {
        self.with_db(|db| core_db::query_documents(db).map_err(db_err))
    }

    pub fn get_document(&self, id: &str) -> CoreResult<Document> {
        self.with_db(|db| core_db::query_document(db, id).map_err(db_err))
    }

    pub fn create_document(&self, title: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            let doc = Document {
                id: uuid::Uuid::new_v4().to_string(),
                title: title.to_string(),
                created_at: now.clone(),
                updated_at: now.clone(),
                is_favorite: false,
                is_archived: false,
                rev: 0,
                deleted_at: None,
                folder_id: None,
            };
            core_db::insert_document(db, &doc).map_err(db_err)?;

            let block = Block {
                id: uuid::Uuid::new_v4().to_string(),
                document_id: doc.id.clone(),
                block_type: "paragraph".into(),
                content: serde_json::json!({}),
                sort_order: 1.0,
                created_at: now.clone(),
                updated_at: now,
            };
            core_db::insert_block(db, &block).map_err(db_err)?;
            self.bump_revision();
            Ok(doc)
        })
    }

    pub fn delete_document(&self, id: &str) -> CoreResult<()> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::delete_document(db, id, &now).map_err(db_err)?;
            self.bump_revision();
            Ok(())
        })
    }

    pub fn archive_document(&self, id: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::archive_document(db, id, &now).map_err(db_err)?;
            self.bump_revision();
            core_db::query_document(db, id).map_err(db_err)
        })
    }

    pub fn restore_document(&self, id: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::restore_document(db, id, &now).map_err(db_err)?;
            self.bump_revision();
            core_db::query_document(db, id).map_err(db_err)
        })
    }

    pub fn list_archived_documents(&self) -> CoreResult<Vec<Document>> {
        self.with_db(|db| core_db::query_archived_documents(db).map_err(db_err))
    }

    pub fn update_document_title(&self, id: &str, title: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::update_document_title(db, id, title, &now).map_err(db_err)?;
            self.bump_revision();
            core_db::query_document(db, id).map_err(db_err)
        })
    }

    pub fn find_or_create_document(&self, title: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::find_or_create_document(db, title, &now).map_err(db_err)
        })
    }

    pub fn toggle_favorite(&self, id: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::toggle_document_favorite(db, id, &now).map_err(db_err)?;
            core_db::query_document(db, id).map_err(db_err)
        })
    }

    pub fn duplicate_document(&self, id: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::duplicate_document(db, id, &now).map_err(db_err)
        })
    }

    /// Move a page into a folder (`folder_id` None = root).
    pub fn move_document(&self, id: &str, folder_id: Option<&str>) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::move_document(db, id, folder_id, &now).map_err(db_err)?;
            core_db::query_document(db, id).map_err(db_err)
        })
    }

    // ── Folders ─────────────────────────────────────────────────────────────

    pub fn list_folders(&self) -> CoreResult<Vec<Folder>> {
        self.with_db(|db| core_db::query_folders(db).map_err(db_err))
    }

    pub fn create_folder(&self, name: &str) -> CoreResult<Folder> {
        self.with_db(|db| {
            let folder = Folder {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
            core_db::insert_folder(db, &folder).map_err(db_err)?;
            Ok(folder)
        })
    }

    pub fn rename_folder(&self, id: &str, name: &str) -> CoreResult<()> {
        self.with_db(|db| core_db::rename_folder(db, id, name).map_err(db_err))
    }

    /// Deleting a folder never deletes its pages — they fall back to the root.
    pub fn delete_folder(&self, id: &str) -> CoreResult<()> {
        self.with_db(|db| core_db::delete_folder(db, id).map_err(db_err))
    }

    // ── Blocks ──────────────────────────────────────────────────────────────

    pub fn get_blocks(&self, document_id: &str) -> CoreResult<Vec<Block>> {
        self.with_db(|db| core_db::query_blocks(db, document_id).map_err(db_err))
    }

    pub fn upsert_block(
        &self,
        id: &str,
        document_id: &str,
        block_type: &str,
        content: serde_json::Value,
        sort_order: f64,
    ) -> CoreResult<Block> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            let block = Block {
                id: id.to_string(),
                document_id: document_id.to_string(),
                block_type: block_type.to_string(),
                content,
                sort_order,
                created_at: now.clone(),
                updated_at: now,
            };
            let saved = core_db::upsert_block(db, &block).map_err(db_err)?;
            self.bump_revision();
            Ok(saved)
        })
    }

    pub fn delete_block(&self, id: &str) -> CoreResult<()> {
        self.with_db(|db| {
            core_db::delete_block(db, id).map_err(db_err)?;
            self.bump_revision();
            Ok(())
        })
    }

    // ── Home-screen widgets ─────────────────────────────────────────────────

    /// Notes the user shared to widgets, projected in one query (issue #122).
    /// The Android cache wraps this in its Keystore envelope; widgets render
    /// the same rows on every platform.
    pub fn widget_notes(&self) -> CoreResult<Vec<WidgetNote>> {
        self.with_db(|db| {
            let sources = core_db::query_widget_sources(db).map_err(db_err)?;
            Ok(project_widget_notes(sources))
        })
    }

    /// Tick/untick one checklist row from a widget. Writes the document's own
    /// `doc` block through the same upsert path the editor uses, so rev, FTS
    /// and sync react exactly as a normal save.
    pub fn toggle_checklist_item(&self, doc_id: &str, index: usize) -> CoreResult<()> {
        self.with_db(|db| {
            let blocks = core_db::query_blocks(db, doc_id).map_err(db_err)?;
            let doc_block = blocks
                .iter()
                .find(|b| b.block_type == "doc")
                .ok_or_else(|| CoreError::InvalidInput("This page has no checklist".into()))?;
            let mut content = doc_block.content.clone();
            if !toggle_task_at(&mut content, index) {
                return Err(CoreError::InvalidInput("Checklist item not found".into()));
            }
            let now = chrono::Utc::now().to_rfc3339();
            let block = Block {
                id: doc_block.id.clone(),
                document_id: doc_id.to_string(),
                block_type: "doc".to_string(),
                content,
                sort_order: doc_block.sort_order,
                created_at: now.clone(),
                updated_at: now,
            };
            core_db::upsert_block(db, &block).map_err(db_err)?;
            Ok(())
        })
    }

    // ── Search, tags, pages, backlinks ──────────────────────────────────────

    pub fn search_all(&self, query: &str) -> CoreResult<Vec<SearchResult>> {
        self.with_db(|db| core_db::search_all(db, query).map_err(db_err))
    }

    pub fn get_all_tags(&self) -> CoreResult<Vec<TagInfo>> {
        self.with_db(|db| core_db::query_all_tags(db).map_err(db_err))
    }

    pub fn get_page_list(&self) -> CoreResult<Vec<PageInfo>> {
        self.with_db(|db| core_db::query_all_page_titles(db).map_err(db_err))
    }

    pub fn get_backlinks(&self, title: &str) -> CoreResult<Vec<Backlink>> {
        self.with_db(|db| core_db::query_backlinks(db, title).map_err(db_err))
    }

    pub fn find_relation_backlinks(&self, doc_id: &str) -> CoreResult<Vec<Backlink>> {
        self.with_db(|db| core_db::find_relation_backlinks(db, doc_id).map_err(db_err))
    }

    // ── Vault-scoped settings (encrypted at rest) ───────────────────────────

    pub fn get_setting(&self, key: &str) -> CoreResult<Option<String>> {
        self.with_db(|db| core_db::get_setting(db, key).map_err(db_err))
    }

    pub fn set_setting(&self, key: &str, value: &str) -> CoreResult<()> {
        self.with_db(|db| core_db::set_setting(db, key, value).map_err(db_err))
    }

    // ── Embeddings (RAG storage; inference stays in the shell) ─────────────

    pub fn upsert_embedding(
        &self,
        block_id: &str,
        document_id: &str,
        text: &str,
        vector: &[f64],
    ) -> CoreResult<()> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::upsert_embedding(db, block_id, document_id, text, vector, &now).map_err(db_err)
        })
    }

    /// Ranked retrieval: ANN top-k via the in-DB vec0 index (exact-cosine
    /// re-ranked), with an exact-scan fallback for unknown dimensions.
    pub fn search_embeddings(&self, query: &[f64], limit: usize) -> CoreResult<Vec<Embedding>> {
        self.with_db(|db| core_db::query_embeddings_topk(db, query, limit).map_err(db_err))
    }

    // ── Files, attachments, backup ──────────────────────────────────────────

    /// Write arbitrary bytes (markdown text or PNG) into the exports dir.
    pub fn export_file(&self, filename: &str, data: &[u8]) -> CoreResult<String> {
        let exports_dir = self.app_dir.join("exports");
        std::fs::create_dir_all(&exports_dir).map_err(io_err)?;
        let path = exports_dir.join(sanitize_filename(filename));
        std::fs::write(&path, data).map_err(io_err)?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn read_file(&self, path: &str) -> CoreResult<String> {
        std::fs::read_to_string(path).map_err(|e| io_err(format!("Failed to read file: {e}")))
    }

    /// Write bytes to a user-chosen path (markdown vault export, PNG saves).
    pub fn write_file(&self, path: &str, data: &[u8]) -> CoreResult<()> {
        std::fs::write(path, data).map_err(|e| io_err(format!("Failed to write file: {e}")))
    }

    /// Writes an attachment under <app_data>/attachments/<document_id>/ and
    /// returns the absolute path (the web UI serves it via the asset protocol).
    pub fn save_attachment(
        &self,
        document_id: &str,
        filename: &str,
        data: &[u8],
    ) -> CoreResult<String> {
        let dir = self.app_dir.join("attachments").join(document_id);
        std::fs::create_dir_all(&dir).map_err(io_err)?;
        let base = sanitize_filename(filename);
        let mut path = dir.join(&base);
        let mut i = 1;
        while path.exists() {
            // ponytail: naive "name (2)" dedupe, fine for local usage
            let stem = base.rsplit_once('.').map(|(s, _)| s).unwrap_or(&base);
            let ext = base.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
            path = dir.join(format!("{stem} ({i}).{ext}"));
            i += 1;
        }
        std::fs::write(&path, data).map_err(io_err)?;
        Ok(path.to_string_lossy().into_owned())
    }

    /// Consistent encrypted snapshot via VACUUM INTO (SQLCipher/WAL-safe,
    /// unlike a raw file copy). Restore = replace enclave.db while closed.
    pub fn backup_vault(&self) -> CoreResult<String> {
        let exports_dir = self.app_dir.join("exports");
        std::fs::create_dir_all(&exports_dir).map_err(io_err)?;
        let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let dest = exports_dir.join(format!("enclave-backup-{stamp}.db"));
        let sql = format!("VACUUM INTO '{}'", dest.to_string_lossy().replace('\'', "''"));
        self.with_db(|db| db.execute(&sql, []).map(|_| ()).map_err(db_err))?;
        Ok(dest.to_string_lossy().to_string())
    }

    // ── Vault key file (encrypted seed phrase for password-based login) ─────

    pub fn store_vault_key(&self, key_data: &[u8]) -> CoreResult<()> {
        std::fs::write(self.app_dir.join(KEY_FILENAME), key_data).map_err(io_err)
    }

    pub fn load_vault_key(&self) -> CoreResult<Vec<u8>> {
        std::fs::read(self.app_dir.join(KEY_FILENAME))
            .map_err(|_| CoreError::InvalidInput("No password set".to_string()))
    }

    // ── P2P network ─────────────────────────────────────────────────────────

    pub async fn start_network(&self, name: Option<String>) -> CoreResult<()> {
        let name = name.unwrap_or_else(|| "Enclave".to_string());
        let key = self
            .sync_key
            .lock()
            .map_err(|_| poisoned())?
            .ok_or_else(|| {
                CoreError::InvalidInput("Vault is locked — unlock before enabling sync".to_string())
            })?;
        self.network.start(&name, key).await.map_err(CoreError::Network)
    }

    pub async fn stop_network(&self) -> CoreResult<()> {
        self.network.stop().await.map_err(CoreError::Network)
    }

    /// Manual peer connect for networks where mDNS discovery is blocked.
    pub async fn connect_peer(&self, host: &str, port: u16) -> CoreResult<()> {
        self.network.connect_peer(host, port).await.map_err(CoreError::Network)
    }

    pub async fn network_status(&self) -> NetworkStatus {
        self.network.status().await
    }

    /// Merge a peer snapshot (doc-level LWW). Used by the built-in message
    /// handler and by shells that drive the sync loop themselves.
    pub fn merge_snapshot(&self, docs: &[Document], blocks: &[Block]) -> CoreResult<SyncStats> {
        self.with_db(|db| {
            let stats = core_db::sync_merge(db, docs, blocks).map_err(db_err)?;
            // Merges happen inside the core, so this bump is what lets a shell
            // notice that widget-visible data changed.
            self.bump_revision();
            Ok(stats)
        })
    }

    // ── Sync protocol (LAN v3 — incremental) ────────────────────────────────

    /// Wire protocol: both sides send hello on connect; each answers with a
    /// digest of doc metadata (id, rev, updated_at, deleted_at). The digest
    /// receiver diffs it against its own index and pulls only newer/missing
    /// docs via need → partial snapshot → merge (doc-level LWW) → ack. A
    /// converged pair exchanges need([]) → snapshot([]) → ack, which also
    /// refreshes "last synced" without moving payload.
    /// Consume peer messages for the lifetime of the process and merge
    /// snapshots into the vault. Call once after unlock — the first caller
    /// wins the message receiver; events fan out to [`subscribe_sync_events`].
    ///
    /// Owning the loop in the core (not in a shell) is what lets the native
    /// Android app sync without the WebView editor island running.
    pub async fn run_sync_loop(self: Arc<Self>) {
        let rx = {
            let mut guard = self.network.message_rx.lock().await;
            guard.take()
        };
        let Some(mut rx) = rx else {
            return; // loop already running (or the network was never set up)
        };
        while let Some(msg) = rx.recv().await {
            let tx = self.sync_events.clone();
            self.handle_sync_message(msg, move |event| {
                let _ = tx.send(event);
            })
            .await;
        }
    }

    pub fn sync_digest(&self) -> Option<String> {
        let payload = self.with_db(|db| {
            let index = core_db::query_doc_index(db).map_err(db_err)?;
            Ok(serde_json::json!({ "kind": "digest", "docs": index }).to_string())
        });
        payload.ok()
    }

    pub fn sync_snapshot_for(&self, ids: &[String]) -> Option<String> {
        let payload = self.with_db(|db| {
            let (docs, blocks, folders) = core_db::query_sync_data_for(db, ids).map_err(db_err)?;
            Ok(serde_json::json!({
                "kind": "snapshot",
                "docs": docs,
                "blocks": blocks,
                "folders": folders,
            })
            .to_string())
        });
        payload.ok()
    }

    /// Handle one peer message; `emit` receives shell-facing events. The
    /// Android shell passes a callback that maps these onto UI/notification.
    pub async fn handle_sync_message(&self, msg: PeerMessage, emit: impl Fn(SyncEvent)) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&msg.payload) else {
            return;
        };
        match v["kind"].as_str() {
            Some("hello") => {
                let peer_id = v["peer_id"].as_str().unwrap_or(&msg.from_peer).to_string();
                if let Some(digest) = self.sync_digest() {
                    self.network.send_to(&peer_id, digest).await;
                    eprintln!("sync: hello from {peer_id} — digest sent");
                } else {
                    eprintln!("sync: hello from {peer_id} — digest unavailable (vault locked?)");
                }
            }
            Some("digest") => {
                let peer_id = v["peer_id"].as_str().unwrap_or(&msg.from_peer).to_string();
                // Strict parse (issue #57 lesson): a malformed digest is
                // dropped, never silently treated as "nothing changed".
                let Ok(remote_index) =
                    serde_json::from_value::<Vec<DocIndexEntry>>(v["docs"].clone())
                else {
                    eprintln!("sync: malformed digest from {peer_id} ignored");
                    return;
                };
                let want = self.with_db(|db| {
                    let local = core_db::query_doc_index(db).map_err(db_err)?;
                    Ok(core_db::diff_doc_index(&local, &remote_index))
                });
                match want {
                    Ok(want) => {
                        if !want.is_empty() {
                            eprintln!("sync: digest from {peer_id} — pulling {} doc(s)", want.len());
                        }
                        let need = serde_json::json!({ "kind": "need", "ids": want }).to_string();
                        self.network.send_to(&peer_id, need).await;
                    }
                    Err(e) => eprintln!("sync: digest from {peer_id} — local index failed: {e}"),
                }
            }
            Some("need") => {
                let peer_id = v["peer_id"].as_str().unwrap_or(&msg.from_peer).to_string();
                let Ok(ids) = serde_json::from_value::<Vec<String>>(v["ids"].clone()) else {
                    eprintln!("sync: malformed need from {peer_id} ignored");
                    return;
                };
                if let Some(snapshot) = self.sync_snapshot_for(&ids) {
                    self.network.send_to(&peer_id, snapshot).await;
                    eprintln!("sync: need from {peer_id} — sent {} doc(s)", ids.len());
                } else {
                    eprintln!("sync: need from {peer_id} — snapshot unavailable (vault locked?)");
                }
            }
            Some("snapshot") => {
                let peer_id = v["peer_id"].as_str().unwrap_or(&msg.from_peer).to_string();
                // Parse strictly: a malformed snapshot must NOT merge an empty
                // set and must NOT report a successful sync.
                let (docs, blocks) = match (
                    serde_json::from_value::<Vec<Document>>(v["docs"].clone()),
                    serde_json::from_value::<Vec<Block>>(v["blocks"].clone()),
                ) {
                    (Ok(d), Ok(b)) => (d, b),
                    (e1, e2) => {
                        eprintln!(
                            "sync: malformed snapshot from {peer_id} ignored (docs: {:?}, blocks: {:?})",
                            e1.err(),
                            e2.err()
                        );
                        return;
                    }
                };
                // Folders first so the docs that reference them land in place.
                // Missing `folders` (older peer) is fine — empty list.
                let folders = serde_json::from_value::<Vec<Folder>>(v["folders"].clone())
                    .unwrap_or_default();
                match self.with_db(|db| core_db::merge_folders(db, &folders).map_err(db_err)) {
                    Ok(added) if added > 0 => {
                        eprintln!("sync: folders from {peer_id} — {} new folder(s)", added)
                    }
                    Ok(_) => {}
                    Err(e) => eprintln!("sync: folder merge from {peer_id} failed: {e}"),
                }
                match self.merge_snapshot(&docs, &blocks) {
                    Ok(stats) => {
                        eprintln!(
                            "sync: merged snapshot from {peer_id} — {} docs / {} blocks changed (received {} docs, {} blocks)",
                            stats.docs_changed, stats.blocks_changed, docs.len(), blocks.len()
                        );
                        self.network.mark_synced().await;
                        let ack = serde_json::json!({
                            "kind": "ack",
                            "docs_changed": stats.docs_changed,
                            "blocks_changed": stats.blocks_changed,
                        })
                        .to_string();
                        self.network.send_to(&peer_id, ack).await;
                        emit(SyncEvent::Done {
                            peer: peer_id,
                            docs_changed: stats.docs_changed as u64,
                            blocks_changed: stats.blocks_changed as u64,
                        });
                    }
                    Err(CoreError::VaultLocked) => {
                        eprintln!("sync: snapshot from {peer_id} ignored — vault locked");
                    }
                    Err(e) => {
                        // Never silent: a merge failure used to vanish into the
                        // old `Err(_) => ignore` arm. Log it and toast it.
                        eprintln!("sync: snapshot merge from {peer_id} failed: {e}");
                        emit(SyncEvent::PeerFailed {
                            host: peer_id,
                            error: format!("merge failed: {e}"),
                        });
                    }
                }
            }
            Some("session_failed") => {
                // Transport-level failure worth surfacing (wrong-key peer,
                // dead handshake) — the UI listens for this and toasts it.
                emit(SyncEvent::PeerFailed {
                    host: v["host"].as_str().unwrap_or(&msg.from_peer).to_string(),
                    error: v["error"].as_str().unwrap_or("connection failed").to_string(),
                });
            }
            Some("ack") => {
                self.network.mark_synced().await;
                emit(SyncEvent::Done {
                    peer: v["peer_id"].as_str().unwrap_or(&msg.from_peer).to_string(),
                    docs_changed: v["docs_changed"].as_u64().unwrap_or(0),
                    blocks_changed: v["blocks_changed"].as_u64().unwrap_or(0),
                });
            }
            _ => {}
        }
    }
}

/// Keep unicode letters/digits (CJK, accents survive); only path separators
/// and control characters become underscores.
fn sanitize_filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = safe.trim();
    if trimmed.is_empty() {
        "untitled".into()
    } else {
        trimmed.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unlocked() -> (tempfile::TempDir, EnclaveCore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let core = EnclaveCore::new(dir.path());
        assert!(!core.is_vault_initialized());
        core.init_vault(&[7u8; 32]).expect("init vault");
        (dir, core)
    }

    #[test]
    fn locked_vault_rejects_data_access() {
        let dir = tempfile::tempdir().unwrap();
        let core = EnclaveCore::new(dir.path());
        assert!(matches!(core.list_documents(), Err(CoreError::VaultLocked)));
        assert!(matches!(core.get_blocks("x"), Err(CoreError::VaultLocked)));
    }

    #[test]
    fn vault_lifecycle() {
        let (_dir, core) = unlocked();
        assert!(core.is_vault_initialized());
        assert!(matches!(
            core.init_vault(&[1u8; 32]),
            Err(CoreError::VaultAlreadyExists)
        ));
        core.reset_vault().unwrap();
        assert!(!core.is_vault_initialized());
    }

    #[test]
    fn document_and_block_round_trip() {
        let (_dir, core) = unlocked();

        let doc = core.create_document("Hello").unwrap();
        assert_eq!(core.list_documents().unwrap().len(), 1);
        // create_document seeds one empty paragraph block.
        assert_eq!(core.get_blocks(&doc.id).unwrap().len(), 1);

        let block = core
            .upsert_block(
                "block-1",
                &doc.id,
                "doc",
                serde_json::json!({ "type": "doc", "content": [] }),
                0.0,
            )
            .unwrap();
        assert_eq!(block.block_type, "doc");
        assert_eq!(core.get_blocks(&doc.id).unwrap().len(), 2);

        core.update_document_title(&doc.id, "Renamed").unwrap();
        assert_eq!(core.get_document(&doc.id).unwrap().title, "Renamed");

        core.delete_block("block-1").unwrap();
        assert_eq!(core.get_blocks(&doc.id).unwrap().len(), 1);
    }

    #[test]
    fn folders_favorites_and_archive() {
        let (_dir, core) = unlocked();
        let doc = core.create_document("Doc").unwrap();
        let folder = core.create_folder("Projects").unwrap();
        assert_eq!(core.list_folders().unwrap().len(), 1);

        let moved = core.move_document(&doc.id, Some(&folder.id)).unwrap();
        assert_eq!(moved.folder_id.as_deref(), Some(folder.id.as_str()));
        core.rename_folder(&folder.id, "Work").unwrap();
        assert_eq!(core.list_folders().unwrap()[0].name, "Work");

        assert!(!core.get_document(&doc.id).unwrap().is_favorite);
        assert!(core.toggle_favorite(&doc.id).unwrap().is_favorite);

        core.archive_document(&doc.id).unwrap();
        assert_eq!(core.list_documents().unwrap().len(), 0);
        assert_eq!(core.list_archived_documents().unwrap().len(), 1);
        core.restore_document(&doc.id).unwrap();
        assert_eq!(core.list_archived_documents().unwrap().len(), 0);

        // Deleting a folder keeps its pages.
        core.delete_folder(&folder.id).unwrap();
        assert_eq!(core.get_document(&doc.id).unwrap().folder_id, None);
    }

    #[test]
    fn search_tags_and_meta() {
        let (_dir, core) = unlocked();
        let doc = core.create_document("Searchable").unwrap();
        core.set_setting("theme", "dark").unwrap();
        assert_eq!(core.get_setting("theme").unwrap().as_deref(), Some("dark"));

        core.upsert_block(
            "tags-1",
            &doc.id,
            "tags",
            serde_json::json!({ "tags": ["alpha", "beta"] }),
            2.0,
        )
        .unwrap();
        let tags = core.get_all_tags().unwrap();
        let row = tags.iter().find(|t| t.doc_id == doc.id).expect("tag row");
        assert!(row.tags.iter().any(|t| t == "alpha"));

        assert!(core
            .get_page_list()
            .unwrap()
            .iter()
            .any(|p| p.title == "Searchable"));
    }

    #[test]
    fn files_attachments_and_key_file() {
        let (dir, core) = unlocked();

        let export = core.export_file("note.md", b"# hi").unwrap();
        assert!(Path::new(&export).exists());
        assert_eq!(core.read_file(&export).unwrap(), "# hi");

        let attachment = core.save_attachment("doc-1", "photo.png", b"png").unwrap();
        assert!(Path::new(&attachment).exists());
        // Second write with the same name must not clobber the first.
        let attachment2 = core.save_attachment("doc-1", "photo.png", b"png2").unwrap();
        assert_ne!(attachment, attachment2);

        let backup = core.backup_vault().unwrap();
        assert!(Path::new(&backup).exists());

        core.store_vault_key(b"encrypted-seed").unwrap();
        assert_eq!(core.load_vault_key().unwrap(), b"encrypted-seed");
        assert!(dir.path().join("vault.key").exists());
    }

    #[test]
    fn sync_digest_diff_round_trip() {
        let (_dir, core) = unlocked();
        let doc = core.create_document("Synced").unwrap();

        let digest: serde_json::Value = serde_json::from_str(&core.sync_digest().unwrap()).unwrap();
        assert_eq!(digest["kind"], "digest");
        assert_eq!(digest["docs"][0]["id"], doc.id);

        let index: Vec<DocIndexEntry> =
            serde_json::from_value(digest["docs"].clone()).unwrap();
        assert!(core_db::diff_doc_index(&index, &index).is_empty());

        let snapshot: serde_json::Value =
            serde_json::from_str(&core.sync_snapshot_for(&[doc.id.clone()]).unwrap()).unwrap();
        assert_eq!(snapshot["kind"], "snapshot");
        assert_eq!(snapshot["docs"][0]["id"], doc.id);
        assert!(!snapshot["blocks"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn lock_stops_access() {
        let (_dir, core) = unlocked();
        let doc = core.create_document("Doc").unwrap();
        core.lock_vault().await.unwrap();
        assert!(matches!(core.get_document(&doc.id), Err(CoreError::VaultLocked)));
        // Unlocking again restores access (same key).
        core.unlock_vault(&[7u8; 32]).unwrap();
        assert_eq!(core.get_document(&doc.id).unwrap().title, "Doc");
    }

    #[tokio::test]
    async fn password_vault_flow() {
        let dir = tempfile::tempdir().unwrap();
        let core = EnclaveCore::new(dir.path());
        let mnemonic = core.create_vault("s3cret-password").unwrap();
        assert_eq!(mnemonic.split(' ').count(), 12);
        assert!(dir.path().join("enclave.db").exists());
        assert!(dir.path().join("vault.key").exists());

        let doc = core.create_document("Note").unwrap();
        core.lock_vault().await.unwrap();
        assert!(matches!(core.list_documents(), Err(CoreError::VaultLocked)));
        assert!(core.unlock_with_password("wrong").is_err());

        core.unlock_with_password("s3cret-password").unwrap();
        assert_eq!(core.get_document(&doc.id).unwrap().title, "Note");
    }

    #[tokio::test]
    async fn mnemonic_unlock_then_password_setup() {
        let dir = tempfile::tempdir().unwrap();
        let core = EnclaveCore::new(dir.path());
        let mnemonic = core.create_vault("first-password").unwrap();
        core.lock_vault().await.unwrap();

        // Seed-phrase unlock, then opt into a new password (VaultGuard flow).
        core.unlock_with_mnemonic(&mnemonic.to_uppercase()).unwrap();
        core.set_vault_password(&mnemonic, "second-password").unwrap();
        core.lock_vault().await.unwrap();

        assert!(core.unlock_with_password("first-password").is_err());
        core.unlock_with_password("second-password").unwrap();
        assert!(core.is_vault_initialized());
    }

    #[test]
    fn error_messages_match_existing_contract() {
        assert_eq!(CoreError::VaultLocked.to_string(), "Vault is locked");
        assert_eq!(CoreError::VaultAlreadyExists.to_string(), "Vault already exists");
        assert_eq!(
            CoreError::InvalidInput("No password set".into()).to_string(),
            "No password set"
        );
    }

    // ── Widget projection (#122) ────────────────────────────────────────────

    fn sample_doc_json() -> serde_json::Value {
        serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [
                    { "type": "text", "text": "Intro line" },
                    { "type": "hardBreak" },
                    { "type": "text", "text": "continued" }
                ]},
                { "type": "taskList", "content": [
                    { "type": "taskItem", "attrs": { "checked": false }, "content": [
                        { "type": "paragraph", "content": [ { "type": "text", "text": "milk" } ] }
                    ]},
                    { "type": "taskItem", "attrs": { "checked": true }, "content": [
                        { "type": "paragraph", "content": [ { "type": "text", "text": "bread" } ] }
                    ]}
                ]}
            ]
        })
    }

    fn widget_source(
        widget: Option<&str>,
        tags: Option<&str>,
        doc: Option<&str>,
    ) -> core_db::WidgetSource {
        core_db::WidgetSource {
            document: Document {
                id: "doc-1".into(),
                title: "Groceries".into(),
                created_at: "2026-01-01T00:00:00Z".into(),
                updated_at: "2026-01-02T00:00:00Z".into(),
                is_favorite: true,
                is_archived: false,
                rev: 1,
                deleted_at: None,
                folder_id: None,
            },
            widget_json: widget.map(str::to_string),
            tags_json: tags.map(str::to_string),
            doc_json: doc.map(str::to_string),
        }
    }

    #[test]
    fn widget_projection_extracts_body_checklist_and_labels() {
        let notes = project_widget_notes(vec![widget_source(
            Some(r#"{"enabled":true}"#),
            Some(r#"{"tags":["home","","errands"]}"#),
            Some(&sample_doc_json().to_string()),
        )]);
        assert_eq!(notes.len(), 1);
        let note = &notes[0];
        assert_eq!(note.text, "Intro line\ncontinued");
        assert_eq!(note.labels, vec!["home", "errands"]);
        assert_eq!(
            note.checklist,
            vec![
                ChecklistRow { text: "milk".into(), checked: false },
                ChecklistRow { text: "bread".into(), checked: true },
            ]
        );
        assert!(note.is_favorite);
    }

    #[test]
    fn widget_projection_skips_unshared_and_malformed_notes() {
        assert!(project_widget_notes(vec![widget_source(None, None, None)]).is_empty());
        assert!(
            project_widget_notes(vec![widget_source(Some(r#"{"enabled":false}"#), None, None)])
                .is_empty()
        );
        assert!(project_widget_notes(vec![widget_source(Some("junk"), None, None)]).is_empty());

        // Shared but empty document: still a note, with empty fields.
        let notes =
            project_widget_notes(vec![widget_source(Some(r#"{"enabled":true}"#), None, None)]);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].text.is_empty());
        assert!(notes[0].checklist.is_empty());
        assert!(notes[0].labels.is_empty());
    }

    #[test]
    fn widget_notes_and_toggle_work_through_a_vault() {
        let (_dir, core) = unlocked();
        let doc = core.create_document("Groceries").unwrap();
        core.upsert_block(&format!("{}-content", doc.id), &doc.id, "doc", sample_doc_json(), 0.0)
            .unwrap();
        core.upsert_block(
            &format!("{}-tags", doc.id),
            &doc.id,
            "tags",
            serde_json::json!({ "tags": ["home"] }),
            2.0,
        )
        .unwrap();
        core.upsert_block(
            &format!("{}-widget", doc.id),
            &doc.id,
            "widget",
            serde_json::json!({ "enabled": true }),
            4.0,
        )
        .unwrap();

        // A second, unshared note must not appear.
        core.create_document("Private").unwrap();

        let notes = core.widget_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Groceries");
        assert!(notes[0].checklist[1].checked);

        core.toggle_checklist_item(&doc.id, 1).unwrap();
        let notes = core.widget_notes().unwrap();
        assert!(!notes[0].checklist[1].checked);

        // Out-of-range and checklist-less documents are rejected, not ignored.
        assert!(matches!(
            core.toggle_checklist_item(&doc.id, 9),
            Err(CoreError::InvalidInput(_))
        ));
        let plain = core.create_document("No checklist").unwrap();
        assert!(matches!(
            core.toggle_checklist_item(&plain.id, 0),
            Err(CoreError::InvalidInput(_))
        ));
    }

    #[test]
    fn widget_notes_reject_a_locked_vault() {
        let dir = tempfile::tempdir().unwrap();
        let core = EnclaveCore::new(dir.path());
        assert!(matches!(core.widget_notes(), Err(CoreError::VaultLocked)));
        assert!(matches!(core.toggle_checklist_item("x", 0), Err(CoreError::VaultLocked)));
    }
}
