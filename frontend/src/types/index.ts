// ─── Agent ─────────────────────────────────────────────
export interface Agent {
  id: string;
  name: string;
  hostname: string | null;
  os_info: string | null;
  version: string | null;
  ip_address: string | null;
  is_online: boolean;
  last_seen_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface AgentRegistration {
  agent: Agent;
  api_key: string;
}

// ─── Target ────────────────────────────────────────────
export interface Target {
  id: string;
  agent_id: string;
  address: string;
  resolved_ip: string | null;
  display_name: string | null;
  probe_method: 'icmp' | 'tcp' | 'udp';
  probe_port: number | null;
  packet_size: number;
  interval_ms: number;
  max_hops: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateTarget {
  address: string;
  display_name?: string;
  probe_method?: string;
  probe_port?: number;
  packet_size?: number;
  interval_ms?: number;
  max_hops?: number;
}

// ─── Trace Session ─────────────────────────────────────
export interface TraceSession {
  id: string;
  target_id: string;
  started_at: string;
  ended_at: string | null;
  sample_count: number;
}

// ─── Hop ───────────────────────────────────────────────
export interface Hop {
  id: string;
  session_id: string;
  hop_number: number;
  ip_address: string | null;
  hostname: string | null;
  asn: number | null;
  as_name: string | null;
  geo_country: string | null;
  geo_city: string | null;
  whois_data: Record<string, unknown> | null;
}

// ─── Time Series ───────────────────────────────────────
export interface TimeSeriesDatapoint {
  timestamp: string;
  rtt_avg_us: number | null;
  rtt_min_us: number | null;
  rtt_max_us: number | null;
  loss_pct: number;
  jitter_avg_us: number | null;
  sample_count: number;
}

// ─── Alert ─────────────────────────────────────────────
export interface AlertRule {
  id: string;
  name: string;
  target_id: string | null;
  hop_number: number | null;
  metric: string;
  comparator: string;
  threshold: number;
  window_seconds: number;
  cooldown_seconds: number;
  notify_email: string | null;
  notify_webhook: string | null;
  is_enabled: boolean;
}

export interface AlertEvent {
  id: string;
  rule_id: string;
  triggered_at: string;
  metric_value: number;
  threshold_value: number;
  message: string;
  notified: boolean;
  resolved_at: string | null;
}

// ─── Trace Profile (Named Configuration) ──────────────
export interface TraceProfile {
  id: string;
  name: string;
  description: string | null;
  probe_method: string;
  probe_port: number | null;
  packet_size: number;
  interval_ms: number;
  max_hops: number;
  created_at: string;
  updated_at: string;
}

// ─── Share Token ──────────────────────────────────────
export interface ShareToken {
  id: string;
  token: string;
  target_id: string;
  label: string | null;
  is_readonly: boolean;
  expires_at: string | null;
  created_at: string;
}

// ─── Dashboard ─────────────────────────────────────────
export interface DashboardSummary {
  total_agents: number;
  online_agents: number;
  total_targets: number;
  active_targets: number;
  active_alerts: number;
  total_samples_24h: number;
}

// ─── Live Data (WebSocket) ─────────────────────────────
export interface LiveTraceUpdate {
  agent_id: string;
  target_id: string;
  session_id: string;
  round_number: number;
  sent_at: string;
  hops: LiveHopData[];
}

export interface LiveHopData {
  hop_number: number;
  ip_address: string | null;
  hostname: string | null;
  rtt_us: number | null;
  is_lost: boolean;
  jitter_us: number | null;
  stats: HopRunningStats;
}

export interface HopRunningStats {
  min_rtt_us: number;
  avg_rtt_us: number;
  max_rtt_us: number;
  loss_pct: number;
  jitter_avg_us: number;
  sample_count: number;
}

// ─── Real-time Hop Data (Store) ────────────────────────
export interface HopRealtimeData {
  hopNumber: number;
  ip: string | null;
  hostname: string | null;
  lossPct: number;
  sent: number;
  recv: number;
  bestMs: number;
  avgMs: number;
  worstMs: number;
  lastMs: number;
  jitterMs: number;
  mos: number;
}
