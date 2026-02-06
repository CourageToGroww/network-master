use axum::Router;

mod agents;
mod alerts;
mod dashboard;
mod exports;
mod shares;
mod targets;
mod trace_profiles;
mod traces;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(agents::router())
        .merge(targets::router())
        .merge(traces::router())
        .merge(alerts::router())
        .merge(exports::router())
        .merge(dashboard::router())
        .merge(trace_profiles::router())
        .merge(shares::router())
}
