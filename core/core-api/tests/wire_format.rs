//! Golden-fixture guard for the app-layer sync frames (ADR 0006). If a field
//! is renamed, retyped, reordered or dropped, or a payload shape changes,
//! this test fails — before two apps can silently drift apart.

use core_api::frames::SyncFrame;

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// Parse the fixture and assert the serializer emits the fixture bytes
/// exactly — the file is the canonical form.
fn parse(name: &str) -> SyncFrame {
    let raw = fixture(name);
    let parsed: SyncFrame =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{name} does not parse: {e}"));
    assert_eq!(
        parsed.to_json().unwrap(),
        raw,
        "{name} serialization drifted from the fixture"
    );
    parsed
}

#[test]
fn digest_frame_matches_its_golden_fixture() {
    let SyncFrame::Digest { docs } = parse("digest-v3.json") else {
        panic!("digest-v3.json is not a digest frame");
    };
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].id, "note-1");
    assert_eq!(docs[0].rev, 3);
    assert!(docs[0].deleted_at.is_none());
    assert_eq!(docs[1].id, "note-2");
    assert_eq!(docs[1].deleted_at.as_deref(), Some("2026-10-17T08:00:00Z"));
}

#[test]
fn need_frame_matches_its_golden_fixture() {
    let SyncFrame::Need { ids } = parse("need-v3.json") else {
        panic!("need-v3.json is not a need frame");
    };
    assert_eq!(ids, vec!["note-1", "note-3"]);
}

#[test]
fn snapshot_frame_matches_its_golden_fixture() {
    let SyncFrame::Snapshot {
        docs,
        blocks,
        folders,
    } = parse("snapshot-v3.json")
    else {
        panic!("snapshot-v3.json is not a snapshot frame");
    };
    assert_eq!(docs.len(), 1);
    assert_eq!(blocks.len(), 1);
    assert_eq!(folders.len(), 1);
    assert_eq!(docs[0].folder_id.as_deref(), Some("folder-1"));
    assert_eq!(folders[0].name, "Contracts");
    assert_eq!(blocks[0].document_id, "note-1");
    // The block payload is stored and shipped verbatim — the core never
    // rewrites editor JSON.
    assert_eq!(blocks[0].block_type, "doc");
    assert_eq!(blocks[0].content["type"], "doc");
    assert_eq!(
        blocks[0].content["content"][0]["content"][0]["text"],
        "Hello contract"
    );
}

#[test]
fn ack_frame_matches_its_golden_fixture() {
    let SyncFrame::Ack {
        docs_changed,
        blocks_changed,
    } = parse("ack-v3.json")
    else {
        panic!("ack-v3.json is not an ack frame");
    };
    assert_eq!(docs_changed, 2);
    assert_eq!(blocks_changed, 5);
}
