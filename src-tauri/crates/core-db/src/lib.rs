//! Encrypted SQLite storage engine for Enclave.
//!
//! Provides Document and Block types, query helpers, and database
//! initialization with SQLCipher encryption. The encryption key is
//! passed in at vault creation / unlock time (never hardcoded).

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_favorite: bool,
    pub is_archived: bool,
    /// Monotonic per-document edit counter — the LWW clock for LAN sync.
    #[serde(default)]
    pub rev: i64,
    /// Tombstone for permanent deletes; NULL while the doc is alive.
    #[serde(default)]
    pub deleted_at: Option<String>,
    /// Parent folder id; NULL = root (no folder).
    #[serde(default)]
    pub folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub document_id: String,
    #[serde(rename = "type")]
    pub block_type: String,
    pub content: serde_json::Value,
    pub sort_order: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// A stored embedding vector for a block (RAG). The vector is JSON-serialized
/// into the `embeddings.vector` column; it is plaintext-derived at runtime and
/// lives in the same encrypted (sqlcipher) DB as everything else.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub block_id: String,
    pub document_id: String,
    pub doc_title: String,
    /// The plain text that was embedded, so retrieval can inject it verbatim.
    pub text: String,
    pub vector: Vec<f64>,
    pub updated_at: String,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn set_cipher_pragmas(conn: &Connection, key: &[u8]) -> rusqlite::Result<()> {
    let hex_key = bytes_to_hex(key);
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{hex_key}'\";
         PRAGMA cipher_page_size = 4096;
         PRAGMA cipher_hmac_algorithm = HMAC_SHA512;",
    ))
}

fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS documents (
            id           TEXT PRIMARY KEY,
            title        TEXT NOT NULL DEFAULT '',
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            is_favorite  INTEGER NOT NULL DEFAULT 0,
            is_archived  INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS blocks (
            id           TEXT PRIMARY KEY,
            document_id  TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            type         TEXT NOT NULL DEFAULT 'paragraph',
            content      TEXT NOT NULL DEFAULT '{}',
            sort_order   REAL NOT NULL,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_blocks_document
         ON blocks(document_id, sort_order);
        CREATE TABLE IF NOT EXISTS folders (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL,
            created_at   TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS embeddings (
            block_id    TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            text        TEXT NOT NULL,
            vector      TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_embeddings_document
         ON embeddings(document_id);
        CREATE VIRTUAL TABLE IF NOT EXISTS blocks_fts USING fts5(
            doc_id UNINDEXED,
            block_id UNINDEXED,
            title,
            content
        );
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )?;
    // Migration for pre-sync vaults: CREATE TABLE IF NOT EXISTS won't add
    // columns to an existing database.
    ensure_column(conn, "documents", "rev", "ALTER TABLE documents ADD COLUMN rev INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "documents", "deleted_at", "ALTER TABLE documents ADD COLUMN deleted_at TEXT")?;
    ensure_column(conn, "documents", "folder_id", "ALTER TABLE documents ADD COLUMN folder_id TEXT")?;
    Ok(())
}

fn ensure_column(db: &Connection, table: &str, col: &str, ddl: &str) -> rusqlite::Result<()> {
    let mut stmt = db.prepare(&format!("PRAGMA table_info({table})"))?;
    let names: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    if !names.iter().any(|n| n == col) {
        db.execute_batch(ddl)?;
    }
    Ok(())
}

/// Pragmas that make the vault fast on modern hardware without weakening
/// SQLCipher encryption (WAL is encrypted in SQLCipher 4+).
/// ponytail: single connection behind a Mutex, so no busy_timeout pressure
/// today; it only costs one line to be ready for the async sync engine.
fn set_perf_pragmas(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA busy_timeout = 5000;",
    )
}

/// Plain-text extract of a doc block's JSON so the FTS index holds prose,
/// not JSON keys or node-type names.
fn extract_text(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::String(s) => out.push_str(s),
        serde_json::Value::Array(arr) => {
            for v in arr {
                extract_text(v, out)
            }
        }
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                if k == "type" || k == "attrs" {
                    continue;
                }
                extract_text(v, out)
            }
        }
        _ => {}
    }
}

/// Build an FTS5 MATCH query: every token prefix-matched, AND-joined.
/// Quotes inside the user input are escaped per the FTS5 string grammar.
fn fts_match_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

// ── FTS sync (kept manual — 5 mutation sites, zero trigger cleverness) ──────

fn fts_remove_doc(db: &Connection, doc_id: &str) -> rusqlite::Result<()> {
    db.execute("DELETE FROM blocks_fts WHERE doc_id = ?1", rusqlite::params![doc_id])?;
    Ok(())
}

fn fts_remove_block(db: &Connection, block_id: &str) -> rusqlite::Result<()> {
    db.execute("DELETE FROM blocks_fts WHERE block_id = ?1", rusqlite::params![block_id])?;
    Ok(())
}

fn fts_index_doc_title(db: &Connection, doc_id: &str, title: &str) -> rusqlite::Result<()> {
    fts_remove_block(db, &format!("t:{doc_id}"))?;
    db.execute(
        "INSERT INTO blocks_fts (doc_id, block_id, title, content)
         VALUES (?1, ?2, ?3, '')",
        rusqlite::params![doc_id, format!("t:{doc_id}"), title],
    )?;
    Ok(())
}

fn fts_index_block(db: &Connection, block: &Block) -> rusqlite::Result<()> {
    if block.block_type != "doc" {
        return Ok(()); // only prose blocks are searchable
    }
    let mut text = String::new();
    extract_text(&block.content, &mut text);
    let title: String = db
        .query_row(
            "SELECT title FROM documents WHERE id = ?1",
            rusqlite::params![block.document_id],
            |r| r.get(0),
        )
        .unwrap_or_default();
    fts_remove_block(db, &block.id)?;
    db.execute(
        "INSERT INTO blocks_fts (doc_id, block_id, title, content)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![block.document_id, block.id, title, text],
    )?;
    Ok(())
}

// ── Embeddings (RAG) — manual sync, mirrors FTS ─────────────────────────────
// Stale vectors are cleaned at the same mutation sites FTS touches; a doc
// never has an embedding whose text disagrees with its current content for
// long, and retrieval falls back to FTS for anything missing.

// ── Vector index (sqlite-vec) ───────────────────────────────────────────────
// vec0 virtual tables live INSIDE the encrypted SQLCipher file — the same
// encryption boundary as the embeddings rows. One table per vector dimension
// (models vary: MiniLM=384, nomic-embed=768, text-embedding-3-small=1536, …),
// created lazily on first upsert. Queries use the table matching the query
// vector's dimension and fall back to an exact scan when none exists
// (e.g. vaults created before the index, or a dimension never seen before).

static VEC_EXT: std::sync::Once = std::sync::Once::new();

/// Register the sqlite-vec extension for every future connection. Must run
/// before the first Connection is opened; safe to call repeatedly.
pub fn ensure_vec_extension() {
    VEC_EXT.call_once(|| {
        // The crate declares the init fn as `fn()`; its real ABI is the
        // standard sqlite extension entry point, so transmute the same way
        // the crate's own tests do.
        unsafe {
            rusqlite::auto_extension::register_auto_extension(std::mem::transmute::<
                *const (),
                rusqlite::auto_extension::RawAutoExtension,
            >(sqlite_vec::sqlite3_vec_init as *const ()))
            .expect("failed to register sqlite-vec auto-extension");
        }
    });
}

fn vec_table(dim: usize) -> String {
    format!("embeddings_vec_{dim}")
}

fn ensure_vec_table(db: &Connection, dim: usize) -> rusqlite::Result<()> {
    db.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS \"{}\" USING vec0(
             block_id TEXT PRIMARY KEY,
             vector float[{}]
         )",
        vec_table(dim),
        dim
    ))
}

/// Names of existing vec0 tables (any dimension) — shadows tables carry a
/// `_chunks`/`_rowids`/… suffix and are filtered out by the digit check.
fn vec_table_names(db: &Connection) -> rusqlite::Result<Vec<String>> {
    let mut stmt = db.prepare(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE 'embeddings_vec_%'",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for name in rows {
        let name = name?;
        let Some(dim) = name.strip_prefix("embeddings_vec_") else {
            continue;
        };
        if !dim.is_empty() && dim.chars().all(|c| c.is_ascii_digit()) {
            out.push(name);
        }
    }
    Ok(out)
}

/// Write one vector into the ANN index (upsert by block_id).
fn vec_upsert(db: &Connection, block_id: &str, vector: &[f64]) -> rusqlite::Result<()> {
    if vector.is_empty() {
        return Ok(());
    }
    ensure_vec_table(db, vector.len())?;
    let table = vec_table(vector.len());
    let json = serde_json::to_string(vector).unwrap_or_default();
    db.execute(
        &format!("DELETE FROM \"{table}\" WHERE block_id = ?1"),
        rusqlite::params![block_id],
    )?;
    db.execute(
        &format!("INSERT INTO \"{table}\" (block_id, vector) VALUES (?1, ?2)"),
        rusqlite::params![block_id, json],
    )?;
    Ok(())
}

/// Remove one block's vectors from every dimension's index.
fn vec_remove_block(db: &Connection, block_id: &str) -> rusqlite::Result<()> {
    for table in vec_table_names(db)? {
        db.execute(
            &format!("DELETE FROM \"{table}\" WHERE block_id = ?1"),
            rusqlite::params![block_id],
        )?;
    }
    Ok(())
}

/// Remove every vector belonging to a document from every dimension's index.
fn vec_remove_doc(db: &Connection, doc_id: &str) -> rusqlite::Result<()> {
    let tables = vec_table_names(db)?;
    if tables.is_empty() {
        return Ok(());
    }
    let mut stmt = db.prepare("SELECT block_id FROM embeddings WHERE document_id = ?1")?;
    let ids: Vec<String> = stmt
        .query_map(rusqlite::params![doc_id], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for table in &tables {
        for id in &ids {
            db.execute(
                &format!("DELETE FROM \"{table}\" WHERE block_id = ?1"),
                rusqlite::params![id],
            )?;
        }
    }
    Ok(())
}

/// Cosine similarity; 0 for empty or dimension-mismatched vectors (the same
/// contract the frontend used before ranking moved into Rust).
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// Nearest-neighbour retrieval: ANN candidates from the vec0 index (with a
/// 4× recall buffer) re-ranked by exact cosine, or a full exact scan when no
/// index exists for the query dimension. Archived/deleted docs are excluded.
pub fn query_embeddings_topk(
    db: &Connection,
    query: &[f64],
    limit: usize,
) -> rusqlite::Result<Vec<Embedding>> {
    if query.is_empty() || limit == 0 {
        return Ok(vec![]);
    }
    let indexed = vec_table_names(db)?.iter().any(|t| *t == vec_table(query.len()));
    let mut scored: Vec<(f64, Embedding)> = if indexed {
        let want = (limit.saturating_mul(4)).max(limit) as i64;
        let table = vec_table(query.len());
        let json = serde_json::to_string(query).unwrap_or_default();
        let mut stmt = db.prepare(&format!(
            "SELECT block_id FROM \"{table}\" WHERE vector MATCH ?1 AND k = ?2"
        ))?;
        let ids: Vec<String> = stmt
            .query_map(rusqlite::params![json, want], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders = vec!["?"; ids.len()].join(",");
        let mut stmt = db.prepare(&format!(
            "SELECT e.block_id, e.document_id, d.title, e.text, e.vector, e.updated_at
             FROM embeddings e JOIN documents d ON d.id = e.document_id
             WHERE e.block_id IN ({placeholders})
               AND d.is_archived = 0 AND d.deleted_at IS NULL"
        ))?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(ids.iter()), |row| {
                let vector_json: String = row.get(4)?;
                Ok(Embedding {
                    block_id: row.get(0)?,
                    document_id: row.get(1)?,
                    doc_title: row.get(2)?,
                    text: row.get(3)?,
                    vector: serde_json::from_str(&vector_json).unwrap_or_default(),
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(|e| (cosine_similarity(query, &e.vector), e))
            .collect()
    } else {
        query_embeddings(db)?
            .into_iter()
            .map(|e| (cosine_similarity(query, &e.vector), e))
            .collect()
    };
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(scored.into_iter().take(limit).map(|(_, e)| e).collect())
}

pub fn upsert_embedding(
    db: &Connection,
    block_id: &str,
    document_id: &str,
    text: &str,
    vector: &[f64],
    updated_at: &str,
) -> rusqlite::Result<()> {
    let vector_json = serde_json::to_string(vector).unwrap_or_default();
    db.execute(
        "INSERT INTO embeddings (block_id, document_id, text, vector, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(block_id) DO UPDATE SET
             document_id = excluded.document_id,
             text = excluded.text,
             vector = excluded.vector,
             updated_at = excluded.updated_at",
        rusqlite::params![block_id, document_id, text, vector_json, updated_at],
    )?;
    vec_upsert(db, block_id, vector)
}

/// All stored embeddings joined with their (live, non-archived) doc titles.
/// The frontend fetches the whole set and does cosine top-k client-side.
// ponytail: no vector index — a personal vault is a few hundred vectors max;
// add a sqlite-vec or similar index if the vault grows beyond that.
pub fn query_embeddings(db: &Connection) -> rusqlite::Result<Vec<Embedding>> {
    let mut stmt = db.prepare(
        "SELECT e.block_id, e.document_id, d.title, e.text, e.vector, e.updated_at
         FROM embeddings e
         JOIN documents d ON d.id = e.document_id
         WHERE d.is_archived = 0 AND d.deleted_at IS NULL
         ORDER BY e.updated_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        let vector_json: String = row.get(4)?;
        let vector = serde_json::from_str(&vector_json).unwrap_or_default();
        Ok(Embedding {
            block_id: row.get(0)?,
            document_id: row.get(1)?,
            doc_title: row.get(2)?,
            text: row.get(3)?,
            vector,
            updated_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

// ── Settings (vault-scoped KV) ──────────────────────────────────────────────
// AI settings (endpoint, model, API key) live here — encrypted at rest by
// SQLCipher like everything else, instead of localStorage plaintext.

pub fn get_setting(db: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    db.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        rusqlite::params![key],
        |r| r.get(0),
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
}

pub fn set_setting(db: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

// ── Vault Lifecycle ─────────────────────────────────────────────────────────

/// Check whether a vault database file already exists on disk.
pub fn vault_exists(db_path: &std::path::Path) -> bool {
    db_path.exists()
}

/// Create a new vault: open the database, set the key, create tables.
/// Returns the open connection.
pub fn init_vault(db_path: &std::path::Path, key: &[u8]) -> Result<Connection, String> {
    ensure_vec_extension();
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to create database: {e}"))?;
    set_cipher_pragmas(&conn, key).map_err(|e| format!("Failed to set encryption key: {e}"))?;
    create_tables(&conn).map_err(|e| format!("Failed to create tables: {e}"))?;
    set_perf_pragmas(&conn).map_err(|e| format!("Failed to set perf pragmas: {e}"))?;
    Ok(conn)
}

/// Open an existing vault: open the database, set the key, ensure tables exist.
/// Returns the open connection or an error if the key is wrong.
pub fn open_vault(db_path: &std::path::Path, key: &[u8]) -> Result<Connection, String> {
    ensure_vec_extension();
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open database: {e}"))?;
    set_cipher_pragmas(&conn, key).map_err(|e| format!("Failed to set encryption key: {e}"))?;

    // Verify the key by reading the schema — wrong key produces a corrupted view
    conn.query_row("SELECT COUNT(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|_| "Invalid vault key".to_string())?;

    // Ensure tables exist (idempotent — safe to call on every unlock)
    create_tables(&conn).map_err(|e| format!("Failed to create tables: {e}"))?;
    set_perf_pragmas(&conn).map_err(|e| format!("Failed to set perf pragmas: {e}"))?;

    Ok(conn)
}

// ── Document Queries ────────────────────────────────────────────────────────

/// Monotonic per-document revision bump — the LWW clock used by LAN sync.
/// updated_at is refreshed together with rev so sync can compare both.
fn bump_rev(db: &Connection, id: &str, now: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET rev = rev + 1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, id],
    )
    .map(|_| ())
}

fn row_to_document(row: &rusqlite::Row) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        title: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        is_favorite: row.get::<_, i32>(4)? != 0,
        is_archived: row.get::<_, i32>(5)? != 0,
        rev: row.get(6)?,
        deleted_at: row.get(7)?,
        folder_id: row.get(8)?,
    })
}

const DOC_COLS: &str =
    "SELECT id, title, created_at, updated_at, is_favorite, is_archived, rev, deleted_at, folder_id FROM documents";

pub fn query_documents(db: &Connection) -> rusqlite::Result<Vec<Document>> {
    let mut stmt = db.prepare(&format!(
        "{DOC_COLS} WHERE is_archived = 0 AND deleted_at IS NULL ORDER BY updated_at DESC"
    ))?;
    let rows = stmt.query_map([], row_to_document)?;
    rows.collect()
}

pub fn query_document(db: &Connection, id: &str) -> rusqlite::Result<Document> {
    db.query_row(
        &format!("{DOC_COLS} WHERE id = ?1"),
        rusqlite::params![id],
        row_to_document,
    )
}

pub fn insert_document(db: &Connection, doc: &Document) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO documents (id, title, created_at, updated_at, is_favorite, is_archived, folder_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            doc.id, doc.title, doc.created_at, doc.updated_at,
            doc.is_favorite as i32, doc.is_archived as i32, doc.folder_id
        ],
    )?;
    bump_rev(db, &doc.id, &doc.updated_at)?;
    fts_index_doc_title(db, &doc.id, &doc.title)
}

// ── Folders ──────────────────────────────────────────────────────────────────
// Folders are vault-local organization (not synced): a peer receiving a doc
// whose folder_id it doesn't know simply shows it at the root.

pub fn query_folders(db: &Connection) -> rusqlite::Result<Vec<Folder>> {
    let mut stmt = db.prepare("SELECT id, name, created_at FROM folders ORDER BY name COLLATE NOCASE")?;
    let rows = stmt.query_map([], |r| {
        Ok(Folder { id: r.get(0)?, name: r.get(1)?, created_at: r.get(2)? })
    })?;
    rows.collect()
}

pub fn insert_folder(db: &Connection, folder: &Folder) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO folders (id, name, created_at) VALUES (?1, ?2, ?3)",
        rusqlite::params![folder.id, folder.name, folder.created_at],
    )
    .map(|_| ())
}

pub fn rename_folder(db: &Connection, id: &str, name: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE folders SET name = ?1 WHERE id = ?2",
        rusqlite::params![name, id],
    )
    .map(|_| ())
}

/// Delete a folder: its pages fall back to the root (folder_id cleared) —
/// deleting a folder must never delete pages.
pub fn delete_folder(db: &Connection, id: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET folder_id = NULL, rev = rev + 1 WHERE folder_id = ?1",
        rusqlite::params![id],
    )?;
    db.execute("DELETE FROM folders WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

/// Move a page into a folder (None = root). Bumps rev so the move syncs.
pub fn move_document(
    db: &Connection,
    id: &str,
    folder_id: Option<&str>,
    updated_at: &str,
) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET folder_id = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![folder_id, updated_at, id],
    )
    .map(|_| ())
}

pub fn update_document_title(db: &Connection, id: &str, title: &str, updated_at: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET title = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![title, updated_at, id],
    )?;
    bump_rev(db, id, updated_at)?;
    db.execute(
        "UPDATE blocks_fts SET title = ?1 WHERE doc_id = ?2",
        rusqlite::params![title, id],
    )?;
    Ok(())
}

/// Permanent delete becomes a tombstone (deleted_at) so sync peers can
/// converge instead of resurrecting the doc. Blocks are removed immediately.
pub fn delete_document(db: &Connection, id: &str, now: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET deleted_at = ?1, rev = rev + 1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![now, now, id],
    )?;
    db.execute("DELETE FROM blocks WHERE document_id = ?1", rusqlite::params![id])?;
    db.execute("DELETE FROM embeddings WHERE document_id = ?1", rusqlite::params![id])?;
    vec_remove_doc(db, id)?;
    fts_remove_doc(db, id)
}

// ── Block Queries ───────────────────────────────────────────────────────────

pub fn row_to_block(row: &rusqlite::Row) -> rusqlite::Result<Block> {
    Ok(Block {
        id: row.get(0)?,
        document_id: row.get(1)?,
        content: {
            let s: String = row.get(2)?;
            serde_json::from_str(&s).unwrap_or(serde_json::json!({}))
        },
        block_type: row.get(3)?,
        sort_order: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub const BLOCK_COLS: &str =
    "SELECT b.id, b.document_id, b.content, b.type, b.sort_order, b.created_at, b.updated_at FROM blocks b";

pub fn query_blocks(db: &Connection, document_id: &str) -> rusqlite::Result<Vec<Block>> {
    let mut stmt = db.prepare(&format!(
        "{BLOCK_COLS} WHERE b.document_id = ?1 ORDER BY b.sort_order ASC"
    ))?;
    let rows = stmt.query_map(rusqlite::params![document_id], row_to_block)?;
    rows.collect()
}

pub fn insert_block(db: &Connection, block: &Block) -> rusqlite::Result<()> {
    db.execute(
        "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            block.id, block.document_id, block.block_type,
            block.content.to_string(), block.sort_order, block.created_at, block.updated_at
        ],
    )?;
    fts_index_block(db, block)
}

/// Upsert a block (the shape the Tauri command uses) and keep the FTS index
/// in sync. created_at is preserved by not touching it in DO UPDATE.
pub fn upsert_block(db: &Connection, block: &Block) -> rusqlite::Result<Block> {
    db.execute(
        "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
         ON CONFLICT(id) DO UPDATE SET
             document_id = excluded.document_id,
             type = excluded.type,
             content = excluded.content,
             sort_order = excluded.sort_order,
             updated_at = excluded.updated_at",
        rusqlite::params![
            block.id, block.document_id, block.block_type,
            block.content.to_string(), block.sort_order, block.updated_at
        ],
    )?;
    fts_index_block(db, block)?;
    bump_rev(db, &block.document_id, &block.updated_at)?;
    db.query_row(
        "SELECT b.id, b.document_id, b.content, b.type, b.sort_order, b.created_at, b.updated_at FROM blocks b WHERE b.id = ?1",
        rusqlite::params![block.id],
        row_to_block,
    )
}

pub fn delete_block(db: &Connection, id: &str) -> rusqlite::Result<()> {
    let doc_id: Option<String> = db
        .query_row("SELECT document_id FROM blocks WHERE id = ?1", rusqlite::params![id], |r| r.get(0))
        .ok();
    db.execute("DELETE FROM blocks WHERE id = ?1", rusqlite::params![id])?;
    db.execute("DELETE FROM embeddings WHERE block_id = ?1", rusqlite::params![id])?;
    vec_remove_block(db, id)?;
    fts_remove_block(db, id)?;
    if let Some(doc_id) = doc_id {
        let now = chrono::Utc::now().to_rfc3339();
        bump_rev(db, &doc_id, &now)?;
    }
    Ok(())
}

// ── Backlinks ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backlink {
    pub doc_id: String,
    pub doc_title: String,
    pub block_content: String,
}

/// Escape LIKE wildcards so `%`/`_`/`\` in titles and queries match literally.
fn like_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

pub fn query_backlinks(db: &Connection, page_title: &str) -> rusqlite::Result<Vec<Backlink>> {
    let pattern = format!("%[[{}]]%", like_escape(page_title));
    let mut stmt = db.prepare(
        "SELECT d.id, d.title, b.content FROM blocks b
         JOIN documents d ON d.id = b.document_id
         WHERE b.content LIKE ?1 ESCAPE '\\'
         ORDER BY d.updated_at DESC"
    )?;
    let rows = stmt.query_map(rusqlite::params![pattern], |row| {
        Ok(Backlink {
            doc_id: row.get(0)?,
            doc_title: row.get(1)?,
            block_content: row.get(2)?,
        })
    })?;
    rows.collect()
}

/// Collect database-block payloads (their `attrs.data`) found anywhere in a
/// doc's JSON — top-level blocks or nested content.
fn collect_databases<'a>(v: &'a serde_json::Value, out: &mut Vec<serde_json::Value>) {
    if v.get("type").and_then(|t| t.as_str()) == Some("database") {
        if let Some(data) = v
            .pointer("/attrs/data")
            .and_then(|d| d.as_str())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        {
            out.push(data);
        }
    }
    if let Some(arr) = v.as_array() {
        for c in arr {
            collect_databases(c, out);
        }
    } else if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if k == "attrs" {
                continue; // attrs.data is a string, nothing to descend into
            }
            collect_databases(val, out);
        }
    }
}

/// Databases whose `relation`-typed columns reference `doc_id` — the
/// database-flavored backlink. `block_content` carries "db · column" context
/// so the existing backlinks panel can render it.
pub fn find_relation_backlinks(db: &Connection, doc_id: &str) -> rusqlite::Result<Vec<Backlink>> {
    let mut stmt = db.prepare(
        "SELECT d.id, d.title, b.content FROM blocks b
         JOIN documents d ON d.id = b.document_id
         WHERE d.is_archived = 0 AND d.id != ?1
         ORDER BY d.updated_at DESC",
    )?;
    let rows = stmt.query_map(rusqlite::params![doc_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, title, content) = row?;
        if let Some(context) = relation_hit(&content, doc_id) {
            out.push(Backlink {
                doc_id: id,
                doc_title: title,
                block_content: context,
            });
        }
    }
    Ok(out)
}

fn relation_hit(content: &str, doc_id: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(content).ok()?;
    let mut dbs = Vec::new();
    collect_databases(&json, &mut dbs);
    for data in dbs {
        let cols = data.get("columns")?.as_array()?;
        let rel_cols: Vec<&serde_json::Value> = cols
            .iter()
            .filter(|c| c.get("type").and_then(|t| t.as_str()) == Some("relation"))
            .collect();
        if rel_cols.is_empty() {
            continue;
        }
        let db_name = cols
            .first()
            .and_then(|c| c.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("Database");
        let rows = data.get("rows").and_then(|r| r.as_array());
        if let Some(rows) = rows {
            for col in &rel_cols {
                let col_id = col.get("id")?.as_str()?;
                let col_name = col.get("name")?.as_str()?;
                let hit = rows.iter().any(|row| {
                    row.get("cells")
                        .and_then(|c| c.get(col_id))
                        .and_then(|v| v.as_str())
                        == Some(doc_id)
                });
                if hit {
                    return Some(format!("{db_name} · {col_name}"));
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub id: String,
    pub title: String,
}

// Returns objects, not tuples — tuples serialize to JSON arrays, which the
// frontend would misread as { id, title }.
pub fn query_all_page_titles(db: &Connection) -> rusqlite::Result<Vec<PageInfo>> {
    let mut stmt = db.prepare("SELECT id, title FROM documents WHERE is_archived = 0 ORDER BY title ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(PageInfo { id: row.get(0)?, title: row.get(1)? })
    })?;
    rows.collect()
}

// ── Tags ─────────────────────────────────────────────────────────────────────
// Tags live in a per-document block (id "<docId>-tags", type "tags",
// content {"tags":[...]}) so no schema migration is needed.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    pub doc_id: String,
    pub tags: Vec<String>,
}

pub fn query_all_tags(db: &Connection) -> rusqlite::Result<Vec<TagInfo>> {
    let mut stmt = db.prepare(
        "SELECT document_id, content FROM blocks WHERE type = 'tags'
         AND document_id IN (SELECT id FROM documents WHERE is_archived = 0)",
    )?;
    let rows = stmt.query_map([], |row| {
        let content: String = row.get(1)?;
        let tags = serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .and_then(|v| {
                v.get("tags")
                    .and_then(|t| t.as_array())
                    .map(|arr| arr.iter().filter_map(|t| t.as_str().map(String::from)).collect())
            })
            .unwrap_or_default();
        Ok(TagInfo { doc_id: row.get(0)?, tags })
    })?;
    rows.collect()
}

// ── Full-text search ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub doc_id: String,
    pub doc_title: String,
    pub block_content: String,
    pub r#type: String, // "title" | "content"
}

pub fn search_all(db: &Connection, query: &str) -> rusqlite::Result<Vec<SearchResult>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }
    let match_q = fts_match_query(trimmed);
    let mut stmt = db.prepare(
        "SELECT doc_id, title, content,
                CASE WHEN block_id LIKE 't:%' THEN 'title' ELSE 'content' END AS type
         FROM blocks_fts
         WHERE blocks_fts MATCH ?1
           AND doc_id IN (SELECT id FROM documents WHERE is_archived = 0)
         ORDER BY rank
         LIMIT 30",
    )?;
    let rows = stmt.query_map(rusqlite::params![match_q], |row| {
        Ok(SearchResult {
            doc_id: row.get(0)?,
            doc_title: row.get(1)?,
            block_content: row.get(2)?,
            r#type: row.get(3)?,
        })
    })?;
    rows.collect()
}

// ── Trash (soft delete) ────────────────────────────────────────────────────────
// is_archived has always existed; these make it usable as a trash can.

pub fn archive_document(db: &Connection, id: &str, updated_at: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET is_archived = 1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![updated_at, id],
    )?;
    bump_rev(db, id, updated_at)
}

pub fn restore_document(db: &Connection, id: &str, updated_at: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET is_archived = 0, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![updated_at, id],
    )?;
    bump_rev(db, id, updated_at)
}

pub fn query_archived_documents(db: &Connection) -> rusqlite::Result<Vec<Document>> {
    let mut stmt = db.prepare(&format!(
        "{DOC_COLS} WHERE is_archived = 1 AND deleted_at IS NULL ORDER BY updated_at DESC"
    ))?;
    let rows = stmt.query_map([], row_to_document)?;
    rows.collect()
}

pub fn find_or_create_document(db: &Connection, title: &str, now: &str) -> rusqlite::Result<Document> {
    let doc = query_document_by_title(db, title);
    match doc {
        Ok(d) => Ok(d),
        Err(_) => {
            let doc = Document {
                id: uuid::Uuid::new_v4().to_string(),
                title: title.to_string(),
                created_at: now.to_string(),
                updated_at: now.to_string(),
                is_favorite: false,
                is_archived: false,
                rev: 0,
                deleted_at: None,
                folder_id: None,
            };
            insert_document(db, &doc)?;
            let block = Block {
                id: uuid::Uuid::new_v4().to_string(),
                document_id: doc.id.clone(),
                block_type: "paragraph".into(),
                content: serde_json::json!({}),
                sort_order: 1.0,
                created_at: now.to_string(),
                updated_at: now.to_string(),
            };
            insert_block(db, &block)?;
            Ok(doc)
        }
    }
}

fn query_document_by_title(db: &Connection, title: &str) -> rusqlite::Result<Document> {
    db.query_row(
        &format!("{DOC_COLS} WHERE title = ?1 AND is_archived = 0 AND deleted_at IS NULL"),
        rusqlite::params![title],
        row_to_document,
    )
}

// ── Favorites ──────────────────────────────────────────────────────────────────

pub fn toggle_document_favorite(db: &Connection, id: &str, updated_at: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET is_favorite = CASE WHEN is_favorite = 0 THEN 1 ELSE 0 END, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![updated_at, id],
    )?;
    bump_rev(db, id, updated_at)
}

// ── Duplicate ──────────────────────────────────────────────────────────────────

pub fn duplicate_document(db: &Connection, id: &str, now: &str) -> rusqlite::Result<Document> {
    let original = query_document(db, id)?;
    let new_id = uuid::Uuid::new_v4().to_string();
    let new_title = format!("Copy of {}", original.title);

    let doc = Document {
        id: new_id.clone(),
        title: new_title,
        created_at: now.to_string(),
        updated_at: now.to_string(),
        is_favorite: false,
        is_archived: false,
        rev: 0,
        deleted_at: None,
        // Duplicates stay in the same folder as the original.
        folder_id: original.folder_id.clone(),
    };
    insert_document(db, &doc)?;

    // Copy all blocks
    let blocks = query_blocks(db, id)?;
    for block in blocks {
        let new_block = Block {
            id: uuid::Uuid::new_v4().to_string(),
            document_id: new_id.clone(),
            block_type: block.block_type,
            content: block.content,
            sort_order: block.sort_order,
            created_at: now.to_string(),
            updated_at: now.to_string(),
        };
        insert_block(db, &new_block)?;
    }

    Ok(doc)
}

// ── LAN Sync (doc-level last-write-wins) ─────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncStats {
    pub docs_changed: usize,
    pub blocks_changed: usize,
}

/// Everything a peer needs to converge: every document (tombstones included)
/// and the blocks of all alive documents. Tombstoned docs carry no blocks.
pub fn query_sync_data(db: &Connection) -> rusqlite::Result<(Vec<Document>, Vec<Block>)> {
    let docs = {
        let mut stmt = db.prepare(&format!("{DOC_COLS} ORDER BY id"))?;
        let rows = stmt.query_map([], row_to_document)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let blocks = {
        let mut stmt = db.prepare(&format!(
            "{BLOCK_COLS} JOIN documents d ON d.id = b.document_id WHERE d.deleted_at IS NULL ORDER BY b.id"
        ))?;
        let rows = stmt.query_map([], row_to_block)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    Ok((docs, blocks))
}

/// Merge a peer snapshot. Winner per document = higher (rev, updated_at),
/// exact ties broken by (title, deleted_at) so both devices pick the same
/// winner. The winning side's blocks replace the local set wholesale.
/// ponytail: doc-level LWW, not block CRDT — concurrent edits to the same doc
/// on two devices can silently lose one side. Upgrade path: per-block LWW
/// with tombstones, or a full CRDT (automerge/yjs) in a later tranche.
pub fn sync_merge(db: &Connection, docs: &[Document], blocks: &[Block]) -> rusqlite::Result<SyncStats> {
    let mut stats = SyncStats::default();
    for doc in docs {
        let local = query_document(db, &doc.id).ok();
        let incoming_wins = match &local {
            None => true,
            Some(l) => {
                let incoming_key = (doc.rev, doc.updated_at.as_str());
                let local_key = (l.rev, l.updated_at.as_str());
                match incoming_key.cmp(&local_key) {
                    std::cmp::Ordering::Greater => true,
                    std::cmp::Ordering::Less => false,
                    std::cmp::Ordering::Equal => {
                        (doc.title.as_str(), doc.deleted_at.as_deref()) > (l.title.as_str(), l.deleted_at.as_deref())
                    }
                }
            }
        };
        if !incoming_wins {
            continue;
        }

        let mut blocks_changed = 0;
        if doc.deleted_at.is_none() {
            // Swap the whole block set of this doc.
            fts_remove_doc(db, &doc.id)?;
            db.execute("DELETE FROM blocks WHERE document_id = ?1", rusqlite::params![doc.id])?;
            db.execute("DELETE FROM embeddings WHERE document_id = ?1", rusqlite::params![doc.id])?;
            vec_remove_doc(db, &doc.id)?;
            for b in blocks.iter().filter(|b| b.document_id == doc.id) {
                insert_block(db, b)?;
                blocks_changed += 1;
            }
            fts_index_doc_title(db, &doc.id, &doc.title)?;
        } else {
            db.execute("DELETE FROM blocks WHERE document_id = ?1", rusqlite::params![doc.id])?;
            db.execute("DELETE FROM embeddings WHERE document_id = ?1", rusqlite::params![doc.id])?;
            vec_remove_doc(db, &doc.id)?;
            fts_remove_doc(db, &doc.id)?;
        }

        db.execute(
            "INSERT INTO documents (id, title, created_at, updated_at, is_favorite, is_archived, rev, deleted_at, folder_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                 title = excluded.title,
                 updated_at = excluded.updated_at,
                 is_favorite = excluded.is_favorite,
                 is_archived = excluded.is_archived,
                 rev = excluded.rev,
                 deleted_at = excluded.deleted_at,
                 folder_id = excluded.folder_id",
            rusqlite::params![
                doc.id, doc.title, doc.created_at, doc.updated_at,
                doc.is_favorite as i32, doc.is_archived as i32,
                doc.rev, doc.deleted_at, doc.folder_id
            ],
        )?;
        stats.docs_changed += 1;
        stats.blocks_changed += blocks_changed;
    }
    Ok(stats)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_crud_move_and_delete() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        let f = Folder { id: "f1".into(), name: "Work".into(), created_at: "c".into() };
        insert_folder(&conn, &f).unwrap();
        assert_eq!(query_folders(&conn).unwrap().len(), 1);

        rename_folder(&conn, "f1", "Projects").unwrap();
        assert_eq!(query_folders(&conn).unwrap()[0].name, "Projects");

        // Move two docs into the folder, one back out to root.
        for id in ["d1", "d2"] {
            let doc = Document {
                id: id.into(),
                title: id.into(),
                created_at: "c".into(),
                updated_at: "u".into(),
                is_favorite: false,
                is_archived: false,
                rev: 0,
                deleted_at: None,
                folder_id: None,
            };
            insert_document(&conn, &doc).unwrap();
        }
        move_document(&conn, "d1", Some("f1"), "u1").unwrap();
        move_document(&conn, "d2", Some("f1"), "u2").unwrap();
        let docs = query_documents(&conn).unwrap();
        assert_eq!(docs.iter().filter(|d| d.folder_id.as_deref() == Some("f1")).count(), 2);

        move_document(&conn, "d1", None, "u3").unwrap();
        let docs = query_documents(&conn).unwrap();
        assert_eq!(docs.iter().filter(|d| d.folder_id.is_some()).count(), 1);
        assert!(docs.iter().any(|d| d.id == "d1" && d.folder_id.is_none()));

        // Deleting a folder clears its pages' folder_id; pages survive.
        delete_folder(&conn, "f1").unwrap();
        assert!(query_folders(&conn).unwrap().is_empty());
        let docs = query_documents(&conn).unwrap();
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().all(|d| d.folder_id.is_none()));
    }

    #[test]
    fn like_escape_escapes_wildcards() {
        assert_eq!(like_escape("a%b_c\\d"), "a\\%b\\_c\\\\d");
        assert_eq!(like_escape("plain"), "plain");
    }

    #[test]
    fn upsert_preserves_created_at() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        conn.execute(
            "INSERT INTO documents (id, title, created_at, updated_at) VALUES ('d1', 't', 'c', 'u')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
             VALUES ('b1', 'd1', 'paragraph', '{}', 1.0, 'old-created', 'old-updated')",
            [],
        )
        .unwrap();
        // Same shape as the tauri command's upsert (no OR REPLACE)
        conn.execute(
            "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 document_id = excluded.document_id,
                 type = excluded.type,
                 content = excluded.content,
                 sort_order = excluded.sort_order,
                 updated_at = excluded.updated_at",
            rusqlite::params!["b1", "d1", "paragraph", "{}", 1.0, "new-now"],
        )
        .unwrap();
        let row: (String, String) = conn
            .query_row("SELECT created_at, updated_at FROM blocks WHERE id = 'b1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(row, ("old-created".to_string(), "new-now".to_string()));
    }

    #[test]
    fn backlinks_escape_wildcards_in_titles() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        conn.execute(
            "INSERT INTO documents (id, title, created_at, updated_at) VALUES ('d1', 'd', 'c', 'u')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
             VALUES ('b1', 'd1', 'paragraph', '{\"text\":\"see [[50%_off]]\"}', 1.0, 'c', 'u')",
            [],
        )
        .unwrap();
        // Literal `%` in the title must only match the literal `%` in content
        let links = query_backlinks(&conn, "50%_off").unwrap();
        assert_eq!(links.len(), 1);
        let none = query_backlinks(&conn, "50X_off").unwrap();
        assert!(none.is_empty(), "_ must not act as a single-char wildcard");
        let none = query_backlinks(&conn, "50%anything").unwrap();
        assert!(none.is_empty(), "% must not match everything");
    }

    #[test]
    fn find_relation_backlinks_scans_database_blocks() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        let insert_doc = |id: &str, title: &str, archived: i64| {
            conn.execute(
                "INSERT INTO documents (id, title, created_at, updated_at, is_archived)
                 VALUES (?1, ?2, 'c', 'u', ?3)",
                rusqlite::params![id, title, archived],
            )
            .unwrap();
        };
        insert_doc("d1", "Tracker", 0);
        insert_doc("d2", "Sprint", 0);
        insert_doc("d3", "Backlog", 1);
        let db_data = serde_json::json!({
            "id": "db1",
            "columns": [
                {"id": "c1", "name": "Task", "type": "text"},
                {"id": "c2", "name": "Related", "type": "relation"}
            ],
            "rows": [
                {"id": "r1", "cells": {"c1": "Ship", "c2": "d2"}},
                {"id": "r2", "cells": {"c1": "Other", "c2": "d3"}}
            ]
        });
        let content = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": "hi"}]},
                {"type": "database", "attrs": {"data": db_data.to_string()}}
            ]
        })
        .to_string();
        conn.execute(
            "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
             VALUES ('b1', 'd1', 'doc', ?1, 0.0, 'c', 'u')",
            rusqlite::params![content],
        )
        .unwrap();

        let hit_d2 = find_relation_backlinks(&conn, "d2").unwrap();
        assert_eq!(hit_d2.len(), 1, "one doc references d2");
        assert_eq!(hit_d2[0].doc_id, "d1");
        assert_eq!(hit_d2[0].doc_title, "Tracker");
        assert_eq!(hit_d2[0].block_content, "Task · Related");
        // Referenced docs may be archived; the REFERENCING doc's status matters
        let hit_d3 = find_relation_backlinks(&conn, "d3").unwrap();
        assert_eq!(hit_d3.len(), 1, "d1 also references archived d3");
        assert!(find_relation_backlinks(&conn, "missing").unwrap().is_empty());
        // Archiving the referencing doc removes the backlink
        conn.execute("UPDATE documents SET is_archived = 1 WHERE id = 'd1'", [])
            .unwrap();
        assert!(find_relation_backlinks(&conn, "d2").unwrap().is_empty());
        // A database without relation columns never matches
        let plain = serde_json::json!({
            "type": "doc",
            "content": [
                {"type": "database", "attrs": {"data": serde_json::json!({
                    "columns": [{"id": "c1", "name": "Task", "type": "text"}],
                    "rows": [{"id": "r1", "cells": {"c1": "d2"}}]
                }).to_string()}}
            ]
        })
        .to_string();
        conn.execute(
            "UPDATE blocks SET content = ?1 WHERE id = 'b1'",
            rusqlite::params![plain],
        )
        .unwrap();
        assert!(find_relation_backlinks(&conn, "d2").unwrap().is_empty());
    }

    #[test]
    fn query_all_tags_parses_blocks_and_skips_archived() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        let insert = |id: &str, archived: i64| {
            conn.execute(
                "INSERT INTO documents (id, title, created_at, updated_at, is_archived) VALUES (?1, ?2, 'c', 'u', ?3)",
                rusqlite::params![id, id, archived],
            )
            .unwrap();
        };
        insert("d1", 0);
        insert("d2", 0);
        insert("d3", 1);
        let block = |id: &str, doc: &str, tags: &str| {
            conn.execute(
                "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
                 VALUES (?1, ?2, 'tags', ?3, 2.0, 'c', 'u')",
                rusqlite::params![id, doc, tags],
            )
            .unwrap();
        };
        block("d1-tags", "d1", r#"{"tags":["work","urgent"]}"#);
        block("d2-tags", "d2", r#"{"tags":["work"]}"#);
        block("d3-tags", "d3", r#"{"tags":["archived"]}"#);
        // Malformed JSON must not fail the query
        block("d1-bad", "d1", "not json");

        let tags = query_all_tags(&conn).unwrap();
        // d1-tags, d1-bad (malformed -> empty), d2-tags; d3 excluded (archived)
        assert_eq!(tags.len(), 3);
        let d1 = tags.iter().find(|t| t.doc_id == "d1" && !t.tags.is_empty()).unwrap();
        assert_eq!(d1.tags, vec!["work", "urgent"]);
        assert!(tags.iter().any(|t| t.doc_id == "d2" && t.tags == vec!["work"]));
        assert!(tags.iter().all(|t| t.doc_id != "d3"), "archived doc's tags excluded");
    }

    #[test]
    fn fts_search_ranks_and_excludes_archived() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        let now = "t";
        let doc = Document {
            id: "d1".into(),
            title: "Meeting Notes".into(),
            created_at: now.into(),
            updated_at: now.into(),
            is_favorite: false,
            is_archived: false,
            rev: 0,
            deleted_at: None,
            folder_id: None,
        };
        insert_document(&conn, &doc).unwrap();
        let block = Block {
            id: "b1".into(),
            document_id: "d1".into(),
            block_type: "doc".into(),
            content: serde_json::json!({
                "type": "doc",
                "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Quarterly review with the sales team"}]}]
            }),
            sort_order: 0.0,
            created_at: now.into(),
            updated_at: now.into(),
        };
        insert_block(&conn, &block).unwrap();
        // Non-prose blocks (tags/meta/whiteboard) must not be indexed
        insert_block(&conn, &Block {
            id: "b2".into(),
            document_id: "d1".into(),
            block_type: "whiteboard".into(),
            content: serde_json::json!({"type":"doc","content":[{"type":"text","text":"quarterly secret"}]}),
            sort_order: 1.0,
            created_at: now.into(),
            updated_at: now.into(),
        }).unwrap();

        let hits = search_all(&conn, "quarterly").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].doc_id, "d1");
        assert!(!hits[0].block_content.contains("secret"), "whiteboard JSON not indexed");
        assert!(hits[0].block_content.contains("sales team"));

        // Title search matches the title-only row
        let by_title = search_all(&conn, "meeting").unwrap();
        assert!(by_title.iter().any(|h| h.r#type == "title"));

        // Renaming the doc must re-index the title
        update_document_title(&conn, "d1", "Renamed Doc", now).unwrap();
        assert!(search_all(&conn, "renamed").unwrap().len() >= 1);
        assert!(search_all(&conn, "meeting").unwrap().is_empty());

        // Updating block content refreshes the index
        upsert_block(&conn, &Block {
            id: "b1".into(),
            document_id: "d1".into(),
            block_type: "doc".into(),
            content: serde_json::json!({"type":"doc","content":[]}),
            sort_order: 0.0,
            created_at: now.into(),
            updated_at: now.into(),
        }).unwrap();
        assert!(search_all(&conn, "quarterly").unwrap().is_empty());

        // Archived docs drop out of search but stay in the trash
        archive_document(&conn, "d1", now).unwrap();
        assert!(search_all(&conn, "renamed").unwrap().is_empty());
        let trash = query_archived_documents(&conn).unwrap();
        assert_eq!(trash.len(), 1);
        restore_document(&conn, "d1", now).unwrap();
        assert!(search_all(&conn, "renamed").unwrap().len() >= 1);
    }

    #[test]
    fn fts_query_escapes_user_input() {
        assert_eq!(fts_match_query("hello"), "\"hello\"*");
        assert_eq!(fts_match_query("a b"), "\"a\"* AND \"b\"*");
        // Double quotes become doubled (FTS5 string escaping), no injection
        assert_eq!(fts_match_query("say \"hi\""), "\"say\"* AND \"\"\"hi\"\"\"*");
    }

    #[test]
    fn extract_text_walks_doc_json() {
        let v = serde_json::json!({"type":"doc","content":[
            {"type":"paragraph","content":[{"type":"text","text":"alpha "}]},
            {"type":"heading","attrs":{"level":2},"content":[{"type":"text","text":"beta"}]}
        ]});
        let mut out = String::new();
        extract_text(&v, &mut out);
        assert_eq!(out, "alpha beta");
    }

    #[test]
    fn sqlcipher_encrypted_roundtrip() {
        // Exercise the real production path: keyed (encrypted) on-disk DB,
        // same statements the tauri commands run.
        let dir = std::env::temp_dir().join(format!("enclave-db-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test.db");
        let key = [7u8; 32];

        let conn = init_vault(&db_path, &key).unwrap();
        conn.execute(
            "INSERT INTO documents (id, title, created_at, updated_at) VALUES ('d1', 'Doc', 'c', 'u')",
            [],
        )
        .unwrap();
        // First insert (new block)
        conn.execute(
            "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(id) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
            rusqlite::params!["b1", "d1", "doc", "{\"type\":\"doc\"}", 0.0, "t1"],
        )
        .unwrap();
        // Second upsert (update path) — must preserve created_at
        conn.execute(
            "INSERT INTO blocks (id, document_id, type, content, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(id) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
            rusqlite::params!["b1", "d1", "doc", "{\"type\":\"doc\",\"edited\":true}", 0.0, "t2"],
        )
        .unwrap();
        drop(conn);

        // Reopen with the same key — data must survive
        let conn = open_vault(&db_path, &key).unwrap();
        let blocks = query_blocks(&conn, "d1").unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content["edited"], true);
        assert_eq!(blocks[0].created_at, "t1");
        drop(conn);

        // Wrong key must be rejected
        assert!(open_vault(&db_path, &[1u8; 32]).is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn embeddings_upsert_query_and_cleanup() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        let now = "t";
        insert_document(
            &conn,
            &Document {
                id: "d1".into(),
                title: "Notes".into(),
                created_at: now.into(),
                updated_at: now.into(),
                is_favorite: false,
                is_archived: false,
                rev: 0,
                deleted_at: None,
                folder_id: None,
            },
        )
        .unwrap();
        insert_document(
            &conn,
            &Document {
                id: "d2".into(),
                title: "Archive".into(),
                created_at: now.into(),
                updated_at: now.into(),
                is_favorite: false,
                is_archived: true,
                rev: 0,
                deleted_at: None,
                folder_id: None,
            },
        )
        .unwrap();

        upsert_embedding(&conn, "b1", "d1", "first text", &[1.0, 0.0], "t1").unwrap();
        // Re-upsert the same block updates, does not duplicate
        upsert_embedding(&conn, "b1", "d1", "updated text", &[0.0, 1.0], "t2").unwrap();
        upsert_embedding(&conn, "b2", "d2", "archived text", &[0.5, 0.5], "t1").unwrap();

        let rows = query_embeddings(&conn).unwrap();
        assert_eq!(rows.len(), 1, "archived doc's embedding excluded");
        assert_eq!(rows[0].block_id, "b1");
        assert_eq!(rows[0].doc_title, "Notes");
        assert_eq!(rows[0].text, "updated text", "upsert replaces text");
        assert_eq!(rows[0].vector, vec![0.0, 1.0], "upsert replaces vector");

        // Deleting the block cleans its embedding
        delete_block(&conn, "b1").unwrap();
        assert!(query_embeddings(&conn).unwrap().is_empty());
    }

    #[test]
    fn embeddings_topk_uses_vec_index_and_reranks() {
        ensure_vec_extension(); // must run before the first connection opens
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        let now = "t";
        for (id, archived) in [("d1", false), ("d2", false), ("d3", true)] {
            insert_document(
                &conn,
                &Document {
                    id: id.into(),
                    title: id.into(),
                    created_at: now.into(),
                    updated_at: now.into(),
                    is_favorite: false,
                    is_archived: archived,
                    rev: 0,
                    deleted_at: None,
                    folder_id: None,
                },
            )
            .unwrap();
        }
        upsert_embedding(&conn, "b1", "d1", "alpha", &[1.0, 0.0, 0.0], "t1").unwrap();
        upsert_embedding(&conn, "b2", "d2", "beta", &[0.0, 1.0, 0.0], "t1").unwrap();
        upsert_embedding(&conn, "b3", "d3", "archived", &[0.9, 0.1, 0.0], "t1").unwrap();

        // Nearest first; archived doc's vector excluded from results.
        let top = query_embeddings_topk(&conn, &[1.0, 0.0, 0.0], 5).unwrap();
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].block_id, "b1", "exact match ranks first");
        assert_eq!(top[1].block_id, "b2");

        // limit is honored
        let one = query_embeddings_topk(&conn, &[1.0, 0.0, 0.0], 1).unwrap();
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].block_id, "b1");

        // Re-upsert replaces the vector in the index (b1 now anti-correlated)
        upsert_embedding(&conn, "b1", "d1", "alpha v2", &[-1.0, 0.0, 0.0], "t2").unwrap();
        let top = query_embeddings_topk(&conn, &[1.0, 0.0, 0.0], 1).unwrap();
        assert_eq!(top[0].block_id, "b2");

        // Deleting a block removes it from the vector index too
        delete_block(&conn, "b2").unwrap();
        let top = query_embeddings_topk(&conn, &[1.0, 0.0, 0.0], 5).unwrap();
        assert!(top.iter().all(|e| e.block_id != "b2"));

        // Deleting a document cleans all of its vectors
        delete_document(&conn, "d1", "t3").unwrap();
        let top = query_embeddings_topk(&conn, &[1.0, 0.0, 0.0], 5).unwrap();
        assert!(top.iter().all(|e| e.block_id != "b1"));

        // Unknown dimension (no vec0 table) falls back to an exact scan
        upsert_embedding(&conn, "b4", "d2", "wide", &[1.0, 0.0], "t1").unwrap();
        let top = query_embeddings_topk(&conn, &[1.0, 0.0], 5).unwrap();
        assert_eq!(top[0].block_id, "b4");
    }

    #[test]
    fn settings_roundtrip_and_missing() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        assert_eq!(get_setting(&conn, "ai").unwrap(), None);
        set_setting(&conn, "ai", "{\"enabled\":true}").unwrap();
        assert_eq!(get_setting(&conn, "ai").unwrap().as_deref(), Some("{\"enabled\":true}"));
        set_setting(&conn, "ai", "{\"enabled\":false}").unwrap();
        assert_eq!(get_setting(&conn, "ai").unwrap().as_deref(), Some("{\"enabled\":false}"));
        assert_eq!(get_setting(&conn, "nope").unwrap(), None);
    }

    #[test]
    fn sync_merge_lww_converges_and_honors_tombstones() {
        // Two devices, same doc edited concurrently to different titles.
        let a = Connection::open_in_memory().unwrap();
        let b = Connection::open_in_memory().unwrap();
        create_tables(&a).unwrap();
        create_tables(&b).unwrap();

        let mk = |id: &str, title: &str, rev: i64, ts: &str| Document {
            id: id.into(),
            title: title.into(),
            created_at: "c".into(),
            updated_at: ts.into(),
            is_favorite: false,
            is_archived: false,
            rev,
            deleted_at: None,
            folder_id: None,
        };

        // Seed both sides with the same base doc.
        for conn in [&a, &b] {
            conn.execute(
                "INSERT INTO documents (id, title, created_at, updated_at, rev)
                 VALUES ('d1', 'base', 'c', 't0', 3)",
                [],
            )
            .unwrap();
        }

        // Concurrent edits: same rev+ts → tie must resolve identically.
        // Each device applies its own edit, then they exchange snapshots.
        let edit_a = mk("d1", "title from A", 4, "t4");
        let edit_b = mk("d1", "title from B", 4, "t4");
        sync_merge(&a, &[edit_a.clone()], &[]).unwrap();
        sync_merge(&b, &[edit_b.clone()], &[]).unwrap();
        sync_merge(&a, &[edit_b.clone()], &[]).unwrap();
        sync_merge(&b, &[edit_a.clone()], &[]).unwrap();
        let (a_docs, _) = query_sync_data(&a).unwrap();
        let (b_docs, _) = query_sync_data(&b).unwrap();
        assert_eq!(a_docs[0].title, b_docs[0].title, "tie must converge");

        // A newer edit on A (higher rev) wins on B.
        let newer = mk("d1", "title from A v2", 5, "t5");
        sync_merge(&a, &[newer.clone()], &[],).unwrap();
        sync_merge(&b, &[newer.clone()], &[],).unwrap();
        let (b_docs, _) = query_sync_data(&b).unwrap();
        assert_eq!(b_docs[0].title, "title from A v2");

        // Tombstone on A (rev 6) propagates to B; a stale resurrect (rev 4)
        // must not bring the doc back.
        let tomb = Document {
            deleted_at: Some("t6".into()),
            ..newer.clone()
        };
        sync_merge(&a, &[tomb.clone()], &[]).unwrap();
        sync_merge(&b, &[tomb], &[]).unwrap();
        let (a_docs, _) = query_sync_data(&a).unwrap();
        let (b_docs, _) = query_sync_data(&b).unwrap();
        assert!(a_docs[0].deleted_at.is_some());
        assert!(b_docs[0].deleted_at.is_some());
        sync_merge(&b, &[mk("d1", "stale", 4, "t4")], &[]).unwrap();
        let (b_docs, _) = query_sync_data(&b).unwrap();
        assert!(b_docs[0].deleted_at.is_some(), "stale edit must not resurrect");

        // A higher-rev alive doc resurrects the tombstone.
        let alive = mk("d1", "resurrected", 7, "t7");
        sync_merge(&b, &[alive], &[],).unwrap();
        let (b_docs, _) = query_sync_data(&b).unwrap();
        assert!(b_docs[0].deleted_at.is_none());
        assert_eq!(b_docs[0].title, "resurrected");
    }
}