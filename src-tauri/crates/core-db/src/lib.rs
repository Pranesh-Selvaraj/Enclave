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
         ON blocks(document_id, sort_order);",
    )
}

// ── Vault Lifecycle ─────────────────────────────────────────────────────────

/// Check whether a vault database file already exists on disk.
pub fn vault_exists(db_path: &std::path::Path) -> bool {
    db_path.exists()
}

/// Create a new vault: open the database, set the key, create tables.
/// Returns the open connection.
pub fn init_vault(db_path: &std::path::Path, key: &[u8]) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to create database: {e}"))?;
    set_cipher_pragmas(&conn, key).map_err(|e| format!("Failed to set encryption key: {e}"))?;
    create_tables(&conn).map_err(|e| format!("Failed to create tables: {e}"))?;
    Ok(conn)
}

/// Open an existing vault: open the database, set the key, ensure tables exist.
/// Returns the open connection or an error if the key is wrong.
pub fn open_vault(db_path: &std::path::Path, key: &[u8]) -> Result<Connection, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("Failed to open database: {e}"))?;
    set_cipher_pragmas(&conn, key).map_err(|e| format!("Failed to set encryption key: {e}"))?;

    // Verify the key by reading the schema — wrong key produces a corrupted view
    conn.query_row("SELECT COUNT(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|_| "Invalid vault key".to_string())?;

    // Ensure tables exist (idempotent — safe to call on every unlock)
    create_tables(&conn).map_err(|e| format!("Failed to create tables: {e}"))?;

    Ok(conn)
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
    Ok(())
}

pub fn delete_block(db: &Connection, id: &str) -> rusqlite::Result<()> {
    db.execute("DELETE FROM blocks WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

// ── Backlinks ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backlink {
    pub doc_id: String,
    pub doc_title: String,
    pub block_content: String,
}

pub fn query_backlinks(db: &Connection, page_title: &str) -> rusqlite::Result<Vec<Backlink>> {
    let pattern = format!("%[[{}]]%", page_title);
    let mut stmt = db.prepare(
        "SELECT d.id, d.title, b.content FROM blocks b
         JOIN documents d ON d.id = b.document_id
         WHERE b.content LIKE ?1
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

pub fn query_all_page_titles(db: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut stmt = db.prepare("SELECT id, title FROM documents WHERE is_archived = 0 ORDER BY title ASC")?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
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
    let like_pattern = format!("%{}%", query);
    let mut stmt = db.prepare(
        "SELECT d.id, d.title, b.content, 'content' AS type FROM blocks b
         JOIN documents d ON d.id = b.document_id
         WHERE b.content LIKE ?1 AND d.is_archived = 0
         UNION ALL
         SELECT d.id, d.title, '', 'title' AS type FROM documents d
         WHERE d.title LIKE ?1 AND d.is_archived = 0
         ORDER BY type DESC
         LIMIT 30"
    )?;
    let rows = stmt.query_map(rusqlite::params![like_pattern], |row| {
        Ok(SearchResult {
            doc_id: row.get(0)?,
            doc_title: row.get(1)?,
            block_content: row.get(2)?,
            r#type: row.get(3)?,
        })
    })?;
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
        &format!("{DOC_COLS} WHERE title = ?1 AND is_archived = 0"),
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
    Ok(())
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
