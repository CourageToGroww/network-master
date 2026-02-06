use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use nm_common::config::AgentConfig;
use nm_common::protocol::*;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub async fn run(
    config: AgentConfig,
    mut outgoing_rx: mpsc::Receiver<WsEnvelope>,
    target_tx: mpsc::Sender<TargetConfig>,
) {
    let mut reconnect_delay = Duration::from_secs(1);
    let max_delay = Duration::from_secs(config.reconnect_max_delay_secs);

    loop {
        tracing::info!(url = %config.server_url, "Connecting to server...");

        match connect_async(&config.server_url).await {
            Ok((ws_stream, _)) => {
                reconnect_delay = Duration::from_secs(1);
                tracing::info!("WebSocket connected");

                let (mut ws_tx, mut ws_rx) = ws_stream.split();

                // Send AuthRequest
                let auth = WsEnvelope::new(WsPayload::AuthRequest(AuthRequest {
                    agent_id: config.agent_id.parse().unwrap_or_default(),
                    api_key: config.api_key.clone(),
                    agent_version: env!("CARGO_PKG_VERSION").to_string(),
                    hostname: hostname::get()
                        .map(|h| h.to_string_lossy().to_string())
                        .unwrap_or_else(|_| "unknown".to_string()),
                    os_info: std::env::consts::OS.to_string(),
                }));

                let auth_bytes = rmp_serde::to_vec(&auth).unwrap();
                if ws_tx.send(Message::Binary(auth_bytes.into())).await.is_err() {
                    tracing::error!("Failed to send auth request");
                    continue;
                }

                // Wait for AuthResponse
                if let Some(Ok(msg)) = ws_rx.next().await {
                    match msg {
                        Message::Binary(data) => {
                            if let Ok(envelope) = rmp_serde::from_slice::<WsEnvelope>(&data) {
                                if let WsPayload::AuthResponse(resp) = envelope.payload {
                                    if resp.success {
                                        tracing::info!("Authentication successful");
                                        // Send assigned targets to scheduler
                                        for target in resp.assigned_targets {
                                            let _ = target_tx.send(target).await;
                                        }
                                    } else {
                                        tracing::error!("Auth failed: {:?}", resp.error);
                                        tokio::time::sleep(Duration::from_secs(30)).await;
                                        continue;
                                    }
                                }
                            }
                        }
                        _ => {
                            tracing::warn!("Unexpected auth response format");
                            continue;
                        }
                    }
                }

                // Main communication loop
                let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));

                loop {
                    tokio::select! {
                        // Send outgoing messages from probe scheduler
                        Some(msg) = outgoing_rx.recv() => {
                            let bytes = rmp_serde::to_vec(&msg).unwrap();
                            if ws_tx.send(Message::Binary(bytes.into())).await.is_err() {
                                tracing::error!("Failed to send message, reconnecting...");
                                break;
                            }
                        }

                        // Receive messages from server
                        Some(Ok(msg)) = ws_rx.next() => {
                            match msg {
                                Message::Binary(data) => {
                                    if let Ok(envelope) = rmp_serde::from_slice::<WsEnvelope>(&data) {
                                        handle_server_message(envelope, &target_tx).await;
                                    }
                                }
                                Message::Close(_) => {
                                    tracing::info!("Server closed connection");
                                    break;
                                }
                                _ => {}
                            }
                        }

                        // Send periodic heartbeat
                        _ = heartbeat_interval.tick() => {
                            let hb = WsEnvelope::new(WsPayload::Heartbeat(AgentHeartbeat {
                                agent_id: config.agent_id.parse().unwrap_or_default(),
                                active_target_count: 0,
                                uptime_seconds: 0,
                                cpu_usage_pct: 0.0,
                                memory_usage_mb: 0,
                            }));
                            let bytes = rmp_serde::to_vec(&hb).unwrap();
                            if ws_tx.send(Message::Binary(bytes.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Connection failed");
            }
        }

        tracing::info!(delay = ?reconnect_delay, "Reconnecting in...");
        tokio::time::sleep(reconnect_delay).await;
        reconnect_delay = (reconnect_delay * 2).min(max_delay);
    }
}

async fn handle_server_message(envelope: WsEnvelope, target_tx: &mpsc::Sender<TargetConfig>) {
    match envelope.payload {
        WsPayload::TargetAssignment(assignment) => {
            for target in assignment.targets {
                let _ = target_tx.send(target).await;
            }
        }
        WsPayload::ServerHeartbeat(_) => {
            // Server is alive
        }
        WsPayload::ConfigUpdate(update) => {
            tracing::info!(target_id = %update.target_id, "Received config update");
            // TODO: Update running probe configuration
        }
        _ => {
            tracing::debug!("Unhandled server message");
        }
    }
}
