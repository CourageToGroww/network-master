use axum::{
    Router,
    extract::{Path, Query, State},
    routing::get,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/agents/{agent_id}/traffic", get(get_agent_traffic))
}

#[derive(Deserialize)]
struct TrafficQuery {
    limit: Option<i64>,
}

async fn get_agent_traffic(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Query(query): Query<TrafficQuery>,
) -> Json<Vec<crate::db::traffic::TrafficSnapshotRow>> {
    let limit = query.limit.unwrap_or(500);
    match crate::db::traffic::recent_traffic(&state.pool, agent_id, limit).await {
        Ok(rows) => Json(rows),
        Err(e) => {
            tracing::error!("Failed to query traffic: {e}");
            Json(vec![])
        }
    }
}
