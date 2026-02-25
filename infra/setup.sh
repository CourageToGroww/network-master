#!/bin/bash
set -euo pipefail

# ── Network Master EC2 First-Boot Setup ──
# Runs once via user_data when the instance first starts.
# Terraform injects: db_password, jwt_secret, admin_password, domain_name

LOG_FILE="/var/log/nm-setup.log"
exec > >(tee -a "$LOG_FILE") 2>&1
echo "=== Network Master Setup - $(date) ==="

# ── 1. System packages ──
dnf update -y
dnf install -y docker nginx certbot python3-certbot-nginx

# ── 2. Start Docker ──
systemctl enable docker
systemctl start docker

# ── 3. Create nm system user and directory layout ──
useradd -r -s /bin/false nm || true
mkdir -p /opt/nm/static /opt/nm/data /opt/nm/updates
chown -R nm:nm /opt/nm

# ── 4. Start PostgreSQL container ──
docker volume create nm-pgdata

docker run -d \
  --name nm-postgres \
  --restart always \
  -e POSTGRES_DB=network_master \
  -e POSTGRES_USER=nm_user \
  -e POSTGRES_PASSWORD="${db_password}" \
  -v nm-pgdata:/var/lib/postgresql/data \
  -p 127.0.0.1:5432:5432 \
  postgres:16

echo "Waiting for PostgreSQL to be ready..."
until docker exec nm-postgres pg_isready -U nm_user -d network_master 2>/dev/null; do
  sleep 2
done
echo "PostgreSQL is ready."

# ── 5. Write /opt/nm/.env ──
# Single-quoted ENVEOF: shell won't expand these.
# Terraform already replaced ${db_password} and ${jwt_secret} before this runs.
cat > /opt/nm/.env << 'ENVEOF'
DATABASE_URL=postgresql://nm_user:${db_password}@127.0.0.1:5432/network_master
NM_LISTEN_ADDR=127.0.0.1:8080
NM_LOG_LEVEL=info
NM_JWT_SECRET=${jwt_secret}
NM_STATIC_DIR=/opt/nm/static
ENVEOF

chmod 600 /opt/nm/.env
chown nm:nm /opt/nm/.env

# ── 6. Create systemd service for nm-server ──
cat > /etc/systemd/system/nm-server.service << 'SVCEOF'
[Unit]
Description=Network Master Server
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=nm
Group=nm
WorkingDirectory=/opt/nm
EnvironmentFile=/opt/nm/.env
ExecStart=/opt/nm/nm-server
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/opt/nm/data /opt/nm/updates
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
SVCEOF

systemctl daemon-reload
systemctl enable nm-server
# Service will keep restarting until the binary is deployed — that's intentional.

# ── 7. Configure nginx as WebSocket-aware reverse proxy ──
DOMAIN="${domain_name}"
if [ -z "$DOMAIN" ]; then
  SERVER_NAME="_"
else
  SERVER_NAME="$DOMAIN"
fi

# Unquoted heredoc: shell expands $SERVER_NAME.
# \$var escapes nginx variables from shell expansion.
cat > /etc/nginx/conf.d/nm.conf << NGXEOF
server {
    listen 80;
    server_name $SERVER_NAME;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;

        # WebSocket upgrade — required for agent connections
        proxy_set_header Upgrade \$http_upgrade;
        proxy_set_header Connection "upgrade";

        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;

        # Keep WebSocket connections alive (agents hold long-lived connections)
        proxy_read_timeout 86400;

        # Allow agent binary uploads via the OTA endpoint
        client_max_body_size 60M;
    }
}
NGXEOF

rm -f /etc/nginx/conf.d/default.conf 2>/dev/null || true
systemctl enable nginx
systemctl start nginx

# ── 8. SSL via Let's Encrypt (only if domain_name is provided) ──
if [ -n "$DOMAIN" ]; then
  echo "Setting up SSL for $DOMAIN..."
  certbot --nginx -d "$DOMAIN" --non-interactive --agree-tos \
    --email "admin@$DOMAIN" --redirect || {
      echo "Certbot failed — SSL will need manual configuration"
    }
  echo "0 0,12 * * * root certbot renew --quiet" > /etc/cron.d/certbot-renew
fi

echo "=== Setup complete ==="
echo "Run 'make deploy' from your machine to deploy the nm-server binary."
