-- ============================================================
-- AGENTS: Each Windows service instance that connects to server
-- ============================================================
CREATE TABLE agents (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(255) NOT NULL,
    hostname        VARCHAR(255),
    os_info         VARCHAR(255),
    version         VARCHAR(50),
    api_key_hash    VARCHAR(512) NOT NULL,
    ip_address      VARCHAR(45),
    is_online       BOOLEAN NOT NULL DEFAULT FALSE,
    last_seen_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_agents_online ON agents(is_online);

-- ============================================================
-- TARGETS: Destinations being monitored (belongs to an agent)
-- ============================================================
CREATE TABLE targets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id        UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    address         VARCHAR(512) NOT NULL,
    resolved_ip     VARCHAR(45),
    display_name    VARCHAR(255),
    probe_method    VARCHAR(10) NOT NULL DEFAULT 'icmp',
    probe_port      INTEGER,
    packet_size     INTEGER NOT NULL DEFAULT 64,
    interval_ms     INTEGER NOT NULL DEFAULT 2500,
    max_hops        INTEGER NOT NULL DEFAULT 30,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_targets_agent ON targets(agent_id);
CREATE INDEX idx_targets_active ON targets(is_active);

-- ============================================================
-- TRACE SESSIONS
-- ============================================================
CREATE TABLE trace_sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_id       UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at        TIMESTAMPTZ,
    sample_count    BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX idx_sessions_target ON trace_sessions(target_id);
CREATE INDEX idx_sessions_started ON trace_sessions(started_at);

-- ============================================================
-- HOPS: Discovered routers along the path
-- ============================================================
CREATE TABLE hops (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id      UUID NOT NULL REFERENCES trace_sessions(id) ON DELETE CASCADE,
    hop_number      SMALLINT NOT NULL,
    ip_address      VARCHAR(45),
    hostname        VARCHAR(512),
    asn             INTEGER,
    as_name         VARCHAR(255),
    geo_country     VARCHAR(3),
    geo_city        VARCHAR(255),
    geo_lat         DOUBLE PRECISION,
    geo_lon         DOUBLE PRECISION,
    whois_data      JSONB,
    first_seen_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_hops_session ON hops(session_id);
CREATE INDEX idx_hops_session_number ON hops(session_id, hop_number);
CREATE UNIQUE INDEX idx_hops_unique ON hops(session_id, hop_number, ip_address);

-- ============================================================
-- SAMPLES: Individual probe results (high-volume table)
-- ============================================================
CREATE TABLE samples (
    id              BIGSERIAL PRIMARY KEY,
    session_id      UUID NOT NULL REFERENCES trace_sessions(id) ON DELETE CASCADE,
    hop_id          UUID NOT NULL REFERENCES hops(id) ON DELETE CASCADE,
    round_number    BIGINT NOT NULL,
    sent_at         TIMESTAMPTZ NOT NULL,
    rtt_us          INTEGER,
    is_lost         BOOLEAN NOT NULL DEFAULT FALSE,
    jitter_us       INTEGER,
    probe_method    VARCHAR(10) NOT NULL DEFAULT 'icmp',
    packet_size     INTEGER NOT NULL DEFAULT 64,
    ttl_sent        SMALLINT NOT NULL,
    ttl_received    SMALLINT
);
CREATE INDEX idx_samples_session_time ON samples(session_id, sent_at);
CREATE INDEX idx_samples_hop_time ON samples(hop_id, sent_at);

-- ============================================================
-- ROUTE SNAPSHOTS
-- ============================================================
CREATE TABLE route_snapshots (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id      UUID NOT NULL REFERENCES trace_sessions(id) ON DELETE CASCADE,
    captured_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    hop_count       SMALLINT NOT NULL,
    hop_sequence    TEXT[] NOT NULL,
    route_hash      VARCHAR(64) NOT NULL
);
CREATE INDEX idx_route_snapshots_session ON route_snapshots(session_id, captured_at);
CREATE INDEX idx_route_snapshots_hash ON route_snapshots(session_id, route_hash);

-- ============================================================
-- ROUTE CHANGES
-- ============================================================
CREATE TABLE route_changes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id      UUID NOT NULL REFERENCES trace_sessions(id) ON DELETE CASCADE,
    detected_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    previous_snapshot_id UUID REFERENCES route_snapshots(id),
    new_snapshot_id UUID NOT NULL REFERENCES route_snapshots(id),
    hops_changed    SMALLINT NOT NULL
);
CREATE INDEX idx_route_changes_session ON route_changes(session_id, detected_at);

-- ============================================================
-- ALERT RULES
-- ============================================================
CREATE TABLE alert_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(255) NOT NULL,
    target_id       UUID REFERENCES targets(id) ON DELETE CASCADE,
    hop_number      SMALLINT,
    metric          VARCHAR(20) NOT NULL,
    comparator      VARCHAR(5) NOT NULL,
    threshold       DOUBLE PRECISION NOT NULL,
    window_seconds  INTEGER NOT NULL DEFAULT 60,
    cooldown_seconds INTEGER NOT NULL DEFAULT 300,
    notify_email    VARCHAR(512),
    notify_webhook  VARCHAR(1024),
    is_enabled      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- ALERT EVENTS
-- ============================================================
CREATE TABLE alert_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rule_id         UUID NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    session_id      UUID REFERENCES trace_sessions(id),
    hop_id          UUID REFERENCES hops(id),
    triggered_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metric_value    DOUBLE PRECISION NOT NULL,
    threshold_value DOUBLE PRECISION NOT NULL,
    message         TEXT NOT NULL,
    notified        BOOLEAN NOT NULL DEFAULT FALSE,
    resolved_at     TIMESTAMPTZ
);
CREATE INDEX idx_alert_events_rule ON alert_events(rule_id, triggered_at);
CREATE INDEX idx_alert_events_active ON alert_events(resolved_at) WHERE resolved_at IS NULL;

-- ============================================================
-- HOP STATS (hourly rollups)
-- ============================================================
CREATE TABLE hop_stats_hourly (
    id              BIGSERIAL PRIMARY KEY,
    hop_id          UUID NOT NULL REFERENCES hops(id) ON DELETE CASCADE,
    session_id      UUID NOT NULL REFERENCES trace_sessions(id) ON DELETE CASCADE,
    hour            TIMESTAMPTZ NOT NULL,
    sample_count    INTEGER NOT NULL,
    loss_count      INTEGER NOT NULL,
    loss_pct        DOUBLE PRECISION NOT NULL,
    rtt_min_us      INTEGER,
    rtt_avg_us      INTEGER,
    rtt_max_us      INTEGER,
    rtt_stddev_us   INTEGER,
    jitter_avg_us   INTEGER,
    jitter_max_us   INTEGER,
    quality_score   DOUBLE PRECISION
);
CREATE UNIQUE INDEX idx_hop_stats_hourly ON hop_stats_hourly(hop_id, hour);
CREATE INDEX idx_hop_stats_session_hour ON hop_stats_hourly(session_id, hour);
