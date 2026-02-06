pub mod engine;
pub mod icmp;
pub mod icmp_win;
pub mod tcp;
pub mod udp;

use nm_common::protocol::ProbeMethod;

#[derive(Debug)]
pub struct ProbeResult {
    pub hop_number: u8,
    pub responding_ip: Option<std::net::IpAddr>,
    pub rtt_us: Option<u32>,
    pub timed_out: bool,
    pub ttl_received: Option<u8>,
}

/// Send a single probe with the given TTL using the specified method.
pub async fn send_probe(
    method: ProbeMethod,
    dest: std::net::IpAddr,
    ttl: u8,
    packet_size: u16,
    timeout_ms: u64,
    port: Option<u16>,
) -> ProbeResult {
    match method {
        ProbeMethod::Icmp => icmp_win::send_icmp_probe(dest, ttl, packet_size, timeout_ms).await,
        ProbeMethod::Tcp => tcp::send_tcp_probe(dest, ttl, port.unwrap_or(80), timeout_ms).await,
        ProbeMethod::Udp => udp::send_udp_probe(dest, ttl, timeout_ms).await,
    }
}
