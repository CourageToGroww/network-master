use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use uuid::Uuid;

use nm_common::models::{AlertEvent, AlertRule, CreateAlertRule};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/alert-rules", get(list_rules).post(create_rule))
        .route("/alert-rules/{id}", get(get_rule).delete(delete_rule))
        .route("/alert-events", get(list_events))
}

async fn list_rules(State(state): State<AppState>) -> Result<Json<Vec<AlertRule>>, StatusCode> {
    crate::db::alerts::list_rules(&state.pool)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<AlertRule>, StatusCode> {
    crate::db::alerts::get_rule(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_rule(
    State(state): State<AppState>,
    Json(input): Json<CreateAlertRule>,
) -> Result<(StatusCode, Json<AlertRule>), StatusCode> {
    crate::db::alerts::create_rule(&state.pool, &input)
        .await
        .map(|r| (StatusCode::CREATED, Json(r)))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn delete_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    crate::db::alerts::delete_rule(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct EventsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    100
}

async fn list_events(
    State(state): State<AppState>,
    Query(params): Query<EventsQuery>,
) -> Result<Json<Vec<AlertEvent>>, StatusCode> {
    crate::db::alerts::list_events(&state.pool, params.limit)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
