#!/bin/bash
# infra/scripts/frontend-userdata.sh
set -euo pipefail

# Install Docker + nginx
dnf update -y
dnf install -y docker nginx
systemctl enable docker nginx
systemctl start docker

# Create app directory
mkdir -p /opt/network-master/static

# Write nginx config
cat > /etc/nginx/conf.d/network-master.conf << 'NGINX'
server {
    listen 80;
    server_name _;

    root /opt/network-master/static;
    index index.html;

    # SPA fallback
    location / {
        try_files $uri $uri/ /index.html;
    }

    # Cache static assets
    location /assets/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # Gzip
    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml;
    gzip_min_length 1000;
}
NGINX

# Remove default nginx config
rm -f /etc/nginx/conf.d/default.conf

systemctl restart nginx

echo "Frontend setup complete"
