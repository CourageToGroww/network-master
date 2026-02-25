-- migrations/004_users_rbac.sql

-- User accounts with role-based access
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'viewer'
        CHECK (role IN ('admin', 'operator', 'viewer')),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_role ON users(role);

-- Seed default admin user (password: admin -- MUST change in production)
-- bcrypt hash of "admin"
INSERT INTO users (email, password_hash, display_name, role)
VALUES ('admin@networkmaster.local', '$2b$12$LJ3m4ys4Fp.FiEOOsM0aGuVvkCkFDr0yl.VRCfyd4VRz8CSxjsLYC', 'Admin', 'admin');
