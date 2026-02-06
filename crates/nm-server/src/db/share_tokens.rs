use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use nm_common::models::{CreateShareToken, ShareToken};

pub async fn list_for_target(pool: &PgPool, target_id: Uuid) -> anyhow::Result<Vec<ShareToken>> {
    let tokens = sqlx::query_as::<_, ShareToken>(
        r#"SELECT id, token, target_id, label, is_readonly, expires_at, created_at
           FROM share_tokens
           WHERE target_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(target_id)
    .fetch_all(pool)
    .await?;
    Ok(tokens)
}

pub async fn get_by_token(pool: &PgPool, token: &str) -> anyhow::Result<Option<ShareToken>> {
    let share = sqlx::query_as::<_, ShareToken>(
        r#"SELECT id, token, target_id, label, is_readonly, expires_at, created_at
           FROM share_tokens
           WHERE token = $1
             AND (expires_at IS NULL OR expires_at > NOW())"#,
    )
    .bind(token)
    .fetch_optional(pool)
    .await?;
    Ok(share)
}

pub async fn create(pool: &PgPool, input: &CreateShareToken) -> anyhow::Result<ShareToken> {
    // Generate a random URL-safe token
    let token = generate_token();

    let expires_at = input
        .expires_in_hours
        .map(|h| Utc::now() + chrono::Duration::hours(h));

    let share = sqlx::query_as::<_, ShareToken>(
        r#"INSERT INTO share_tokens (token, target_id, label, is_readonly, expires_at)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, token, target_id, label, is_readonly, expires_at, created_at"#,
    )
    .bind(&token)
    .bind(input.target_id)
    .bind(&input.label)
    .bind(input.is_readonly)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok(share)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM share_tokens WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

fn generate_token() -> String {
    // Concatenate two UUID v4 simple representations for a 64-char hex token
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}
