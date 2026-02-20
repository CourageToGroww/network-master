use chrono::{DateTime, Utc};
use nm_common::protocol::ProcessNetworkEntry;
use sqlx::PgPool;
use uuid::Uuid;

/// Store a traffic snapshot and its per-process entries.
pub async fn store_snapshot(
    pool: &PgPool,
    agent_id: Uuid,
    captured_at: DateTime<Utc>,
    interval_ms: i32,
    entries: &[ProcessNetworkEntry],
) -> anyhow::Result<()> {
    let snapshot_id: (i64,) = sqlx::query_as(
        r#"INSERT INTO process_traffic_snapshots (agent_id, captured_at, interval_ms)
           VALUES ($1, $2, $3)
           RETURNING id"#,
    )
    .bind(agent_id)
    .bind(captured_at)
    .bind(interval_ms)
    .fetch_one(pool)
    .await?;

    for entry in entries {
        sqlx::query(
            r#"INSERT INTO process_traffic_entries
               (snapshot_id, pid, process_name, exe_path, bytes_in, bytes_out, active_connections)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(snapshot_id.0)
        .bind(entry.pid as i32)
        .bind(&entry.process_name)
        .bind(&entry.exe_path)
        .bind(entry.total_bytes_in as i64)
        .bind(entry.total_bytes_out as i64)
        .bind(entry.active_connection_count as i32)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Query recent traffic entries for an agent.
pub async fn recent_traffic(
    pool: &PgPool,
    agent_id: Uuid,
    limit: i64,
) -> anyhow::Result<Vec<TrafficSnapshotRow>> {
    let rows = sqlx::query_as::<_, TrafficSnapshotRow>(
        r#"SELECT s.id, s.captured_at, s.interval_ms,
                  e.pid, e.process_name, e.exe_path, e.bytes_in, e.bytes_out, e.active_connections
           FROM process_traffic_snapshots s
           JOIN process_traffic_entries e ON e.snapshot_id = s.id
           WHERE s.agent_id = $1
           ORDER BY s.captured_at DESC, e.bytes_in + e.bytes_out DESC
           LIMIT $2"#,
    )
    .bind(agent_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct TrafficSnapshotRow {
    pub id: i64,
    pub captured_at: DateTime<Utc>,
    pub interval_ms: i32,
    pub pid: i32,
    pub process_name: String,
    pub exe_path: Option<String>,
    pub bytes_in: i64,
    pub bytes_out: i64,
    pub active_connections: i32,
}
