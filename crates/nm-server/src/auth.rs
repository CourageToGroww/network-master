use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde_json::json;
use uuid::Uuid;

use nm_common::models::JwtClaims;
use crate::state::AppState;

/// Create a JWT token for a user
pub fn create_token(
    user_id: Uuid,
    email: &str,
    role: &str,
    secret: &str,
    expiry_hours: u64,
) -> anyhow::Result<String> {
    let now = chrono::Utc::now();
    let claims = JwtClaims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::hours(expiry_hours as i64)).timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// Validate a JWT token and return claims
pub fn validate_token(token: &str, secret: &str) -> Result<JwtClaims, StatusCode> {
    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Extract claims from request extensions (used by handlers after auth middleware)
pub fn get_claims(request: &Request) -> Option<&JwtClaims> {
    request.extensions().get::<JwtClaims>()
}

/// Extract bearer token from Authorization header
fn extract_bearer(request: &Request) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Middleware: require valid JWT
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(&request) {
        Some(t) => t.to_owned(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization header"})),
            )
                .into_response();
        }
    };

    match validate_token(&token, &state.config.jwt_secret) {
        Ok(claims) => {
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
            .into_response(),
    }
}

/// Middleware: require admin role
pub async fn require_admin(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(&request) {
        Some(t) => t.to_owned(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization header"})),
            )
                .into_response();
        }
    };

    match validate_token(&token, &state.config.jwt_secret) {
        Ok(claims) => {
            if claims.role != "admin" {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Admin access required"})),
                )
                    .into_response();
            }
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
            .into_response(),
    }
}

/// Middleware: require operator or admin role
pub async fn require_operator(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = match extract_bearer(&request) {
        Some(t) => t.to_owned(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization header"})),
            )
                .into_response();
        }
    };

    match validate_token(&token, &state.config.jwt_secret) {
        Ok(claims) => {
            if claims.role != "admin" && claims.role != "operator" {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Operator access required"})),
                )
                    .into_response();
            }
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
            .into_response(),
    }
}
