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
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

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
}

impl EnclaveCore {
    pub fn new(app_dir: impl Into<PathBuf>) -> Self {
        Self {
            app_dir: app_dir.into(),
            db: Mutex::new(None),
            sync_key: Mutex::new(None),
            network: Arc::new(NetworkState::new()),
        }
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
            Ok(doc)
        })
    }

    pub fn delete_document(&self, id: &str) -> CoreResult<()> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::delete_document(db, id, &now).map_err(db_err)
        })
    }

    pub fn archive_document(&self, id: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::archive_document(db, id, &now).map_err(db_err)?;
            core_db::query_document(db, id).map_err(db_err)
        })
    }

    pub fn restore_document(&self, id: &str) -> CoreResult<Document> {
        self.with_db(|db| {
            let now = chrono::Utc::now().to_rfc3339();
            core_db::restore_document(db, id, &now).map_err(db_err)?;
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
            core_db::upsert_block(db, &block).map_err(db_err)
        })
    }

    pub fn delete_block(&self, id: &str) -> CoreResult<()> {
        self.with_db(|db| core_db::delete_block(db, id).map_err(db_err))
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
        self.with_db(|db| core_db::sync_merge(db, docs, blocks).map_err(db_err))
    }

    // ── Sync protocol (LAN v3 — incremental) ────────────────────────────────

    /// Wire protocol: both sides send hello on connect; each answers with a
    /// digest of doc metadata (id, rev, updated_at, deleted_at). The digest
    /// receiver diffs it against its own index and pulls only newer/missing
    /// docs via need → partial snapshot → merge (doc-level LWW) → ack. A
    /// converged pair exchanges need([]) → snapshot([]) → ack, which also
    /// refreshes "last synced" without moving payload.
    pub fn sync_digest(&self) -> Option<String> {
        let payload = self.with_db(|db| {
            let index = core_db::query_doc_index(db).map_err(db_err)?;
            Ok(serde_json::json!({ "kind": "digest", "docs": index }).to_string())
        });
        payload.ok()
    }

    pub fn sync_snapshot_for(&self, ids: &[String]) -> Option<String> {
        let payload = self.with_db(|db| {
            let (docs, blocks) = core_db::query_sync_data_for(db, ids).map_err(db_err)?;
            Ok(serde_json::json!({ "kind": "snapshot", "docs": docs, "blocks": blocks }).to_string())
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
                if let Ok(want) = want {
                    let need = serde_json::json!({ "kind": "need", "ids": want }).to_string();
                    self.network.send_to(&peer_id, need).await;
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
                match self.merge_snapshot(&docs, &blocks) {
                    Ok(stats) => {
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
                    Err(_) => { /* vault locked — ignore */ }
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

    #[test]
    fn error_messages_match_existing_contract() {
        assert_eq!(CoreError::VaultLocked.to_string(), "Vault is locked");
        assert_eq!(CoreError::VaultAlreadyExists.to_string(), "Vault already exists");
        assert_eq!(
            CoreError::InvalidInput("No password set".into()).to_string(),
            "No password set"
        );
    }
}
