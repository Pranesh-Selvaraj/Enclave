//! UniFFI surface for native shells (Android Kotlin today).
//!
//! This module is the *frozen* wire API: its own record types (so core-db
//! internals can change without breaking the generated Kotlin) and a single
//! `EnclaveError` carrying the same message strings the desktop UI uses.
//! Block payloads cross as JSON strings — the same document format the web
//! editor already speaks, parsed in Kotlin with kotlinx.serialization.
//!
//! Build + generate bindings:
//!   cargo build -p core-api --features uniffi
//!   cargo run -p core-api --features uniffi,cli --bin uniffi-bindgen -- \
//!       generate --library target/debug/libcore_api.so \
//!       --language kotlin --out-dir <dir>

use std::sync::Arc;

use super::{
    Backlink as CoreBacklink, Block as CoreBlock, CoreError, Document as CoreDocument,
    Embedding as CoreEmbedding, EnclaveCore, Folder as CoreFolder, NetworkStatus as CoreNetworkStatus,
    PageInfo as CorePageInfo, Peer as CorePeer, SearchResult as CoreSearchResult, TagInfo as CoreTagInfo,
};

// ── Errors ──────────────────────────────────────────────────────────────────

/// One flat exception on the Kotlin side: `EnclaveException` with `message`
/// exactly as the desktop UI would display it.
#[derive(Debug, uniffi::Error)]
#[uniffi(flat_error)]
pub enum EnclaveError {
    Core { message: String },
}

impl std::fmt::Display for EnclaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnclaveError::Core { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for EnclaveError {}

impl From<CoreError> for EnclaveError {
    fn from(e: CoreError) -> Self {
        EnclaveError::Core { message: e.to_string() }
    }
}

// ── Records ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, uniffi::Record)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub rev: i64,
    pub deleted_at: Option<String>,
    pub folder_id: Option<String>,
}

impl From<CoreDocument> for Document {
    fn from(d: CoreDocument) -> Self {
        Self {
            id: d.id,
            title: d.title,
            created_at: d.created_at,
            updated_at: d.updated_at,
            is_favorite: d.is_favorite,
            is_archived: d.is_archived,
            rev: d.rev,
            deleted_at: d.deleted_at,
            folder_id: d.folder_id,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

impl From<CoreFolder> for Folder {
    fn from(f: CoreFolder) -> Self {
        Self { id: f.id, name: f.name, created_at: f.created_at }
    }
}

/// A block. `content_json` is the serialized block payload (TipTap JSON or
/// `{"tags":[...]}` / meta objects) — identical to what the web editor stores.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Block {
    pub id: String,
    pub document_id: String,
    pub block_type: String,
    pub content_json: String,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CoreBlock> for Block {
    fn from(b: CoreBlock) -> Self {
        Self {
            id: b.id,
            document_id: b.document_id,
            block_type: b.block_type,
            content_json: serde_json::to_string(&b.content).unwrap_or_else(|_| "{}".to_string()),
            sort_order: b.sort_order,
            created_at: b.created_at,
            updated_at: b.updated_at,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct TagInfo {
    pub doc_id: String,
    pub tags: Vec<String>,
}

impl From<CoreTagInfo> for TagInfo {
    fn from(t: CoreTagInfo) -> Self {
        Self { doc_id: t.doc_id, tags: t.tags }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct PageInfo {
    pub id: String,
    pub title: String,
}

impl From<CorePageInfo> for PageInfo {
    fn from(p: CorePageInfo) -> Self {
        Self { id: p.id, title: p.title }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct SearchResult {
    pub doc_id: String,
    pub doc_title: String,
    pub block_content: String,
    /// "title" | "content"
    pub result_type: String,
}

impl From<CoreSearchResult> for SearchResult {
    fn from(r: CoreSearchResult) -> Self {
        Self {
            doc_id: r.doc_id,
            doc_title: r.doc_title,
            block_content: r.block_content,
            result_type: r.r#type,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Backlink {
    pub doc_id: String,
    pub doc_title: String,
    pub block_content: String,
}

impl From<CoreBacklink> for Backlink {
    fn from(b: CoreBacklink) -> Self {
        Self { doc_id: b.doc_id, doc_title: b.doc_title, block_content: b.block_content }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Embedding {
    pub block_id: String,
    pub document_id: String,
    pub doc_title: String,
    pub text: String,
    pub vector: Vec<f64>,
    pub updated_at: String,
}

impl From<CoreEmbedding> for Embedding {
    fn from(e: CoreEmbedding) -> Self {
        Self {
            block_id: e.block_id,
            document_id: e.document_id,
            doc_title: e.doc_title,
            text: e.text,
            vector: e.vector,
            updated_at: e.updated_at,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Peer {
    pub id: String,
    pub host: String,
    pub hosts: Vec<String>,
    pub port: u16,
    pub connected: bool,
    pub name: String,
}

impl From<CorePeer> for Peer {
    fn from(p: CorePeer) -> Self {
        Self {
            id: p.id,
            host: p.host,
            hosts: p.hosts,
            port: p.port,
            connected: p.connected,
            name: p.name,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct NetworkStatus {
    pub local_peer_id: String,
    pub local_host: String,
    pub running: bool,
    pub port: u16,
    pub peers: Vec<Peer>,
    /// Epoch millis of the last successful sync; None = never synced.
    pub last_sync_at: Option<u64>,
}

impl From<CoreNetworkStatus> for NetworkStatus {
    fn from(s: CoreNetworkStatus) -> Self {
        Self {
            local_peer_id: s.local_peer_id,
            local_host: s.local_host,
            running: s.running,
            port: s.port,
            peers: s.peers.into_iter().map(Peer::from).collect(),
            last_sync_at: s.last_sync_at,
        }
    }
}

// ── Object ──────────────────────────────────────────────────────────────────

/// The Android/mobile handle to a vault. One per process; `appDir` is the
/// app's private data directory.
#[derive(uniffi::Object)]
pub struct FfiCore {
    core: Arc<EnclaveCore>,
}

#[uniffi::export(async_runtime = "tokio")]
impl FfiCore {
    #[uniffi::constructor]
    pub fn new(app_dir: String) -> Arc<Self> {
        // Shared with the Tauri editor island when both run in one process.
        Arc::new(Self { core: EnclaveCore::global(app_dir) })
    }

    /// Consume peer sync messages for the process lifetime (merges snapshots
    /// into the vault). Start once after unlocking; the call runs until the
    /// process exits, so drive it from a background coroutine.
    pub async fn run_sync_loop(&self) {
        self.core.clone().run_sync_loop().await
    }

    pub fn app_dir(&self) -> String {
        self.core.app_dir().to_string_lossy().into_owned()
    }

    // ── Vault lifecycle ─────────────────────────────────────────────────────

    pub fn is_vault_initialized(&self) -> bool {
        self.core.is_vault_initialized()
    }

    /// True while the vault is unlocked in this process. Widgets use it to
    /// decide between live toggles and opening the app.
    pub fn is_unlocked(&self) -> bool {
        self.core.is_unlocked()
    }

    pub fn init_vault(&self, key: Vec<u8>) -> Result<(), EnclaveError> {
        Ok(self.core.init_vault(&key)?)
    }

    pub fn unlock_vault(&self, key: Vec<u8>) -> Result<(), EnclaveError> {
        Ok(self.core.unlock_vault(&key)?)
    }

    pub async fn lock_vault(&self) -> Result<(), EnclaveError> {
        Ok(self.core.lock_vault().await?)
    }

    pub fn reset_vault(&self) -> Result<(), EnclaveError> {
        Ok(self.core.reset_vault()?)
    }

    /// Create a vault protected by `password`; returns the recovery mnemonic
    /// to show once. CPU-heavy (Argon2id) — call off the main thread.
    pub fn create_vault(&self, password: String) -> Result<String, EnclaveError> {
        Ok(self.core.create_vault(&password)?)
    }

    /// Unlock using the stored password. CPU-heavy — call off the main thread.
    pub fn unlock_with_password(&self, password: String) -> Result<(), EnclaveError> {
        Ok(self.core.unlock_with_password(&password)?)
    }

    /// Unlock with the 12-word recovery phrase. CPU-heavy — background it.
    pub fn unlock_with_mnemonic(&self, mnemonic: String) -> Result<(), EnclaveError> {
        Ok(self.core.unlock_with_mnemonic(&mnemonic)?)
    }

    /// After a mnemonic unlock, store a password for next time.
    pub fn set_vault_password(&self, mnemonic: String, password: String) -> Result<(), EnclaveError> {
        Ok(self.core.set_vault_password(&mnemonic, &password)?)
    }

    /// BIP39 validation for live form feedback.
    pub fn validate_mnemonic(&self, mnemonic: String) -> bool {
        super::crypto::validate_mnemonic(&mnemonic)
    }

    // ── Documents ───────────────────────────────────────────────────────────

    pub fn list_documents(&self) -> Result<Vec<Document>, EnclaveError> {
        Ok(self.core.list_documents()?.into_iter().map(Document::from).collect())
    }

    pub fn get_document(&self, id: String) -> Result<Document, EnclaveError> {
        Ok(self.core.get_document(&id)?.into())
    }

    pub fn create_document(&self, title: String) -> Result<Document, EnclaveError> {
        Ok(self.core.create_document(&title)?.into())
    }

    pub fn delete_document(&self, id: String) -> Result<(), EnclaveError> {
        Ok(self.core.delete_document(&id)?)
    }

    pub fn archive_document(&self, id: String) -> Result<Document, EnclaveError> {
        Ok(self.core.archive_document(&id)?.into())
    }

    pub fn restore_document(&self, id: String) -> Result<Document, EnclaveError> {
        Ok(self.core.restore_document(&id)?.into())
    }

    pub fn list_archived_documents(&self) -> Result<Vec<Document>, EnclaveError> {
        Ok(self.core.list_archived_documents()?.into_iter().map(Document::from).collect())
    }

    pub fn update_document_title(&self, id: String, title: String) -> Result<Document, EnclaveError> {
        Ok(self.core.update_document_title(&id, &title)?.into())
    }

    pub fn find_or_create_document(&self, title: String) -> Result<Document, EnclaveError> {
        Ok(self.core.find_or_create_document(&title)?.into())
    }

    pub fn toggle_favorite(&self, id: String) -> Result<Document, EnclaveError> {
        Ok(self.core.toggle_favorite(&id)?.into())
    }

    pub fn duplicate_document(&self, id: String) -> Result<Document, EnclaveError> {
        Ok(self.core.duplicate_document(&id)?.into())
    }

    pub fn move_document(&self, id: String, folder_id: Option<String>) -> Result<Document, EnclaveError> {
        Ok(self.core.move_document(&id, folder_id.as_deref())?.into())
    }

    // ── Folders ─────────────────────────────────────────────────────────────

    pub fn list_folders(&self) -> Result<Vec<Folder>, EnclaveError> {
        Ok(self.core.list_folders()?.into_iter().map(Folder::from).collect())
    }

    pub fn create_folder(&self, name: String) -> Result<Folder, EnclaveError> {
        Ok(self.core.create_folder(&name)?.into())
    }

    pub fn rename_folder(&self, id: String, name: String) -> Result<(), EnclaveError> {
        Ok(self.core.rename_folder(&id, &name)?)
    }

    pub fn delete_folder(&self, id: String) -> Result<(), EnclaveError> {
        Ok(self.core.delete_folder(&id)?)
    }

    // ── Blocks ──────────────────────────────────────────────────────────────

    pub fn get_blocks(&self, document_id: String) -> Result<Vec<Block>, EnclaveError> {
        Ok(self.core.get_blocks(&document_id)?.into_iter().map(Block::from).collect())
    }

    pub fn upsert_block(
        &self,
        id: String,
        document_id: String,
        block_type: String,
        content_json: String,
        sort_order: f64,
    ) -> Result<Block, EnclaveError> {
        let content = serde_json::from_str(&content_json).unwrap_or(serde_json::json!({}));
        Ok(self
            .core
            .upsert_block(&id, &document_id, &block_type, content, sort_order)?
            .into())
    }

    pub fn delete_block(&self, id: String) -> Result<(), EnclaveError> {
        Ok(self.core.delete_block(&id)?)
    }

    // ── Search / tags / pages / backlinks ───────────────────────────────────

    pub fn search_all(&self, query: String) -> Result<Vec<SearchResult>, EnclaveError> {
        Ok(self.core.search_all(&query)?.into_iter().map(SearchResult::from).collect())
    }

    pub fn get_all_tags(&self) -> Result<Vec<TagInfo>, EnclaveError> {
        Ok(self.core.get_all_tags()?.into_iter().map(TagInfo::from).collect())
    }

    pub fn get_page_list(&self) -> Result<Vec<PageInfo>, EnclaveError> {
        Ok(self.core.get_page_list()?.into_iter().map(PageInfo::from).collect())
    }

    pub fn get_backlinks(&self, title: String) -> Result<Vec<Backlink>, EnclaveError> {
        Ok(self.core.get_backlinks(&title)?.into_iter().map(Backlink::from).collect())
    }

    pub fn find_relation_backlinks(&self, doc_id: String) -> Result<Vec<Backlink>, EnclaveError> {
        Ok(self.core.find_relation_backlinks(&doc_id)?.into_iter().map(Backlink::from).collect())
    }

    // ── Settings ────────────────────────────────────────────────────────────

    pub fn get_setting(&self, key: String) -> Result<Option<String>, EnclaveError> {
        Ok(self.core.get_setting(&key)?)
    }

    pub fn set_setting(&self, key: String, value: String) -> Result<(), EnclaveError> {
        Ok(self.core.set_setting(&key, &value)?)
    }

    // ── Embeddings storage (inference stays shell-side) ─────────────────────

    pub fn upsert_embedding(
        &self,
        block_id: String,
        document_id: String,
        text: String,
        vector: Vec<f64>,
    ) -> Result<(), EnclaveError> {
        Ok(self.core.upsert_embedding(&block_id, &document_id, &text, &vector)?)
    }

    pub fn search_embeddings(&self, query: Vec<f64>, limit: u32) -> Result<Vec<Embedding>, EnclaveError> {
        Ok(self.core.search_embeddings(&query, limit as usize)?.into_iter().map(Embedding::from).collect())
    }

    // ── Network ─────────────────────────────────────────────────────────────

    pub async fn start_network(&self, name: Option<String>) -> Result<(), EnclaveError> {
        Ok(self.core.start_network(name).await?)
    }

    pub async fn stop_network(&self) -> Result<(), EnclaveError> {
        Ok(self.core.stop_network().await?)
    }

    pub async fn connect_peer(&self, host: String, port: u16) -> Result<(), EnclaveError> {
        Ok(self.core.connect_peer(&host, port).await?)
    }

    pub async fn network_status(&self) -> Result<NetworkStatus, EnclaveError> {
        Ok(self.core.network_status().await.into())
    }

    // ── Sync protocol payloads (for a native sync loop) ─────────────────────

    pub fn sync_digest(&self) -> Option<String> {
        self.core.sync_digest()
    }

    pub fn sync_snapshot_for(&self, ids: Vec<String>) -> Option<String> {
        self.core.sync_snapshot_for(&ids)
    }

    pub fn merge_snapshot(&self, docs_json: String, blocks_json: String) -> Result<SyncStats, EnclaveError> {
        let docs: Vec<super::Document> = serde_json::from_str(&docs_json)
            .map_err(|e| EnclaveError::Core { message: format!("invalid docs: {e}") })?;
        let blocks: Vec<super::Block> = serde_json::from_str(&blocks_json)
            .map_err(|e| EnclaveError::Core { message: format!("invalid blocks: {e}") })?;
        let stats = self.core.merge_snapshot(&docs, &blocks)?;
        Ok(SyncStats {
            docs_changed: stats.docs_changed as u64,
            blocks_changed: stats.blocks_changed as u64,
        })
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct SyncStats {
    pub docs_changed: u64,
    pub blocks_changed: u64,
}
