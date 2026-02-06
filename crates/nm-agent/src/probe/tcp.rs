use std::net::IpAddr;

use super::ProbeResult;

pub async fn send_tcp_probe(
    dest: IpAddr,
    ttl: u8,
    port: u16,
    timeout_ms: u64,
) -> ProbeResult {
    // TCP probe: attempt a TCP connect with limited TTL
    // Intermediate hops return ICMP Time Exceeded
    // Destination returns SYN-ACK or RST
    let result = tokio::task::spawn_blocking(move || {
        send_tcp_probe_sync(dest, ttl, port, timeout_ms)
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

fn send_tcp_probe_sync(
    dest: IpAddr,
    ttl: u8,
    port: u16,
    timeout_ms: u64,
) -> ProbeResult {
    use std::net::SocketAddr;
    use std::time::{Duration, Instant};
    use socket2::{Domain, Socket, Type};

    let domain = match dest {
        IpAddr::V4(_) => Domain::IPV4,
        IpAddr::V6(_) => Domain::IPV6,
    };

    let socket = match Socket::new(domain, Type::STREAM, None) {
        Ok(s) => s,
        Err(_) => {
            return ProbeResult {
                hop_number: ttl,
                responding_ip: None,
                rtt_us: None,
                timed_out: true,
                ttl_received: None,
            };
        }
    };

    let _ = socket.set_ttl(ttl as u32);
    socket.set_nonblocking(true).ok();

    let addr = SocketAddr::new(dest, port);
    let start = Instant::now();

    // Attempt connect (non-blocking)
    let _ = socket.connect(&addr.into());

    // Poll for completion
    let timeout = Duration::from_millis(timeout_ms);
    let _ = socket.set_read_timeout(Some(timeout));

    // Use a simple poll: try connect_timeout equivalent
    std::thread::sleep(Duration::from_millis(timeout_ms.min(100)));

    let rtt = start.elapsed();

    // Check if connection succeeded or got a response
    match socket.peer_addr() {
        Ok(_) => ProbeResult {
            hop_number: ttl,
            responding_ip: Some(dest),
            rtt_us: Some(rtt.as_micros() as u32),
            timed_out: false,
            ttl_received: None,
        },
        Err(_) => {
            // Connection failed - could be TTL exceeded or timeout
            if rtt < timeout {
                // Got a quick rejection - likely ICMP response
                ProbeResult {
                    hop_number: ttl,
                    responding_ip: Some(dest),
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
    }
}
