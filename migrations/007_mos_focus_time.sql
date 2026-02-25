-- migrations/007_mos_focus_time.sql

-- Add MOS score to hop_stats_hourly
ALTER TABLE hop_stats_hourly ADD COLUMN mos_score REAL;

-- Add MOS to running stats view (computed, not stored in samples)
-- Focus Time is computed at query time, no schema change needed

-- Add quality score threshold settings
CREATE TABLE user_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    setting_key VARCHAR(100) NOT NULL,
    setting_value JSONB NOT NULL,
    UNIQUE (user_id, setting_key)
);

-- Seed default display thresholds
INSERT INTO user_settings (user_id, setting_key, setting_value)
SELECT id, 'display_thresholds', '{"warning_ms": 200, "critical_ms": 500, "loss_warning_pct": 5, "loss_critical_pct": 15}'::jsonb
FROM users WHERE role = 'admin' LIMIT 1;
