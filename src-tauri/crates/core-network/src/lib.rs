//! Local-network P2P transport for Enclave.
//!
//! mDNS service discovery + WebSocket channels (LAN only, no internet).
//! Peers exchange hello messages (peer id + device name), then the app
//! layer exchanges sync snapshots over the same socket.

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
    pub name: String,
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
    name: String,
    peers: HashMap<String, Peer>,
    /// Established sessions, keyed by peer id (registered on hello).
    sessions: HashMap<String, mpsc::UnboundedSender<String>>,
    mdns_handle: Option<mdns::MdnsHandle>,
    ws_shutdown: Option<tokio::sync::watch::Sender<bool>>,
}

pub struct NetworkState {
    inner: Arc<RwLock<Inner>>,
    /// Messages from peers forwarded to the app layer. The app takes the
    /// receiver out once at setup.
    pub message_rx: Mutex<Option<mpsc::UnboundedReceiver<PeerMessage>>>,
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
                name: String::new(),
                peers: HashMap::new(),
                sessions: HashMap::new(),
                mdns_handle: None,
                ws_shutdown: None,
            })),
            message_rx: Mutex::new(Some(rx)),
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

    /// Start mDNS advertising + discovery, WebSocket listener on an
    /// OS-assigned port, and auto-connect to every discovered peer.
    pub async fn start(self: &Arc<Self>, name: &str) -> Result<(), String> {
        let mut inner = self.inner.write().await;
        if inner.mdns_handle.is_some() {
            return Err("Network already running".into());
        }
        inner.name = name.to_string();

        let peer_id = inner.peer_id.clone();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        // Bind WebSocket server on port 0 (OS picks a free port)
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Failed to bind: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        inner.port = port;
        inner.ws_shutdown = Some(shutdown_tx.clone());

        // Spawn WS accept loop
        let shutdown = shutdown_rx;
        let net = self.shared();
        tokio::spawn(async move {
            ws::accept_loop(listener, shutdown, net).await;
        });

        // mDNS browse → connect to discovered peers.
        let (discovery_tx, mut discovery_rx) = mpsc::unbounded_channel::<(String, String, u16)>();
        let mdns_handle = match mdns::start(peer_id.clone(), port, discovery_tx).await {
            Ok(h) => h,
            Err(e) => {
                let _ = shutdown_tx.send(true);
                inner.ws_shutdown = None;
                inner.port = 0;
                return Err(e);
            }
        };
        inner.mdns_handle = Some(mdns_handle);

        let net = self.shared();
        tokio::spawn(async move {
            while let Some((pid, host, port)) = discovery_rx.recv().await {
                {
                    let inner = net.inner().read().await;
                    if pid == inner.peer_id || inner.sessions.contains_key(&pid) {
                        continue;
                    }
                }
                {
                    let mut inner = net.inner().write().await;
                    inner.peers.insert(
                        pid.clone(),
                        Peer { id: pid.clone(), host: host.clone(), port, connected: false, name: String::new() },
                    );
                }
                if let Err(e) = ws::connect(&pid, &host, port, net.clone()).await {
                    eprintln!("WS connect to {pid}@{host}:{port} failed: {e}");
                }
            }
        });

        Ok(())
    }

    /// Send a raw JSON payload to a connected peer. No-op if not connected.
    pub async fn send_to(&self, peer_id: &str, payload: String) {
        let tx = self.inner.read().await.sessions.get(peer_id).cloned();
        if let Some(tx) = tx {
            let _ = tx.send(payload);
        }
    }

    /// Stop the network (mDNS + WS server + client sessions).
    pub async fn stop(&self) -> Result<(), String> {
        let mut inner = self.inner.write().await;
        if let Some(handle) = inner.mdns_handle.take() {
            mdns::stop(handle)?;
        }
        if let Some(tx) = inner.ws_shutdown.take() {
            let _ = tx.send(true);
        }
        inner.sessions.clear();
        inner.peers.clear();
        inner.port = 0;
        Ok(())
    }

    pub(crate) fn shared(self: &Arc<Self>) -> Arc<Self> {
        Arc::clone(self)
    }

    pub(crate) fn inner(&self) -> &Arc<RwLock<Inner>> {
        &self.inner
    }

    pub(crate) fn message_tx(&self) -> mpsc::UnboundedSender<PeerMessage> {
        self.message_tx.clone()
    }

    /// Drop a peer from the session map (called when its socket dies).
    pub(crate) async fn forget_session(&self, peer_id: &str) {
        let mut inner = self.inner.write().await;
        inner.sessions.remove(peer_id);
        if let Some(peer) = inner.peers.get_mut(peer_id) {
            peer.connected = false;
        }
    }

    /// Register an established session under the peer's id.
    pub(crate) async fn register_session(&self, peer_id: &str, name: &str, tx: mpsc::UnboundedSender<String>) {
        let mut inner = self.inner.write().await;
        inner.sessions.insert(peer_id.to_string(), tx);
        if let Some(peer) = inner.peers.get_mut(peer_id) {
            peer.connected = true;
            peer.name = name.to_string();
        }
    }

    pub(crate) fn local_peer_id(&self) -> String {
        self.inner.try_read().map(|g| g.peer_id.clone()).unwrap_or_default()
    }

    pub(crate) fn local_name(&self) -> String {
        self.inner.try_read().map(|g| g.name.clone()).unwrap_or_default()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Full wire path without mDNS: A listens, B dials A, hello exchange
    /// registers both sessions, and app-layer payloads flow both ways.
    #[tokio::test]
    async fn ws_sessions_exchange_hello_and_payloads() {
        let a = Arc::new(NetworkState::new());
        let b = Arc::new(NetworkState::new());
        a.start("alice").await.unwrap();
        let a_status = a.status().await;
        let a_id = a_status.local_peer_id.clone();
        let b_id = b.status().await.local_peer_id.clone();

        ws::connect(&a_id, "127.0.0.1", a_status.port, b.clone())
            .await
            .expect("dial should succeed");

        // First message on A's side is B's hello, from the registered peer id.
        let hello = {
            let mut rx = a.message_rx.lock().await;
            rx.as_mut().unwrap().recv().await.unwrap()
        };
        let hello_json: serde_json::Value = serde_json::from_str(&hello.payload).unwrap();
        assert_eq!(hello_json["kind"], "hello");
        assert_eq!(hello_json["peer_id"], b_id);
        assert_eq!(hello.from_peer, b_id);

        // B's first message is A's hello; consume it before sending.
        {
            let mut rx = b.message_rx.lock().await;
            let hello = rx.as_mut().unwrap().recv().await.unwrap();
            let hello_json: serde_json::Value = serde_json::from_str(&hello.payload).unwrap();
            assert_eq!(hello_json["kind"], "hello");
            assert_eq!(hello_json["peer_id"], a_id);
        }

        // A sends a payload to B (session keyed by peer id, not socket).
        a.send_to(&b_id, "ping".into()).await;
        let payload = {
            let mut rx = b.message_rx.lock().await;
            rx.as_mut().unwrap().recv().await.unwrap()
        };
        assert_eq!(payload.payload, "ping");
        assert_eq!(payload.from_peer, a_id);

        b.stop().await.unwrap();
        a.stop().await.unwrap();
    }
}
