//! WebSocket server accept loop for P2P sync transport.
//! Each accepted peer connection is handled in its own task.

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::PeerMessage;

/// Accept loop — spawns a handler per incoming connection. Shuts down when
/// the watch channel signals.
pub async fn accept_loop(
    listener: TcpListener,
    mut shutdown: tokio::sync::watch::Receiver<bool>,
    local_peer_id: String,
    message_tx: mpsc::UnboundedSender<PeerMessage>,
) {
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        let peer_id = addr.to_string(); // use address as temp peer ID
                        let tx = message_tx.clone();
                        let local = local_peer_id.clone();
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, local, peer_id.clone(), tx).await {
                                eprintln!("WS peer {peer_id} error: {e}");
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

async fn handle_connection(
    stream: tokio::net::TcpStream,
    _local_id: String,
    peer_addr: String,
    message_tx: mpsc::UnboundedSender<PeerMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Forward encrypted sync blob to frontend
                        let _ = message_tx.send(PeerMessage {
                            from_peer: peer_addr.clone(),
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
        }
    }

    // ponytail: send a close frame back (if still connected)
    let _ = write.close().await;
    Ok(())
}
