#!/bin/bash
# infra/scripts/backend-userdata.sh
set -euo pipefail

# Install Docker
dnf update -y
dnf install -y docker
systemctl enable docker
systemctl start docker

# Install Docker Compose
DOCKER_COMPOSE_VERSION="2.24.0"
curl -L "https://github.com/docker/compose/releases/download/v$${DOCKER_COMPOSE_VERSION}/docker-compose-linux-$(uname -m)" \
  -o /usr/local/bin/docker-compose
chmod +x /usr/local/bin/docker-compose

# Create app directory
mkdir -p /opt/network-master
cd /opt/network-master

# Write docker-compose.yml for backend
cat > docker-compose.yml << 'COMPOSE'
services:
  postgres:
    image: postgres:16
    restart: unless-stopped
    environment:
      POSTGRES_DB: network_master
      POSTGRES_USER: nm_user
      POSTGRES_PASSWORD: $${DB_PASSWORD}
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U nm_user -d network_master"]
      interval: 5s
      timeout: 5s
      retries: 5

  server:
    image: network-master-server:latest
    build:
      context: .
      dockerfile: Dockerfile.backend
    restart: unless-stopped
    ports:
      - "8080:8080"
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      DATABASE_URL: postgresql://nm_user:$${DB_PASSWORD}@postgres:5432/network_master
      NM_LISTEN_ADDR: 0.0.0.0:8080
      NM_LOG_LEVEL: info
      NM_JWT_SECRET: $${JWT_SECRET}

volumes:
  pgdata:
COMPOSE

# Write .env
cat > .env << ENV
DB_PASSWORD=${db_password}
JWT_SECRET=${jwt_secret}
ENV

echo "Backend setup complete"
