//! Local-network P2P transport for Enclave.
//!
//! mDNS service discovery + encrypted WebSocket channels.
//! No internet, no relay — strictly LAN device-to-device.

mod mdns;
mod ws;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use uuid::Uuid;

// ── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Peer {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub connected: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkStatus {
    pub local_peer_id: String,
    pub running: bool,
    pub port: u16,
    pub peers: Vec<Peer>,
}

// ── Internal State ──────────────────────────────────────────────────────────

struct Inner {
    peer_id: String,
    port: u16,
    peers: HashMap<String, Peer>,
    mdns_handle: Option<mdns::MdnsHandle>,
    ws_shutdown: Option<tokio::sync::watch::Sender<bool>>,
}

pub struct NetworkState {
    inner: Arc<RwLock<Inner>>,
    /// Messages from peers forwarded to the frontend via Tauri events.
    pub message_rx: Mutex<mpsc::UnboundedReceiver<PeerMessage>>,
    message_tx: mpsc::UnboundedSender<PeerMessage>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerMessage {
    pub from_peer: String,
    pub payload: String,
}

impl NetworkState {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let peer_id = Uuid::new_v4().to_string();
        Self {
            inner: Arc::new(RwLock::new(Inner {
                peer_id: peer_id.clone(),
                port: 0,
                peers: HashMap::new(),
                mdns_handle: None,
                ws_shutdown: None,
            })),
            message_rx: Mutex::new(rx),
            message_tx: tx,
        }
    }

    pub async fn status(&self) -> NetworkStatus {
        let inner = self.inner.read().await;
        NetworkStatus {
            local_peer_id: inner.peer_id.clone(),
            running: inner.mdns_handle.is_some(),
            port: inner.port,
            peers: inner.peers.values().cloned().collect(),
        }
    }

    /// Start mDNS advertising + WebSocket listener on an OS-assigned port.
    pub async fn start(&self) -> Result<(), String> {
        let mut inner = self.inner.write().await;
        if inner.mdns_handle.is_some() {
            return Err("Network already running".into());
        }

        let peer_id = inner.peer_id.clone();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let peer_tx = self.message_tx.clone();

        // Bind WebSocket server on port 0 (OS picks a free port)
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Failed to bind: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        inner.port = port;
        inner.ws_shutdown = Some(shutdown_tx);

        // Spawn WS accept loop (clone what it needs)
        let ws_peer_id = peer_id.clone();
        let shutdown = shutdown_rx;
        tokio::spawn(async move {
            ws::accept_loop(listener, shutdown, ws_peer_id, peer_tx).await;
        });

        // Start mDNS
        let mdns_handle = mdns::start(peer_id.clone(), port).await?;
        inner.mdns_handle = Some(mdns_handle);

        Ok(())
    }

    /// Stop the network (mDNS + WS server).
    pub async fn stop(&self) -> Result<(), String> {
        let mut inner = self.inner.write().await;
        if let Some(handle) = inner.mdns_handle.take() {
            mdns::stop(handle)?;
        }
        if let Some(tx) = inner.ws_shutdown.take() {
            let _ = tx.send(true);
        }
        inner.peers.clear();
        inner.port = 0;
        Ok(())
    }

    /// Send an encrypted sync message to a connected peer.
    pub async fn send_to_peer(&self, peer_id: &str, payload: &str) -> Result<(), String> {
        // For now, this is handled by the WS layer keeping connections.
        // Future: maintain a connection map per peer.
        let _ = peer_id;
        let _ = payload;
        Ok(())
    }
}
