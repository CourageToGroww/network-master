use nm_common::protocol::{AlertFiredNotification, TraceRoundReport};
use uuid::Uuid;

use crate::state::AppState;

/// Evaluate all enabled alert rules against the current round's running stats.
/// Called inline after ingestion updates the running hop stats.
pub async fn evaluate_for_round(
    report: &TraceRoundReport,
    state: &AppState,
) {
    // Load enabled rules that match this target
    let rules = match sqlx::query_as::<_, RuleRow>(
        r#"SELECT id, name, target_id, hop_number, metric, comparator,
                  threshold, window_seconds, cooldown_seconds,
                  notify_email, notify_webhook
           FROM alert_rules
           WHERE is_enabled = TRUE
             AND (target_id IS NULL OR target_id = $1)"#,
    )
    .bind(report.target_id)
    .fetch_all(&state.pool)
    .await
    {
        Ok(rules) => rules,
        Err(e) => {
            tracing::error!("Failed to load alert rules: {}", e);
            return;
        }
    };

    if rules.is_empty() {
        return;
    }

    let session_id = report.session_id;

    for rule in &rules {
        // Determine which hops to evaluate
        let hops_to_check: Vec<u8> = if let Some(hop_num) = rule.hop_number {
            vec![hop_num as u8]
        } else {
            report.hops.iter().map(|h| h.hop_number).collect()
        };

        for &hop_number in &hops_to_check {
            let key = (session_id, hop_number);
            let Some(stats) = state.hop_stats.get(&key) else {
                continue;
            };

            // Compute the metric value from running stats
            let metric_value = match rule.metric.as_str() {
                "avg_rtt" => stats.avg_rtt_us() as f64 / 1000.0, // convert to ms
                "max_rtt" => stats.max_rtt_us as f64 / 1000.0,
                "min_rtt" => {
                    if stats.min_rtt_us == u32::MAX { 0.0 }
                    else { stats.min_rtt_us as f64 / 1000.0 }
                }
                "loss_pct" => stats.loss_pct(),
                "jitter" => stats.avg_jitter_us() as f64 / 1000.0,
                _ => continue, // Unknown metric, skip
            };

            // Compare against threshold
            let triggered = match rule.comparator.as_str() {
                "gt" | ">" => metric_value > rule.threshold,
                "gte" | ">=" => metric_value >= rule.threshold,
                "lt" | "<" => metric_value < rule.threshold,
                "lte" | "<=" => metric_value <= rule.threshold,
                "eq" | "==" => (metric_value - rule.threshold).abs() < f64::EPSILON,
                _ => false,
            };

            if !triggered {
                continue;
            }

            // Check cooldown: don't fire if we fired recently for this rule
            let recently_fired = sqlx::query_scalar::<_, bool>(
                r#"SELECT EXISTS(
                    SELECT 1 FROM alert_events
                    WHERE rule_id = $1
                      AND triggered_at > NOW() - ($2 || ' seconds')::interval
                )"#,
            )
            .bind(rule.id)
            .bind(rule.cooldown_seconds.to_string())
            .fetch_one(&state.pool)
            .await
            .unwrap_or(false);

            if recently_fired {
                continue;
            }

            // Fire the alert!
            let message = format!(
                "{}: {} {} {:.2} (threshold: {:.2}) on hop {}",
                rule.name, rule.metric, rule.comparator, metric_value, rule.threshold, hop_number,
            );

            let event_id = match sqlx::query_scalar::<_, Uuid>(
                r#"INSERT INTO alert_events (rule_id, session_id, metric_value, threshold_value, message)
                   VALUES ($1, $2, $3, $4, $5)
                   RETURNING id"#,
            )
            .bind(rule.id)
            .bind(session_id)
            .bind(metric_value)
            .bind(rule.threshold)
            .bind(&message)
            .fetch_one(&state.pool)
            .await
            {
                Ok(id) => id,
                Err(e) => {
                    tracing::error!("Failed to insert alert event: {}", e);
                    continue;
                }
            };

            tracing::warn!("Alert fired: {}", message);

            // Broadcast to frontend
            let notification = AlertFiredNotification {
                alert_event_id: event_id,
                rule_name: rule.name.clone(),
                target_address: String::new(), // filled below if available
                hop_number: Some(hop_number),
                metric: rule.metric.clone(),
                value: metric_value,
                threshold: rule.threshold,
                message: message.clone(),
            };

            let _ = state.alert_tx.send(notification);

            // Send webhook notification if configured
            if let Some(ref url) = rule.notify_webhook {
                tokio::spawn(send_webhook(url.clone(), message.clone(), metric_value, rule.threshold));
            }
        }
    }
}

/// Row type for alert rule queries (avoids pulling in the full model).
#[derive(sqlx::FromRow)]
struct RuleRow {
    id: Uuid,
    name: String,
    #[allow(dead_code)]
    target_id: Option<Uuid>,
    hop_number: Option<i16>,
    metric: String,
    comparator: String,
    threshold: f64,
    #[allow(dead_code)]
    window_seconds: i32,
    cooldown_seconds: i32,
    #[allow(dead_code)]
    notify_email: Option<String>,
    notify_webhook: Option<String>,
}

/// Send a webhook POST notification (fire and forget).
async fn send_webhook(url: String, message: String, value: f64, threshold: f64) {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "text": message,
        "metric_value": value,
        "threshold": threshold,
        "source": "network-master",
    });

    match client.post(&url).json(&payload).send().await {
        Ok(resp) => {
            if !resp.status().is_success() {
                tracing::warn!("Webhook returned {}: {}", resp.status(), url);
            }
        }
        Err(e) => {
            tracing::error!("Webhook failed for {}: {}", url, e);
        }
    }
}
