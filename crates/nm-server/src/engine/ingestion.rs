use nm_common::protocol::{HopRunningStats, LiveHopData, LiveTraceUpdate, TraceRoundReport};
use uuid::Uuid;

use crate::state::{AppState, RunningHopStats};

/// Ingest a complete trace round from an agent.
/// This is the hottest code path in the server.
pub async fn ingest_trace_round(report: TraceRoundReport, agent_id: Uuid, state: &AppState) {
    let session_id = report.session_id;
    let target_id = report.target_id;

    // Upsert hops and collect hop_ids
    let mut hop_ids: Vec<(u8, Uuid)> = Vec::with_capacity(report.hops.len());
    for hop in &report.hops {
        let hop_id = upsert_hop(&state.pool, session_id, hop).await;
        if let Some(id) = hop_id {
            hop_ids.push((hop.hop_number, id));
        }
    }

    // Batch insert samples using a single query
    batch_insert_samples(&state.pool, &report, &hop_ids).await;

    // Update session sample count
    let _ = sqlx::query(
        "UPDATE trace_sessions SET sample_count = sample_count + $1 WHERE id = $2",
    )
    .bind(report.hops.len() as i64)
    .bind(session_id)
    .execute(&state.pool)
    .await;

    // Update running stats and build live update
    let live_hops: Vec<LiveHopData> = report
        .hops
        .iter()
        .map(|hop| {
            let key = (session_id, hop.hop_number);
            let mut entry = state.hop_stats.entry(key).or_insert_with(RunningHopStats::new);

            // Compute jitter before updating stats
            let jitter_us = if let (Some(rtt), Some(prev)) = (hop.rtt_us, entry.last_rtt_us) {
                Some((rtt as i64 - prev as i64).unsigned_abs() as u32)
            } else {
                None
            };

            entry.update(hop.rtt_us, hop.is_lost);

            LiveHopData {
                hop_number: hop.hop_number,
                ip_address: hop.ip_address.clone(),
                hostname: None,
                rtt_us: hop.rtt_us,
                is_lost: hop.is_lost,
                jitter_us,
                stats: HopRunningStats {
                    min_rtt_us: if entry.min_rtt_us == u32::MAX { 0 } else { entry.min_rtt_us },
                    avg_rtt_us: entry.avg_rtt_us(),
                    max_rtt_us: entry.max_rtt_us,
                    loss_pct: entry.loss_pct(),
                    jitter_avg_us: entry.avg_jitter_us(),
                    sample_count: entry.total_count,
                },
            }
        })
        .collect();

    // Inline route change detection from probe data
    detect_route_change_from_round(&report, session_id, state).await;

    // Broadcast live update to frontend subscribers
    let live_update = LiveTraceUpdate {
        agent_id,
        target_id,
        session_id,
        round_number: report.round_number,
        sent_at: report.sent_at,
        hops: live_hops,
    };

    let _ = state.live_tx.send(live_update);

    // Evaluate alert rules against updated running stats
    crate::engine::alert_evaluator::evaluate_for_round(&report, state).await;
}

/// Detect route changes by comparing hop IPs from the current round against cached route.
async fn detect_route_change_from_round(
    report: &TraceRoundReport,
    session_id: Uuid,
    state: &AppState,
) {
    // Build current route from probe results (only responding hops)
    let current_route: Vec<Option<String>> = report
        .hops
        .iter()
        .map(|h| h.ip_address.clone())
        .collect();

    // Check against cached route
    let route_changed = if let Some(cached) = state.route_cache.get(&session_id) {
        *cached != current_route
    } else {
        // First round - store initial route, create initial snapshot
        true
    };

    if route_changed {
        let old_route = state.route_cache.insert(session_id, current_route.clone());

        // Only create route change records after the first round (when we have a previous route)
        if old_route.is_some() {
            // Delegate to the route detector for DB operations
            crate::engine::route_detector::record_route_change(
                session_id,
                &current_route,
                state,
            )
            .await;
        } else {
            // First round: just create the initial snapshot
            crate::engine::route_detector::create_initial_snapshot(
                session_id,
                &current_route,
                state,
            )
            .await;
        }
    }
}

async fn upsert_hop(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    hop: &nm_common::protocol::HopSample,
) -> Option<Uuid> {
    let ip_str = hop.ip_address.as_deref();

    let result = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO hops (session_id, hop_number, ip_address)
           VALUES ($1, $2, $3::inet)
           ON CONFLICT (session_id, hop_number, ip_address) DO UPDATE
           SET last_seen_at = NOW()
           RETURNING id"#,
    )
    .bind(session_id)
    .bind(hop.hop_number as i16)
    .bind(ip_str)
    .fetch_one(pool)
    .await;

    match result {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::error!("Failed to upsert hop: {}", e);
            None
        }
    }
}

/// Batch insert samples using a dynamically-built multi-row INSERT.
async fn batch_insert_samples(
    pool: &sqlx::PgPool,
    report: &TraceRoundReport,
    hop_ids: &[(u8, Uuid)],
) {
    use std::collections::HashMap;
    let hop_map: HashMap<u8, Uuid> = hop_ids.iter().copied().collect();

    // Use a transaction so all inserts are committed atomically
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("Failed to begin transaction: {}", e);
            return;
        }
    };

    for hop in &report.hops {
        let Some(&hop_id) = hop_map.get(&hop.hop_number) else {
            continue;
        };

        let _ = sqlx::query(
            r#"INSERT INTO samples (session_id, hop_id, round_number, sent_at, rtt_us, is_lost, probe_method, packet_size, ttl_sent, ttl_received)
               VALUES ($1, $2, $3, $4, $5, $6, 'icmp', 64, $7, $8)"#,
        )
        .bind(report.session_id)
        .bind(hop_id)
        .bind(report.round_number as i64)
        .bind(report.sent_at)
        .bind(hop.rtt_us.map(|v| v as i32))
        .bind(hop.is_lost)
        .bind(hop.hop_number as i16)
        .bind(hop.ttl_received.map(|v| v as i16))
        .execute(&mut *tx)
        .await;
    }

    if let Err(e) = tx.commit().await {
        tracing::error!("Failed to commit sample batch: {}", e);
    }
}
