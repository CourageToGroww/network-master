# infra/backend/Dockerfile
FROM rust:1.83-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build release
RUN cargo build --release -p nm-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/nm-server /usr/local/bin/nm-server

RUN mkdir -p /app/data/updates

EXPOSE 8080

ENV NM_LISTEN_ADDR=0.0.0.0:8080
ENV NM_LOG_LEVEL=info

CMD ["nm-server"]
