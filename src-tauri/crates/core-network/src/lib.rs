//! Local-network P2P transport for Enclave.
//!
//! mDNS service discovery + WebSocket channels (LAN only, no internet).
//! Peers exchange hello messages (peer id + device name), then the app
//! layer exchanges sync snapshots over the same socket.

pub mod crypto;
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
    #[serde(default)]
    pub hosts: Vec<String>,
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
    /// Epoch millis of the last successful sync (snapshot merge or ack).
    /// None = never synced.
    pub last_sync_at: Option<u64>,
}

// ── Internal State ──────────────────────────────────────────────────────────

struct Inner {
    peer_id: String,
    port: u16,
    name: String,
    /// Vault-derived PSK for peer auth + transport encryption. Set when the
    /// network starts (the app only starts it with an unlocked vault) and
    /// cleared on stop — a stopped/locked network holds no key material.
    sync_key: Option<[u8; 32]>,
    peers: HashMap<String, Peer>,
    /// Established sessions, keyed by peer id (registered on hello).
    sessions: HashMap<String, mpsc::UnboundedSender<String>>,
    mdns_handle: Option<mdns::MdnsHandle>,
    ws_shutdown: Option<tokio::sync::watch::Sender<bool>>,
    last_sync_at: Option<u64>,
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
                sync_key: None,
                peers: HashMap::new(),
                sessions: HashMap::new(),
                mdns_handle: None,
                ws_shutdown: None,
                last_sync_at: None,
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
            last_sync_at: inner.last_sync_at,
        }
    }

    /// Record a successful sync (snapshot merge or ack) as "last synced" —
    /// surfaced to the UI via network_status. Epoch millis.
    pub async fn mark_synced(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.inner.write().await.last_sync_at = Some(now);
    }

    /// Start mDNS advertising + discovery, WebSocket listener on an
    /// OS-assigned port, and auto-connect to every discovered peer.
    /// `sync_key` is the vault-derived PSK: peers that can't prove it are
    /// rejected and everything on the wire is encrypted with it.
    pub async fn start(self: &Arc<Self>, name: &str, sync_key: [u8; 32]) -> Result<(), String> {
        let mut inner = self.inner.write().await;
        if inner.mdns_handle.is_some() {
            return Err("Network already running".into());
        }
        inner.name = name.to_string();
        inner.sync_key = Some(sync_key);

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
        let (discovery_tx, mut discovery_rx) = mpsc::unbounded_channel::<(String, Vec<String>, u16)>();
        let mdns_handle = match mdns::start(peer_id.clone(), port, discovery_tx).await {
            Ok(h) => Some(h),
            Err(e) => {
                // ponytail: mDNS is best-effort — some networks block it.
                // Keep the WS server running so manual connect_peer still
                // works; the UI surfaces the warning.
                eprintln!("mDNS unavailable (manual peer connect still works): {e}");
                None
            }
        };
        inner.mdns_handle = mdns_handle;

        let net = self.shared();
        tokio::spawn(async move {
            while let Some((pid, hosts, port)) = discovery_rx.recv().await {
                {
                    let inner = net.inner().read().await;
                    if pid == inner.peer_id || inner.sessions.contains_key(&pid) {
                        continue;
                    }
                }
                let host = hosts.first().cloned().unwrap_or_default();
                {
                    let mut inner = net.inner().write().await;
                    inner.peers.insert(
                        pid.clone(),
                        Peer { id: pid.clone(), host: host.clone(), hosts: hosts.clone(), port, connected: false, name: String::new() },
                    );
                }
                if let Err(e) = ws::connect(&pid, &hosts, &port, net.clone()).await {
                    eprintln!("WS connect to {pid}@{host}:{port} failed: {e}");
                }
            }
        });

        Ok(())
    }

    /// Manually dial a peer by host:port (mDNS-blocked networks). Creates a
    /// peer record so the UI shows it and the redial loop can keep it alive.
    pub async fn connect_peer(self: &Arc<Self>, host: &str, port: u16) -> Result<(), String> {
        let pid = {
            let inner = self.inner().read().await;
            if inner.mdns_handle.is_none() && inner.port == 0 {
                return Err("Network is not running".into());
            }
            // A synthetic peer id: manual dials aren't discoverable via
            // mDNS, so sessions key by host:port instead of a real id.
            format!("manual:{host}:{port}")
        };
        {
            let mut inner = self.inner().write().await;
            inner.peers.insert(
                pid.clone(),
                Peer { id: pid.clone(), host: host.to_string(), hosts: vec![host.to_string()], port, connected: false, name: String::new() },
            );
        }
        ws::connect(&pid, &[host.to_string()], &port, self.clone()).await
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
        inner.sync_key = None;
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

    /// Drop a peer from the session map (called when its socket dies) and
    /// redial its last known host:port until a session re-registers. Without
    /// this a transient blip would permanently orphan the peer (mDNS does not
    /// reliably re-fire resolved events for a known service).
    ///
    /// Sync fn so the dying session's future doesn't form a type cycle with
    /// the redial task (which spawns a new session that can itself die). The
    /// dying sender is compared before removal so a re-registered session that
    /// raced ahead of cleanup is never dropped.
    /// ponytail: fixed 3s retry, no backoff — LAN only, peers are few and a
    /// dial is cheap. Upgrade: capped exponential backoff if mDNS churn ever
    /// floods the loop.
    fn forget_session(self: &Arc<Self>, peer_id: &str, dead_tx: &mpsc::UnboundedSender<String>) {
        let net = self.shared();
        let pid = peer_id.to_string();
        let dead_tx = dead_tx.clone();
        tokio::spawn(async move {
            {
                let mut inner = net.inner().write().await;
                let still_mine = inner
                    .sessions
                    .get(&pid)
                    .map(|tx| tx.same_channel(&dead_tx))
                    .unwrap_or(false);
                if still_mine {
                    inner.sessions.remove(&pid);
                }
                if let Some(peer) = inner.peers.get_mut(&pid) {
                    peer.connected = false;
                }
            }
            loop {
                let dial = {
                    let inner = net.inner().read().await;
                    if inner.mdns_handle.is_none() {
                        None
                    } else if inner.sessions.contains_key(&pid) {
                        None
                    } else {
                        inner.peers.get(&pid).map(|p| (p.hosts.clone(), p.port))
                    }
                };
                let Some((hosts, port)) = dial else { break };
                let _ = ws::connect(&pid, &hosts, &port, net.clone()).await;
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }
        });
    }

    /// Register an established session under the peer's id (learned from its
    /// hello). Creates/updates the peer record, and drops any stale manual
    /// connect record for the same host:port so the UI shows no ghosts.
    pub(crate) async fn register_session(
        &self,
        peer_id: &str,
        name: &str,
        host: &str,
        port: u16,
        tx: mpsc::UnboundedSender<String>,
    ) {
        let mut inner = self.inner.write().await;
        inner.sessions.insert(peer_id.to_string(), tx);
        let stale: Vec<String> = inner
            .peers
            .iter()
            .filter(|(k, p)| k.starts_with("manual:") && p.host == host && p.port == port)
            .map(|(k, _)| k.clone())
            .collect();
        for k in stale {
            inner.peers.remove(&k);
        }
        match inner.peers.get_mut(peer_id) {
            Some(peer) => {
                peer.connected = true;
                peer.name = name.to_string();
                peer.host = host.to_string();
                peer.port = port;
            }
            None => {
                inner.peers.insert(
                    peer_id.to_string(),
                    Peer {
                        id: peer_id.to_string(),
                        host: host.to_string(),
                        hosts: vec![host.to_string()],
                        port,
                        connected: true,
                        name: name.to_string(),
                    },
                );
            }
        }
    }

    pub(crate) fn local_peer_id(&self) -> String {
        self.inner.try_read().map(|g| g.peer_id.clone()).unwrap_or_default()
    }

    pub(crate) fn local_name(&self) -> String {
        self.inner.try_read().map(|g| g.name.clone()).unwrap_or_default()
    }

    /// The PSK this network is running with; None when stopped.
    pub(crate) async fn sync_key(&self) -> Option<[u8; 32]> {
        self.inner.read().await.sync_key
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
        let key = crate::crypto::derive_sync_key(b"same-vault-key");
        a.start("alice", key).await.unwrap();
        let a_status = a.status().await;
        let a_id = a_status.local_peer_id.clone();
        let b_id = b.status().await.local_peer_id.clone();
        b.start("bob", key).await.unwrap();

        ws::connect(&a_id, &["127.0.0.1".to_string()], &a_status.port, b.clone())
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

    /// The dial must fall through advertised addresses: an unroutable first
    /// host must not prevent connecting via the second. (192.0.2.1 is
    /// TEST-NET-1, documented non-routable — machines without a route to it
    /// fail fast with ENETUNREACH; machines that route it would just time
    /// out, hence the short dial timeout below.)
    #[tokio::test]
    async fn connect_falls_through_dead_hosts() {
        let a = Arc::new(NetworkState::new());
        let b = Arc::new(NetworkState::new());
        let key = crate::crypto::derive_sync_key(b"same-vault-key");
        a.start("alice", key).await.unwrap();
        let a_status = a.status().await;
        let a_id = a_status.local_peer_id.clone();
        b.start("bob", key).await.unwrap();

        ws::connect(
            &a_id,
            &["192.0.2.1".to_string(), "127.0.0.1".to_string()],
            &a_status.port,
            b.clone(),
        )
        .await
        .expect("dial should succeed via the second host");

        let hello = {
            let mut rx = a.message_rx.lock().await;
            rx.as_mut().unwrap().recv().await.unwrap()
        };
        let hello_json: serde_json::Value = serde_json::from_str(&hello.payload).unwrap();
        assert_eq!(hello_json["kind"], "hello");
        assert_eq!(hello_json["peer_id"], b.status().await.local_peer_id);

        b.stop().await.unwrap();
        a.stop().await.unwrap();
    }

    /// A peer with a different vault key must be rejected: no session is
    /// registered on either side and no payload ever flows.
    #[tokio::test]
    async fn wrong_key_peer_is_rejected() {
        let a = Arc::new(NetworkState::new());
        let b = Arc::new(NetworkState::new());
        a.start("alice", crate::crypto::derive_sync_key(b"vault-A")).await.unwrap();
        let a_status = a.status().await;
        let a_id = a_status.local_peer_id.clone();
        b.start("mallory", crate::crypto::derive_sync_key(b"vault-B")).await.unwrap();

        ws::connect(&a_id, &["127.0.0.1".to_string()], &a_status.port, b.clone())
            .await
            .expect("dial should succeed");

        // Give the failed handshake time to unwind on both sides.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        assert!(a.inner().read().await.sessions.is_empty(), "A must not register a session");
        assert!(b.inner().read().await.sessions.is_empty(), "B must not register a session");

        // No payloads may arrive on either side.
        for (name, net) in [("a", &a), ("b", &b)] {
            let mut rx = net.message_rx.lock().await;
            assert!(
                rx.as_mut().unwrap().try_recv().is_err(),
                "{name} must receive nothing from a wrong-key peer"
            );
        }

        b.stop().await.unwrap();
        a.stop().await.unwrap();
    }
}
