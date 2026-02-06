use sqlx::PgPool;
use uuid::Uuid;

use nm_common::models::TraceSession;

pub async fn list_for_target(pool: &PgPool, target_id: Uuid) -> anyhow::Result<Vec<TraceSession>> {
    let sessions = sqlx::query_as::<_, TraceSession>(
        r#"SELECT id, target_id, started_at, ended_at, sample_count
           FROM trace_sessions WHERE target_id = $1 ORDER BY started_at DESC"#,
    )
    .bind(target_id)
    .fetch_all(pool)
    .await?;
    Ok(sessions)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<TraceSession>> {
    let session = sqlx::query_as::<_, TraceSession>(
        r#"SELECT id, target_id, started_at, ended_at, sample_count
           FROM trace_sessions WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(session)
}

pub async fn create(pool: &PgPool, target_id: Uuid) -> anyhow::Result<TraceSession> {
    let session = sqlx::query_as::<_, TraceSession>(
        r#"INSERT INTO trace_sessions (target_id)
           VALUES ($1)
           RETURNING id, target_id, started_at, ended_at, sample_count"#,
    )
    .bind(target_id)
    .fetch_one(pool)
    .await?;
    Ok(session)
}

pub async fn end_session(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE trace_sessions SET ended_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
