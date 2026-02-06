use std::collections::HashSet;

use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use nm_common::protocol::{ProbeMethod, TargetConfig, WsEnvelope, WsPayload};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::state::AppState;
use crate::ws::connection_mgr::ConnectedAgent;

pub async fn handle(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_socket(socket, state))
}

async fn handle_agent_socket(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<WsEnvelope>(256);

    // Wait for AuthRequest (timeout 10s)
    let auth_result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        ws_rx.next(),
    )
    .await;

    let (agent_id, agent_name) = match auth_result {
        Ok(Some(Ok(Message::Binary(data)))) => {
            match rmp_serde::from_slice::<WsEnvelope>(&data) {
                Ok(envelope) => {
                    if let WsPayload::AuthRequest(ref auth) = envelope.payload {
                        match validate_and_respond(&state, auth, &mut ws_tx).await {
                            Some((id, name)) => (id, name),
                            None => return,
                        }
                    } else {
                        tracing::warn!("Expected AuthRequest, got {:?}", envelope.payload);
                        return;
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to deserialize auth message: {}", e);
                    return;
                }
            }
        }
        Ok(Some(Ok(Message::Text(text)))) => {
            match serde_json::from_str::<WsEnvelope>(&text) {
                Ok(envelope) => {
                    if let WsPayload::AuthRequest(ref auth) = envelope.payload {
                        match validate_and_respond(&state, auth, &mut ws_tx).await {
                            Some((id, name)) => (id, name),
                            None => return,
                        }
                    } else {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
        _ => {
            tracing::warn!("Auth timeout or connection closed");
            return;
        }
    };

    // Register agent
    state.agent_registry.register(
        agent_id,
        ConnectedAgent {
            agent_id,
            name: agent_name.clone(),
            connected_at: Utc::now(),
            tx: cmd_tx,
            active_targets: HashSet::new(),
        },
    );

    // Update DB: set agent online + update metadata
    let _ = sqlx::query(
        "UPDATE agents SET is_online = true, last_seen_at = NOW() WHERE id = $1",
    )
    .bind(agent_id)
    .execute(&state.pool)
    .await;

    tracing::info!(agent_id = %agent_id, name = %agent_name, "Agent connected and authenticated");

    // Spawn writer task (server -> agent)
    let writer = tokio::spawn(async move {
        while let Some(msg) = cmd_rx.recv().await {
            let bytes = match rmp_serde::to_vec(&msg) {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                    continue;
                }
            };
            if ws_tx.send(Message::Binary(bytes.into())).await.is_err() {
                break;
            }
        }
    });

    // Reader loop (agent -> server)
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Binary(data) => {
                match rmp_serde::from_slice::<WsEnvelope>(&data) {
                    Ok(envelope) => {
                        handle_agent_message(agent_id, envelope, &state).await;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize agent message: {}", e);
                    }
                }
            }
            Message::Text(text) => {
                match serde_json::from_str::<WsEnvelope>(&text) {
                    Ok(envelope) => {
                        handle_agent_message(agent_id, envelope, &state).await;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse agent JSON message: {}", e);
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup
    state.agent_registry.unregister(&agent_id);
    let _ = sqlx::query("UPDATE agents SET is_online = false, last_seen_at = NOW() WHERE id = $1")
        .bind(agent_id)
        .execute(&state.pool)
        .await;

    tracing::info!(agent_id = %agent_id, "Agent disconnected");
    writer.abort();
}

/// Validate the agent's API key against the DB and send an AuthResponse.
/// Returns (agent_id, agent_name) on success, None on failure.
async fn validate_and_respond(
    state: &AppState,
    auth: &nm_common::protocol::AuthRequest,
    ws_tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Option<(Uuid, String)> {
    let agent_id = auth.agent_id;

    // Look up agent in DB and get api_key_hash
    let row = sqlx::query_as::<_, (String,)>(
        "SELECT api_key_hash FROM agents WHERE id = $1",
    )
    .bind(agent_id)
    .fetch_optional(&state.pool)
    .await;

    let api_key_hash = match row {
        Ok(Some((hash,))) => hash,
        Ok(None) => {
            tracing::warn!(agent_id = %agent_id, "Agent not found in DB");
            send_auth_failure(ws_tx, "Agent not found").await;
            return None;
        }
        Err(e) => {
            tracing::error!("DB error during auth: {}", e);
            send_auth_failure(ws_tx, "Internal server error").await;
            return None;
        }
    };

    // Verify API key against bcrypt hash
    match bcrypt::verify(&auth.api_key, &api_key_hash) {
        Ok(true) => { /* valid */ }
        Ok(false) => {
            tracing::warn!(agent_id = %agent_id, "Invalid API key");
            send_auth_failure(ws_tx, "Invalid API key").await;
            return None;
        }
        Err(e) => {
            tracing::error!("bcrypt verify error: {}", e);
            send_auth_failure(ws_tx, "Authentication error").await;
            return None;
        }
    }

    // Update agent metadata from the auth request
    let _ = sqlx::query(
        r#"UPDATE agents SET
            hostname = $2,
            os_info = $3,
            version = $4,
            last_seen_at = NOW()
        WHERE id = $1"#,
    )
    .bind(agent_id)
    .bind(&auth.hostname)
    .bind(&auth.os_info)
    .bind(&auth.agent_version)
    .execute(&state.pool)
    .await;

    // Load assigned targets for this agent
    let assigned_targets = load_agent_targets(state, agent_id).await;

    tracing::info!(
        agent_id = %agent_id,
        hostname = %auth.hostname,
        targets = assigned_targets.len(),
        "Agent authenticated successfully"
    );

    // Send AuthResponse
    let response = WsEnvelope::new(WsPayload::AuthResponse(
        nm_common::protocol::AuthResponse {
            success: true,
            error: None,
            session_token: None,
            assigned_targets,
        },
    ));
    let response_bytes = rmp_serde::to_vec(&response).unwrap();
    if ws_tx.send(Message::Binary(response_bytes.into())).await.is_err() {
        return None;
    }

    Some((agent_id, auth.hostname.clone()))
}

/// Load active targets for an agent and create trace sessions for each.
async fn load_agent_targets(state: &AppState, agent_id: Uuid) -> Vec<TargetConfig> {
    let targets = match crate::db::targets::list_for_agent(&state.pool, agent_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Failed to load targets for agent {}: {}", agent_id, e);
            return vec![];
        }
    };

    let mut configs = Vec::with_capacity(targets.len());
    for target in targets {
        if !target.is_active {
            continue;
        }

        // Create a new trace session for this target
        let session = match crate::db::sessions::create(&state.pool, target.id).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to create session for target {}: {}", target.id, e);
                continue;
            }
        };

        let probe_method = match target.probe_method.as_str() {
            "tcp" => ProbeMethod::Tcp,
            "udp" => ProbeMethod::Udp,
            _ => ProbeMethod::Icmp,
        };

        configs.push(TargetConfig {
            target_id: target.id,
            session_id: session.id,
            address: target.address,
            probe_method,
            probe_port: target.probe_port.map(|p| p as u16),
            packet_size: target.packet_size as u16,
            interval_ms: target.interval_ms as u32,
            max_hops: target.max_hops as u8,
        });
    }

    configs
}

async fn send_auth_failure(
    ws_tx: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    error: &str,
) {
    let response = WsEnvelope::new(WsPayload::AuthResponse(
        nm_common::protocol::AuthResponse {
            success: false,
            error: Some(error.to_string()),
            session_token: None,
            assigned_targets: vec![],
        },
    ));
    let bytes = rmp_serde::to_vec(&response).unwrap();
    let _ = ws_tx.send(Message::Binary(bytes.into())).await;
}

async fn handle_agent_message(agent_id: Uuid, envelope: WsEnvelope, state: &AppState) {
    match envelope.payload {
        WsPayload::TraceRound(report) => {
            crate::engine::ingestion::ingest_trace_round(report, agent_id, state).await;
        }
        WsPayload::RouteDiscovery(report) => {
            crate::engine::route_detector::check_route_change(report, state).await;
        }
        WsPayload::Heartbeat(hb) => {
            let _ = sqlx::query("UPDATE agents SET last_seen_at = NOW() WHERE id = $1")
                .bind(hb.agent_id)
                .execute(&state.pool)
                .await;
        }
        WsPayload::HopMetadata(meta) => {
            let _ = sqlx::query(
                r#"UPDATE hops SET
                    hostname = COALESCE($1, hostname),
                    asn = COALESCE($2, asn),
                    as_name = COALESCE($3, as_name),
                    geo_country = COALESCE($4, geo_country),
                    geo_city = COALESCE($5, geo_city),
                    geo_lat = COALESCE($6, geo_lat),
                    geo_lon = COALESCE($7, geo_lon),
                    last_seen_at = NOW()
                WHERE session_id = $8 AND hop_number = $9 AND ip_address = $10::inet"#,
            )
            .bind(&meta.hostname)
            .bind(meta.asn.map(|v| v as i32))
            .bind(&meta.as_name)
            .bind(&meta.geo_country)
            .bind(&meta.geo_city)
            .bind(meta.geo_lat)
            .bind(meta.geo_lon)
            .bind(meta.session_id)
            .bind(meta.hop_number as i16)
            .bind(&meta.ip_address)
            .execute(&state.pool)
            .await;
        }
        WsPayload::AgentStatus(status) => {
            tracing::info!(agent_id = %agent_id, status = ?status.status, "Agent status update");
        }
        _ => {
            tracing::debug!("Unhandled agent message type");
        }
    }
}
