//! Vault cryptography — the exact contract the desktop (TypeScript) side uses,
//! reimplemented in Rust so the native Android shell can create, unlock and
//! password-protect the same vaults.
//!
//! Parameters are frozen; a mismatch would silently produce a different master
//! key and make desktop↔mobile vaults unreadable to each other:
//!   - Argon2id, 64 MiB, 3 iterations, 4 lanes, 32-byte output
//!   - master key = Argon2id(mnemonic, "enclave-vault-master-key-v1")
//!   - vault.key  = [salt_len][salt][iv_len][iv][AES-256-GCM ciphertext]
//!     where the AES key is Argon2id(password, random salt).
//!
//! Cross-compatibility is covered by a fixture generated with the TypeScript
//! implementation (see the tests at the bottom).

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;

use crate::{CoreError, CoreResult};

// ── Frozen parameters (must mirror packages/crypto/src/index.ts) ────────────

pub const ARGON2_MEMORY_KIB: u32 = 65536; // 64 MiB
pub const ARGON2_ITERATIONS: u32 = 3;
pub const ARGON2_PARALLELISM: u32 = 4;
pub const KEY_LENGTH: usize = 32;
/// Deterministic app-level salt for the master key (versioned for rotation).
pub const MASTER_KEY_SALT: &[u8] = b"enclave-vault-master-key-v1";

const SALT_LENGTH: usize = 32;
const IV_LENGTH: usize = 12;

// ── Key derivation ──────────────────────────────────────────────────────────

/// Derive a 256-bit key from any password + salt using Argon2id.
pub fn derive_key_bytes(password: &[u8], salt: &[u8]) -> CoreResult<[u8; KEY_LENGTH]> {
    let params = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        Some(KEY_LENGTH),
    )
    .map_err(|e| CoreError::InvalidInput(format!("invalid Argon2 parameters: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; KEY_LENGTH];
    argon
        .hash_password_into(password, salt, &mut out)
        .map_err(|e| CoreError::InvalidInput(format!("key derivation failed: {e}")))?;
    Ok(out)
}

/// Derive the vault master key from a BIP39 mnemonic.
pub fn derive_master_key(mnemonic: &str) -> CoreResult<[u8; KEY_LENGTH]> {
    derive_key_bytes(mnemonic.as_bytes(), MASTER_KEY_SALT)
}

// ── BIP39 mnemonics ─────────────────────────────────────────────────────────

/// Generate a fresh 12-word BIP39 English mnemonic (128 bits of entropy).
pub fn generate_mnemonic() -> CoreResult<String> {
    let mut entropy = [0u8; 16];
    rand::rng().fill_bytes(&mut entropy);
    bip39::Mnemonic::from_entropy_in(bip39::Language::English, &entropy)
        .map(|m| m.to_string())
        .map_err(|e| CoreError::InvalidInput(format!("mnemonic generation failed: {e}")))
}

/// Validate a BIP39 English mnemonic (wordlist + checksum).
pub fn validate_mnemonic(mnemonic: &str) -> bool {
    bip39::Mnemonic::parse_in(bip39::Language::English, mnemonic).is_ok()
}

// ── Vault key file ──────────────────────────────────────────────────────────

pub struct EncryptedNote {
    pub salt: [u8; SALT_LENGTH],
    pub iv: [u8; IV_LENGTH],
    pub ciphertext: Vec<u8>,
}

/// `[salt_len][salt][iv_len][iv][ciphertext]` — the on-disk vault.key format.
pub fn serialize_note(note: &EncryptedNote) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + note.salt.len() + note.iv.len() + note.ciphertext.len());
    out.push(note.salt.len() as u8);
    out.extend_from_slice(&note.salt);
    out.push(note.iv.len() as u8);
    out.extend_from_slice(&note.iv);
    out.extend_from_slice(&note.ciphertext);
    out
}

pub fn deserialize_note(data: &[u8]) -> CoreResult<EncryptedNote> {
    if data.len() < 3 {
        return Err(CoreError::InvalidInput("Corrupt vault key file".into()));
    }
    let salt_len = data[0] as usize;
    if 1 + salt_len >= data.len() {
        return Err(CoreError::InvalidInput("Corrupt vault key file".into()));
    }
    let iv_len = data[1 + salt_len] as usize;
    if 2 + salt_len + iv_len > data.len() {
        return Err(CoreError::InvalidInput("Corrupt vault key file".into()));
    }
    let mut salt = [0u8; SALT_LENGTH];
    if salt_len != SALT_LENGTH {
        return Err(CoreError::InvalidInput("Corrupt vault key file".into()));
    }
    salt.copy_from_slice(&data[1..1 + salt_len]);
    let mut iv = [0u8; IV_LENGTH];
    if iv_len != IV_LENGTH {
        return Err(CoreError::InvalidInput("Corrupt vault key file".into()));
    }
    iv.copy_from_slice(&data[2 + salt_len..2 + salt_len + iv_len]);
    Ok(EncryptedNote {
        salt,
        iv,
        ciphertext: data[2 + salt_len + iv_len..].to_vec(),
    })
}

/// Encrypt a mnemonic with a password → serialized vault.key bytes.
pub fn encrypt_with_password(plaintext: &str, password: &str) -> CoreResult<Vec<u8>> {
    let mut salt = [0u8; SALT_LENGTH];
    rand::rng().fill_bytes(&mut salt);
    let key = derive_key_bytes(password.as_bytes(), &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CoreError::InvalidInput(format!("cipher init failed: {e}")))?;
    let mut iv = [0u8; IV_LENGTH];
    rand::rng().fill_bytes(&mut iv);
    let ciphertext = cipher
        .encrypt(iv.as_slice().into(), plaintext.as_bytes())
        .map_err(|_| CoreError::InvalidInput("Encryption failed".into()))?;
    Ok(serialize_note(&EncryptedNote { salt, iv, ciphertext }))
}

/// Decrypt serialized vault.key bytes with a password.
pub fn decrypt_with_password(data: &[u8], password: &str) -> CoreResult<String> {
    let note = deserialize_note(data)?;
    let key = derive_key_bytes(password.as_bytes(), &note.salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CoreError::InvalidInput(format!("cipher init failed: {e}")))?;
    let plaintext = cipher
        .decrypt(note.iv.as_slice().into(), note.ciphertext.as_slice())
        .map_err(|_| CoreError::InvalidInput("Wrong password or recovery phrase.".into()))?;
    String::from_utf8(plaintext).map_err(|_| CoreError::InvalidInput("Corrupt vault key file".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonic_round_trip_and_validation() {
        let m = generate_mnemonic().unwrap();
        assert_eq!(m.split(' ').count(), 12);
        assert!(validate_mnemonic(&m));
        assert!(!validate_mnemonic("not a real mnemonic phrase at all"));
    }

    #[test]
    fn master_key_is_deterministic() {
        let m = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let k1 = derive_master_key(m).unwrap();
        let k2 = derive_master_key(m).unwrap();
        assert_eq!(k1, k2);
        // Different mnemonic → different key.
        let other = generate_mnemonic().unwrap();
        assert_ne!(k1, derive_master_key(&other).unwrap());
    }

    #[test]
    fn password_note_round_trip() {
        let m = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let blob = encrypt_with_password(m, "hunter2-correct-horse").unwrap();
        assert_eq!(decrypt_with_password(&blob, "hunter2-correct-horse").unwrap(), m);

        // Wrong password and corrupt data must fail cleanly.
        assert!(decrypt_with_password(&blob, "wrong").is_err());
        assert!(decrypt_with_password(&blob[..5], "hunter2-correct-horse").is_err());
    }

    /// Compatibility proof for the native shell: this fixture was generated by
    /// the *TypeScript* implementation (`packages/crypto`):
    ///   mnemonic  = "abandon … about"
    ///   password  = "enclave-native-test"
    ///   masterKey = 116a2dfb404dcbc18265c08aff9030c04b335dd7f34704553899e8558a269d3a
    /// Rust must decrypt the TS-produced vault.key and derive the same master
    /// key, or desktop and Android vaults would not interoperate.
    #[test]
    fn decrypts_typescript_generated_vault_key() {
        const TS_BLOB_HEX: &str = "20c9bb8e615633f2fc3792079806e77bccadc458acea943a30a091d40ac8e2fdde0c8ad1796921fa8f1dfa585176d51bba790142ee80c07f99e5ecad2e9ae9e4aa11a70bc224d31c0141f8e99c95d9fea6a7feb54e9e06d5d2724d4375ebc3841d6a163a21dc7eed700505c7240b9b2ddbff9e4555004ee20778d8c0714e46510c64fa4d238d467e8bd178570c1ff1d04f8c83c79447ebf4207a93";
        const TS_MASTER_KEY_HEX: &str = "116a2dfb404dcbc18265c08aff9030c04b335dd7f34704553899e8558a269d3a";
        const MNEMONIC: &str =
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        const PASSWORD: &str = "enclave-native-test";

        let blob: Vec<u8> = (0..TS_BLOB_HEX.len() / 2)
            .map(|i| u8::from_str_radix(&TS_BLOB_HEX[i * 2..i * 2 + 2], 16).unwrap())
            .collect();
        assert_eq!(decrypt_with_password(&blob, PASSWORD).unwrap(), MNEMONIC);

        let expected: [u8; 32] = {
            let mut out = [0u8; 32];
            for i in 0..32 {
                out[i] = u8::from_str_radix(&TS_MASTER_KEY_HEX[i * 2..i * 2 + 2], 16).unwrap();
            }
            out
        };
        assert_eq!(derive_master_key(MNEMONIC).unwrap(), expected, "master key mismatch with TS");
    }
}
