-- migrations/005_workspaces_sessions_configs.sql

-- Add state to trace_sessions
ALTER TABLE trace_sessions ADD COLUMN state VARCHAR(20) NOT NULL DEFAULT 'active'
    CHECK (state IN ('active', 'paused', 'archived', 'will_delete'));
CREATE INDEX idx_sessions_state ON trace_sessions(state);

-- Named configurations (extends trace_profiles with per-target assignment)
ALTER TABLE targets ADD COLUMN config_id UUID REFERENCES trace_profiles(id) ON DELETE SET NULL;

-- Workspaces
CREATE TABLE workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    layout_json JSONB NOT NULL DEFAULT '{}',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_workspaces_owner ON workspaces(owner_id);

-- Workspace targets (which targets are in this workspace)
CREATE TABLE workspace_targets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    show_on_timeline BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE (workspace_id, target_id)
);

-- Comments / annotations on timeline
CREATE TABLE timeline_comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_id UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    session_id UUID REFERENCES trace_sessions(id) ON DELETE SET NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    text TEXT NOT NULL,
    auto_generated BOOLEAN NOT NULL DEFAULT FALSE,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_comments_target_time ON timeline_comments(target_id, timestamp);

-- Summary screens
CREATE TABLE summary_screens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    owner_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    focus_time_seconds INTEGER NOT NULL DEFAULT 600,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE summary_screen_targets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    summary_id UUID NOT NULL REFERENCES summary_screens(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    UNIQUE (summary_id, target_id)
);
