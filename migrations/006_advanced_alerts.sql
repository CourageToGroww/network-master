-- migrations/006_advanced_alerts.sql

-- Extend alert_rules with more condition types and action types
ALTER TABLE alert_rules ADD COLUMN condition_type VARCHAR(30) NOT NULL DEFAULT 'threshold'
    CHECK (condition_type IN (
        'latency_over_time', 'loss_over_time', 'latency_over_samples',
        'mos_threshold', 'route_changed', 'ip_in_route', 'timer'
    ));

ALTER TABLE alert_rules ADD COLUMN condition_params JSONB NOT NULL DEFAULT '{}';
ALTER TABLE alert_rules ADD COLUMN actions JSONB NOT NULL DEFAULT '[]';
ALTER TABLE alert_rules ADD COLUMN notify_on_start BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE alert_rules ADD COLUMN notify_on_end BOOLEAN NOT NULL DEFAULT FALSE;

-- LiveShare links
CREATE TABLE liveshare_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token VARCHAR(64) NOT NULL UNIQUE,
    target_id UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    label VARCHAR(200),
    notes TEXT,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_liveshare_token ON liveshare_links(token);

-- Discovered devices (Local Network Discovery)
CREATE TABLE discovered_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    ip_address VARCHAR(45) NOT NULL,
    mac_address VARCHAR(17),
    hostname VARCHAR(255),
    vendor VARCHAR(255),
    latency_us INTEGER,
    description TEXT,
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (agent_id, ip_address)
);

CREATE INDEX idx_devices_agent ON discovered_devices(agent_id);

-- Insights (automated analysis)
CREATE TABLE insights (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_id UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    analysis_period VARCHAR(10) NOT NULL CHECK (analysis_period IN ('24h', '48h', '7d')),
    overall_quality VARCHAR(10) NOT NULL CHECK (overall_quality IN ('good', 'fair', 'poor')),
    good_pct REAL NOT NULL DEFAULT 0,
    fair_pct REAL NOT NULL DEFAULT 0,
    poor_pct REAL NOT NULL DEFAULT 0,
    events JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_insights_target ON insights(target_id, created_at DESC);
