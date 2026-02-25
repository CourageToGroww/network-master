use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
};
use uuid::Uuid;

use nm_common::models::{CreateShareToken, ShareToken, Target};
use crate::state::AppState;

/// Protected share management routes (require auth)
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/targets/{target_id}/shares", get(list_shares).post(create_share))
        .route("/shares/{id}", axum::routing::delete(delete_share))
}

/// Public share lookup route (no auth required)
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/share/{token}", get(get_shared_target))
}

async fn list_shares(
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
) -> Result<Json<Vec<ShareToken>>, StatusCode> {
    crate::db::share_tokens::list_for_target(&state.pool, target_id)
        .await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn create_share(
    State(state): State<AppState>,
    Path(target_id): Path<Uuid>,
    Json(mut input): Json<CreateShareToken>,
) -> Result<(StatusCode, Json<ShareToken>), StatusCode> {
    input.target_id = target_id;
    let share = crate::db::share_tokens::create(&state.pool, &input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::CREATED, Json(share)))
}

async fn delete_share(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    crate::db::share_tokens::delete(&state.pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Public endpoint: look up target info by share token (read-only).
async fn get_shared_target(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<SharedTargetInfo>, StatusCode> {
    let share = crate::db::share_tokens::get_by_token(&state.pool, &token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let target = crate::db::targets::get_by_id(&state.pool, share.target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(SharedTargetInfo {
        target,
        share_label: share.label,
        is_readonly: share.is_readonly,
    }))
}

#[derive(serde::Serialize)]
struct SharedTargetInfo {
    target: Target,
    share_label: Option<String>,
    is_readonly: bool,
}
