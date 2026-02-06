use sqlx::PgPool;
use uuid::Uuid;

use nm_common::models::{CreateTarget, Target, UpdateTarget};

pub async fn list_for_agent(pool: &PgPool, agent_id: Uuid) -> anyhow::Result<Vec<Target>> {
    let targets = sqlx::query_as::<_, Target>(
        r#"SELECT id, agent_id, address, resolved_ip, display_name,
                  probe_method, probe_port, packet_size, interval_ms,
                  max_hops, is_active, created_at, updated_at
           FROM targets WHERE agent_id = $1 ORDER BY created_at"#,
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await?;
    Ok(targets)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<Target>> {
    let target = sqlx::query_as::<_, Target>(
        r#"SELECT id, agent_id, address, resolved_ip, display_name,
                  probe_method, probe_port, packet_size, interval_ms,
                  max_hops, is_active, created_at, updated_at
           FROM targets WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(target)
}

pub async fn create(pool: &PgPool, agent_id: Uuid, input: &CreateTarget) -> anyhow::Result<Target> {
    let target = sqlx::query_as::<_, Target>(
        r#"INSERT INTO targets (agent_id, address, display_name, probe_method, probe_port,
                                packet_size, interval_ms, max_hops)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, agent_id, address, resolved_ip, display_name,
                     probe_method, probe_port, packet_size, interval_ms,
                     max_hops, is_active, created_at, updated_at"#,
    )
    .bind(agent_id)
    .bind(&input.address)
    .bind(&input.display_name)
    .bind(&input.probe_method)
    .bind(input.probe_port)
    .bind(input.packet_size)
    .bind(input.interval_ms)
    .bind(input.max_hops)
    .fetch_one(pool)
    .await?;
    Ok(target)
}

pub async fn update(pool: &PgPool, id: Uuid, input: &UpdateTarget) -> anyhow::Result<Option<Target>> {
    let target = sqlx::query_as::<_, Target>(
        r#"UPDATE targets SET
            address = COALESCE($2, address),
            display_name = COALESCE($3, display_name),
            probe_method = COALESCE($4, probe_method),
            probe_port = COALESCE($5, probe_port),
            packet_size = COALESCE($6, packet_size),
            interval_ms = COALESCE($7, interval_ms),
            max_hops = COALESCE($8, max_hops),
            is_active = COALESCE($9, is_active),
            updated_at = NOW()
           WHERE id = $1
           RETURNING id, agent_id, address, resolved_ip, display_name,
                     probe_method, probe_port, packet_size, interval_ms,
                     max_hops, is_active, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&input.address)
    .bind(&input.display_name)
    .bind(&input.probe_method)
    .bind(input.probe_port)
    .bind(input.packet_size)
    .bind(input.interval_ms)
    .bind(input.max_hops)
    .bind(input.is_active)
    .fetch_optional(pool)
    .await?;
    Ok(target)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM targets WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
