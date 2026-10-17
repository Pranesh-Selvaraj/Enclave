//! Update-signature verification, re-exported from the core.
//!
//! There is exactly one verifier and one pinned release key (ADR 0003/0006):
//! it lives in `core-api` so the desktop shell and the native Android app
//! cannot drift apart. Tests live beside the implementation in
//! `core/core-api/src/updater.rs`.

pub use core_api::updater::{verify_update_signature, UPDATE_PUBKEY_HEX};
