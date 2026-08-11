//! Enumerate TCP LISTEN ports bound to loopback or unspecified (reachable via 127.0.0.1).

use crate::config::Config;
use anyhow::{Context, Result};
use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
};
use std::net::IpAddr;
use tracing::warn;

/// True when a bind address is loopback or unspecified (`0.0.0.0` / `::`).
pub fn is_loopback_or_unspecified(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v) => v.is_loopback() || v.is_unspecified(),
        IpAddr::V6(v) => v.is_loopback() || v.is_unspecified(),
    }
}

/// Unique local ports currently in TCP `LISTEN` on loopback/unspecified addresses.
pub fn loopback_listen_tcp_ports() -> Result<Vec<u16>> {
    let af = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let sockets = get_sockets_info(af, ProtocolFlags::TCP)
        .context("enumerate TCP sockets (netstat2)")?;

    let mut ports = Vec::new();
    for si in sockets {
        let ProtocolSocketInfo::Tcp(tcp) = si.protocol_socket_info else {
            continue;
        };
        if tcp.state != TcpState::Listen {
            continue;
        }
        if !is_loopback_or_unspecified(tcp.local_addr) {
            continue;
        }
        if tcp.local_port == 0 {
            continue;
        }
        ports.push(tcp.local_port);
    }
    ports.sort_unstable();
    ports.dedup();
    Ok(ports)
}

/// Merge discovered listeners with optional pinned extras; apply a safety cap.
pub fn merge_probe_ports(discovered: &[u16], extras: &[u16], max_ports: usize) -> Vec<u16> {
    let mut ports = Vec::with_capacity(discovered.len() + extras.len());
    ports.extend_from_slice(discovered);
    for p in extras {
        if *p != 0 {
            ports.push(*p);
        }
    }
    ports.sort_unstable();
    ports.dedup();
    if max_ports > 0 && ports.len() > max_ports {
        ports.truncate(max_ports);
    }
    ports
}

/// Port set for scan + soft watch: live listeners (default) plus optional pins.
pub fn resolve_probe_ports(cfg: &Config, extra_ports: &[u16]) -> Vec<u16> {
    let discovered = if cfg.scan.discover_listeners {
        match loopback_listen_tcp_ports() {
            Ok(p) => p,
            Err(err) => {
                warn!(error = %err, "listener discovery failed; falling back to pinned ports");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let mut extras = cfg.scan.ports.clone();
    for p in extra_ports {
        extras.push(*p);
    }
    if !cfg.scan.discover_listeners && extras.is_empty() {
        extras.extend_from_slice(&[50551, 52412, 3000, 8080]);
    }
    merge_probe_ports(&discovered, &extras, cfg.scan.max_probe_ports)
}
