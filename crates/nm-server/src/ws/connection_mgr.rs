use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use nm_common::protocol::WsEnvelope;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AgentRegistry {
    inner: Arc<DashMap<Uuid, ConnectedAgent>>,
}

pub struct ConnectedAgent {
    pub agent_id: Uuid,
    pub name: String,
    pub connected_at: DateTime<Utc>,
    pub tx: mpsc::Sender<WsEnvelope>,
    pub active_targets: HashSet<Uuid>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, agent_id: Uuid, agent: ConnectedAgent) {
        tracing::info!(agent_id = %agent_id, name = %agent.name, "Agent registered");
        self.inner.insert(agent_id, agent);
    }

    pub fn unregister(&self, agent_id: &Uuid) {
        if let Some((_, agent)) = self.inner.remove(agent_id) {
            tracing::info!(agent_id = %agent_id, name = %agent.name, "Agent unregistered");
        }
    }

    pub async fn send_to_agent(&self, agent_id: &Uuid, msg: WsEnvelope) -> anyhow::Result<()> {
        let tx = {
            let agent = self.inner.get(agent_id)
                .ok_or_else(|| anyhow::anyhow!("Agent {} not connected", agent_id))?;
            agent.tx.clone()
        };
        // DashMap guard is dropped before awaiting
        tx.send(msg)
            .await
            .map_err(|_| anyhow::anyhow!("Failed to send to agent {}", agent_id))?;
        Ok(())
    }

    pub fn is_online(&self, agent_id: &Uuid) -> bool {
        self.inner.contains_key(agent_id)
    }

    pub fn online_count(&self) -> usize {
        self.inner.len()
    }

    pub fn online_agent_ids(&self) -> Vec<Uuid> {
        self.inner.iter().map(|entry| *entry.key()).collect()
    }
}
