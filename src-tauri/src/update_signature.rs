//! Ed25519 verification of signed release assets (issue #83).
//!
//! Deliberately free of Tauri and network types so the crypto is unit-testable
//! in every environment with `cargo test`, and so the exact same function the
//! updater calls is the one tested here.
//!
//! Signature format: for each release asset `<name>`, the release carries
//! `<name>.sig` containing the hex Ed25519 signature (64 bytes) over the
//! lowercase SHA-256 hex digest of the asset. Signing the digest — not the
//! file — means the updater never has to hold the whole installer in memory
//! again after streaming it to disk.

use ed25519_dalek::{Signature, VerifyingKey};

/// Pinned Ed25519 release-signing public key (32 bytes, hex).
///
/// Rotating it requires republishing this constant (and therefore an update
/// signed by the *current* key, or a manual install) plus a new
/// `UPDATE_SIGNING_KEY` CI secret. Keep the private key offline; see
/// `scripts/sign-release.mjs`.
pub const UPDATE_PUBKEY_HEX: &str =
    "966b2a4e0f69852842e9fd2b916724f9e5288fa9c8eea18b235a35a50b593e90";

/// Decode a hex string (even length, either case). Trims surrounding
/// whitespace so a `.sig` file with a trailing newline verifies.
pub fn decode_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Verify a detached Ed25519 signature over an asset's SHA-256 hex digest.
/// Returns a user-facing error on any malformed input or mismatch (fail
/// closed); never panics.
pub fn verify_update_signature(
    digest_hex: &str,
    sig_hex: &str,
    pubkey_hex: &str,
) -> Result<(), String> {
    let key_bytes = decode_hex(pubkey_hex).ok_or("malformed update public key")?;
    let key_bytes: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| "malformed update public key")?;
    let key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| format!("invalid update public key: {e}"))?;
    let sig_bytes = decode_hex(sig_hex).ok_or("malformed update signature")?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| "malformed update signature")?;
    key.verify_strict(digest_hex.as_bytes(), &sig)
        .map_err(|_| "Update signature check failed — the download was discarded".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// sha256("abc") — a digest whose value is fixed and public, so the tests
    /// need no hash dependency.
    const DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn keypair() -> (String, String) {
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let pub_hex: String = signing
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let sig_hex: String = signing
            .sign(DIGEST.as_bytes())
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        (pub_hex, sig_hex)
    }

    #[test]
    fn accepts_a_signature_over_the_exact_digest() {
        let (pub_hex, sig_hex) = keypair();
        assert!(verify_update_signature(DIGEST, &sig_hex, &pub_hex).is_ok());
        // A `.sig` file with a trailing newline must still verify.
        assert!(verify_update_signature(DIGEST, &format!("{sig_hex}\n"), &pub_hex).is_ok());
    }

    #[test]
    fn rejects_a_different_digest_or_key() {
        let (pub_hex, sig_hex) = keypair();
        let other_digest = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(verify_update_signature(other_digest, &sig_hex, &pub_hex).is_err());

        let other_pub: String = SigningKey::from_bytes(&[9u8; 32])
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(verify_update_signature(DIGEST, &sig_hex, &other_pub).is_err());
    }

    #[test]
    fn malformed_inputs_fail_closed() {
        let (pub_hex, sig_hex) = keypair();
        assert!(verify_update_signature(DIGEST, "not-hex", &pub_hex).is_err());
        assert!(verify_update_signature(DIGEST, "", &pub_hex).is_err());
        assert!(verify_update_signature(DIGEST, &sig_hex, "not-hex").is_err());
        assert!(verify_update_signature(DIGEST, &sig_hex, "").is_err());
        // 31-byte key (odd valid prefix) is not a VerifyingKey length.
        assert!(verify_update_signature(DIGEST, &sig_hex, &pub_hex[..62]).is_err());
    }

    #[test]
    fn pinned_update_key_is_a_valid_ed25519_point() {
        let bytes = decode_hex(UPDATE_PUBKEY_HEX).expect("pinned public key is hex");
        assert_eq!(bytes.len(), 32, "ed25519 public keys are 32 bytes");
        let key: [u8; 32] = bytes.try_into().unwrap();
        VerifyingKey::from_bytes(&key).expect("pinned public key is a valid point");
    }
}
