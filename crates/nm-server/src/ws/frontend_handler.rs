use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use nm_common::protocol::FrontendCommand;
use serde::Serialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::state::AppState;

/// Typed envelope for messages sent to the frontend.
/// This lets the frontend distinguish between message types.
#[derive(Serialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum FrontendMessage {
    LiveTrace(nm_common::protocol::LiveTraceUpdate),
    AlertFired(nm_common::protocol::AlertFiredNotification),
    AgentStatus(nm_common::protocol::AgentOnlineStatusChange),
    UpdateStatus(nm_common::protocol::UpdateProgressReport),
    ProcessTraffic(nm_common::protocol::LiveProcessTrafficUpdate),
}

pub async fn handle(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_frontend_socket(socket, state))
}

async fn handle_frontend_socket(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let mut live_rx = state.live_tx.subscribe();
    let mut alert_rx = state.alert_tx.subscribe();
    let mut update_rx = state.update_tx.subscribe();
    let mut traffic_rx = state.traffic_tx.subscribe();
    let mut agent_status_rx = state.agent_status_tx.subscribe();

    let subscriptions: Arc<RwLock<HashSet<Uuid>>> = Arc::new(RwLock::new(HashSet::new()));
    let traffic_subs: Arc<RwLock<HashSet<Uuid>>> = Arc::new(RwLock::new(HashSet::new()));
    let subs_clone = subscriptions.clone();
    let traffic_subs_clone = traffic_subs.clone();

    tracing::info!("Frontend WebSocket client connected");

    // Writer task: forward matching broadcasts to this frontend
    let writer = tokio::spawn(async move {
        loop {
            tokio::select! {
                result = live_rx.recv() => {
                    match result {
                        Ok(update) => {
                            let subs = subs_clone.read().await;
                            // If no subscriptions, send everything; otherwise filter
                            if subs.is_empty() || subs.contains(&update.target_id) {
                                let msg = FrontendMessage::LiveTrace(update);
                                let json = serde_json::to_string(&msg).unwrap_or_default();
                                if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Frontend WS lagged by {} messages", n);
                        }
                        Err(_) => break,
                    }
                }
                result = alert_rx.recv() => {
                    match result {
                        Ok(alert) => {
                            let msg = FrontendMessage::AlertFired(alert);
                            let json = serde_json::to_string(&msg).unwrap_or_default();
                            if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Frontend alert WS lagged by {} messages", n);
                        }
                        Err(_) => break,
                    }
                }
                result = update_rx.recv() => {
                    match result {
                        Ok(progress) => {
                            let msg = FrontendMessage::UpdateStatus(progress);
                            let json = serde_json::to_string(&msg).unwrap_or_default();
                            if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Frontend update WS lagged by {} messages", n);
                        }
                        Err(_) => break,
                    }
                }
                result = traffic_rx.recv() => {
                    match result {
                        Ok(update) => {
                            let tsubs = traffic_subs_clone.read().await;
                            if tsubs.is_empty() || tsubs.contains(&update.agent_id) {
                                let msg = FrontendMessage::ProcessTraffic(update);
                                let json = serde_json::to_string(&msg).unwrap_or_default();
                                if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                    break;
                                }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Frontend traffic WS lagged by {} messages", n);
                        }
                        Err(_) => break,
                    }
                }
                result = agent_status_rx.recv() => {
                    match result {
                        Ok(status) => {
                            let msg = FrontendMessage::AgentStatus(status);
                            let json = serde_json::to_string(&msg).unwrap_or_default();
                            if ws_tx.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Frontend agent status WS lagged by {} messages", n);
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });

    // Reader loop: handle subscription commands
    while let Some(Ok(msg)) = ws_rx.next().await {
        if let Message::Text(text) = msg {
            if let Ok(cmd) = serde_json::from_str::<FrontendCommand>(&text) {
                match cmd {
                    FrontendCommand::Subscribe { target_ids } => {
                        let mut subs = subscriptions.write().await;
                        tracing::info!(targets = ?target_ids, "Frontend subscribed to targets");
                        subs.extend(target_ids);
                    }
                    FrontendCommand::Unsubscribe { target_ids } => {
                        let mut subs = subscriptions.write().await;
                        for id in target_ids {
                            subs.remove(&id);
                        }
                    }
                    FrontendCommand::SubscribeTraffic { agent_ids } => {
                        let mut tsubs = traffic_subs.write().await;
                        tracing::info!(agents = ?agent_ids, "Frontend subscribed to agent traffic");
                        tsubs.extend(agent_ids);
                    }
                    FrontendCommand::UnsubscribeTraffic { agent_ids } => {
                        let mut tsubs = traffic_subs.write().await;
                        for id in agent_ids {
                            tsubs.remove(&id);
                        }
                    }
                }
            }
        }
    }

    tracing::info!("Frontend WebSocket client disconnected");
    writer.abort();
}
