//! Proves the desktop can use the core as a normal Rust crate (no C ABI).

use core_api::EnclaveCore;

#[test]
fn creates_persists_and_reopens_a_vault() {
    let dir = tempfile::tempdir().expect("temp app dir");
    let core = EnclaveCore::new(dir.path());

    assert!(!core.is_vault_initialized());
    let mnemonic = core
        .create_vault("correct horse battery")
        .expect("create vault");
    assert_eq!(mnemonic.split_whitespace().count(), 12);
    assert!(core.is_vault_initialized());
    assert!(core.home_notes().expect("home notes").is_empty());

    core.create_document("Hello from Rust")
        .expect("create document");
    let notes = core.home_notes().expect("home notes");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].document.title, "Hello from Rust");
}
