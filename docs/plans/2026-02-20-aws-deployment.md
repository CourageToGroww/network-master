# AWS Deployment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Deploy Network Master to an AWS t2.micro EC2 instance with a stable Elastic IP, 1-command deploy (`make up`), and a Datto-like Windows agent that self-enrolls.

**Architecture:** Cross-compile `nm-server` to a static musl Linux binary locally (avoids the 2GB RAM Rust compilation problem on t2.micro). SCP binary + React build to EC2. nm-server runs as a systemd service; PostgreSQL runs as a Docker container on the same instance. nginx sits in front as a WebSocket-aware reverse proxy. Infrastructure mirrors the prodactive-sm pattern.

**Tech Stack:** Terraform 1.5+, AWS (EC2 t2.micro, Elastic IP, VPC), Amazon Linux 2023, Docker (PostgreSQL only), nginx, systemd, Rust musl cross-compilation, Node.js 18+

---

### Task 1: Verify prerequisites and add musl Cargo config

**Files:**
- Create: `.cargo/config.toml`

**Step 1: Install musl toolchain (WSL2/Linux)**

```bash
sudo apt-get update && sudo apt-get install -y musl-tools
rustup target add x86_64-unknown-linux-musl
```

Expected: `x86_64-unknown-linux-musl` appears in `rustup target list --installed`

**Step 2: Create `.cargo/config.toml` to set the musl linker**

```toml
[target.x86_64-unknown-linux-musl]
linker = "x86_64-linux-musl-gcc"
```

This tells Cargo which linker to use for the musl target. Without it, the build may fail on bcrypt/ring crates.

**Step 3: Verify the server binary compiles for musl**

```bash
cargo build --release --target x86_64-unknown-linux-musl -p nm-server
```

Expected: completes without errors in ~5-10 minutes (first time, dependencies cached after).
Binary at: `target/x86_64-unknown-linux-musl/release/nm-server`

Verify it's static:
```bash
file target/x86_64-unknown-linux-musl/release/nm-server
```
Expected: `... ELF 64-bit LSB executable, x86-64, statically linked ...`

**Step 4: Commit**

```bash
git add .cargo/config.toml
git commit -m "build: add musl cross-compilation config for Linux deployment"
```

---

### Task 2: Create infra/ directory and Terraform provider + VPC (main.tf)

**Files:**
- Create: `infra/main.tf`

**Step 1: Create the infra directory and main.tf**

```bash
mkdir -p infra
```

Create `infra/main.tf`:

```hcl
terraform {
  required_version = ">= 1.5"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

# ── VPC ──

resource "aws_vpc" "main" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_support   = true
  enable_dns_hostnames = true

  tags = { Name = "nm-${var.environment}" }
}

resource "aws_subnet" "public" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.0.1.0/24"
  availability_zone       = "${var.aws_region}a"
  map_public_ip_on_launch = true

  tags = { Name = "nm-public-${var.environment}" }
}

resource "aws_internet_gateway" "gw" {
  vpc_id = aws_vpc.main.id
  tags   = { Name = "nm-igw-${var.environment}" }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.gw.id
  }

  tags = { Name = "nm-rt-${var.environment}" }
}

resource "aws_route_table_association" "public" {
  subnet_id      = aws_subnet.public.id
  route_table_id = aws_route_table.public.id
}
```

**Step 2: Commit**

```bash
git add infra/main.tf
git commit -m "infra: add Terraform VPC and provider config"
```

---

### Task 3: Create infra/variables.tf

**Files:**
- Create: `infra/variables.tf`

**Step 1: Write variables.tf**

```hcl
variable "aws_region" {
  description = "AWS region to deploy into"
  type        = string
  default     = "us-east-1"
}

variable "ssh_key_name" {
  description = "Name of the SSH key pair in AWS (without .pem extension)"
  type        = string
}

variable "admin_ip" {
  description = "Your public IP for SSH access in CIDR notation (e.g. 1.2.3.4/32)"
  type        = string
}

variable "environment" {
  description = "Environment name, used in resource naming"
  type        = string
  default     = "prod"
}

variable "instance_type" {
  description = "EC2 instance type"
  type        = string
  default     = "t2.micro"
}

variable "jwt_secret" {
  description = "JWT signing secret — generate with: openssl rand -hex 32"
  type        = string
  sensitive   = true
}

variable "db_password" {
  description = "PostgreSQL password for nm_user"
  type        = string
  sensitive   = true
}

variable "admin_password" {
  description = "Initial admin dashboard password"
  type        = string
  sensitive   = true
}

variable "domain_name" {
  description = "Domain name for SSL via Let's Encrypt (optional — leave empty for IP-only access)"
  type        = string
  default     = ""
}
```

**Step 2: Commit**

```bash
git add infra/variables.tf
git commit -m "infra: add Terraform variables"
```

---

### Task 4: Create infra/ec2.tf (Security Group, IAM, EC2, Elastic IP)

**Files:**
- Create: `infra/ec2.tf`

**Step 1: Write ec2.tf**

```hcl
# ── Security Group ──

resource "aws_security_group" "server" {
  name        = "nm-server-${var.environment}"
  description = "Network Master server — agents + dashboard"
  vpc_id      = aws_vpc.main.id

  # HTTP — agents connect and dashboard loads here
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "HTTP"
  }

  # HTTPS (optional, for when a domain + SSL is configured)
  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
    description = "HTTPS"
  }

  # SSH — admin only
  ingress {
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = [var.admin_ip]
    description = "SSH from admin"
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = { Name = "nm-sg-${var.environment}" }
}

# ── IAM Role (no AWS service access needed — binary delivered via SCP) ──

resource "aws_iam_role" "server" {
  name = "nm-server-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Action    = "sts:AssumeRole"
      Effect    = "Allow"
      Principal = { Service = "ec2.amazonaws.com" }
    }]
  })

  tags = { Name = "nm-server-role-${var.environment}" }
}

resource "aws_iam_instance_profile" "server" {
  name = "nm-server-${var.environment}"
  role = aws_iam_role.server.name
}

# ── AMI: Latest Amazon Linux 2023 ──

data "aws_ami" "amazon_linux" {
  most_recent = true
  owners      = ["amazon"]

  filter {
    name   = "name"
    values = ["al2023-ami-*-x86_64"]
  }

  filter {
    name   = "virtualization-type"
    values = ["hvm"]
  }
}

# ── EC2 Instance ──

resource "aws_instance" "server" {
  ami                    = data.aws_ami.amazon_linux.id
  instance_type          = var.instance_type
  key_name               = var.ssh_key_name
  vpc_security_group_ids = [aws_security_group.server.id]
  subnet_id              = aws_subnet.public.id
  iam_instance_profile   = aws_iam_instance_profile.server.name

  root_block_device {
    volume_size = 20
    volume_type = "gp3"
    encrypted   = true
  }

  user_data = base64encode(templatefile("${path.module}/setup.sh", {
    db_password    = var.db_password
    jwt_secret     = var.jwt_secret
    admin_password = var.admin_password
    domain_name    = var.domain_name
  }))

  tags = { Name = "nm-server-${var.environment}" }
}

# ── Elastic IP — stable public address ──

resource "aws_eip" "server" {
  instance = aws_instance.server.id
  domain   = "vpc"
  tags     = { Name = "nm-eip-${var.environment}" }
}
```

**Step 2: Commit**

```bash
git add infra/ec2.tf
git commit -m "infra: add security group, IAM, EC2 instance, and Elastic IP"
```

---

### Task 5: Create infra/outputs.tf

**Files:**
- Create: `infra/outputs.tf`

**Step 1: Write outputs.tf**

```hcl
output "server_public_ip" {
  description = "Public IP of the Network Master server (Elastic IP)"
  value       = aws_eip.server.public_ip
}

output "ssh_command" {
  description = "SSH command to connect to the server"
  value       = "ssh -i ~/.ssh/${var.ssh_key_name}.pem ec2-user@${aws_eip.server.public_ip}"
}

output "dashboard_url" {
  description = "URL to access the Network Master dashboard"
  value       = var.domain_name != "" ? "https://${var.domain_name}" : "http://${aws_eip.server.public_ip}"
}

output "agent_install_cmd" {
  description = "Command to install the agent on a Windows PC (run as Administrator)"
  value       = "nm-agent.exe install --server ${aws_eip.server.public_ip}:80"
}
```

**Step 2: Commit**

```bash
git add infra/outputs.tf
git commit -m "infra: add Terraform outputs"
```

---

### Task 6: Create terraform.tfvars.example and run terraform init + validate

**Files:**
- Create: `infra/terraform.tfvars.example`
- Create: `infra/.gitignore`

**Step 1: Write terraform.tfvars.example**

```hcl
# Copy this to terraform.tfvars and fill in your values.
# terraform.tfvars is gitignored — never commit secrets.

aws_region     = "us-east-1"
ssh_key_name   = "your-key-name"      # name of the .pem key pair in AWS
admin_ip       = "1.2.3.4/32"         # your public IP — run: curl ifconfig.me
environment    = "prod"
instance_type  = "t2.micro"

# Secrets — generate jwt_secret with: openssl rand -hex 32
jwt_secret     = "change-me"
db_password    = "change-me"
admin_password = "change-me"

# Optional: set a domain for HTTPS via Let's Encrypt. Leave empty for IP-only.
domain_name    = ""
```

**Step 2: Write infra/.gitignore**

```
terraform.tfvars
.terraform/
*.tfstate
*.tfstate.backup
*.tfstate.d/
.terraform.lock.hcl
```

**Step 3: Create your own terraform.tfvars**

```bash
cd infra
cp terraform.tfvars.example terraform.tfvars
# Edit terraform.tfvars with your real values:
#   - ssh_key_name: name of a key pair you've created in the AWS console
#   - admin_ip: run `curl ifconfig.me` to get your current public IP, append /32
#   - jwt_secret: run `openssl rand -hex 32`
#   - db_password: choose a strong password
#   - admin_password: choose a strong password
```

**Step 4: Run terraform init**

```bash
cd infra
terraform init
```

Expected: `Terraform has been successfully initialized!`

**Step 5: Run terraform validate**

```bash
terraform validate
```

Expected: `Success! The configuration is valid.`

**Step 6: Commit**

```bash
cd ..
git add infra/terraform.tfvars.example infra/.gitignore
git commit -m "infra: add tfvars example and gitignore for secrets"
```

---

### Task 7: Create infra/setup.sh (EC2 first-boot user_data)

**Files:**
- Create: `infra/setup.sh`

**Step 1: Write setup.sh**

IMPORTANT: This file is processed by Terraform's `templatefile()`. Rules:
- `${variable}` (curly braces) = Terraform replaces with the actual value before the script runs
- `$VAR` (no curly braces) = Terraform ignores; shell expands at runtime
- In nginx heredocs (unquoted delimiter): use `\$host`, `\$remote_addr` etc. so the shell writes literal `$host` to the nginx config file

```bash
#!/bin/bash
set -euo pipefail

LOG_FILE="/var/log/nm-setup.log"
exec > >(tee -a "$LOG_FILE") 2>&1
echo "=== Network Master Setup - $(date) ==="

# ── 1. Install dependencies ──
dnf update -y
dnf install -y docker nginx certbot python3-certbot-nginx

# ── 2. Start Docker ──
systemctl enable docker
systemctl start docker

# ── 3. Create nm system user and directories ──
useradd -r -s /bin/false nm || true
mkdir -p /opt/nm/static /opt/nm/data /opt/nm/updates
chown -R nm:nm /opt/nm

# ── 4. Start PostgreSQL in Docker ──
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
until docker exec nm-postgres pg_isready -U nm_user -d network_master; do
  sleep 2
done
echo "PostgreSQL is ready."

# ── 5. Write /opt/nm/.env ──
# Using single-quoted heredoc: shell won't expand these.
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

# ── 6. Create systemd service ──
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
# Service starts once binary is deployed — it will fail until then, but Restart=always handles it

# ── 7. Configure nginx (WebSocket-aware reverse proxy) ──
DOMAIN="${domain_name}"
if [ -z "$DOMAIN" ]; then
  SERVER_NAME="_"
else
  SERVER_NAME="$DOMAIN"
fi

# Unquoted heredoc: shell expands $SERVER_NAME; \$ escapes nginx variables from shell
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

        # Keep WebSocket connections alive
        proxy_read_timeout 86400;

        # Allow agent binary uploads (~10MB max for nm-agent.exe)
        client_max_body_size 60M;
    }
}
NGXEOF

rm -f /etc/nginx/conf.d/default.conf 2>/dev/null || true
systemctl enable nginx
systemctl start nginx

# ── 8. SSL via Let's Encrypt (only if domain is provided) ──
if [ -n "$DOMAIN" ]; then
  echo "Setting up SSL for $DOMAIN..."
  certbot --nginx -d "$DOMAIN" --non-interactive --agree-tos \
    --email "admin@$DOMAIN" --redirect || {
      echo "Certbot failed — SSL will need manual configuration"
    }
  echo "0 0,12 * * * root certbot renew --quiet" > /etc/cron.d/certbot-renew
fi

echo "=== Setup complete ==="
echo "Deploy nm-server binary and static files with: make deploy"
```

**Step 2: Make setup.sh executable**

```bash
chmod +x infra/setup.sh
```

**Step 3: Commit**

```bash
git add infra/setup.sh
git commit -m "infra: add EC2 first-boot setup script"
```

---

### Task 8: Create infra/deploy.sh

**Files:**
- Create: `infra/deploy.sh`

**Step 1: Write deploy.sh**

```bash
#!/bin/bash
set -euo pipefail

# ── Network Master Deploy Script ──
# Builds nm-server (Linux musl binary) + React frontend, deploys to EC2 via SCP.
#
# Prerequisites:
#   - terraform apply already run (infra provisioned)
#   - rustup target add x86_64-unknown-linux-musl
#   - sudo apt install musl-tools
#   - SSH key at ~/.ssh/<key-name>.pem
#   - Node.js 18+ installed
#
# Usage:
#   cd infra && ./deploy.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# ── Read Terraform outputs ──
echo "Reading Terraform outputs..."
SERVER_IP=$(terraform -chdir="$SCRIPT_DIR" output -raw server_public_ip 2>/dev/null) || {
  echo "ERROR: Could not read server IP."
  echo "Run 'cd infra && terraform apply' first to provision the infrastructure."
  exit 1
}

SSH_KEY=$(grep 'ssh_key_name' "$SCRIPT_DIR/terraform.tfvars" | sed 's/.*= *"\(.*\)"/\1/')
SSH_KEY_PATH="$HOME/.ssh/${SSH_KEY}.pem"

if [ ! -f "$SSH_KEY_PATH" ]; then
  echo "ERROR: SSH key not found at $SSH_KEY_PATH"
  echo "Download your .pem key from the AWS console and place it there."
  exit 1
fi

SSH_CMD="ssh -i $SSH_KEY_PATH -o StrictHostKeyChecking=no ec2-user@$SERVER_IP"
SCP_CMD="scp -i $SSH_KEY_PATH -o StrictHostKeyChecking=no"

echo ""
echo "=== Deploying Network Master to $SERVER_IP ==="
echo ""

# ── Step 1: Cross-compile nm-server for Linux x86_64 (musl, static) ──
echo "[1/4] Building nm-server (Linux x86_64 musl)..."
cd "$PROJECT_ROOT"
cargo build --release --target x86_64-unknown-linux-musl -p nm-server
SERVER_BIN="$PROJECT_ROOT/target/x86_64-unknown-linux-musl/release/nm-server"
echo "  Binary: $SERVER_BIN ($(du -h "$SERVER_BIN" | cut -f1))"

# ── Step 2: Build React frontend ──
echo "[2/4] Building frontend..."
cd "$PROJECT_ROOT/frontend"
npm ci
npm run build
DIST_DIR="$PROJECT_ROOT/frontend/dist"
echo "  Built: $DIST_DIR"

# ── Step 3: Upload to server ──
echo "[3/4] Uploading to $SERVER_IP..."
$SCP_CMD "$SERVER_BIN" "ec2-user@$SERVER_IP:/tmp/nm-server"
# Create a clean tmp dir for static files, then SCP the whole dist
$SSH_CMD "rm -rf /tmp/nm-static && mkdir -p /tmp/nm-static"
$SCP_CMD -r "$DIST_DIR/." "ec2-user@$SERVER_IP:/tmp/nm-static/"
echo "  Uploaded binary and static files"

# ── Step 4: Install and restart ──
echo "[4/4] Installing and restarting service..."
$SSH_CMD << 'REMOTE'
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
echo "Dashboard: http://$SERVER_IP"
echo "Agent install: nm-agent.exe install --server $SERVER_IP:80"
echo ""
```

**Step 2: Make deploy.sh executable**

```bash
chmod +x infra/deploy.sh
```

**Step 3: Commit**

```bash
git add infra/deploy.sh
git commit -m "infra: add deploy script (cross-compile + SCP + restart)"
```

---

### Task 9: Create root Makefile

**Files:**
- Create: `Makefile`

**Step 1: Write Makefile**

```makefile
.PHONY: up deploy infra ssh destroy

# Provision infrastructure (if not yet done) then deploy the app.
# This is the one command to rule them all.
up:
	cd infra && terraform init -upgrade && terraform apply -auto-approve
	cd infra && ./deploy.sh

# Deploy app only (infra already provisioned).
deploy:
	cd infra && ./deploy.sh

# Provision / update infrastructure only (no deploy).
infra:
	cd infra && terraform init -upgrade && terraform apply -auto-approve

# SSH into the server.
ssh:
	$(eval IP := $(shell cd infra && terraform output -raw server_public_ip))
	$(eval KEY := $(shell grep ssh_key_name infra/terraform.tfvars | sed 's/.*= *"\(.*\)"/\1/'))
	ssh -i ~/.ssh/$(KEY).pem ec2-user@$(IP)

# Tear down all AWS infrastructure. Prompts for confirmation.
destroy:
	cd infra && terraform destroy
```

**Step 2: Commit**

```bash
git add Makefile
git commit -m "infra: add Makefile with up/deploy/infra/ssh/destroy targets"
```

---

### Task 10: First deploy — provision infrastructure

**Step 1: Create your SSH key pair in AWS** (if you don't have one)

Go to AWS Console → EC2 → Key Pairs → Create key pair.
- Name: e.g. `nm-key`
- Type: RSA, format: .pem
- Download the `.pem` file, move to `~/.ssh/nm-key.pem`
- Set permissions: `chmod 400 ~/.ssh/nm-key.pem`

**Step 2: Fill in terraform.tfvars**

```bash
cd infra
# Get your public IP:
curl ifconfig.me
# Generate a JWT secret:
openssl rand -hex 32
```

Edit `infra/terraform.tfvars` with your real values.

**Step 3: Run terraform apply**

```bash
cd infra
terraform apply
```

Review the plan (VPC, subnet, security group, EC2 t2.micro, Elastic IP). Type `yes` to confirm.

Expected: `Apply complete! Resources: 10 added.`
Note the `server_public_ip` output.

The EC2 instance is now booting and running `setup.sh` (installs Docker, nginx, PostgreSQL, systemd service). This takes ~2-3 minutes. Check progress:

```bash
make ssh
# then on the server:
sudo tail -f /var/log/nm-setup.log
```

Wait until you see `=== Setup complete ===` before deploying the app.

---

### Task 11: First deploy — deploy the application

**Step 1: Run the deploy**

From the project root:

```bash
make deploy
```

This will:
1. Cross-compile `nm-server` (~5-10 min first time, cached after)
2. Build React frontend (~30s)
3. SCP binary + static files to EC2
4. Restart nm-server via systemd

Expected final output:
```
=== Deploy complete ===
Dashboard: http://54.x.x.x
Agent install: nm-agent.exe install --server 54.x.x.x:80
```

**Step 2: Verify the server is running**

```bash
curl http://$(cd infra && terraform output -raw server_public_ip)/health
```

Expected: `OK`

**Step 3: Open the dashboard**

Navigate to `http://<your-elastic-ip>` in a browser.
Log in with the `admin_password` you set in terraform.tfvars.

**Step 4: Verify WebSocket agent connections work**

On a Windows PC (or VM), download `nm-agent.exe` from the releases.
Run as Administrator:
```
nm-agent.exe install --server <your-elastic-ip>:80
```

The agent should appear in the dashboard within ~5 seconds.

---

### Task 12: Add agent binary OTA hosting (optional but Datto-like)

**Files:**
- No new files — this is a deployment step

**Step 1: Build the Windows agent**

On a Windows machine with Rust:
```
cargo build --release -p nm-agent
```

Or cross-compile from Linux:
```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu -p nm-agent
```

Binary at: `target/x86_64-pc-windows-gnu/release/nm-agent.exe`

**Step 2: Upload the agent binary to the server**

```bash
IP=$(cd infra && terraform output -raw server_public_ip)
KEY=$(grep ssh_key_name infra/terraform.tfvars | sed 's/.*= *"\(.*\)"/\1/')
scp -i ~/.ssh/$KEY.pem nm-agent.exe ec2-user@$IP:/tmp/nm-agent.exe
ssh -i ~/.ssh/$KEY.pem ec2-user@$IP \
  "sudo cp /tmp/nm-agent.exe /opt/nm/updates/nm-agent.exe && sudo chown nm:nm /opt/nm/updates/nm-agent.exe"
```

The server's `/api/agent/download` endpoint serves this file. Users download it from the dashboard and run `nm-agent.exe install --server <ip>:80`.

---

## Quick Reference

```bash
# First time: provision infra + deploy app
make up

# Subsequent deploys (code changes)
make deploy

# SSH into the server
make ssh

# View server logs
make ssh
# then: sudo journalctl -u nm-server -f

# Tear everything down
make destroy

# Check Terraform state
cd infra && terraform output
```

## Troubleshooting

**musl build fails with linker error:**
```bash
# Check musl-gcc is available:
which x86_64-linux-musl-gcc
# If missing: sudo apt install musl-tools
```

**Deploy fails — can't read server IP:**
```bash
# Infra not provisioned yet:
cd infra && terraform apply
```

**nm-server crashes on start — database not ready:**
```bash
make ssh
sudo journalctl -u nm-server -n 50
# Check PostgreSQL container:
sudo docker ps
sudo docker logs nm-postgres
```

**nginx 502 Bad Gateway:**
```bash
# nm-server not running:
make ssh
sudo systemctl status nm-server
sudo systemctl start nm-server
```
