use sqlx::PgPool;
use uuid::Uuid;

use nm_common::models::{AlertEvent, AlertRule, CreateAlertRule};

pub async fn list_rules(pool: &PgPool) -> anyhow::Result<Vec<AlertRule>> {
    let rules = sqlx::query_as::<_, AlertRule>(
        r#"SELECT id, name, target_id, hop_number, metric, comparator,
                  threshold, window_seconds, cooldown_seconds,
                  notify_email, notify_webhook, is_enabled,
                  created_at, updated_at
           FROM alert_rules ORDER BY name"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(rules)
}

pub async fn get_rule(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<AlertRule>> {
    let rule = sqlx::query_as::<_, AlertRule>(
        r#"SELECT id, name, target_id, hop_number, metric, comparator,
                  threshold, window_seconds, cooldown_seconds,
                  notify_email, notify_webhook, is_enabled,
                  created_at, updated_at
           FROM alert_rules WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rule)
}

pub async fn create_rule(pool: &PgPool, input: &CreateAlertRule) -> anyhow::Result<AlertRule> {
    let rule = sqlx::query_as::<_, AlertRule>(
        r#"INSERT INTO alert_rules (name, target_id, hop_number, metric, comparator,
                                    threshold, window_seconds, cooldown_seconds,
                                    notify_email, notify_webhook)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
           RETURNING id, name, target_id, hop_number, metric, comparator,
                     threshold, window_seconds, cooldown_seconds,
                     notify_email, notify_webhook, is_enabled,
                     created_at, updated_at"#,
    )
    .bind(&input.name)
    .bind(input.target_id)
    .bind(input.hop_number)
    .bind(&input.metric)
    .bind(&input.comparator)
    .bind(input.threshold)
    .bind(input.window_seconds)
    .bind(input.cooldown_seconds)
    .bind(&input.notify_email)
    .bind(&input.notify_webhook)
    .fetch_one(pool)
    .await?;
    Ok(rule)
}

pub async fn delete_rule(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM alert_rules WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_events(pool: &PgPool, limit: i64) -> anyhow::Result<Vec<AlertEvent>> {
    let events = sqlx::query_as::<_, AlertEvent>(
        r#"SELECT id, rule_id, session_id, hop_id, triggered_at,
                  metric_value, threshold_value, message,
                  notified, resolved_at
           FROM alert_events ORDER BY triggered_at DESC LIMIT $1"#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(events)
}
