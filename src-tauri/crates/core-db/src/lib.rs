//! Encrypted SQLite storage engine for Enclave.
//!
//! Provides Document and Block types, query helpers, and database
//! initialization with SQLCipher encryption. Used by the Tauri command layer.

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

// ── Database Initialization ─────────────────────────────────────────────────

pub fn init_db(db_path: &std::path::Path) -> Connection {
    let conn = Connection::open(db_path).expect("Failed to open SQLite database");

    conn.execute_batch(
        "PRAGMA key = 'enclave-dev-key';
         PRAGMA cipher_page_size = 4096;
         PRAGMA kdf_iter = 256000;
         PRAGMA cipher_hmac_algorithm = HMAC_SHA512;
         PRAGMA cipher_kdf_algorithm = PBKDF2_HMAC_SHA512;",
    )
    .expect("Failed to configure sqlcipher encryption");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            id           TEXT PRIMARY KEY,
            title        TEXT NOT NULL DEFAULT '',
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            is_favorite  INTEGER NOT NULL DEFAULT 0,
            is_archived  INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .expect("Failed to create documents table");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS blocks (
            id           TEXT PRIMARY KEY,
            document_id  TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            type         TEXT NOT NULL DEFAULT 'paragraph',
            content      TEXT NOT NULL DEFAULT '{}',
            sort_order   REAL NOT NULL,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL
        )",
        [],
    )
    .expect("Failed to create blocks table");

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_blocks_document
         ON blocks(document_id, sort_order)",
        [],
    )
    .expect("Failed to create blocks index");

    conn
}

// ── Document Queries ────────────────────────────────────────────────────────

fn row_to_document(row: &rusqlite::Row) -> rusqlite::Result<Document> {
    Ok(Document {
        id: row.get(0)?,
        title: row.get(1)?,
        created_at: row.get(2)?,
        updated_at: row.get(3)?,
        is_favorite: row.get::<_, i32>(4)? != 0,
        is_archived: row.get::<_, i32>(5)? != 0,
    })
}

const DOC_COLS: &str = "SELECT id, title, created_at, updated_at, is_favorite, is_archived FROM documents";

pub fn query_documents(db: &Connection) -> rusqlite::Result<Vec<Document>> {
    let mut stmt = db.prepare(&format!(
        "{DOC_COLS} WHERE is_archived = 0 ORDER BY updated_at DESC"
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
        "INSERT INTO documents (id, title, created_at, updated_at, is_favorite, is_archived)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            doc.id, doc.title, doc.created_at, doc.updated_at,
            doc.is_favorite as i32, doc.is_archived as i32
        ],
    )?;
    Ok(())
}

pub fn update_document_title(db: &Connection, id: &str, title: &str, updated_at: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET title = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![title, updated_at, id],
    )?;
    Ok(())
}

pub fn delete_document(db: &Connection, id: &str) -> rusqlite::Result<()> {
    db.execute("DELETE FROM blocks WHERE document_id = ?1", rusqlite::params![id])?;
    db.execute("DELETE FROM documents WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

// ── Block Queries ───────────────────────────────────────────────────────────

pub fn row_to_block(row: &rusqlite::Row) -> rusqlite::Result<Block> {
    let content_str: String = row.get(3)?;
    Ok(Block {
        id: row.get(0)?,
        document_id: row.get(1)?,
        block_type: row.get(4)?,
        content: serde_json::from_str(&content_str).unwrap_or(serde_json::json!({})),
        sort_order: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
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
    Ok(())
}

pub fn update_block(
    db: &Connection,
    id: &str,
    content: &serde_json::Value,
    updated_at: &str,
) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE blocks SET content = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![content.to_string(), updated_at, id],
    )?;
    Ok(())
}

pub fn delete_block(db: &Connection, id: &str) -> rusqlite::Result<()> {
    db.execute("DELETE FROM blocks WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}
