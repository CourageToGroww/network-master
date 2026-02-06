use std::mem::MaybeUninit;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use socket2::{Domain, Protocol, Socket, Type};

use super::ProbeResult;

pub async fn send_icmp_probe(
    dest: IpAddr,
    ttl: u8,
    packet_size: u16,
    timeout_ms: u64,
) -> ProbeResult {
    // Run blocking socket operations in a spawn_blocking to avoid blocking the runtime
    let result = tokio::task::spawn_blocking(move || {
        send_icmp_probe_sync(dest, ttl, packet_size, timeout_ms)
    })
    .await;

    match result {
        Ok(r) => r,
        Err(_) => ProbeResult {
            hop_number: ttl,
            responding_ip: None,
            rtt_us: None,
            timed_out: true,
            ttl_received: None,
        },
    }
}

fn send_icmp_probe_sync(
    dest: IpAddr,
    ttl: u8,
    packet_size: u16,
    timeout_ms: u64,
) -> ProbeResult {
    let domain = match dest {
        IpAddr::V4(_) => Domain::IPV4,
        IpAddr::V6(_) => Domain::IPV6,
    };

    let socket = match Socket::new(domain, Type::RAW, Some(Protocol::ICMPV4)) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("Failed to create raw socket: {} (try running as admin)", e);
            return ProbeResult {
                hop_number: ttl,
                responding_ip: None,
                rtt_us: None,
                timed_out: true,
                ttl_received: None,
            };
        }
    };

    // Set TTL
    if socket.set_ttl(ttl as u32).is_err() {
        return ProbeResult {
            hop_number: ttl,
            responding_ip: None,
            rtt_us: None,
            timed_out: true,
            ttl_received: None,
        };
    }

    // Set receive timeout
    let _ = socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)));

    // Build ICMP Echo Request
    let payload_size = (packet_size as usize).saturating_sub(8).max(0);
    let mut buf = vec![0u8; 8 + payload_size];
    buf[0] = 8; // Type: Echo Request
    buf[1] = 0; // Code: 0

    // Identifier: use TTL as a simple identifier
    let id = (std::process::id() as u16) ^ (ttl as u16);
    buf[4..6].copy_from_slice(&id.to_be_bytes());

    // Sequence number
    buf[6..8].copy_from_slice(&(ttl as u16).to_be_bytes());

    // Compute checksum
    let checksum = icmp_checksum(&buf);
    buf[2..4].copy_from_slice(&checksum.to_be_bytes());

    let send_time = Instant::now();

    // Send
    let dest_addr = SocketAddr::new(dest, 0);
    if socket.send_to(&buf, &dest_addr.into()).is_err() {
        return ProbeResult {
            hop_number: ttl,
            responding_ip: None,
            rtt_us: None,
            timed_out: true,
            ttl_received: None,
        };
    }

    // Receive
    let mut recv_buf = [MaybeUninit::<u8>::uninit(); 1500];
    match socket.recv_from(&mut recv_buf) {
        Ok((n, addr)) => {
            let recv_buf: Vec<u8> = recv_buf[..n].iter().map(|b| unsafe { b.assume_init() }).collect();
            let rtt = send_time.elapsed();
            let responding_ip = addr
                .as_socket()
                .map(|sa| sa.ip());

            // Parse ICMP response type (after IP header, typically 20 bytes for IPv4)
            let icmp_offset = if n > 20 { 20 } else { 0 };
            let icmp_type = recv_buf.get(icmp_offset).copied().unwrap_or(0);

            // Type 0 = Echo Reply (reached destination)
            // Type 11 = Time Exceeded (intermediate hop)
            // Type 3 = Destination Unreachable
            let is_valid = icmp_type == 0 || icmp_type == 11 || icmp_type == 3;

            if is_valid {
                ProbeResult {
                    hop_number: ttl,
                    responding_ip,
                    rtt_us: Some(rtt.as_micros() as u32),
                    timed_out: false,
                    ttl_received: None,
                }
            } else {
                ProbeResult {
                    hop_number: ttl,
                    responding_ip: None,
                    rtt_us: None,
                    timed_out: true,
                    ttl_received: None,
                }
            }
        }
        Err(_) => ProbeResult {
            hop_number: ttl,
            responding_ip: None,
            rtt_us: None,
            timed_out: true,
            ttl_received: None,
        },
    }
}

fn icmp_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i < data.len() - 1 {
        sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        i += 2;
    }
    if data.len() % 2 != 0 {
        sum += (data[data.len() - 1] as u32) << 8;
    }
    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}
