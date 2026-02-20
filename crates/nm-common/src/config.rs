use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
    pub log_level: String,
    pub database_url: String,
    pub db_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_expiry_hours: u64,
    pub agent_heartbeat_timeout_secs: u64,
    pub stats_aggregation_interval_secs: u64,
    pub static_dir: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8080".to_string(),
            log_level: "info".to_string(),
            database_url: "postgresql://nm_user:nm_secret@localhost:5432/network_master"
                .to_string(),
            db_max_connections: 20,
            jwt_secret: "change-me-in-production".to_string(),
            jwt_expiry_hours: 24,
            agent_heartbeat_timeout_secs: 90,
            stats_aggregation_interval_secs: 300,
            static_dir: "./frontend/dist".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub server_url: String,
    pub agent_id: String,
    pub api_key: String,
    pub reconnect_max_delay_secs: u64,
    pub default_timeout_ms: u64,
    pub max_concurrent_probes: usize,
    pub dns_cache_ttl_secs: u64,
    pub log_level: String,
    pub log_file: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: "ws://localhost:8080/ws/agent".to_string(),
            agent_id: uuid::Uuid::new_v4().to_string(),
            api_key: String::new(),
            reconnect_max_delay_secs: 60,
            default_timeout_ms: 2000,
            max_concurrent_probes: 100,
            dns_cache_ttl_secs: 300,
            log_level: "info".to_string(),
            log_file: None,
        }
    }
}
