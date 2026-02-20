# ── Stage 1: Build Rust server ──────────────────────────
FROM rust:1.83-bookworm AS rust-builder

WORKDIR /build

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/nm-common/Cargo.toml crates/nm-common/Cargo.toml
COPY crates/nm-server/Cargo.toml crates/nm-server/Cargo.toml
COPY crates/nm-agent/Cargo.toml crates/nm-agent/Cargo.toml
COPY crates/nm-cli/Cargo.toml crates/nm-cli/Cargo.toml

# Create stub lib/main files so cargo can resolve the workspace
RUN mkdir -p crates/nm-common/src && echo "pub mod config; pub mod protocol; pub mod quality;" > crates/nm-common/src/lib.rs \
    && mkdir -p crates/nm-common/src && touch crates/nm-common/src/config.rs crates/nm-common/src/protocol.rs crates/nm-common/src/quality.rs \
    && mkdir -p crates/nm-server/src && echo "fn main() {}" > crates/nm-server/src/main.rs \
    && mkdir -p crates/nm-agent/src && echo "fn main() {}" > crates/nm-agent/src/main.rs \
    && mkdir -p crates/nm-cli/src && echo "fn main() {}" > crates/nm-cli/src/main.rs \
    && mkdir -p migrations && touch migrations/.keep

# Pre-build dependencies (cached layer)
RUN cargo build --release --package nm-server 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/
COPY migrations/ migrations/

# Rebuild with real source (dependencies already cached)
RUN touch crates/nm-common/src/lib.rs crates/nm-server/src/main.rs \
    && cargo build --release --package nm-server

# ── Stage 2: Build Frontend ─────────────────────────────
FROM node:22-slim AS frontend-builder

WORKDIR /build/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

# ── Stage 3: Runtime ────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy server binary
COPY --from=rust-builder /build/target/release/nm-server .

# Copy frontend static build
COPY --from=frontend-builder /build/frontend/dist ./static/

# Create data directory for updates
RUN mkdir -p data/updates

# Environment defaults
ENV NM_LISTEN_ADDR=0.0.0.0:8080
ENV NM_LOG_LEVEL=info
ENV NM_STATIC_DIR=/app/static
ENV DATABASE_URL=postgresql://nm_user:nm_secret@postgres:5432/network_master
ENV NM_JWT_SECRET=change-me-in-production

EXPOSE 8080

CMD ["./nm-server"]
