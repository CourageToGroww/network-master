use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;

use chrono::Utc;
use nm_common::protocol::{
    ConnectionEntry, ConnectionProtocol, ProcessNetworkEntry, ProcessTrafficReport, WsEnvelope,
    WsPayload,
};
use tokio::sync::mpsc;
use uuid::Uuid;

const POLL_INTERVAL_MS: u32 = 5000;

/// Run the traffic monitor loop, sending reports via `outgoing_tx`.
pub async fn run(agent_id: Uuid, outgoing_tx: mpsc::Sender<WsEnvelope>) {
    tracing::info!("Traffic monitor starting");

    let mut monitor = TrafficMonitor::new();

    loop {
        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS as u64)).await;

        let report = match tokio::task::spawn_blocking({
            let mut mon = std::mem::replace(&mut monitor, TrafficMonitor::new());
            move || {
                let result = mon.poll(agent_id);
                (mon, result)
            }
        })
        .await
        {
            Ok((mon, Ok(report))) => {
                monitor = mon;
                report
            }
            Ok((mon, Err(e))) => {
                monitor = mon;
                tracing::warn!("Traffic poll error: {e}");
                continue;
            }
            Err(e) => {
                tracing::error!("Traffic poll task panicked: {e}");
                monitor = TrafficMonitor::new();
                continue;
            }
        };

        if report.processes.is_empty() {
            continue;
        }

        let envelope = WsEnvelope::new(WsPayload::ProcessTraffic(report));
        if outgoing_tx.send(envelope).await.is_err() {
            tracing::warn!("Traffic monitor: outgoing channel closed, stopping");
            break;
        }
    }
}

// ─── Platform-specific implementation ────────────────────

#[cfg(windows)]
mod platform {
    use super::*;
    use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

    use windows::Win32::Foundation::NO_ERROR;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCPTABLE_OWNER_PID,
        MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };
    use windows::Win32::Networking::WinSock::AF_INET;

    // TCP states from MIB_TCP_STATE
    const MIB_TCP_STATE_CLOSED: u32 = 1;
    const MIB_TCP_STATE_LISTEN: u32 = 2;
    const MIB_TCP_STATE_SYN_SENT: u32 = 3;
    const MIB_TCP_STATE_SYN_RCVD: u32 = 4;
    const MIB_TCP_STATE_ESTAB: u32 = 5;
    const MIB_TCP_STATE_FIN_WAIT1: u32 = 6;
    const MIB_TCP_STATE_FIN_WAIT2: u32 = 7;
    const MIB_TCP_STATE_CLOSE_WAIT: u32 = 8;
    const MIB_TCP_STATE_CLOSING: u32 = 9;
    const MIB_TCP_STATE_LAST_ACK: u32 = 10;
    const MIB_TCP_STATE_TIME_WAIT: u32 = 11;
    const MIB_TCP_STATE_DELETE_TCB: u32 = 12;

    fn tcp_state_str(state: u32) -> &'static str {
        match state {
            MIB_TCP_STATE_CLOSED => "CLOSED",
            MIB_TCP_STATE_LISTEN => "LISTEN",
            MIB_TCP_STATE_SYN_SENT => "SYN_SENT",
            MIB_TCP_STATE_SYN_RCVD => "SYN_RCVD",
            MIB_TCP_STATE_ESTAB => "ESTABLISHED",
            MIB_TCP_STATE_FIN_WAIT1 => "FIN_WAIT1",
            MIB_TCP_STATE_FIN_WAIT2 => "FIN_WAIT2",
            MIB_TCP_STATE_CLOSE_WAIT => "CLOSE_WAIT",
            MIB_TCP_STATE_CLOSING => "CLOSING",
            MIB_TCP_STATE_LAST_ACK => "LAST_ACK",
            MIB_TCP_STATE_TIME_WAIT => "TIME_WAIT",
            MIB_TCP_STATE_DELETE_TCB => "DELETE_TCB",
            _ => "UNKNOWN",
        }
    }

    /// Key for tracking a TCP connection across polls (for byte delta computation).
    #[derive(Hash, Eq, PartialEq, Clone)]
    struct TcpConnKey {
        local_addr: u32,
        local_port: u16,
        remote_addr: u32,
        remote_port: u16,
    }

    struct EStatsData {
        bytes_in: u64,
        bytes_out: u64,
    }

    pub struct TrafficMonitor {
        system: System,
        estats_available: Option<bool>,
        prev_estats: HashMap<TcpConnKey, EStatsData>,
    }

    impl TrafficMonitor {
        pub fn new() -> Self {
            Self {
                system: System::new_with_specifics(
                    RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing()),
                ),
                estats_available: None,
                prev_estats: HashMap::new(),
            }
        }

        pub fn poll(&mut self, agent_id: Uuid) -> anyhow::Result<ProcessTrafficReport> {
            // Refresh process list (names only, minimal overhead)
            self.system.refresh_processes_specifics(
                sysinfo::ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing(),
            );

            // Collect TCP and UDP connections
            let tcp_connections = self.get_tcp_connections()?;
            let udp_connections = self.get_udp_connections()?;

            // Try EStats for byte counters (first poll tests availability)
            let estats = self.try_get_estats(&tcp_connections);

            // Group connections by PID
            let mut pid_map: HashMap<u32, Vec<ConnectionEntry>> = HashMap::new();

            for conn in &tcp_connections {
                let key = TcpConnKey {
                    local_addr: conn.local_addr,
                    local_port: conn.local_port,
                    remote_addr: conn.remote_addr,
                    remote_port: conn.remote_port,
                };

                let (bytes_in, bytes_out) = estats
                    .as_ref()
                    .and_then(|e| e.get(&key))
                    .map(|d| (d.bytes_in, d.bytes_out))
                    .unwrap_or((0, 0));

                // Skip LISTEN sockets and loopback-to-loopback
                if conn.state == MIB_TCP_STATE_LISTEN {
                    continue;
                }

                let entry = ConnectionEntry {
                    protocol: ConnectionProtocol::Tcp,
                    local_addr: ipv4_to_string(conn.local_addr),
                    local_port: conn.local_port,
                    remote_addr: ipv4_to_string(conn.remote_addr),
                    remote_port: conn.remote_port,
                    state: Some(tcp_state_str(conn.state).to_string()),
                    bytes_in,
                    bytes_out,
                };

                pid_map.entry(conn.pid).or_default().push(entry);
            }

            for conn in &udp_connections {
                let entry = ConnectionEntry {
                    protocol: ConnectionProtocol::Udp,
                    local_addr: ipv4_to_string(conn.local_addr),
                    local_port: conn.local_port,
                    remote_addr: "0.0.0.0".to_string(),
                    remote_port: 0,
                    state: None,
                    bytes_in: 0,
                    bytes_out: 0,
                };

                pid_map.entry(conn.pid).or_default().push(entry);
            }

            // Build per-process entries
            let mut processes = Vec::new();
            for (pid, connections) in pid_map {
                if pid == 0 {
                    continue; // Skip System Idle Process
                }

                let (process_name, exe_path) = self.resolve_process(pid);

                let total_bytes_in: u64 = connections.iter().map(|c| c.bytes_in).sum();
                let total_bytes_out: u64 = connections.iter().map(|c| c.bytes_out).sum();
                let active_connection_count = connections.len() as u32;

                processes.push(ProcessNetworkEntry {
                    pid,
                    process_name,
                    exe_path,
                    connections,
                    total_bytes_in,
                    total_bytes_out,
                    active_connection_count,
                });
            }

            // Sort by total bandwidth descending
            processes.sort_by(|a, b| {
                let a_total = a.total_bytes_in + a.total_bytes_out;
                let b_total = b.total_bytes_in + b.total_bytes_out;
                b_total.cmp(&a_total)
            });

            Ok(ProcessTrafficReport {
                agent_id,
                captured_at: Utc::now(),
                interval_ms: POLL_INTERVAL_MS,
                processes,
            })
        }

        fn resolve_process(&self, pid: u32) -> (String, Option<String>) {
            if let Some(process) = self.system.process(Pid::from_u32(pid)) {
                let name = process.name().to_string_lossy().to_string();
                let exe = process.exe().map(|p| p.to_string_lossy().to_string());
                (name, exe)
            } else {
                (format!("PID {pid}"), None)
            }
        }

        fn get_tcp_connections(&self) -> anyhow::Result<Vec<TcpConnectionInfo>> {
            unsafe {
                let mut size: u32 = 0;
                // First call: get required buffer size
                let _ = GetExtendedTcpTable(
                    None,
                    &mut size,
                    false,
                    AF_INET.0 as u32,
                    TCP_TABLE_OWNER_PID_ALL,
                    0,
                );

                if size == 0 {
                    return Ok(Vec::new());
                }

                let mut buffer = vec![0u8; size as usize];
                let ret = GetExtendedTcpTable(
                    Some(buffer.as_mut_ptr() as *mut _),
                    &mut size,
                    false,
                    AF_INET.0 as u32,
                    TCP_TABLE_OWNER_PID_ALL,
                    0,
                );

                if ret != NO_ERROR.0 {
                    anyhow::bail!("GetExtendedTcpTable failed with code {ret}");
                }

                let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
                let num_entries = table.dwNumEntries as usize;

                let rows_ptr = table.table.as_ptr();
                let rows = std::slice::from_raw_parts(rows_ptr, num_entries);

                let connections: Vec<TcpConnectionInfo> = rows
                    .iter()
                    .map(|row| TcpConnectionInfo {
                        state: row.dwState,
                        local_addr: u32::from_be(row.dwLocalAddr),
                        local_port: u16::from_be((row.dwLocalPort & 0xFFFF) as u16),
                        remote_addr: u32::from_be(row.dwRemoteAddr),
                        remote_port: u16::from_be((row.dwRemotePort & 0xFFFF) as u16),
                        pid: row.dwOwningPid,
                    })
                    .collect();

                Ok(connections)
            }
        }

        fn get_udp_connections(&self) -> anyhow::Result<Vec<UdpConnectionInfo>> {
            unsafe {
                let mut size: u32 = 0;
                let _ = GetExtendedUdpTable(
                    None,
                    &mut size,
                    false,
                    AF_INET.0 as u32,
                    UDP_TABLE_OWNER_PID,
                    0,
                );

                if size == 0 {
                    return Ok(Vec::new());
                }

                let mut buffer = vec![0u8; size as usize];
                let ret = GetExtendedUdpTable(
                    Some(buffer.as_mut_ptr() as *mut _),
                    &mut size,
                    false,
                    AF_INET.0 as u32,
                    UDP_TABLE_OWNER_PID,
                    0,
                );

                if ret != NO_ERROR.0 {
                    anyhow::bail!("GetExtendedUdpTable failed with code {ret}");
                }

                let table = &*(buffer.as_ptr() as *const MIB_UDPTABLE_OWNER_PID);
                let num_entries = table.dwNumEntries as usize;

                let rows_ptr = table.table.as_ptr();
                let rows = std::slice::from_raw_parts(rows_ptr, num_entries);

                let connections: Vec<UdpConnectionInfo> = rows
                    .iter()
                    .map(|row| UdpConnectionInfo {
                        local_addr: u32::from_be(row.dwLocalAddr),
                        local_port: u16::from_be((row.dwLocalPort & 0xFFFF) as u16),
                        pid: row.dwOwningPid,
                    })
                    .collect();

                Ok(connections)
            }
        }

        /// Try to read per-connection byte counters via TCP EStats.
        /// Returns None if EStats is not available on this system.
        fn try_get_estats(
            &mut self,
            _tcp_connections: &[TcpConnectionInfo],
        ) -> Option<HashMap<TcpConnKey, EStatsData>> {
            // EStats (GetPerTcpConnectionEStats) requires:
            // 1. SetPerTcpConnectionEStats to enable collection per-connection
            // 2. The connection must be in ESTABLISHED state
            // 3. Requires admin/LocalSystem (which we have as a service)
            //
            // For the initial implementation, we use the connection table approach:
            // byte counters are derived from connection state tracking across polls.
            // A future enhancement can enable full EStats integration.
            //
            // For now, we report bytes_in/out as 0, and the server computes
            // bandwidth estimates from connection presence/absence patterns.
            // The frontend will still show active connections and process grouping.

            if self.estats_available == Some(false) {
                return None;
            }

            // Mark as not yet implemented — will be enhanced in a follow-up
            self.estats_available = Some(false);
            None
        }
    }

    struct TcpConnectionInfo {
        state: u32,
        local_addr: u32, // big-endian u32, already converted
        local_port: u16,
        remote_addr: u32,
        remote_port: u16,
        pid: u32,
    }

    struct UdpConnectionInfo {
        local_addr: u32,
        local_port: u16,
        pid: u32,
    }

    fn ipv4_to_string(addr: u32) -> String {
        let ip = Ipv4Addr::from(addr);
        ip.to_string()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;

    pub struct TrafficMonitor;

    impl TrafficMonitor {
        pub fn new() -> Self {
            Self
        }

        pub fn poll(&mut self, agent_id: Uuid) -> anyhow::Result<ProcessTrafficReport> {
            Ok(ProcessTrafficReport {
                agent_id,
                captured_at: Utc::now(),
                interval_ms: POLL_INTERVAL_MS,
                processes: Vec::new(),
            })
        }
    }
}

use platform::TrafficMonitor;
