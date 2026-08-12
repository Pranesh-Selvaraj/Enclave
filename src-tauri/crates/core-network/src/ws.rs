//! WebSocket server + client sessions for P2P sync transport.
//!
//! A session is a duplex pipe: inbound frames go to the app layer via the
//! message channel, outbound JSON payloads come from the app layer via an
//! mpsc channel registered in the session map.
//!
//! Security: every session first runs a mutual-auth handshake (challenge /
//! HMAC proof over the vault-derived sync key — see crypto.rs). Only after
//! both proofs verify do peers exchange a hello and app payloads, and every
//! frame is then encrypted with XChaCha20-Poly1305 under a per-session key.
//! A wrong key, a forged frame, or an unauthenticated first frame kills the
//! session — non-owners on the LAN can neither read nor join.

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::{crypto, NetworkState, PeerMessage};

/// Upper bound for the auth handshake (challenge + proof exchange).
const HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf).map_err(|e| format!("getrandom failed: {e}"))?;
    Ok(buf)
}

/// Accept loop — spawns a session per incoming connection. Shuts down when
/// the watch channel signals.
pub async fn accept_loop(
    listener: TcpListener,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    net: std::sync::Arc<NetworkState>,
) {
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        let net = net.clone();
                        tokio::spawn(async move {
                            let ws = match tokio_tungstenite::accept_async(stream).await {
                                Ok(ws) => ws,
                                Err(e) => {
                                    eprintln!("WS accept from {addr} failed: {e}");
                                    return;
                                }
                            };
                            if let Err(e) = session(ws, net, addr.to_string()).await {
                                eprintln!("WS peer {addr} error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("WS accept error: {e}");
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    break;
                }
            }
        }
    }
}

/// Dial a discovered peer and run a session against it (spawned — this
/// returns as soon as the socket + handshake are up).
pub async fn connect(
    peer_id: &str,
    host: &str,
    port: u16,
    net: std::sync::Arc<NetworkState>,
) -> Result<(), String> {
    let url = format!("ws://{host}:{port}");
    let ws = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|e| format!("{url}: {e}"))?
        .0;
    let net = net.clone();
    let peer_addr = format!("{peer_id}@{url}");
    tokio::spawn(async move {
        if let Err(e) = session(ws, net, peer_addr.clone()).await {
            eprintln!("WS peer {peer_addr} error: {e}");
        }
    });
    Ok(())
}

/// One step of the handshake: wait for a frame of `kind`, returning its
/// fields. Any other first frame, a timeout, or a socket error aborts.
async fn await_frame(
    read: &mut (impl futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
    kind: &str,
) -> Result<serde_json::Value, String> {
    tokio::time::timeout(HANDSHAKE_TIMEOUT, async {
        loop {
            match read.next().await {
                Some(Ok(Message::Text(t))) => {
                    let v: serde_json::Value =
                        serde_json::from_str(&t).map_err(|e| e.to_string())?;
                    if v["kind"] == kind {
                        return Ok(v);
                    }
                    return Err(format!("expected {kind} frame, got {}", v["kind"]));
                }
                Some(Ok(_)) => return Err(format!("expected text {kind} frame")),
                Some(Err(e)) => return Err(e.to_string()),
                None => return Err("peer closed during handshake".into()),
            }
        }
    })
    .await
    .map_err(|_| "auth handshake timed out".to_string())?
}

fn b64(v: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(v)
}

fn unb64(s: &str, expected: usize) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD.decode(s)?;
    let arr: [u8; 32] = raw
        .try_into()
        .map_err(|_| format!("expected {expected}-byte challenge"))?;
    Ok(arr)
}

/// The shared duplex session loop for accepted + dialed connections.
///
/// Phase 1 — mutual auth: both sides send a challenge, prove knowledge of
/// the sync key, and derive a per-session key. Phase 2 — encrypted
/// hello + app payloads.
async fn session<S>(
    ws: WebSocketStream<S>,
    net: std::sync::Arc<NetworkState>,
    peer_addr: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let sync_key = net
        .sync_key()
        .await
        .ok_or("network not started with a sync key")?;
    let local_peer_id = net.local_peer_id();
    let local_name = net.local_name();
    let (mut write, mut read) = ws.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
    let message_tx = net.message_tx();

    // ── Auth phase (plaintext, but only challenges + proofs — no data) ──
    let my_challenge = random_bytes::<32>()?;
    let auth = serde_json::json!({
        "kind": "auth",
        "peer_id": local_peer_id,
        "name": local_name,
        "challenge": b64(&my_challenge),
    });
    write.send(Message::Text(auth.to_string().into())).await?;

    let peer_auth = await_frame(&mut read, "auth").await?;
    let peer_id = peer_auth["peer_id"]
        .as_str()
        .ok_or("auth missing peer_id")?
        .to_string();
    let peer_challenge = unb64(
        peer_auth["challenge"].as_str().ok_or("auth missing challenge")?,
        32,
    )?;

    let my_proof = crypto::proof(&sync_key, &my_challenge, &peer_challenge, &local_peer_id);
    let proof_msg = serde_json::json!({
        "kind": "auth_proof",
        "peer_id": local_peer_id,
        "proof": b64(&my_proof),
    });
    write.send(Message::Text(proof_msg.to_string().into())).await?;

    let peer_proof_frame = await_frame(&mut read, "auth_proof").await?;
    let peer_proof = unb64(
        peer_proof_frame["proof"].as_str().ok_or("auth_proof missing proof")?,
        32,
    )?;
    if !crypto::verify_proof(&sync_key, &peer_challenge, &my_challenge, &peer_id, &peer_proof) {
        return Err("peer failed auth (wrong vault key?)".into());
    }
    let session_key = crypto::session_key(&sync_key, &my_challenge, &peer_challenge);

    // ── Data phase (encrypted) — hello first, then app payloads ──
    let hello = serde_json::json!({
        "kind": "hello",
        "peer_id": local_peer_id,
        "name": local_name,
    });
    let nonce = random_bytes::<24>()?;
    let enc = crypto::encrypt(&session_key, &nonce, hello.to_string().as_bytes())?;
    let mut frame = nonce.to_vec();
    frame.extend_from_slice(&enc);
    write.send(Message::Binary(frame.into())).await?;

    // Register our outbound sender once we learn the peer's id (on its hello).
    let mut registered: Option<String> = None;
    let mut conn_id = peer_addr.clone();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Binary(bytes))) => {
                        // Any frame that fails the MAC (forgery, tamper,
                        // wrong key) kills the session.
                        if bytes.len() < 24 + 16 {
                            break;
                        }
                        let (nonce, ct) = bytes.split_at(24);
                        let nonce_arr: [u8; 24] = match nonce.try_into() {
                            Ok(n) => n,
                            Err(_) => break,
                        };
                        let plain = match crypto::decrypt(&session_key, &nonce_arr, ct) {
                            Ok(p) => p,
                            Err(_) => break,
                        };
                        let text = String::from_utf8_lossy(&plain);
                        let parsed = serde_json::from_str::<serde_json::Value>(&text).ok();
                        if let Some(v) = &parsed {
                            if v["kind"] == "hello" {
                                if let (Some(pid), Some(name)) = (v["peer_id"].as_str(), v["name"].as_str()) {
                                    net.register_session(pid, name, out_tx.clone()).await;
                                    registered = Some(pid.to_string());
                                    conn_id = pid.to_string();
                                }
                            }
                        }
                        let _ = message_tx.send(PeerMessage {
                            from_peer: conn_id.clone(),
                            payload: text.to_string(),
                        });
                    }
                    // Post-auth text frames are a protocol violation.
                    Some(Ok(Message::Text(_))) => break,
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(e)) => {
                        eprintln!("WS read error: {e}");
                        break;
                    }
                    _ => {}
                }
            }
            out = out_rx.recv() => {
                match out {
                    Some(payload) => {
                        let nonce = match random_bytes::<24>() {
                            Ok(n) => n,
                            Err(_) => break,
                        };
                        let enc = match crypto::encrypt(&session_key, &nonce, payload.as_bytes()) {
                            Ok(e) => e,
                            Err(_) => break,
                        };
                        let mut frame = nonce.to_vec();
                        frame.extend_from_slice(&enc);
                        if write.send(Message::Binary(frame.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    if let Some(pid) = registered {
        net.forget_session(&pid, &out_tx);
    }
    let _ = write.close().await;
    Ok(())
}
