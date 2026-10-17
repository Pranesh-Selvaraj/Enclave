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

// ponytail: UDP connect trick kept as last-resort fallback; primary path
// enumerates real interfaces via if-addrs — works offline (no default route
// needed) and avoids dialing out to 1.1.1.1 just to learn our own IP.
pub(crate) fn local_ip() -> Result<String, String> {
    // Prefer a private IPv4 (RFC 1918) on an up interface — that's the
    // interface peers on the same Wi-Fi can reach.
    if let Some(ip) = if_addrs::get_if_addrs()
        .ok()
        .and_then(|ifs| {
            ifs.into_iter()
                .filter_map(|i| match i.addr {
                    if_addrs::IfAddr::V4(v4) => Some(v4.ip.to_string()),
                    _ => None,
                })
                .find(|ip| ip.parse::<std::net::Ipv4Addr>().map(|v| v.is_private()).unwrap_or(false))
        })
    {
        return Ok(ip);
    }
    use std::net::UdpSocket;
    let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    sock.connect("10.255.255.255:1").map_err(|e| e.to_string())?;
    let addr = sock.local_addr().map_err(|e| e.to_string())?;
    let ip = addr.ip().to_string();
    if ip == "0.0.0.0" {
        // ponytail: needs a route to the internet; only reached when no
        // private IPv4 interface was found.
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
    discovery_tx: tokio::sync::mpsc::UnboundedSender<(String, Vec<String>, u16)>,
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
                let port = info.get_port();
                // Ignore our own advertisement.
                if id == peer_id {
                    continue;
                }
                // Try every advertised address, private IPv4 first — a
                // multi-homed device (WiFi + VPN/docker) can resolve to a
                // wrong-interface IP, and mDNS does not reliably re-fire
                // resolved events, so the dial must fall through the list.
                let mut hosts: Vec<String> = info.get_addresses().iter().map(|a| a.to_string()).collect();
                hosts.sort_by_key(|h| addr_priority(h));
                if !hosts.is_empty() {
                    let _ = discovery_tx.send((id, hosts, port));
                }
            }
        }
    });

    Ok(MdnsHandle { daemon })
}

/// Prefer private IPv4 (RFC 1918), then other IPv4, then IPv6.
fn addr_priority(addr: &str) -> u8 {
    if let Ok(ip) = addr.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) => {
                if v4.is_private() {
                    0
                } else {
                    1
                }
            }
            IpAddr::V6(_) => 2,
        }
    } else {
        3
    }
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
