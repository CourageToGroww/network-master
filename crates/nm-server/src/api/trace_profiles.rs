use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use uuid::Uuid;

use nm_common::models::{CreateTraceProfile, TraceProfile, UpdateTraceProfile};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/trace-profiles", get(list_profiles).post(create_profile))
        .route(
            "/trace-profiles/{id}",
            get(get_profile).put(update_profile).delete(delete_profile),
        )
}

async fn list_profiles(
    State(state): State<AppState>,
) -> Result<Json<Vec<TraceProfile>>, StatusCode> {
    crate::db::trace_profiles::list_all(&state.pool)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn get_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TraceProfile>, StatusCode> {
    crate::db::trace_profiles::get_by_id(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn create_profile(
    State(state): State<AppState>,
    Json(input): Json<CreateTraceProfile>,
) -> Result<(StatusCode, Json<TraceProfile>), StatusCode> {
    let profile = crate::db::trace_profiles::create(&state.pool, &input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(profile)))
}

async fn update_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateTraceProfile>,
) -> Result<Json<TraceProfile>, StatusCode> {
    crate::db::trace_profiles::update(&state.pool, id, &input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn delete_profile(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    crate::db::trace_profiles::delete(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}
