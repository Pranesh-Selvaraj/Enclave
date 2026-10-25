//! Transport frames — the JSON exchanged before and around the encrypted
//! sync payloads.
//!
//! `Auth` and `AuthProof` are plaintext (they carry only challenges and HMAC
//! proofs — no vault data); `Hello` is the first frame sealed with the
//! per-session key, and `SessionFailed` tells the app layer that a session
//! died for a user-actionable reason.
//!
//! These shapes are part of the shared contract (ADR 0006): the golden
//! fixtures under `tests/fixtures/` lock them, and the integration test fails
//! if a field is renamed, retyped or reordered.

use serde::{Deserialize, Serialize};

/// Every frame the transport itself constructs or consumes.
///
/// App payloads (`digest`, `need`, `snapshot`, `ack`) are opaque strings to
/// the transport by design; their shapes live in `core-api` (`frames`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportFrame {
    /// Opening frame: announces the protocol version, identity and a fresh
    /// 32-byte challenge (base64).
    Auth {
        v: u64,
        peer_id: String,
        name: String,
        challenge: String,
    },
    /// Proves knowledge of the sync key: HMAC over the auth label, both
    /// challenges (sorted) and the sender's peer id (base64).
    AuthProof { peer_id: String, proof: String },
    /// First encrypted frame; tells the app layer which peer owns the session.
    Hello { peer_id: String, name: String },
    /// A failure the UI must surface (wrong vault key, dead handshake).
    SessionFailed { host: String, error: String },
}
