use sqlx::PgPool;
use uuid::Uuid;

use nm_common::models::Hop;

pub async fn list_for_session(pool: &PgPool, session_id: Uuid) -> anyhow::Result<Vec<Hop>> {
    let hops = sqlx::query_as::<_, Hop>(
        r#"SELECT id, session_id, hop_number, ip_address, hostname,
                  asn, as_name, geo_country, geo_city, geo_lat, geo_lon,
                  whois_data, first_seen_at, last_seen_at
           FROM hops WHERE session_id = $1 ORDER BY hop_number"#,
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    Ok(hops)
}

pub async fn get_by_session_and_number(
    pool: &PgPool,
    session_id: Uuid,
    hop_number: i16,
) -> anyhow::Result<Option<Hop>> {
    let hop = sqlx::query_as::<_, Hop>(
        r#"SELECT id, session_id, hop_number, ip_address, hostname,
                  asn, as_name, geo_country, geo_city, geo_lat, geo_lon,
                  whois_data, first_seen_at, last_seen_at
           FROM hops WHERE session_id = $1 AND hop_number = $2"#,
    )
    .bind(session_id)
    .bind(hop_number)
    .fetch_optional(pool)
    .await?;
    Ok(hop)
}
