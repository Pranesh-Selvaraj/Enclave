//! One-time account pairing — transfer the vault seed phrase to a phone.
//!
//! The phone has no vault key yet, so this cannot ride the authenticated sync
//! transport. The desktop instead opens a one-time TCP listener, shows a
//! high-entropy code, and encrypts the seed phrase with a key derived from
//! that code (HKDF-SHA256 → XChaCha20-Poly1305, both already in the tree via
//! `core-network::crypto`).
//!
//! Properties: single use, 2-minute TTL, burned after five wrong attempts.
//! The code is 12 Crockford-base32 characters = 60 bits; a 6-digit PIN would
//! be trivially brute-forceable offline from a captured ciphertext. Pair on a
//! network you trust — the exchange is encrypted but not authenticated.

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

use core_network::crypto::{decrypt, derive_key, encrypt};

use crate::{CoreError, CoreResult};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CODE_LEN: usize = 12;
const MAX_ATTEMPTS: u32 = 5;
const TTL: Duration = Duration::from_secs(120);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(6);
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const PAIR_INFO: &[u8] = b"enclave-pair-v1";
const MAX_PAYLOAD: usize = 4096;

/// What the desktop shows the user (and the phone types in).
#[derive(Debug, Clone, serde::Serialize)]
pub struct PairingStart {
    pub code: String,
    pub address: String,
    pub expires_in: u64,
}

/// Normalize user input: uppercase, drop separators, map Crockford
/// look-alikes (`I`/`L` → `1`, `O` → `0`).
pub fn normalize_code(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| match c.to_ascii_uppercase() {
            'I' | 'L' => '1',
            'O' => '0',
            other => other,
        })
        .collect()
}

fn random_code() -> String {
    let mut out = String::with_capacity(CODE_LEN);
    for _ in 0..CODE_LEN {
        out.push(ALPHABET[(rand::random::<u8>() as usize) & 31] as char);
    }
    out
}

/// Constant-time comparison for the pairing code.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

enum Handled {
    /// Correct code, seed delivered — the session is spent.
    Delivered,
    /// Wrong code (or a malformed hello) — the listener stays open.
    Rejected,
}

async fn handle_client(stream: &mut TcpStream, code: &str, seed: &str) -> Handled {
    let mut line = String::new();
    let read = {
        let mut reader = BufReader::new(&mut *stream);
        tokio::time::timeout(READ_TIMEOUT, reader.read_line(&mut line)).await
    };
    let got = normalize_code(line.trim());
    if read.is_err() || !ct_eq(got.as_bytes(), code.as_bytes()) {
        let _ = stream.write_all(b"ERR\n").await;
        let _ = stream.flush().await;
        return Handled::Rejected;
    }

    // salt(16) || nonce(24) || AEAD(seed)
    let salt: [u8; 16] = rand::random();
    let nonce: [u8; 24] = rand::random();
    let key = derive_key(code.as_bytes(), &salt, PAIR_INFO);
    let Ok(ciphertext) = encrypt(&key, &nonce, seed.as_bytes()) else {
        let _ = stream.write_all(b"ERR\n").await;
        return Handled::Rejected;
    };
    let mut payload = Vec::with_capacity(salt.len() + nonce.len() + ciphertext.len());
    payload.extend_from_slice(&salt);
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ciphertext);

    let _ = stream.write_all(b"OK\n").await;
    let _ = stream.write_all(&(payload.len() as u32).to_le_bytes()).await;
    let _ = stream.write_all(&payload).await;
    let _ = stream.flush().await;
    Handled::Delivered
}

/// Bind a one-time listener, return what to show, and serve it in the
/// background until redeemed, cancelled or expired.
pub async fn serve(seed: String, mut cancel: oneshot::Receiver<()>) -> CoreResult<PairingStart> {
    let listener = TcpListener::bind(("0.0.0.0", 0))
        .await
        .map_err(|e| CoreError::Network(format!("pairing listener failed: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| CoreError::Network(format!("pairing listener failed: {e}")))?
        .port();
    let address = format!(
        "{}:{port}",
        core_network::local_ip().unwrap_or_else(|_| "127.0.0.1".to_string())
    );
    let code = random_code();
    let task_code = code.clone();

    tokio::spawn(async move {
        let deadline = tokio::time::sleep_until(tokio::time::Instant::now() + TTL);
        tokio::pin!(deadline);
        let mut attempts = 0u32;
        loop {
            tokio::select! {
                _ = &mut cancel => break,
                _ = &mut deadline => break,
                accepted = listener.accept() => {
                    let Ok((mut stream, _)) = accepted else { break };
                    match handle_client(&mut stream, &task_code, &seed).await {
                        Handled::Delivered => break,
                        Handled::Rejected => {
                            attempts += 1;
                            if attempts >= MAX_ATTEMPTS { break; }
                        }
                    }
                }
            }
        }
        drop(listener);
    });

    Ok(PairingStart {
        code,
        address,
        expires_in: TTL.as_secs(),
    })
}

/// Phone side: connect, prove the code, decrypt the seed phrase.
pub async fn redeem(code: &str, address: &str) -> CoreResult<String> {
    let code = normalize_code(code);
    if code.len() != CODE_LEN {
        return Err(CoreError::InvalidInput(format!(
            "Enter the {CODE_LEN}-character pairing code"
        )));
    }
    let mut stream = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(address))
        .await
        .map_err(|_| CoreError::Network("Pairing timed out — check the address".into()))?
        .map_err(|e| CoreError::Network(format!("Cannot reach the desktop: {e}")))?;

    stream
        .write_all(format!("{code}\n").as_bytes())
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    stream
        .flush()
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;

    // One buffered reader for status + length + payload — a second reader
    // would drop bytes already read ahead into its buffer.
    let mut reader = BufReader::new(stream);
    let mut status = String::new();
    tokio::time::timeout(READ_TIMEOUT, reader.read_line(&mut status))
        .await
        .map_err(|_| CoreError::Network("Pairing timed out — no reply".into()))?
        .map_err(|e| CoreError::Network(e.to_string()))?;
    if !status.starts_with("OK") {
        return Err(CoreError::InvalidInput(
            "Wrong or expired pairing code".into(),
        ));
    }

    let mut len_buf = [0u8; 4];
    reader
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len < 16 + 24 || len > MAX_PAYLOAD {
        return Err(CoreError::InvalidInput("Corrupt pairing payload".into()));
    }
    let mut payload = vec![0u8; len];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|e| CoreError::Network(e.to_string()))?;

    let salt = &payload[..16];
    let nonce: [u8; 24] = payload[16..40].try_into().expect("24-byte nonce slice");
    let ciphertext = &payload[40..];
    let key = derive_key(code.as_bytes(), salt, PAIR_INFO);
    let seed = decrypt(&key, &nonce, ciphertext)
        .map_err(|_| CoreError::InvalidInput("Wrong or expired pairing code".into()))?;
    String::from_utf8(seed).map_err(|_| CoreError::InvalidInput("Corrupt pairing payload".into()))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[tokio::test]
    async fn round_trip_and_single_use() {
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let start = serve(SEED.to_string(), cancel_rx).await.unwrap();

        let got = redeem(&start.code, &start.address).await.unwrap();
        assert_eq!(got, SEED);

        // The listener is spent: a second redemption can never succeed.
        let mut second = Err(CoreError::VaultLocked);
        for _ in 0..20 {
            second = redeem(&start.code, &start.address).await;
            if second.is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(second.is_err(), "pairing must be single-use");
        let _ = cancel_tx.send(());
    }

    #[tokio::test]
    async fn wrong_code_is_rejected_and_right_code_still_works() {
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let start = serve("seed words".to_string(), cancel_rx).await.unwrap();

        let mut wrong = start.code.clone().into_bytes();
        wrong[0] = if wrong[0] == b'0' { b'1' } else { b'0' };
        let wrong = String::from_utf8(wrong).unwrap();
        assert!(redeem(&wrong, &start.address).await.is_err());

        // A wrong attempt must not burn the session.
        assert_eq!(
            redeem(&start.code, &start.address).await.unwrap(),
            "seed words"
        );
        let _ = cancel_tx.send(());
    }

    #[tokio::test]
    async fn cancel_closes_the_listener() {
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let start = serve("seed words".to_string(), cancel_rx).await.unwrap();
        let _ = cancel_tx.send(());
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(redeem(&start.code, &start.address).await.is_err());
    }

    #[test]
    fn code_normalizes_lookalikes_and_separators() {
        assert_eq!(normalize_code("abcd-efgh jk1m"), "ABCDEFGHJK1M");
        assert_eq!(normalize_code("i0ol"), "1001");
    }
}
