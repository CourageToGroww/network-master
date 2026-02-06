use std::net::IpAddr;

use nm_common::protocol::{HopSample, TargetConfig, TraceRoundReport};
use uuid::Uuid;
use chrono::Utc;

use super::ProbeResult;

/// Execute a full trace round for a target.
/// Sends probes in parallel to all TTL values.
pub async fn execute_round(
    target: &TargetConfig,
    session_id: Uuid,
    round_number: u64,
    dest_ip: IpAddr,
    known_hops: u8,
    timeout_ms: u64,
) -> TraceRoundReport {
    let max_ttl = known_hops.max(target.max_hops).min(30);

    // Send all probes in parallel
    let mut futures = Vec::with_capacity(max_ttl as usize);
    for ttl in 1..=max_ttl {
        let method = target.probe_method;
        let packet_size = target.packet_size;
        let port = target.probe_port;
        futures.push(tokio::spawn(async move {
            super::send_probe(method, dest_ip, ttl, packet_size, timeout_ms, port).await
        }));
    }

    let mut hops = Vec::with_capacity(max_ttl as usize);
    for (i, future) in futures.into_iter().enumerate() {
        let result = future.await.unwrap_or(ProbeResult {
            hop_number: (i + 1) as u8,
            responding_ip: None,
            rtt_us: None,
            timed_out: true,
            ttl_received: None,
        });

        hops.push(HopSample {
            hop_number: result.hop_number,
            ip_address: result.responding_ip.map(|ip| ip.to_string()),
            rtt_us: result.rtt_us,
            is_lost: result.timed_out,
            ttl_received: result.ttl_received,
        });
    }

    TraceRoundReport {
        target_id: target.target_id,
        session_id,
        round_number,
        sent_at: Utc::now(),
        hops,
    }
}
