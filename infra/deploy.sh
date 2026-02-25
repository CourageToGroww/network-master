#!/bin/bash
set -euo pipefail

# ── Network Master Deploy Script ──
# Builds nm-server (static Linux musl binary) + React frontend,
# then deploys both to the EC2 instance via SCP.
#
# Prerequisites (one-time setup):
#   rustup target add x86_64-unknown-linux-musl
#   sudo apt install musl-tools          # Linux/WSL2
#   SSH key at ~/.ssh/<key-name>.pem
#
# Usage:
#   make deploy          (from project root)
#   cd infra && ./deploy.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# ── Read server IP and SSH key from Terraform ──
echo "Reading Terraform outputs..."
SERVER_IP=$(terraform -chdir="$SCRIPT_DIR" output -raw server_public_ip 2>/dev/null) || {
  echo ""
  echo "ERROR: Could not read server IP from Terraform state."
  echo "Provision the infrastructure first:"
  echo "  make infra"
  exit 1
}

SSH_KEY=$(grep 'ssh_key_name' "$SCRIPT_DIR/terraform.tfvars" | sed 's/.*= *"\(.*\)"/\1/')
SSH_KEY_PATH="$HOME/.ssh/${SSH_KEY}.pem"

if [ ! -f "$SSH_KEY_PATH" ]; then
  echo ""
  echo "ERROR: SSH key not found at $SSH_KEY_PATH"
  echo "Download your .pem file from the AWS console and place it there."
  exit 1
fi

SSH="ssh -i $SSH_KEY_PATH -o StrictHostKeyChecking=no ec2-user@$SERVER_IP"
SCP="scp -i $SSH_KEY_PATH -o StrictHostKeyChecking=no"

echo ""
echo "=== Deploying Network Master to $SERVER_IP ==="
echo ""

# ── Step 1: Cross-compile nm-server for Linux x86_64 (static musl binary) ──
echo "[1/4] Building nm-server for Linux x86_64 (musl)..."
cd "$PROJECT_ROOT"
cargo build --release --target x86_64-unknown-linux-musl -p nm-server
SERVER_BIN="$PROJECT_ROOT/target/x86_64-unknown-linux-musl/release/nm-server"
echo "      $(du -h "$SERVER_BIN" | cut -f1) — $SERVER_BIN"

# ── Step 2: Build React frontend ──
echo "[2/4] Building frontend..."
cd "$PROJECT_ROOT/frontend"
npm ci --silent
npm run build
DIST_DIR="$PROJECT_ROOT/frontend/dist"
echo "      Built: $DIST_DIR"

# ── Step 3: Upload binary and static files ──
echo "[3/4] Uploading to $SERVER_IP..."
$SCP "$SERVER_BIN" "ec2-user@$SERVER_IP:/tmp/nm-server"
$SSH "rm -rf /tmp/nm-static && mkdir -p /tmp/nm-static"
$SCP -r "$DIST_DIR/." "ec2-user@$SERVER_IP:/tmp/nm-static/"
echo "      Uploaded."

# ── Step 4: Install and restart ──
echo "[4/4] Installing and restarting service..."
$SSH << 'REMOTE'
sudo cp /tmp/nm-server /opt/nm/nm-server
sudo chmod +x /opt/nm/nm-server
sudo rm -rf /opt/nm/static/*
sudo cp -r /tmp/nm-static/. /opt/nm/static/
sudo chown -R nm:nm /opt/nm/
sudo systemctl restart nm-server
sleep 3
sudo systemctl status nm-server --no-pager
REMOTE

echo ""
echo "=== Deploy complete ==="
echo "Dashboard:    http://$SERVER_IP"
echo "Agent install (run as Administrator on Windows):"
echo "  nm-agent.exe install --server $SERVER_IP:80"
echo ""
