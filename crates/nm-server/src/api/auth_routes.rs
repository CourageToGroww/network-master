use axum::{
    extract::{Request, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use nm_common::models::{CreateUser, LoginRequest, LoginResponse, UserPublic};
use crate::{auth, state::AppState};

/// Public routes (no auth required)
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/register", post(register))
}

/// Protected routes (auth required)
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/auth/me", get(me))
}

async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    // Validate input
    if input.email.is_empty() || input.password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Email and password required"}))));
    }

    let user = crate::db::users::get_by_email(&state.pool, &input.email)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid credentials"}))))?;

    let valid = bcrypt::verify(&input.password, &user.password_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Auth error"}))))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid credentials"}))));
    }

    let token = auth::create_token(
        user.id,
        &user.email,
        &user.role,
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Token error"}))))?;

    let _ = crate::db::users::update_last_login(&state.pool, user.id).await;

    Ok(Json(LoginResponse {
        token,
        user: UserPublic {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            role: user.role,
        },
    }))
}

async fn register(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserPublic>), (StatusCode, Json<serde_json::Value>)> {
    // Validate email format
    if !input.email.contains('@') || input.email.len() < 5 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid email"}))));
    }

    // Validate password strength
    if input.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Password must be at least 8 characters"}))));
    }

    // Validate role
    if !["admin", "operator", "viewer"].contains(&input.role.as_str()) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid role"}))));
    }

    let password_hash = bcrypt::hash(&input.password, 12)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Hash error"}))))?;

    let user = crate::db::users::create(&state.pool, &input, &password_hash)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate") {
                (StatusCode::CONFLICT, Json(json!({"error": "Email already exists"})))
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
            }
        })?;

    Ok((StatusCode::CREATED, Json(UserPublic {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        role: user.role,
    })))
}

async fn me(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<UserPublic>, (StatusCode, Json<serde_json::Value>)> {
    let claims = auth::get_claims(&request)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Not authenticated"}))))?;

    let user = crate::db::users::get_by_id(&state.pool, claims.sub)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({"error": "User not found"}))))?;

    Ok(Json(UserPublic {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        role: user.role,
    }))
}
