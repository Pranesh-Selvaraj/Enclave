//! Sync transport crypto.
//!
//! Everything derives from the vault key, which is the same for all devices
//! of the same owner (it derives from the shared BIP39 seed phrase):
//!
//! - `derive_sync_key` — domain-separated PSK (never equals the DB key).
//! - `proof` / `verify_proof` — HMAC-SHA256 challenge-response for mutual
//!   auth (fresh random challenge per connection → replay-safe).
//! - `session_key` — per-connection key from both challenges (sorted so both
//!   sides derive the same bytes regardless of who dialed).
//! - `encrypt` / `decrypt` — XChaCha20-Poly1305 AEAD (24-byte random nonce,
//!   16-byte tag; forged frames fail the MAC and kill the session).

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::XChaCha20Poly1305;
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const SYNC_SALT: &[u8] = b"enclave-sync-v1";
const AUTH_LABEL: &[u8] = b"enclave-auth-v1";
const SESSION_INFO: &[u8] = b"enclave-session-v1";

/// Derive the sync PSK from the vault key (32 bytes).
pub fn derive_sync_key(vault_key: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(SYNC_SALT), vault_key);
    let mut out = [0u8; 32];
    hk.expand(b"sync-key", &mut out).expect("32-byte HKDF expand");
    out
}

/// Per-connection session key. Challenges are sorted so both sides derive
/// the same key regardless of connection direction.
pub fn session_key(sync_key: &[u8; 32], c_a: &[u8; 32], c_b: &[u8; 32]) -> [u8; 32] {
    let (first, second) = if c_a <= c_b { (c_a, c_b) } else { (c_b, c_a) };
    let mut salt = [0u8; 64];
    salt[..32].copy_from_slice(first);
    salt[32..].copy_from_slice(second);
    let hk = Hkdf::<Sha256>::new(Some(&salt), sync_key);
    let mut out = [0u8; 32];
    hk.expand(SESSION_INFO, &mut out).expect("32-byte HKDF expand");
    out
}

/// HMAC state over the auth label, both challenges (sorted) and an id.
/// The same state computes a prover's proof and verifies it — sorted
/// challenges make both sides byte-identical regardless of direction.
fn auth_mac(sync_key: &[u8; 32], c1: &[u8; 32], c2: &[u8; 32], id: &str) -> HmacSha256 {
    let mut mac =
        <HmacSha256 as hmac::Mac>::new_from_slice(sync_key).expect("HMAC accepts any key length");
    mac.update(AUTH_LABEL);
    let (first, second) = if c1 <= c2 { (c1, c2) } else { (c2, c1) };
    mac.update(first);
    mac.update(second);
    mac.update(id.as_bytes());
    mac
}

/// Proof of PSK knowledge: HMAC over the auth label, both challenges
/// (sorted) and the prover's peer id.
pub fn proof(sync_key: &[u8; 32], my_challenge: &[u8; 32], peer_challenge: &[u8; 32], my_id: &str) -> Vec<u8> {
    auth_mac(sync_key, my_challenge, peer_challenge, my_id)
        .finalize()
        .into_bytes()
        .to_vec()
}

/// Constant-time verification of a peer's proof (wrong key, wrong id, or
/// replayed bytes all fail).
pub fn verify_proof(
    sync_key: &[u8; 32],
    peer_challenge: &[u8; 32],
    my_challenge: &[u8; 32],
    peer_id: &str,
    provided: &[u8],
) -> bool {
    auth_mac(sync_key, peer_challenge, my_challenge, peer_id)
        .verify_slice(provided)
        .is_ok()
}

/// Encrypt one frame: 24-byte random nonce || ciphertext || 16-byte tag.
pub fn encrypt(session_key: &[u8; 32], nonce: &[u8; 24], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    XChaCha20Poly1305::new(session_key.into())
        .encrypt(nonce.into(), plaintext)
        .map_err(|e| format!("encrypt failed: {e}"))
}

/// Decrypt a frame; returns Err on any forgery/tamper (bad MAC).
pub fn decrypt(session_key: &[u8; 32], nonce: &[u8; 24], ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    XChaCha20Poly1305::new(session_key.into())
        .decrypt(nonce.into(), ciphertext)
        .map_err(|_| "decrypt failed (bad MAC — frame forged or key mismatch)".to_string())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_verifies_and_rejects() {
        let k = derive_sync_key(b"some-vault-key");
        let c_a = [1u8; 32];
        let c_b = [2u8; 32];
        let p = proof(&k, &c_a, &c_b, "peer-B");
        assert!(verify_proof(&k, &c_b, &c_a, "peer-B", &p));
        // Wrong peer id → reject.
        assert!(!verify_proof(&k, &c_b, &c_a, "peer-Evil", &p));
        // Wrong key → reject.
        let k2 = derive_sync_key(b"other-vault-key");
        assert!(!verify_proof(&k2, &c_b, &c_a, "peer-B", &p));
    }

    #[test]
    fn session_keys_match_regardless_of_direction() {
        let k = derive_sync_key(b"key");
        let c_a = [3u8; 32];
        let c_b = [9u8; 32];
        assert_eq!(session_key(&k, &c_a, &c_b), session_key(&k, &c_b, &c_a));
        let k2 = derive_sync_key(b"key2");
        assert_ne!(session_key(&k, &c_a, &c_b), session_key(&k2, &c_a, &c_b));
    }

    #[test]
    fn encrypt_decrypt_round_trip_and_tamper_detection() {
        let k = session_key(&derive_sync_key(b"key"), &[4u8; 32], &[5u8; 32]);
        let nonce = [7u8; 24];
        let enc = encrypt(&k, &nonce, b"hello snapshot payload").unwrap();
        assert_eq!(decrypt(&k, &nonce, &enc).unwrap(), b"hello snapshot payload");
        // Flip one bit → MAC fails.
        let mut tampered = enc.clone();
        tampered[30] ^= 0x01;
        assert!(decrypt(&k, &nonce, &tampered).is_err());
        // Wrong key → MAC fails.
        let k2 = session_key(&derive_sync_key(b"other"), &[4u8; 32], &[5u8; 32]);
        assert!(decrypt(&k2, &nonce, &enc).is_err());
    }
}
