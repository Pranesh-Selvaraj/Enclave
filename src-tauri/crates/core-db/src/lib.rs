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
         ON blocks(document_id, sort_order);
        CREATE VIRTUAL TABLE IF NOT EXISTS blocks_fts USING fts5(
            doc_id UNINDEXED,
            block_id UNINDEXED,
            title,
            content
        );",
    )
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
    set_perf_pragmas(&conn).map_err(|e| format!("Failed to set perf pragmas: {e}"))?;
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
    set_perf_pragmas(&conn).map_err(|e| format!("Failed to set perf pragmas: {e}"))?;

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
    fts_index_doc_title(db, &doc.id, &doc.title)
}

pub fn update_document_title(db: &Connection, id: &str, title: &str, updated_at: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET title = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![title, updated_at, id],
    )?;
    db.execute(
        "UPDATE blocks_fts SET title = ?1 WHERE doc_id = ?2",
        rusqlite::params![title, id],
    )?;
    Ok(())
}

pub fn delete_document(db: &Connection, id: &str) -> rusqlite::Result<()> {
    db.execute("DELETE FROM blocks WHERE document_id = ?1", rusqlite::params![id])?;
    db.execute("DELETE FROM documents WHERE id = ?1", rusqlite::params![id])?;
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
    db.query_row(
        "SELECT b.id, b.document_id, b.content, b.type, b.sort_order, b.created_at, b.updated_at FROM blocks b WHERE b.id = ?1",
        rusqlite::params![block.id],
        row_to_block,
    )
}

pub fn delete_block(db: &Connection, id: &str) -> rusqlite::Result<()> {
    db.execute("DELETE FROM blocks WHERE id = ?1", rusqlite::params![id])?;
    fts_remove_block(db, id)
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
    Ok(())
}

pub fn restore_document(db: &Connection, id: &str, updated_at: &str) -> rusqlite::Result<()> {
    db.execute(
        "UPDATE documents SET is_archived = 0, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![updated_at, id],
    )?;
    Ok(())
}

pub fn query_archived_documents(db: &Connection) -> rusqlite::Result<Vec<Document>> {
    let mut stmt = db.prepare(&format!(
        "{DOC_COLS} WHERE is_archived = 1 ORDER BY updated_at DESC"
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

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
