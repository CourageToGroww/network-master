use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use nm_common::models::TimeSeriesDatapoint;

pub async fn get_timeseries(
    pool: &PgPool,
    session_id: Uuid,
    hop_id: Uuid,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    resolution_seconds: i32,
) -> anyhow::Result<Vec<TimeSeriesDatapoint>> {
    let rows = sqlx::query_as::<_, TimeSeriesDatapoint>(
        r#"SELECT
            time_bucket AS "timestamp",
            rtt_avg_us,
            rtt_min_us,
            rtt_max_us,
            loss_pct,
            jitter_avg_us,
            sample_count
        FROM (
            SELECT
                date_trunc('second', sent_at) -
                    (EXTRACT(EPOCH FROM date_trunc('second', sent_at))::int % $5) * interval '1 second'
                    AS time_bucket,
                AVG(rtt_us)::int AS rtt_avg_us,
                MIN(rtt_us) AS rtt_min_us,
                MAX(rtt_us) AS rtt_max_us,
                CASE WHEN COUNT(*) > 0
                    THEN (COUNT(*) FILTER (WHERE is_lost))::float / COUNT(*)::float * 100.0
                    ELSE 0.0
                END AS loss_pct,
                AVG(jitter_us)::int AS jitter_avg_us,
                COUNT(*) AS sample_count
            FROM samples
            WHERE session_id = $1
                AND hop_id = $2
                AND sent_at >= $3
                AND sent_at < $4
            GROUP BY time_bucket
            ORDER BY time_bucket
        ) sub"#,
    )
    .bind(session_id)
    .bind(hop_id)
    .bind(from)
    .bind(to)
    .bind(resolution_seconds)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn recent_loss_pct(pool: &PgPool, hop_id: Uuid, window_seconds: i32) -> f64 {
    let result = sqlx::query_scalar::<_, Option<f64>>(
        r#"SELECT CASE WHEN COUNT(*) > 0
               THEN (COUNT(*) FILTER (WHERE is_lost))::float / COUNT(*)::float * 100.0
               ELSE 0.0
           END
           FROM samples
           WHERE hop_id = $1 AND sent_at >= NOW() - ($2 || ' seconds')::interval"#,
    )
    .bind(hop_id)
    .bind(window_seconds.to_string())
    .fetch_one(pool)
    .await;

    result.ok().flatten().unwrap_or(0.0)
}
