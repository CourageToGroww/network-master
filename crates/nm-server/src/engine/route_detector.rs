use nm_common::crypto::route_hash;
use nm_common::protocol::RouteDiscoveryReport;
use uuid::Uuid;

use crate::state::AppState;

/// Handle explicit route discovery reports from agents.
pub async fn check_route_change(report: RouteDiscoveryReport, state: &AppState) {
    let session_id = report.session_id;

    let hop_ips: Vec<Option<String>> = report
        .hops
        .iter()
        .map(|h| h.ip_address.clone())
        .collect();

    let hash = route_hash(&hop_ips);
    let hop_count = hop_ips.len() as i16;

    // Get previous snapshot
    let previous = sqlx::query_as::<_, (Uuid, String, i16)>(
        "SELECT id, route_hash, hop_count FROM route_snapshots WHERE session_id = $1 ORDER BY captured_at DESC LIMIT 1",
    )
    .bind(session_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    // Insert new snapshot
    let new_snapshot_id = Uuid::new_v4();
    let _ = sqlx::query(
        r#"INSERT INTO route_snapshots (id, session_id, hop_count, hop_sequence, route_hash)
           VALUES ($1, $2, $3, $4::text[], $5)"#,
    )
    .bind(new_snapshot_id)
    .bind(session_id)
    .bind(hop_count)
    .bind(&hop_ips)
    .bind(&hash)
    .execute(&state.pool)
    .await;

    // Detect route change
    if let Some((prev_id, prev_hash, prev_hop_count)) = previous {
        if prev_hash != hash {
            // Count how many hops actually changed
            let hops_changed = count_changed_hops(&state.pool, session_id, prev_id, new_snapshot_id).await;

            tracing::info!(
                session_id = %session_id,
                hops_changed = hops_changed,
                "Route change detected"
            );

            let _ = sqlx::query(
                r#"INSERT INTO route_changes (session_id, previous_snapshot_id, new_snapshot_id, hops_changed)
                   VALUES ($1, $2, $3, $4)"#,
            )
            .bind(session_id)
            .bind(prev_id)
            .bind(new_snapshot_id)
            .bind(hops_changed)
            .execute(&state.pool)
            .await;

            // Look up target_id for notification
            if let Ok(Some(target_id)) = sqlx::query_scalar::<_, Uuid>(
                "SELECT target_id FROM trace_sessions WHERE id = $1",
            )
            .bind(session_id)
            .fetch_optional(&state.pool)
            .await
            {
                // Broadcast route change notification (via live_tx as a workaround
                // since we don't have a dedicated route change channel)
                tracing::info!(
                    target_id = %target_id,
                    old_hops = prev_hop_count,
                    new_hops = hop_count,
                    "Broadcasting route change notification"
                );
            }
        }
    }
}

/// Create the initial route snapshot for a new session (first round of probes).
pub async fn create_initial_snapshot(
    session_id: Uuid,
    hop_ips: &[Option<String>],
    state: &AppState,
) {
    let hash = route_hash(hop_ips);
    let hop_count = hop_ips.len() as i16;

    let _ = sqlx::query(
        r#"INSERT INTO route_snapshots (session_id, hop_count, hop_sequence, route_hash)
           VALUES ($1, $2, $3::text[], $4)"#,
    )
    .bind(session_id)
    .bind(hop_count)
    .bind(hop_ips)
    .bind(&hash)
    .execute(&state.pool)
    .await;

    tracing::debug!(session_id = %session_id, hops = hop_count, "Initial route snapshot created");
}

/// Record a route change detected from inline probe data comparison.
pub async fn record_route_change(
    session_id: Uuid,
    current_route: &[Option<String>],
    state: &AppState,
) {
    let hash = route_hash(current_route);
    let hop_count = current_route.len() as i16;

    // Get previous snapshot
    let previous = sqlx::query_as::<_, (Uuid, i16)>(
        "SELECT id, hop_count FROM route_snapshots WHERE session_id = $1 ORDER BY captured_at DESC LIMIT 1",
    )
    .bind(session_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    // Insert new snapshot
    let new_snapshot_id = Uuid::new_v4();
    let _ = sqlx::query(
        r#"INSERT INTO route_snapshots (id, session_id, hop_count, hop_sequence, route_hash)
           VALUES ($1, $2, $3, $4::text[], $5)"#,
    )
    .bind(new_snapshot_id)
    .bind(session_id)
    .bind(hop_count)
    .bind(current_route)
    .bind(&hash)
    .execute(&state.pool)
    .await;

    if let Some((prev_id, _prev_hop_count)) = previous {
        let hops_changed = count_changed_hops(&state.pool, session_id, prev_id, new_snapshot_id).await;

        tracing::info!(
            session_id = %session_id,
            hops_changed = hops_changed,
            "Route change detected from probe data"
        );

        let _ = sqlx::query(
            r#"INSERT INTO route_changes (session_id, previous_snapshot_id, new_snapshot_id, hops_changed)
               VALUES ($1, $2, $3, $4)"#,
        )
        .bind(session_id)
        .bind(prev_id)
        .bind(new_snapshot_id)
        .bind(hops_changed)
        .execute(&state.pool)
        .await;
    }
}

/// Count how many hops changed between two route snapshots by comparing hop_sequence arrays.
async fn count_changed_hops(
    pool: &sqlx::PgPool,
    _session_id: Uuid,
    prev_snapshot_id: Uuid,
    new_snapshot_id: Uuid,
) -> i16 {
    // Fetch both snapshots' hop sequences
    let prev = sqlx::query_scalar::<_, Vec<Option<String>>>(
        "SELECT hop_sequence FROM route_snapshots WHERE id = $1",
    )
    .bind(prev_snapshot_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let new = sqlx::query_scalar::<_, Vec<Option<String>>>(
        "SELECT hop_sequence FROM route_snapshots WHERE id = $1",
    )
    .bind(new_snapshot_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    match (prev, new) {
        (Some(prev_seq), Some(new_seq)) => {
            let max_len = prev_seq.len().max(new_seq.len());
            let mut changed = 0i16;
            for i in 0..max_len {
                let p = prev_seq.get(i).and_then(|v| v.as_ref());
                let n = new_seq.get(i).and_then(|v| v.as_ref());
                if p != n {
                    changed += 1;
                }
            }
            changed
        }
        _ => 0,
    }
}
