use sqlx::PgPool;
use uuid::Uuid;

use nm_common::models::{CreateTraceProfile, TraceProfile, UpdateTraceProfile};

pub async fn list_all(pool: &PgPool) -> anyhow::Result<Vec<TraceProfile>> {
    let profiles = sqlx::query_as::<_, TraceProfile>(
        r#"SELECT id, name, description, probe_method, probe_port,
                  packet_size, interval_ms, max_hops, created_at, updated_at
           FROM trace_profiles ORDER BY name"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(profiles)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<TraceProfile>> {
    let profile = sqlx::query_as::<_, TraceProfile>(
        r#"SELECT id, name, description, probe_method, probe_port,
                  packet_size, interval_ms, max_hops, created_at, updated_at
           FROM trace_profiles WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(profile)
}

pub async fn create(pool: &PgPool, input: &CreateTraceProfile) -> anyhow::Result<TraceProfile> {
    let profile = sqlx::query_as::<_, TraceProfile>(
        r#"INSERT INTO trace_profiles (name, description, probe_method, probe_port,
                                        packet_size, interval_ms, max_hops)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, name, description, probe_method, probe_port,
                     packet_size, interval_ms, max_hops, created_at, updated_at"#,
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.probe_method)
    .bind(input.probe_port)
    .bind(input.packet_size)
    .bind(input.interval_ms)
    .bind(input.max_hops)
    .fetch_one(pool)
    .await?;
    Ok(profile)
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateTraceProfile,
) -> anyhow::Result<Option<TraceProfile>> {
    let profile = sqlx::query_as::<_, TraceProfile>(
        r#"UPDATE trace_profiles SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            probe_method = COALESCE($4, probe_method),
            probe_port = COALESCE($5, probe_port),
            packet_size = COALESCE($6, packet_size),
            interval_ms = COALESCE($7, interval_ms),
            max_hops = COALESCE($8, max_hops),
            updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, description, probe_method, probe_port,
                     packet_size, interval_ms, max_hops, created_at, updated_at"#,
    )
    .bind(id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.probe_method)
    .bind(input.probe_port)
    .bind(input.packet_size)
    .bind(input.interval_ms)
    .bind(input.max_hops)
    .fetch_optional(pool)
    .await?;
    Ok(profile)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM trace_profiles WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
