use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/export/csv/{session_id}", get(export_csv))
}

#[derive(Deserialize)]
struct ExportQuery {
    from: Option<DateTime<Utc>>,
    to: Option<DateTime<Utc>>,
}

async fn export_csv(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<ExportQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    let rows = crate::db::exports::get_session_csv_data(
        &state.pool,
        session_id,
        params.from,
        params.to,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut csv = String::from("timestamp,hop_number,ip_address,hostname,rtt_us,is_lost,jitter_us\n");
    for row in &rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            row.sent_at.to_rfc3339(),
            row.hop_number,
            row.ip_address.as_deref().unwrap_or(""),
            row.hostname.as_deref().unwrap_or(""),
            row.rtt_us.map(|v| v.to_string()).unwrap_or_default(),
            row.is_lost,
            row.jitter_us.map(|v| v.to_string()).unwrap_or_default(),
        ));
    }

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"trace_export.csv\""),
        ],
        csv,
    ))
}
