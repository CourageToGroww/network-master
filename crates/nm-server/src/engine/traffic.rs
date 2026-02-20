use nm_common::protocol::{
    LiveProcessTrafficUpdate, ProcessTrafficReport, ProcessTrafficSummary, RemoteEndpoint,
};

use crate::state::AppState;

/// Convert an agent's traffic report to a frontend-friendly update and broadcast it.
pub async fn handle_traffic_report(report: ProcessTrafficReport, state: &AppState) {
    let interval_secs = (report.interval_ms.max(1)) as f64 / 1000.0;

    let processes: Vec<ProcessTrafficSummary> = report
        .processes
        .iter()
        .map(|p| {
            let bytes_in_per_sec = p.total_bytes_in as f64 / interval_secs;
            let bytes_out_per_sec = p.total_bytes_out as f64 / interval_secs;

            // Top remote endpoints (up to 10, sorted by total bytes)
            let mut endpoints: Vec<RemoteEndpoint> = p
                .connections
                .iter()
                .filter(|c| c.remote_addr != "0.0.0.0" && c.remote_addr != "::")
                .map(|c| RemoteEndpoint {
                    remote_addr: c.remote_addr.clone(),
                    remote_port: c.remote_port,
                    protocol: c.protocol,
                    bytes_in_per_sec: c.bytes_in as f64 / interval_secs,
                    bytes_out_per_sec: c.bytes_out as f64 / interval_secs,
                })
                .collect();

            endpoints.sort_by(|a, b| {
                let a_total = a.bytes_in_per_sec + a.bytes_out_per_sec;
                let b_total = b.bytes_in_per_sec + b.bytes_out_per_sec;
                b_total
                    .partial_cmp(&a_total)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            endpoints.truncate(10);

            ProcessTrafficSummary {
                pid: p.pid,
                process_name: p.process_name.clone(),
                exe_path: p.exe_path.clone(),
                bytes_in_per_sec,
                bytes_out_per_sec,
                active_connections: p.active_connection_count,
                top_remote_endpoints: endpoints,
            }
        })
        .collect();

    let live_update = LiveProcessTrafficUpdate {
        agent_id: report.agent_id,
        captured_at: report.captured_at,
        processes,
    };

    // Broadcast to subscribed frontends
    let _ = state.traffic_tx.send(live_update);

    // Store aggregate in DB (fire and forget)
    let pool = state.pool.clone();
    let agent_id = report.agent_id;
    let captured_at = report.captured_at;
    let interval_ms = report.interval_ms as i32;
    let entries = report.processes;

    tokio::spawn(async move {
        if let Err(e) =
            crate::db::traffic::store_snapshot(&pool, agent_id, captured_at, interval_ms, &entries)
                .await
        {
            tracing::warn!("Failed to store traffic snapshot: {e}");
        }
    });
}
