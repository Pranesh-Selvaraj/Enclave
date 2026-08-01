//! WebSocket server + client sessions for P2P sync transport.
//!
//! A session is a duplex pipe: inbound text frames go to the app layer via
//! the message channel, outbound JSON payloads come from the app layer via
//! an mpsc channel registered in the session map. Sessions start by sending
//! a hello so both sides learn each other's peer id + device name.

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use crate::{NetworkState, PeerMessage};

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

/// The shared duplex session loop for accepted + dialed connections.
async fn session<S>(
    ws: WebSocketStream<S>,
    net: std::sync::Arc<NetworkState>,
    peer_addr: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let local_peer_id = net.local_peer_id();
    let local_name = net.local_name();
    let (mut write, mut read) = ws.split();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<String>();
    let message_tx = net.message_tx();

    // Hello first — the app layer only talks to peers it has heard from.
    let hello = serde_json::json!({
        "kind": "hello",
        "peer_id": local_peer_id,
        "name": local_name,
    });
    write.send(Message::Text(hello.to_string().into())).await?;

    // Register our outbound sender once we learn the peer's id (on its hello).
    let mut registered: Option<String> = None;
    let mut conn_id = peer_addr.clone();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
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
                        if write.send(Message::Text(payload.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        }
    }

    if let Some(pid) = registered {
        net.forget_session(&pid).await;
    }
    let _ = write.close().await;
    Ok(())
}
