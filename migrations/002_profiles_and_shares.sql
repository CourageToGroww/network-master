-- ─── Trace Profiles (Named Configurations) ──────────────

CREATE TABLE trace_profiles (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            VARCHAR(255) NOT NULL UNIQUE,
    description     TEXT,
    probe_method    VARCHAR(10) NOT NULL DEFAULT 'icmp',
    probe_port      INTEGER,
    packet_size     INTEGER NOT NULL DEFAULT 64,
    interval_ms     INTEGER NOT NULL DEFAULT 2500,
    max_hops        INTEGER NOT NULL DEFAULT 30,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed default profiles
INSERT INTO trace_profiles (name, description, probe_method, packet_size, interval_ms, max_hops) VALUES
    ('Default ICMP', 'Standard ICMP trace with 2.5s interval', 'icmp', 64, 2500, 30),
    ('Fast ICMP', 'High-frequency ICMP trace (1s interval)', 'icmp', 64, 1000, 30),
    ('Slow ICMP', 'Low-frequency ICMP for long-term monitoring (5s)', 'icmp', 64, 5000, 30),
    ('TCP HTTP', 'TCP probe on port 80', 'tcp', 64, 2500, 30),
    ('TCP HTTPS', 'TCP probe on port 443', 'tcp', 64, 2500, 30),
    ('UDP Traceroute', 'Classic UDP traceroute', 'udp', 64, 2500, 30);

UPDATE trace_profiles SET probe_port = 80 WHERE name = 'TCP HTTP';
UPDATE trace_profiles SET probe_port = 443 WHERE name = 'TCP HTTPS';

-- ─── Share Tokens ────────────────────────────────────────

CREATE TABLE share_tokens (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token           VARCHAR(64) NOT NULL UNIQUE,
    target_id       UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    label           VARCHAR(255),
    is_readonly     BOOLEAN NOT NULL DEFAULT TRUE,
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_share_tokens_token ON share_tokens(token);
CREATE INDEX idx_share_tokens_target ON share_tokens(target_id);
