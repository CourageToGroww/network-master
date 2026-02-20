use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use nm_common::config::ServerConfig;
use nm_common::protocol::{
    AgentOnlineStatusChange, AlertFiredNotification, LiveProcessTrafficUpdate, LiveTraceUpdate,
    UpdateProgressReport,
};
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::ws::connection_mgr::AgentRegistry;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub live_tx: broadcast::Sender<LiveTraceUpdate>,
    pub alert_tx: broadcast::Sender<AlertFiredNotification>,
    pub update_tx: broadcast::Sender<UpdateProgressReport>,
    pub traffic_tx: broadcast::Sender<LiveProcessTrafficUpdate>,
    pub agent_status_tx: broadcast::Sender<AgentOnlineStatusChange>,
    pub agent_registry: AgentRegistry,
    pub config: Arc<ServerConfig>,
    /// In-memory running stats per hop: key = (session_id, hop_number)
    pub hop_stats: Arc<DashMap<(Uuid, u8), RunningHopStats>>,
    /// Last known route per session: key = session_id, value = vec of hop IPs
    pub route_cache: Arc<DashMap<Uuid, Vec<Option<String>>>>,
    /// Directory for storing update binaries
    pub update_dir: PathBuf,
}

/// In-memory running statistics for a single hop within a session.
pub struct RunningHopStats {
    pub min_rtt_us: u32,
    pub max_rtt_us: u32,
    pub sum_rtt_us: u64,
    pub rtt_count: u64,
    pub loss_count: u64,
    pub total_count: u64,
    pub last_rtt_us: Option<u32>,
    pub sum_jitter_us: u64,
    pub jitter_count: u64,
}

impl RunningHopStats {
    pub fn new() -> Self {
        Self {
            min_rtt_us: u32::MAX,
            max_rtt_us: 0,
            sum_rtt_us: 0,
            rtt_count: 0,
            loss_count: 0,
            total_count: 0,
            last_rtt_us: None,
            sum_jitter_us: 0,
            jitter_count: 0,
        }
    }

    pub fn update(&mut self, rtt_us: Option<u32>, is_lost: bool) {
        self.total_count += 1;
        if is_lost {
            self.loss_count += 1;
        }
        if let Some(rtt) = rtt_us {
            self.min_rtt_us = self.min_rtt_us.min(rtt);
            self.max_rtt_us = self.max_rtt_us.max(rtt);
            self.sum_rtt_us += rtt as u64;
            self.rtt_count += 1;

            // Compute jitter as |current - previous|
            if let Some(prev) = self.last_rtt_us {
                let jitter = (rtt as i64 - prev as i64).unsigned_abs() as u64;
                self.sum_jitter_us += jitter;
                self.jitter_count += 1;
            }
            self.last_rtt_us = Some(rtt);
        }
    }

    pub fn avg_rtt_us(&self) -> u32 {
        if self.rtt_count > 0 {
            (self.sum_rtt_us / self.rtt_count) as u32
        } else {
            0
        }
    }

    pub fn loss_pct(&self) -> f64 {
        if self.total_count > 0 {
            (self.loss_count as f64 / self.total_count as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn avg_jitter_us(&self) -> u32 {
        if self.jitter_count > 0 {
            (self.sum_jitter_us / self.jitter_count) as u32
        } else {
            0
        }
    }
}
