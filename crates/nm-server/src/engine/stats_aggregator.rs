use crate::state::AppState;
use std::time::Duration;

/// Background task that periodically computes hourly rollup statistics.
pub async fn run(state: AppState) {
    let interval_secs = state.config.stats_aggregation_interval_secs;
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;
        if let Err(e) = aggregate_hourly_stats(&state).await {
            tracing::error!("Stats aggregation failed: {}", e);
        }
    }
}

async fn aggregate_hourly_stats(state: &AppState) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        WITH jitter_calc AS (
            SELECT
                hop_id,
                session_id,
                sent_at,
                rtt_us,
                is_lost,
                ABS(rtt_us - LAG(rtt_us) OVER (PARTITION BY hop_id ORDER BY sent_at)) AS jitter_us
            FROM samples
            WHERE sent_at >= NOW() - interval '2 hours'
        )
        INSERT INTO hop_stats_hourly (hop_id, session_id, hour, sample_count, loss_count,
            loss_pct, rtt_min_us, rtt_avg_us, rtt_max_us, rtt_stddev_us,
            jitter_avg_us, jitter_max_us)
        SELECT
            s.hop_id,
            s.session_id,
            date_trunc('hour', s.sent_at) AS hour,
            COUNT(*) AS sample_count,
            COUNT(*) FILTER (WHERE s.is_lost) AS loss_count,
            CASE WHEN COUNT(*) > 0
                THEN (COUNT(*) FILTER (WHERE s.is_lost))::float / COUNT(*)::float * 100.0
                ELSE 0.0
            END AS loss_pct,
            MIN(s.rtt_us),
            AVG(s.rtt_us)::int,
            MAX(s.rtt_us),
            STDDEV(s.rtt_us)::int,
            AVG(s.jitter_us)::int,
            MAX(s.jitter_us)
        FROM jitter_calc s
        WHERE s.sent_at >= NOW() - interval '2 hours'
        GROUP BY s.hop_id, s.session_id, date_trunc('hour', s.sent_at)
        ON CONFLICT (hop_id, hour) DO UPDATE SET
            sample_count = EXCLUDED.sample_count,
            loss_count = EXCLUDED.loss_count,
            loss_pct = EXCLUDED.loss_pct,
            rtt_min_us = EXCLUDED.rtt_min_us,
            rtt_avg_us = EXCLUDED.rtt_avg_us,
            rtt_max_us = EXCLUDED.rtt_max_us,
            rtt_stddev_us = EXCLUDED.rtt_stddev_us,
            jitter_avg_us = EXCLUDED.jitter_avg_us,
            jitter_max_us = EXCLUDED.jitter_max_us
        "#,
    )
    .execute(&state.pool)
    .await?;

    tracing::debug!("Hourly stats aggregation completed");
    Ok(())
}
