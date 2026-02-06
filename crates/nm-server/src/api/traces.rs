use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use nm_common::models::{Hop, TraceSession};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/targets/{target_id}/sessions", get(list_sessions))
        .route("/sessions/{id}", get(get_session))
        .route("/sessions/{id}/hops", get(list_hops))
        .route("/sessions/{session_id}/hops/{hop_number}", get(get_hop))
        .route("/sessions/{id}/samples/timeseries", get(get_timeseries))
}

async fn list_sessions(
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
) -> Result<Json<Vec<TraceSession>>, StatusCode> {
    crate::db::sessions::list_for_target(&state.pool, target_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TraceSession>, StatusCode> {
    crate::db::sessions::get_by_id(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn list_hops(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Hop>>, StatusCode> {
    crate::db::hops::list_for_session(&state.pool, id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_hop(
    State(state): State<AppState>,
    Path((session_id, hop_number)): Path<(Uuid, i16)>,
) -> Result<Json<Hop>, StatusCode> {
    crate::db::hops::get_by_session_and_number(&state.pool, session_id, hop_number)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
struct TimeseriesQuery {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    #[serde(default = "default_resolution")]
    resolution: String,
    hop_id: Uuid,
}

fn default_resolution() -> String {
    "1m".to_string()
}

fn parse_resolution(s: &str) -> i32 {
    match s {
        "1s" => 1,
        "10s" => 10,
        "1m" => 60,
        "5m" => 300,
        "1h" => 3600,
        "1d" => 86400,
        _ => 60,
    }
}

async fn get_timeseries(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
    Query(params): Query<TimeseriesQuery>,
) -> Result<Json<Vec<nm_common::models::TimeSeriesDatapoint>>, StatusCode> {
    let resolution_secs = parse_resolution(&params.resolution);
    crate::db::samples::get_timeseries(
        &state.pool,
        session_id,
        params.hop_id,
        params.from,
        params.to,
        resolution_secs,
    )
    .await
    .map(Json)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
