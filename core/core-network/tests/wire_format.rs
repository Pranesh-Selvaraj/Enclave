//! Golden-fixture guard for the transport wire format and handshake crypto
//! (ADR 0006). If a frame field is renamed, retyped, reordered or dropped, or
//! a key derivation/salt changes, this test fails — before two apps can
//! silently drift apart.

use base64::Engine;
use core_network::crypto;
use core_network::frames::TransportFrame;
use core_network::PROTOCOL_VERSION;

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

fn b64(value: &str) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .expect("fixture holds valid base64")
}

fn arr32(value: &str) -> [u8; 32] {
    b64(value)
        .try_into()
        .expect("fixture holds a 32-byte value")
}

/// Parse the fixture, assert the variant and assert the serializer emits the
/// fixture bytes exactly — the file is the canonical form.
fn assert_frame(name: &str, expected: &TransportFrame) {
    let raw = fixture(name);
    let parsed: TransportFrame =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{name} does not parse: {e}"));
    assert_eq!(&parsed, expected, "{name} parsed into a different frame");
    assert_eq!(
        serde_json::to_string(expected).unwrap(),
        raw,
        "{name} serialization drifted from the fixture"
    );
}

#[test]
fn transport_frames_match_their_golden_fixtures() {
    assert_frame(
        "auth-v3.json",
        &TransportFrame::Auth {
            v: PROTOCOL_VERSION,
            peer_id: "peer-a".into(),
            name: "Laptop".into(),
            challenge: "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=".into(),
        },
    );
    assert_frame(
        "auth_proof-v3.json",
        &TransportFrame::AuthProof {
            peer_id: "peer-a".into(),
            proof: "5LBqD8TEoBCZ7T5nqZ1gVmHuQf0aYglTcAGzNV+rWmw=".into(),
        },
    );
    assert_frame(
        "hello-v3.json",
        &TransportFrame::Hello {
            peer_id: "peer-a".into(),
            name: "Laptop".into(),
        },
    );
    assert_frame(
        "session_failed-v3.json",
        &TransportFrame::SessionFailed {
            host: "192.168.1.20".into(),
            error: "peer speaks sync protocol v2 — this Enclave speaks v3; update the peer".into(),
        },
    );

    // The auth fixture must carry the protocol version this build speaks.
    let TransportFrame::Auth { v, .. } = serde_json::from_str(&fixture("auth-v3.json")).unwrap()
    else {
        panic!("auth-v3.json is not an auth frame");
    };
    assert_eq!(v, PROTOCOL_VERSION, "auth fixture is stale");
}

#[test]
fn handshake_crypto_matches_its_golden_vectors() {
    let v: serde_json::Value = serde_json::from_str(&fixture("handshake-v3.json")).unwrap();
    let vault_key = arr32(v["vault_key"].as_str().unwrap());
    let sync_key = arr32(v["sync_key"].as_str().unwrap());
    let challenge_a = arr32(v["challenge_a"].as_str().unwrap());
    let challenge_b = arr32(v["challenge_b"].as_str().unwrap());
    let session_key = arr32(v["session_key"].as_str().unwrap());
    let nonce: [u8; 24] = b64(v["hello_nonce"].as_str().unwrap())
        .try_into()
        .expect("24-byte nonce");
    let plaintext = v["hello_plaintext"].as_str().unwrap();
    let ciphertext = b64(v["hello_ciphertext"].as_str().unwrap());

    // Key derivation: the sync key is HKDF(vault key) — never the vault key.
    assert_eq!(crypto::derive_sync_key(&vault_key), sync_key);
    assert_ne!(sync_key, vault_key);

    // Mutual proof: both sides recompute each other's proof byte-for-byte.
    let proof_a = crypto::proof(&sync_key, &challenge_a, &challenge_b, "peer-a");
    let proof_b = crypto::proof(&sync_key, &challenge_b, &challenge_a, "peer-b");
    assert_eq!(proof_a, b64(v["proof_a"].as_str().unwrap()));
    assert_eq!(proof_b, b64(v["proof_b"].as_str().unwrap()));
    assert!(crypto::verify_proof(
        &sync_key,
        &challenge_a,
        &challenge_b,
        "peer-a",
        &proof_a
    ));
    assert!(!crypto::verify_proof(
        &sync_key,
        &challenge_a,
        &challenge_b,
        "peer-X",
        &proof_a
    ));
    let other_key = crypto::derive_sync_key(&[0x0eu8; 32]);
    assert!(!crypto::verify_proof(
        &other_key,
        &challenge_a,
        &challenge_b,
        "peer-a",
        &proof_a
    ));

    // Session key: derived from both challenges, direction-independent.
    assert_eq!(
        crypto::session_key(&sync_key, &challenge_a, &challenge_b),
        session_key
    );
    assert_eq!(
        crypto::session_key(&sync_key, &challenge_b, &challenge_a),
        session_key
    );

    // Sealed frame: nonce || ciphertext, deterministic for a fixed nonce.
    let enc = crypto::encrypt(&session_key, &nonce, plaintext.as_bytes()).unwrap();
    assert_eq!(enc, ciphertext, "sealed frame drifted from the fixture");
    assert_eq!(
        crypto::decrypt(&session_key, &nonce, &ciphertext).unwrap(),
        plaintext.as_bytes()
    );
    let mut tampered = ciphertext.clone();
    tampered[0] ^= 0x01;
    assert!(crypto::decrypt(&session_key, &nonce, &tampered).is_err());
}
