use axum::{
    Router,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/update/upload", post(upload_binary))
        .route("/update/binary", get(download_binary))
        .route("/update/info", get(get_info))
        .route("/update/push-all", post(trigger_update_all))
        .route("/agents/{id}/update", post(trigger_update))
}

#[derive(Serialize, Deserialize)]
struct UpdateInfo {
    version: String,
    sha256: String,
    size_bytes: u64,
    uploaded_at: String,
}

/// POST /api/v1/update/upload — Multipart upload of agent binary
async fn upload_binary(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    let mut version: Option<String> = None;
    let mut binary_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "version" => {
                version = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| StatusCode::BAD_REQUEST)?,
                );
            }
            "binary" => {
                binary_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| StatusCode::BAD_REQUEST)?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let version = version.ok_or(StatusCode::BAD_REQUEST)?;
    let data = binary_data.ok_or(StatusCode::BAD_REQUEST)?;

    // Compute SHA256
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let sha256 = hex::encode(hasher.finalize());

    let size_bytes = data.len() as u64;

    // Save binary
    let binary_path = state.update_dir.join("nm-agent.exe");
    tokio::fs::write(&binary_path, &data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write update binary: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Save metadata
    let info = UpdateInfo {
        version,
        sha256,
        size_bytes,
        uploaded_at: chrono::Utc::now().to_rfc3339(),
    };

    let info_path = state.update_dir.join("latest.json");
    let info_json = serde_json::to_string_pretty(&info).unwrap();
    tokio::fs::write(&info_path, info_json)
        .await
        .map_err(|e| {
            tracing::error!("Failed to write update info: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::info!(
        version = %info.version,
        size = size_bytes,
        sha256 = %info.sha256,
        "Agent update binary uploaded"
    );

    Ok((StatusCode::OK, axum::Json(info)))
}

/// GET /api/v1/update/binary — Stream the binary file to an agent
async fn download_binary(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let binary_path = state.update_dir.join("nm-agent.exe");

    let data = tokio::fs::read(&binary_path).await.map_err(|_| {
        StatusCode::NOT_FOUND
    })?;

    Ok((
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/octet-stream",
        )],
        data,
    ))
}

/// GET /api/v1/update/info — Return info about the latest uploaded binary
async fn get_info(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let info_path = state.update_dir.join("latest.json");

    let data = tokio::fs::read_to_string(&info_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let info: UpdateInfo =
        serde_json::from_str(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(axum::Json(info))
}

/// POST /api/v1/agents/{id}/update — Send UpdateCommand to a connected agent
async fn trigger_update(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    // Read latest update info
    let info_path = state.update_dir.join("latest.json");
    let data = tokio::fs::read_to_string(&info_path)
        .await
        .map_err(|_| {
            tracing::error!("No update binary uploaded yet");
            StatusCode::NOT_FOUND
        })?;

    let info: UpdateInfo =
        serde_json::from_str(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build the download URL relative to the server
    let cmd = nm_common::protocol::UpdateCommand {
        version: info.version.clone(),
        download_url: "/api/v1/update/binary".to_string(),
        sha256: info.sha256.clone(),
    };

    let envelope =
        nm_common::protocol::WsEnvelope::new(nm_common::protocol::WsPayload::UpdateCommand(cmd));

    state
        .agent_registry
        .send_to_agent(&agent_id, envelope)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send update command to agent {}: {}", agent_id, e);
            StatusCode::BAD_GATEWAY
        })?;

    tracing::info!(
        agent_id = %agent_id,
        version = %info.version,
        "Update command sent to agent"
    );

    Ok(StatusCode::OK)
}

/// POST /api/v1/update/push-all — Send UpdateCommand to ALL connected agents
async fn trigger_update_all(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let info_path = state.update_dir.join("latest.json");
    let data = tokio::fs::read_to_string(&info_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let info: UpdateInfo =
        serde_json::from_str(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cmd = nm_common::protocol::UpdateCommand {
        version: info.version.clone(),
        download_url: "/api/v1/update/binary".to_string(),
        sha256: info.sha256.clone(),
    };

    let agent_ids = state.agent_registry.online_agent_ids();
    let mut pushed = 0;

    for agent_id in &agent_ids {
        let envelope = nm_common::protocol::WsEnvelope::new(
            nm_common::protocol::WsPayload::UpdateCommand(cmd.clone()),
        );
        if state.agent_registry.send_to_agent(agent_id, envelope).await.is_ok() {
            pushed += 1;
        }
    }

    tracing::info!(
        version = %info.version,
        agents = pushed,
        "Update pushed to all online agents"
    );

    Ok(axum::Json(serde_json::json!({
        "pushed": pushed,
        "total_online": agent_ids.len(),
        "version": info.version,
    })))
}
