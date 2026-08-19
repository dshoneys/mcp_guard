//! Soft gate: attribute TCP peers on discovered loopback listen ports to processes.
//!
//! MVP does **detect + audit**, not kernel block. Hard gate (WFP / pf / nft)
//! comes after the attribution pipeline is trustworthy.

use crate::config::{Config, GateConfig};
use crate::contracts::Watcher;
use crate::net_enum::resolve_probe_ports;
use anyhow::{Context, Result};
use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
};
use sysinfo::{Pid, ProcessesToUpdate, System};

pub use crate::contracts::{ConnectionPeer, PeerProcess, PortWatch, WatchReport};

/// Default Watcher adapter (plugin → contracts).
#[derive(Debug, Default, Clone, Copy)]
pub struct SoftWatcher;

impl Watcher for SoftWatcher {
    fn watch(&self, cfg: &Config) -> Result<WatchReport> {
        run(cfg)
    }
}

pub fn run(cfg: &Config) -> Result<WatchReport> {
    let mut gate = cfg.gate.clone();
    crate::config::merge_manual_allows(&mut gate)?;
    let af = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let sockets = get_sockets_info(af, ProtocolFlags::TCP)
        .context("enumerate TCP sockets (netstat2)")?;

    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut ports = Vec::new();
    let mut alert_count = 0usize;
    let targets = resolve_probe_ports(cfg, &[]);

    for port in targets {
        let mut listeners = Vec::new();
        let mut peers = Vec::new();

        // Pass 1: listeners (needed before peer alert heuristics).
        for si in &sockets {
            let ProtocolSocketInfo::Tcp(tcp) = &si.protocol_socket_info else {
                continue;
            };
            if tcp.state != TcpState::Listen || tcp.local_port != port {
                continue;
            }
            for p in si
                .associated_pids
                .iter()
                .map(|pid| resolve_process(&sys, *pid, &gate))
            {
                if !listeners.iter().any(|x: &PeerProcess| x.pid == p.pid) {
                    listeners.push(p);
                }
            }
        }

        let surface = listeners
            .iter()
            .any(|p| p.allowed || looks_mcp_surface_process(&p.name, p.exe.as_deref()));

        // Pass 2: peers
        for si in &sockets {
            let ProtocolSocketInfo::Tcp(tcp) = &si.protocol_socket_info else {
                continue;
            };
            let touches = tcp.local_port == port || tcp.remote_port == port;
            if !touches {
                continue;
            }
            match tcp.state {
                TcpState::Established
                | TcpState::SynSent
                | TcpState::SynReceived
                | TcpState::CloseWait
                | TcpState::FinWait1
                | TcpState::FinWait2 => {
                    let procs: Vec<PeerProcess> = si
                        .associated_pids
                        .iter()
                        .map(|pid| resolve_process(&sys, *pid, &gate))
                        .collect();
                    // Full listen enumerate would flood if every local TCP peer is an alert.
                    // Only raise when the listener side looks like an agent/MCP surface.
                    let unknown = gate.alert_on_unknown
                        && surface
                        && procs.iter().any(|p| !p.allowed)
                        && !procs.is_empty();
                    if unknown {
                        alert_count += 1;
                    }
                    peers.push(ConnectionPeer {
                        local: format!("{}:{}", tcp.local_addr, tcp.local_port),
                        remote: format!("{}:{}", tcp.remote_addr, tcp.remote_port),
                        state: format!("{:?}", tcp.state),
                        processes: procs,
                        unknown_client: unknown,
                    });
                }
                _ => {}
            }
        }

        ports.push(PortWatch {
            port,
            listeners,
            peers,
        });
    }

    Ok(WatchReport {
        watched_at: chrono::Utc::now().to_rfc3339(),
        ports,
        alert_count,
    })
}

fn resolve_process(sys: &System, pid: u32, gate: &GateConfig) -> PeerProcess {
    let name;
    let exe;
    if let Some(proc_) = sys.process(Pid::from_u32(pid)) {
        name = proc_.name().to_string_lossy().to_string();
        exe = proc_.exe().map(|p| p.to_string_lossy().to_string());
    } else {
        name = format!("pid:{pid}");
        exe = None;
    }

    let allowed = is_allowed(&name, exe.as_deref(), gate);
    PeerProcess {
        pid,
        name,
        exe,
        allowed,
    }
}

/// Pure allowlist match (unit-testable).
pub fn is_allowed(name: &str, exe: Option<&str>, gate: &GateConfig) -> bool {
    let name_l = name.to_ascii_lowercase();
    let exe_l = exe.unwrap_or("").to_ascii_lowercase();
    gate.allow_process_names.iter().any(|pat| {
        let p = pat.to_ascii_lowercase();
        name_l.contains(&p) || exe_l.contains(&p)
    })
}

fn looks_mcp_surface_process(name: &str, exe: Option<&str>) -> bool {
    let blob = format!("{} {}", name, exe.unwrap_or("")).to_ascii_lowercase();
    blob.contains("mcp")
        || blob.contains("buddy")
        || blob.contains("ardot")
        || blob.contains("cursor")
        || blob.contains("claude")
        || blob.contains("copilot")
}
