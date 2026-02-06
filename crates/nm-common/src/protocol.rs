use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Envelope ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEnvelope {
    pub msg_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub payload: WsPayload,
}

impl WsEnvelope {
    pub fn new(payload: WsPayload) -> Self {
        Self {
            msg_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsPayload {
    // Agent -> Server
    AuthRequest(AuthRequest),
    Heartbeat(AgentHeartbeat),
    TraceRound(TraceRoundReport),
    RouteDiscovery(RouteDiscoveryReport),
    HopMetadata(HopMetadataUpdate),
    AgentStatus(AgentStatusReport),
    AckResponse(AckResponse),

    // Server -> Agent
    AuthResponse(AuthResponse),
    TargetAssignment(TargetAssignment),
    TargetRemoval(TargetRemoval),
    ConfigUpdate(AgentConfigUpdate),
    ServerHeartbeat(ServerHeartbeat),

    // Server -> Frontend
    LiveTraceUpdate(LiveTraceUpdate),
    AlertFired(AlertFiredNotification),
    AgentOnlineStatus(AgentOnlineStatusChange),
    RouteChangeNotification(RouteChangeNotification),
}

// ─── Authentication ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub agent_id: Uuid,
    pub api_key: String,
    pub agent_version: String,
    pub hostname: String,
    pub os_info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub error: Option<String>,
    pub session_token: Option<String>,
    pub assigned_targets: Vec<TargetConfig>,
}

// ─── Target Management ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    pub target_id: Uuid,
    pub session_id: Uuid,
    pub address: String,
    pub probe_method: ProbeMethod,
    pub probe_port: Option<u16>,
    pub packet_size: u16,
    pub interval_ms: u32,
    pub max_hops: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeMethod {
    Icmp,
    Tcp,
    Udp,
}

impl std::fmt::Display for ProbeMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProbeMethod::Icmp => write!(f, "icmp"),
            ProbeMethod::Tcp => write!(f, "tcp"),
            ProbeMethod::Udp => write!(f, "udp"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetAssignment {
    pub targets: Vec<TargetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetRemoval {
    pub target_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfigUpdate {
    pub target_id: Uuid,
    pub interval_ms: Option<u32>,
    pub packet_size: Option<u16>,
    pub probe_method: Option<ProbeMethod>,
    pub max_hops: Option<u8>,
}

// ─── Trace Data (Hot Path) ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRoundReport {
    pub target_id: Uuid,
    pub session_id: Uuid,
    pub round_number: u64,
    pub sent_at: DateTime<Utc>,
    pub hops: Vec<HopSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopSample {
    pub hop_number: u8,
    pub ip_address: Option<String>,
    pub rtt_us: Option<u32>,
    pub is_lost: bool,
    pub ttl_received: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDiscoveryReport {
    pub target_id: Uuid,
    pub session_id: Uuid,
    pub discovered_at: DateTime<Utc>,
    pub hops: Vec<DiscoveredHop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredHop {
    pub hop_number: u8,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopMetadataUpdate {
    pub session_id: Uuid,
    pub hop_number: u8,
    pub ip_address: String,
    pub hostname: Option<String>,
    pub asn: Option<u32>,
    pub as_name: Option<String>,
    pub geo_country: Option<String>,
    pub geo_city: Option<String>,
    pub geo_lat: Option<f64>,
    pub geo_lon: Option<f64>,
}

// ─── Heartbeat & Status ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeat {
    pub agent_id: Uuid,
    pub active_target_count: u32,
    pub uptime_seconds: u64,
    pub cpu_usage_pct: f32,
    pub memory_usage_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHeartbeat {
    pub server_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusReport {
    pub agent_id: Uuid,
    pub status: AgentStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Starting,
    Running,
    Degraded,
    Stopping,
}

// ─── ACK ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckResponse {
    pub ack_msg_id: Uuid,
    pub success: bool,
    pub error: Option<String>,
}

// ─── Live Feed (Server -> Frontend) ──────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveTraceUpdate {
    pub agent_id: Uuid,
    pub target_id: Uuid,
    pub session_id: Uuid,
    pub round_number: u64,
    pub sent_at: DateTime<Utc>,
    pub hops: Vec<LiveHopData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveHopData {
    pub hop_number: u8,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub rtt_us: Option<u32>,
    pub is_lost: bool,
    pub jitter_us: Option<u32>,
    pub stats: HopRunningStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopRunningStats {
    pub min_rtt_us: u32,
    pub avg_rtt_us: u32,
    pub max_rtt_us: u32,
    pub loss_pct: f64,
    pub jitter_avg_us: u32,
    pub sample_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertFiredNotification {
    pub alert_event_id: Uuid,
    pub rule_name: String,
    pub target_address: String,
    pub hop_number: Option<u8>,
    pub metric: String,
    pub value: f64,
    pub threshold: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOnlineStatusChange {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteChangeNotification {
    pub target_id: Uuid,
    pub session_id: Uuid,
    pub detected_at: DateTime<Utc>,
    pub hops_changed: u8,
    pub old_hop_count: u8,
    pub new_hop_count: u8,
}

// ─── Frontend WS Commands ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FrontendCommand {
    Subscribe { target_ids: Vec<Uuid> },
    Unsubscribe { target_ids: Vec<Uuid> },
}
