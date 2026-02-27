use axum::{middleware, Router};

mod agents;
mod alerts;
mod auth_routes;
mod dashboard;
mod download;
mod exports;
mod shares;
mod targets;
mod trace_profiles;
mod traces;
mod traffic;
mod update;

use crate::auth::require_auth;
use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    // Public routes (no auth required)
    let public = Router::new()
        .merge(auth_routes::public_router())
        .merge(shares::public_router());

    // Protected routes (require valid JWT)
    let protected = Router::new()
        .merge(auth_routes::protected_router())
        .merge(agents::router())
        .merge(targets::router())
        .merge(traces::router())
        .merge(alerts::router())
        .merge(exports::router())
        .merge(dashboard::router())
        .merge(trace_profiles::router())
        .merge(shares::router())
        .merge(traffic::router())
        .merge(update::router())
        .route_layer(middleware::from_fn_with_state(state, require_auth));

    public.merge(protected)
}

/// Public download routes — mounted at root level, not under /api/v1.
pub fn download_router() -> Router<AppState> {
    download::router()
}
