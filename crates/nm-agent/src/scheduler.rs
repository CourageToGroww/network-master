use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use nm_common::config::AgentConfig;
use nm_common::protocol::{TargetConfig, WsEnvelope, WsPayload};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::probe;

pub enum TargetCommand {
    Add(TargetConfig),
    Remove(Vec<Uuid>),
}

struct TargetState {
    config: TargetConfig,
    session_id: Uuid,
    round_counter: u64,
    dest_ip: Option<IpAddr>,
    known_hops: u8,
    last_probe_time: Option<Instant>,
}

pub async fn run(
    config: AgentConfig,
    mut target_rx: mpsc::Receiver<TargetCommand>,
    outgoing_tx: mpsc::Sender<WsEnvelope>,
) {
    let mut targets: HashMap<Uuid, TargetState> = HashMap::new();
    let timeout_ms = config.default_timeout_ms;

    loop {
        // Check for target commands (non-blocking drain)
        while let Ok(cmd) = target_rx.try_recv() {
            match cmd {
                TargetCommand::Add(target_config) => {
                    let target_id = target_config.target_id;
                    let session_id = target_config.session_id;
                    tracing::info!(
                        target_id = %target_id,
                        session_id = %session_id,
                        address = %target_config.address,
                        "New target assigned"
                    );

                    // Resolve destination IP
                    let dest_ip = resolve_target(&target_config.address).await;

                    targets.insert(target_id, TargetState {
                        config: target_config,
                        session_id,
                        round_counter: 0,
                        dest_ip,
                        known_hops: 30,
                        last_probe_time: None,
                    });
                }
                TargetCommand::Remove(target_ids) => {
                    for id in &target_ids {
                        if targets.remove(id).is_some() {
                            tracing::info!(target_id = %id, "Target removed");
                        }
                    }
                }
            }
        }

        if targets.is_empty() {
            // No targets yet, wait a bit then check again
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        // Run probe rounds for targets that are due
        let now = Instant::now();
        for state in targets.values_mut() {
            // Check if this target is due for a probe
            if let Some(last) = state.last_probe_time {
                let interval = Duration::from_millis(state.config.interval_ms as u64);
                if now.duration_since(last) < interval {
                    continue;
                }
            }

            let Some(dest_ip) = state.dest_ip else {
                // Try to resolve again
                state.dest_ip = resolve_target(&state.config.address).await;
                continue;
            };

            state.round_counter += 1;
            state.last_probe_time = Some(now);
            let round = state.round_counter;

            tracing::debug!(
                target = %state.config.address,
                round = round,
                "Executing probe round"
            );

            let report = probe::engine::execute_round(
                &state.config,
                state.session_id,
                round,
                dest_ip,
                state.known_hops,
                timeout_ms,
            )
            .await;

            // Update known_hops based on actual responses
            if let Some(last_responding) = report.hops.iter()
                .rev()
                .find(|h| h.ip_address.is_some())
            {
                state.known_hops = last_responding.hop_number;
            }

            // Log summary
            let responding = report.hops.iter().filter(|h| !h.is_lost).count();
            let total = report.hops.len();
            tracing::info!(
                target = %state.config.address,
                round = round,
                "Probe round complete: {}/{} hops responded",
                responding,
                total
            );

            // Send to server
            let envelope = WsEnvelope::new(WsPayload::TraceRound(report));
            if outgoing_tx.send(envelope).await.is_err() {
                tracing::warn!("Failed to queue trace report (connection down?)");
            }
        }

        // Sleep for a short tick interval to check timing
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

async fn resolve_target(address: &str) -> Option<IpAddr> {
    // Try parsing as IP first
    if let Ok(ip) = address.parse::<IpAddr>() {
        return Some(ip);
    }

    // DNS resolution
    match tokio::net::lookup_host(format!("{}:0", address)).await {
        Ok(mut addrs) => addrs.next().map(|a| a.ip()),
        Err(e) => {
            tracing::warn!(address = %address, error = %e, "DNS resolution failed");
            None
        }
    }
}
