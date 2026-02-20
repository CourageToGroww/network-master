-- Per-process network traffic snapshots
CREATE TABLE process_traffic_snapshots (
    id              BIGSERIAL PRIMARY KEY,
    agent_id        UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    captured_at     TIMESTAMPTZ NOT NULL,
    interval_ms     INTEGER NOT NULL
);
CREATE INDEX idx_traffic_snapshots_agent ON process_traffic_snapshots(agent_id, captured_at);

-- Per-process entries within a snapshot
CREATE TABLE process_traffic_entries (
    id                  BIGSERIAL PRIMARY KEY,
    snapshot_id         BIGINT NOT NULL REFERENCES process_traffic_snapshots(id) ON DELETE CASCADE,
    pid                 INTEGER NOT NULL,
    process_name        VARCHAR(255) NOT NULL,
    exe_path            VARCHAR(1024),
    bytes_in            BIGINT NOT NULL DEFAULT 0,
    bytes_out           BIGINT NOT NULL DEFAULT 0,
    active_connections  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_traffic_entries_snapshot ON process_traffic_entries(snapshot_id);
