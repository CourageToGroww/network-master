use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(FromRow)]
pub struct CsvRow {
    pub sent_at: DateTime<Utc>,
    pub hop_number: i16,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub rtt_us: Option<i32>,
    pub is_lost: bool,
    pub jitter_us: Option<i32>,
}

pub async fn get_session_csv_data(
    pool: &PgPool,
    session_id: Uuid,
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
) -> anyhow::Result<Vec<CsvRow>> {
    let rows = sqlx::query_as::<_, CsvRow>(
        r#"SELECT s.sent_at, h.hop_number, h.ip_address, h.hostname,
                  s.rtt_us, s.is_lost, s.jitter_us
           FROM samples s
           JOIN hops h ON h.id = s.hop_id
           WHERE s.session_id = $1
                AND ($2::timestamptz IS NULL OR s.sent_at >= $2)
                AND ($3::timestamptz IS NULL OR s.sent_at < $3)
           ORDER BY s.sent_at, h.hop_number"#,
    )
    .bind(session_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
