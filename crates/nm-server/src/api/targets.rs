use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use uuid::Uuid;

use nm_common::models::{CreateTarget, Target, UpdateTarget};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agents/{agent_id}/targets", get(list_targets).post(create_target))
        .route("/targets/{id}", get(get_target).put(update_target).delete(delete_target))
}

async fn list_targets(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Vec<Target>>, StatusCode> {
    crate::db::targets::list_for_agent(&state.pool, agent_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_target(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Target>, StatusCode> {
    crate::db::targets::get_by_id(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_target(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Json(input): Json<CreateTarget>,
) -> Result<(StatusCode, Json<Target>), StatusCode> {
    let target = crate::db::targets::create(&state.pool, agent_id, &input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // TODO: Push TargetAssignment to agent via WS if online

    Ok((StatusCode::CREATED, Json(target)))
}

async fn update_target(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTarget>,
) -> Result<Json<Target>, StatusCode> {
    crate::db::targets::update(&state.pool, id, &input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn delete_target(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    crate::db::targets::delete(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
