use axum::Router;

mod agents;
mod alerts;
mod auth_routes;
mod dashboard;
mod exports;
mod shares;
mod targets;
mod trace_profiles;
mod traces;
mod traffic;
mod update;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // Public auth routes (no auth required)
        .merge(auth_routes::public_router())
        // Protected auth routes
        .merge(auth_routes::protected_router())
        // Existing routes
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
}
