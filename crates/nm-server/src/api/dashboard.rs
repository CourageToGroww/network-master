use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::get,
};

use nm_common::models::DashboardSummary;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboard/summary", get(summary))
}

async fn summary(State(state): State<AppState>) -> Result<Json<DashboardSummary>, StatusCode> {
    let result = sqlx::query_as::<_, DashboardSummary>(
        r#"SELECT
            (SELECT COUNT(*) FROM agents) AS total_agents,
            (SELECT COUNT(*) FROM agents WHERE is_online = true) AS online_agents,
            (SELECT COUNT(*) FROM targets) AS total_targets,
            (SELECT COUNT(*) FROM targets WHERE is_active = true) AS active_targets,
            (SELECT COUNT(*) FROM alert_events WHERE resolved_at IS NULL) AS active_alerts,
            (SELECT COUNT(*) FROM samples WHERE sent_at >= NOW() - interval '24 hours') AS total_samples_24h
        "#,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(result))
}
