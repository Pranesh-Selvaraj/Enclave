//! mDNS service advertisement and discovery using `mdns-sd`.
//! Advertises `_enclave._tcp.local.` with peer ID + port in TXT records.

use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Duration;

const SERVICE_TYPE: &str = "_enclave._tcp.local.";

pub struct MdnsHandle {
    daemon: ServiceDaemon,
}

// ponytail: UDP connect trick to discover local IP; works on Linux/macOS/Windows
// but may return 0.0.0.0 if no route exists. If real mDNS breaks, switch to
// the `local-ip-address` crate.
fn local_ip() -> Result<String, String> {
    use std::net::UdpSocket;
    let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    sock.connect("10.255.255.255:1").map_err(|e| e.to_string())?;
    let addr = sock.local_addr().map_err(|e| e.to_string())?;
    let ip = addr.ip().to_string();
    if ip == "0.0.0.0" {
        // ponytail: fallback for machines with no default route (VPN, VM-only)
        let sock2 = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        sock2.connect("1.1.1.1:1").map_err(|e| e.to_string())?;
        let addr2 = sock2.local_addr().map_err(|e| e.to_string())?;
        return Ok(addr2.ip().to_string());
    }
    Ok(ip)
}

pub async fn start(
    peer_id: String,
    port: u16,
    discovery_tx: tokio::sync::mpsc::UnboundedSender<(String, String, u16)>,
) -> Result<MdnsHandle, String> {
    let daemon = ServiceDaemon::new().map_err(|e| format!("mDNS daemon: {e}"))?;

    let host_ip = local_ip()?;
    let host_name = format!("{peer_id}.local.");

    let mut properties = HashMap::new();
    properties.insert("id".into(), peer_id.clone());
    properties.insert("port".into(), port.to_string());

    let service_info = ServiceInfo::new(
        SERVICE_TYPE,
        &peer_id,
        &host_name,
        IpAddr::V4(host_ip.parse().map_err(|e| format!("bad IP: {e}"))?),
        port,
        Some(properties),
    )
    .map_err(|e| format!("ServiceInfo: {e}"))?;

    daemon
        .register(service_info)
        .map_err(|e| format!("mDNS register: {e}"))?;

    // Browse for other Enclave peers; resolved events trigger connections.
    let receiver = daemon.browse(SERVICE_TYPE).map_err(|e| format!("mDNS browse: {e}"))?;

    std::thread::spawn(move || {
        while let Ok(event) = receiver.recv() {
            if let ServiceEvent::ServiceResolved(info) = event {
                let Some(id) = info.get_property_val_str("id").map(str::to_string) else {
                    continue;
                };
                let Some(host) = info.get_addresses().iter().next().map(|a| a.to_string()) else {
                    continue;
                };
                let port = info.get_port();
                // Ignore our own advertisement.
                if id != peer_id {
                    let _ = discovery_tx.send((id, host, port));
                }
            }
        }
    });

    Ok(MdnsHandle { daemon })
}

pub fn stop(handle: MdnsHandle) -> Result<(), String> {
    handle
        .daemon
        .shutdown()
        .map_err(|e| format!("mDNS shutdown: {e}"))?;
    // Small sleep to let daemon threads exit cleanly
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}
