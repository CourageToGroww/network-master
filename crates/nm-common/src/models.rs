use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ─── Agent ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub hostname: Option<String>,
    pub os_info: Option<String>,
    pub version: Option<String>,
    pub ip_address: Option<String>,
    pub is_online: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgent {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub agent: Agent,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgent {
    pub name: Option<String>,
}

// ─── Target ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Target {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub address: String,
    pub resolved_ip: Option<String>,
    pub display_name: Option<String>,
    pub probe_method: String,
    pub probe_port: Option<i32>,
    pub packet_size: i32,
    pub interval_ms: i32,
    pub max_hops: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTarget {
    pub address: String,
    pub display_name: Option<String>,
    #[serde(default = "default_probe_method")]
    pub probe_method: String,
    pub probe_port: Option<i32>,
    #[serde(default = "default_packet_size")]
    pub packet_size: i32,
    #[serde(default = "default_interval")]
    pub interval_ms: i32,
    #[serde(default = "default_max_hops")]
    pub max_hops: i32,
}

fn default_probe_method() -> String {
    "icmp".to_string()
}
fn default_packet_size() -> i32 {
    64
}
fn default_interval() -> i32 {
    2500
}
fn default_max_hops() -> i32 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTarget {
    pub address: Option<String>,
    pub display_name: Option<String>,
    pub probe_method: Option<String>,
    pub probe_port: Option<i32>,
    pub packet_size: Option<i32>,
    pub interval_ms: Option<i32>,
    pub max_hops: Option<i32>,
    pub is_active: Option<bool>,
}

// ─── Trace Session ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TraceSession {
    pub id: Uuid,
    pub target_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub sample_count: i64,
}

// ─── Hop ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Hop {
    pub id: Uuid,
    pub session_id: Uuid,
    pub hop_number: i16,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub asn: Option<i32>,
    pub as_name: Option<String>,
    pub geo_country: Option<String>,
    pub geo_city: Option<String>,
    pub geo_lat: Option<f64>,
    pub geo_lon: Option<f64>,
    pub whois_data: Option<serde_json::Value>,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

// ─── Sample ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub id: i64,
    pub session_id: Uuid,
    pub hop_id: Uuid,
    pub round_number: i64,
    pub sent_at: DateTime<Utc>,
    pub rtt_us: Option<i32>,
    pub is_lost: bool,
    pub jitter_us: Option<i32>,
    pub probe_method: String,
    pub packet_size: i32,
    pub ttl_sent: i16,
    pub ttl_received: Option<i16>,
}

// ─── Time Series (API response) ──────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesResponse {
    pub target_id: Uuid,
    pub session_id: Uuid,
    pub resolution: String,
    pub hops: Vec<HopTimeSeries>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HopTimeSeries {
    pub hop_number: i16,
    pub hop_id: Uuid,
    pub ip_address: Option<String>,
    pub hostname: Option<String>,
    pub datapoints: Vec<TimeSeriesDatapoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TimeSeriesDatapoint {
    pub timestamp: DateTime<Utc>,
    pub rtt_avg_us: Option<i32>,
    pub rtt_min_us: Option<i32>,
    pub rtt_max_us: Option<i32>,
    pub loss_pct: f64,
    pub jitter_avg_us: Option<i32>,
    pub sample_count: i64,
}

// ─── Alert Rule ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertRule {
    pub id: Uuid,
    pub name: String,
    pub target_id: Option<Uuid>,
    pub hop_number: Option<i16>,
    pub metric: String,
    pub comparator: String,
    pub threshold: f64,
    pub window_seconds: i32,
    pub cooldown_seconds: i32,
    pub notify_email: Option<String>,
    pub notify_webhook: Option<String>,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAlertRule {
    pub name: String,
    pub target_id: Option<Uuid>,
    pub hop_number: Option<i16>,
    pub metric: String,
    pub comparator: String,
    pub threshold: f64,
    #[serde(default = "default_window")]
    pub window_seconds: i32,
    #[serde(default = "default_cooldown")]
    pub cooldown_seconds: i32,
    pub notify_email: Option<String>,
    pub notify_webhook: Option<String>,
}

fn default_window() -> i32 {
    60
}
fn default_cooldown() -> i32 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertEvent {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub session_id: Option<Uuid>,
    pub hop_id: Option<Uuid>,
    pub triggered_at: DateTime<Utc>,
    pub metric_value: f64,
    pub threshold_value: f64,
    pub message: String,
    pub notified: bool,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ─── Route ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSnapshot {
    pub id: Uuid,
    pub session_id: Uuid,
    pub captured_at: DateTime<Utc>,
    pub hop_count: i16,
    pub hop_sequence: Vec<Option<String>>,
    pub route_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteChange {
    pub id: Uuid,
    pub session_id: Uuid,
    pub detected_at: DateTime<Utc>,
    pub previous_snapshot_id: Option<Uuid>,
    pub new_snapshot_id: Uuid,
    pub hops_changed: i16,
}

// ─── Trace Profile (Named Configuration) ─────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TraceProfile {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub probe_method: String,
    pub probe_port: Option<i32>,
    pub packet_size: i32,
    pub interval_ms: i32,
    pub max_hops: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTraceProfile {
    pub name: String,
    pub description: Option<String>,
    #[serde(default = "default_probe_method")]
    pub probe_method: String,
    pub probe_port: Option<i32>,
    #[serde(default = "default_packet_size")]
    pub packet_size: i32,
    #[serde(default = "default_interval")]
    pub interval_ms: i32,
    #[serde(default = "default_max_hops")]
    pub max_hops: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTraceProfile {
    pub name: Option<String>,
    pub description: Option<String>,
    pub probe_method: Option<String>,
    pub probe_port: Option<i32>,
    pub packet_size: Option<i32>,
    pub interval_ms: Option<i32>,
    pub max_hops: Option<i32>,
}

// ─── Share Token ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ShareToken {
    pub id: Uuid,
    pub token: String,
    pub target_id: Uuid,
    pub label: Option<String>,
    pub is_readonly: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateShareToken {
    pub target_id: Uuid,
    pub label: Option<String>,
    #[serde(default = "default_readonly")]
    pub is_readonly: bool,
    /// Duration in hours before expiry. None = never expires.
    pub expires_in_hours: Option<i64>,
}

fn default_readonly() -> bool {
    true
}

// ─── Dashboard ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DashboardSummary {
    pub total_agents: i64,
    pub online_agents: i64,
    pub total_targets: i64,
    pub active_targets: i64,
    pub active_alerts: i64,
    pub total_samples_24h: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityScore {
    pub target_id: Uuid,
    pub target_address: String,
    pub agent_name: String,
    pub score: f64,
    pub avg_latency_ms: f64,
    pub avg_jitter_ms: f64,
    pub loss_pct: f64,
}
