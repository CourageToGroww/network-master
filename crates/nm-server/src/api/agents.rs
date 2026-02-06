use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use uuid::Uuid;

use nm_common::models::{Agent, AgentRegistration, CreateAgent};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/agents", get(list_agents).post(register_agent))
        .route("/agents/{id}", get(get_agent).delete(delete_agent))
}

async fn list_agents(State(state): State<AppState>) -> Result<Json<Vec<Agent>>, StatusCode> {
    crate::db::agents::list(&state.pool)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_agent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Agent>, StatusCode> {
    crate::db::agents::get_by_id(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn register_agent(
    State(state): State<AppState>,
    Json(input): Json<CreateAgent>,
) -> Result<(StatusCode, Json<AgentRegistration>), StatusCode> {
    let api_key = nm_common::crypto::generate_api_key();
    let api_key_hash = bcrypt::hash(&api_key, bcrypt::DEFAULT_COST)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let agent = crate::db::agents::create(&state.pool, &input, &api_key_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        StatusCode::CREATED,
        Json(AgentRegistration { agent, api_key }),
    ))
}

async fn delete_agent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    crate::db::agents::delete(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
