#!/bin/bash
set -euo pipefail

# ── Network Master — Interactive Configuration ──
# Prompts for terraform.tfvars values.
# Press Enter to accept the default shown in [brackets].

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TFVARS="$SCRIPT_DIR/terraform.tfvars"

# ── Colors ──
BOLD="\033[1m"
CYAN="\033[36m"
GREEN="\033[32m"
YELLOW="\033[33m"
RESET="\033[0m"

echo ""
echo -e "${BOLD}Network Master — Server Configuration${RESET}"
echo "────────────────────────────────────────"
echo "Press Enter to accept the default shown in [brackets]."
echo ""

# ── If tfvars already exists, ask before overwriting ──
if [ -f "$TFVARS" ]; then
  echo -e "${YELLOW}terraform.tfvars already exists.${RESET}"
  read -rp "Reconfigure? (y/N): " RECONFIGURE
  if [[ ! "$RECONFIGURE" =~ ^[Yy]$ ]]; then
    echo "Keeping existing configuration."
    exit 0
  fi
  echo ""
fi

# ── Helper: prompt with default ──
# Usage: ask "Question" "default" -> result in $REPLY_VAL
ask() {
  local prompt="$1"
  local default="$2"
  echo -en "${CYAN}${prompt}${RESET} [${default}]: "
  read -r input
  if [ -z "$input" ]; then
    REPLY_VAL="$default"
  else
    REPLY_VAL="$input"
  fi
}

# ── Auto-detect public IP ──
echo -n "Detecting your public IP... "
DETECTED_IP=$(curl -s --max-time 5 ifconfig.me 2>/dev/null || echo "")
if [ -n "$DETECTED_IP" ]; then
  echo -e "${GREEN}${DETECTED_IP}${RESET}"
  DEFAULT_ADMIN_IP="${DETECTED_IP}/32"
else
  echo "could not detect (check your internet connection)"
  DEFAULT_ADMIN_IP="0.0.0.0/0"
fi
echo ""

# ── Prompt for each variable ──

ask "AWS region" "us-east-1"
AWS_REGION="$REPLY_VAL"

ask "SSH key pair name in AWS (without .pem)" "nm-key"
SSH_KEY_NAME="$REPLY_VAL"

ask "Your IP for SSH access (CIDR, e.g. 1.2.3.4/32)" "$DEFAULT_ADMIN_IP"
ADMIN_IP="$REPLY_VAL"

ask "Environment name" "prod"
ENVIRONMENT="$REPLY_VAL"

ask "EC2 instance type" "t2.micro"
INSTANCE_TYPE="$REPLY_VAL"

echo ""
echo -e "${BOLD}Secrets${RESET} (press Enter to auto-generate)"

ask "JWT secret (leave blank to auto-generate)" ""
if [ -z "$REPLY_VAL" ]; then
  JWT_SECRET=$(openssl rand -hex 32)
  echo -e "  ${GREEN}Generated JWT secret.${RESET}"
else
  JWT_SECRET="$REPLY_VAL"
fi

ask "Database password (leave blank to auto-generate)" ""
if [ -z "$REPLY_VAL" ]; then
  DB_PASSWORD=$(openssl rand -hex 16)
  echo -e "  ${GREEN}Generated database password.${RESET}"
else
  DB_PASSWORD="$REPLY_VAL"
fi

ask "Admin dashboard password (leave blank to auto-generate)" ""
if [ -z "$REPLY_VAL" ]; then
  ADMIN_PASSWORD=$(openssl rand -base64 12 | tr -d '/+=' | head -c 16)
  echo -e "  ${GREEN}Generated admin password: ${BOLD}${ADMIN_PASSWORD}${RESET}"
  echo -e "  ${YELLOW}Save this — you'll need it to log in.${RESET}"
else
  ADMIN_PASSWORD="$REPLY_VAL"
fi

echo ""
ask "Domain name for HTTPS (leave blank for IP-only access)" ""
DOMAIN_NAME="$REPLY_VAL"

# ── Write terraform.tfvars ──
echo ""
echo "Writing terraform.tfvars..."

cat > "$TFVARS" << EOF
aws_region     = "${AWS_REGION}"
ssh_key_name   = "${SSH_KEY_NAME}"
admin_ip       = "${ADMIN_IP}"
environment    = "${ENVIRONMENT}"
instance_type  = "${INSTANCE_TYPE}"

jwt_secret     = "${JWT_SECRET}"
db_password    = "${DB_PASSWORD}"
admin_password = "${ADMIN_PASSWORD}"

domain_name    = "${DOMAIN_NAME}"
EOF

# ── Summary ──
echo ""
echo -e "${BOLD}Configuration saved to infra/terraform.tfvars${RESET}"
echo "────────────────────────────────────────"
echo -e "  Region:        ${GREEN}${AWS_REGION}${RESET}"
echo -e "  Instance:      ${GREEN}${INSTANCE_TYPE}${RESET}"
echo -e "  SSH key:       ${GREEN}${SSH_KEY_NAME}${RESET}"
echo -e "  Admin SSH IP:  ${GREEN}${ADMIN_IP}${RESET}"
echo -e "  Domain:        ${GREEN}${DOMAIN_NAME:-"(none — IP-only access)"}${RESET}"
echo -e "  Admin password:${GREEN} ${ADMIN_PASSWORD}${RESET}"
echo ""
echo -e "${YELLOW}Make sure ~/.ssh/${SSH_KEY_NAME}.pem exists before deploying.${RESET}"
echo ""
echo -e "Ready to deploy? Run: ${BOLD}make up${RESET}"
echo ""
