use sqlx::PgPool;
use uuid::Uuid;

use nm_common::models::{Agent, CreateAgent};

pub async fn list(pool: &PgPool) -> anyhow::Result<Vec<Agent>> {
    let agents = sqlx::query_as::<_, Agent>(
        r#"SELECT id, name, hostname, os_info, version, ip_address,
                  is_online, last_seen_at, created_at, updated_at
           FROM agents ORDER BY name"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(agents)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<Agent>> {
    let agent = sqlx::query_as::<_, Agent>(
        r#"SELECT id, name, hostname, os_info, version, ip_address,
                  is_online, last_seen_at, created_at, updated_at
           FROM agents WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(agent)
}

pub async fn create(pool: &PgPool, input: &CreateAgent, api_key_hash: &str) -> anyhow::Result<Agent> {
    let agent = sqlx::query_as::<_, Agent>(
        r#"INSERT INTO agents (name, api_key_hash)
           VALUES ($1, $2)
           RETURNING id, name, hostname, os_info, version, ip_address,
                     is_online, last_seen_at, created_at, updated_at"#,
    )
    .bind(&input.name)
    .bind(api_key_hash)
    .fetch_one(pool)
    .await?;
    Ok(agent)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM agents WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_online(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE agents SET is_online = true, last_seen_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_offline(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE agents SET is_online = false, last_seen_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
