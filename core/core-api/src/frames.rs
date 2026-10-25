//! App-layer sync frames — the payloads carried inside the transport's
//! encrypted frames.
//!
//! The sync loop exchanges `Digest` → `Need` → `Snapshot` → `Ack`; `Hello`
//! and `SessionFailed` belong to the transport
//! ([`core_network::frames::TransportFrame`]). These shapes are part of the
//! shared contract (ADR 0006): the golden fixtures under `tests/fixtures/`
//! lock them, and the integration test fails if a field is renamed, retyped
//! or reordered.

use serde::{Deserialize, Serialize};

use crate::{Block, DocIndexEntry, Document, Folder};

/// Every frame the core's sync loop sends or accepts.
///
/// `PartialEq` is deliberately absent: the payload types come from `core-db`
/// and comparing frames is done on the wire JSON (the contract), not on Rust
/// values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SyncFrame {
    /// What this side has: every document id with its LWW revision clock.
    Digest { docs: Vec<DocIndexEntry> },
    /// Ids the sender wants after diffing the peer's digest against its own.
    Need { ids: Vec<String> },
    /// The requested documents, their blocks, and the folders they live in.
    Snapshot {
        docs: Vec<Document>,
        blocks: Vec<Block>,
        folders: Vec<Folder>,
    },
    /// The receiver merged the snapshot with these statistics.
    Ack {
        docs_changed: u64,
        blocks_changed: u64,
    },
}

impl SyncFrame {
    /// Serialize to the wire JSON. Infallible in practice (derived structs,
    /// no non-string map keys), but returns a `Result` so no call site has to
    /// panic across the FFI boundary.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}
