use std::net::IpAddr;

use super::ProbeResult;

pub async fn send_udp_probe(
    dest: IpAddr,
    ttl: u8,
    timeout_ms: u64,
) -> ProbeResult {
    let result = tokio::task::spawn_blocking(move || {
        send_udp_probe_sync(dest, ttl, timeout_ms)
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

fn send_udp_probe_sync(
    dest: IpAddr,
    ttl: u8,
    timeout_ms: u64,
) -> ProbeResult {
    use std::mem::MaybeUninit;
    use std::net::SocketAddr;
    use std::time::{Duration, Instant};
    use socket2::{Domain, Protocol, Socket, Type};

    let domain = match dest {
        IpAddr::V4(_) => Domain::IPV4,
        IpAddr::V6(_) => Domain::IPV6,
    };

    // Create UDP socket
    let socket = match Socket::new(domain, Type::DGRAM, Some(Protocol::UDP)) {
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
    let _ = socket.set_read_timeout(Some(Duration::from_millis(timeout_ms)));

    // Standard traceroute UDP port range: 33434 + (ttl - 1)
    let port = 33434u16.wrapping_add(ttl as u16 - 1);
    let addr = SocketAddr::new(dest, port);

    let payload = vec![0u8; 32];
    let start = Instant::now();

    if socket.send_to(&payload, &addr.into()).is_err() {
        return ProbeResult {
            hop_number: ttl,
            responding_ip: None,
            rtt_us: None,
            timed_out: true,
            ttl_received: None,
        };
    }

    // Try to receive ICMP error via the UDP socket
    // On Windows, ICMP errors are delivered to the UDP socket
    let mut recv_buf = [MaybeUninit::<u8>::uninit(); 1500];
    match socket.recv_from(&mut recv_buf) {
        Ok((_n, addr)) => {
            let rtt = start.elapsed();
            let responding_ip = addr.as_socket().map(|sa| sa.ip());
            ProbeResult {
                hop_number: ttl,
                responding_ip,
                rtt_us: Some(rtt.as_micros() as u32),
                timed_out: false,
                ttl_received: None,
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
