# Full PingPlotter Pro Clone - Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Transform Network Master into a complete PingPlotter Pro clone with all core and advanced features, a polished responsive dark-mode UI, and one-command AWS deployment via Terraform.

**Architecture:** Two-EC2 deployment (frontend on public subnet, backend on private subnet) behind an ALB with path-based routing. Internal VPC with NAT Gateway for backend egress. `nm` CLI wrapper for deploy/stop/start/teardown with cost control.

**Tech Stack:** Rust (Axum), React 19, TypeScript, Tailwind CSS v4, PostgreSQL 16, Terraform, Docker Compose, AWS (VPC, ALB, EC2, NAT GW)

---

## Phase Overview

| Phase | Description | Estimated Tasks |
|-------|-------------|-----------------|
| 1 | AWS Infrastructure (Terraform + nm CLI) | 12 tasks |
| 2 | Database Migrations (new tables) | 4 tasks |
| 3 | Backend - Auth & RBAC | 8 tasks |
| 4 | Backend - Core Features | 14 tasks |
| 5 | Backend - Advanced Features | 12 tasks |
| 6 | Frontend - UI Overhaul | 16 tasks |
| 7 | Frontend - Feature Pages | 18 tasks |
| 8 | Integration & Deployment | 6 tasks |

---

## Phase 1: AWS Infrastructure

### Task 1.1: Terraform VPC + Subnets

**Files:**
- Create: `infra/main.tf`
- Create: `infra/variables.tf`
- Create: `infra/vpc.tf`

**Step 1: Create variables.tf**

```hcl
# infra/variables.tf

variable "aws_region" {
  description = "AWS region"
  default     = "us-east-1"
}

variable "project_name" {
  description = "Project name prefix for resources"
  default     = "network-master"
}

variable "ssh_key_name" {
  description = "Name of existing EC2 key pair"
  type        = string
}

variable "admin_cidr" {
  description = "CIDR block for SSH access (your IP/32)"
  type        = string
}

variable "backend_instance_type" {
  description = "EC2 instance type for backend"
  default     = "t3.small"
}

variable "frontend_instance_type" {
  description = "EC2 instance type for frontend"
  default     = "t3.micro"
}

variable "db_password" {
  description = "PostgreSQL password"
  type        = string
  sensitive   = true
}

variable "jwt_secret" {
  description = "JWT signing secret"
  type        = string
  sensitive   = true
}
```

**Step 2: Create main.tf**

```hcl
# infra/main.tf

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

data "aws_availability_zones" "available" {
  state = "available"
}
```

**Step 3: Create vpc.tf**

```hcl
# infra/vpc.tf

resource "aws_vpc" "main" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = { Name = "${var.project_name}-vpc" }
}

resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id
  tags   = { Name = "${var.project_name}-igw" }
}

# Public subnet (frontend + ALB)
resource "aws_subnet" "public_a" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.0.1.0/24"
  availability_zone       = data.aws_availability_zones.available.names[0]
  map_public_ip_on_launch = true
  tags                    = { Name = "${var.project_name}-public-a" }
}

resource "aws_subnet" "public_b" {
  vpc_id                  = aws_vpc.main.id
  cidr_block              = "10.0.3.0/24"
  availability_zone       = data.aws_availability_zones.available.names[1]
  map_public_ip_on_launch = true
  tags                    = { Name = "${var.project_name}-public-b" }
}

# Private subnet (backend)
resource "aws_subnet" "private" {
  vpc_id            = aws_vpc.main.id
  cidr_block        = "10.0.2.0/24"
  availability_zone = data.aws_availability_zones.available.names[0]
  tags              = { Name = "${var.project_name}-private" }
}

# NAT Gateway for private subnet egress
resource "aws_eip" "nat" {
  domain = "vpc"
  tags   = { Name = "${var.project_name}-nat-eip" }
}

resource "aws_nat_gateway" "main" {
  allocation_id = aws_eip.nat.id
  subnet_id     = aws_subnet.public_a.id
  tags          = { Name = "${var.project_name}-nat" }

  depends_on = [aws_internet_gateway.main]
}

# Route tables
resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id
  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.main.id
  }
  tags = { Name = "${var.project_name}-public-rt" }
}

resource "aws_route_table_association" "public_a" {
  subnet_id      = aws_subnet.public_a.id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table_association" "public_b" {
  subnet_id      = aws_subnet.public_b.id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id
  route {
    cidr_block     = "0.0.0.0/0"
    nat_gateway_id = aws_nat_gateway.main.id
  }
  tags = { Name = "${var.project_name}-private-rt" }
}

resource "aws_route_table_association" "private" {
  subnet_id      = aws_subnet.private.id
  route_table_id = aws_route_table.private.id
}
```

**Step 4: Commit**

```bash
git add infra/
git commit -m "infra: add Terraform VPC with public/private subnets and NAT gateway"
```

---

### Task 1.2: Security Groups

**Files:**
- Create: `infra/security_groups.tf`

**Step 1: Create security_groups.tf**

```hcl
# infra/security_groups.tf

# ALB security group - public facing
resource "aws_security_group" "alb" {
  name_prefix = "${var.project_name}-alb-"
  vpc_id      = aws_vpc.main.id

  ingress {
    description = "HTTP from anywhere"
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    description = "HTTPS from anywhere (future SSL)"
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = { Name = "${var.project_name}-alb-sg" }
}

# Frontend EC2 - only ALB + SSH
resource "aws_security_group" "frontend" {
  name_prefix = "${var.project_name}-frontend-"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "HTTP from ALB"
    from_port       = 80
    to_port         = 80
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  ingress {
    description = "SSH from admin"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = [var.admin_cidr]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = { Name = "${var.project_name}-frontend-sg" }
}

# Backend EC2 - only ALB + frontend + SSH
resource "aws_security_group" "backend" {
  name_prefix = "${var.project_name}-backend-"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "API from ALB"
    from_port       = 8080
    to_port         = 8080
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  ingress {
    description = "SSH from admin via NAT"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = [var.admin_cidr]
  }

  # Agent WebSocket connections come through ALB
  ingress {
    description     = "WebSocket from ALB"
    from_port       = 8080
    to_port         = 8080
    protocol        = "tcp"
    security_groups = [aws_security_group.frontend.id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = { Name = "${var.project_name}-backend-sg" }
}
```

**Step 2: Commit**

```bash
git add infra/security_groups.tf
git commit -m "infra: add security groups for ALB, frontend, and backend"
```

---

### Task 1.3: ALB with Path-Based Routing

**Files:**
- Create: `infra/alb.tf`

**Step 1: Create alb.tf**

```hcl
# infra/alb.tf

resource "aws_lb" "main" {
  name               = "${var.project_name}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = [aws_subnet.public_a.id, aws_subnet.public_b.id]

  tags = { Name = "${var.project_name}-alb" }
}

# Default target group - frontend
resource "aws_lb_target_group" "frontend" {
  name     = "${var.project_name}-frontend-tg"
  port     = 80
  protocol = "HTTP"
  vpc_id   = aws_vpc.main.id

  health_check {
    path                = "/"
    protocol            = "HTTP"
    healthy_threshold   = 2
    unhealthy_threshold = 3
    timeout             = 5
    interval            = 30
  }

  tags = { Name = "${var.project_name}-frontend-tg" }
}

# Backend target group
resource "aws_lb_target_group" "backend" {
  name     = "${var.project_name}-backend-tg"
  port     = 8080
  protocol = "HTTP"
  vpc_id   = aws_vpc.main.id

  health_check {
    path                = "/health"
    protocol            = "HTTP"
    healthy_threshold   = 2
    unhealthy_threshold = 3
    timeout             = 5
    interval            = 30
  }

  # Sticky sessions for WebSocket
  stickiness {
    type            = "lb_cookie"
    cookie_duration = 86400
    enabled         = true
  }

  tags = { Name = "${var.project_name}-backend-tg" }
}

# HTTP listener
resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.main.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.frontend.arn
  }
}

# Route /api/* to backend
resource "aws_lb_listener_rule" "api" {
  listener_arn = aws_lb_listener.http.arn
  priority     = 100

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.backend.arn
  }

  condition {
    path_pattern { values = ["/api/*"] }
  }
}

# Route /ws/* to backend
resource "aws_lb_listener_rule" "websocket" {
  listener_arn = aws_lb_listener.http.arn
  priority     = 110

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.backend.arn
  }

  condition {
    path_pattern { values = ["/ws/*"] }
  }
}

# Route /health to backend
resource "aws_lb_listener_rule" "health" {
  listener_arn = aws_lb_listener.http.arn
  priority     = 120

  action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.backend.arn
  }

  condition {
    path_pattern { values = ["/health"] }
  }
}

# Target group attachments
resource "aws_lb_target_group_attachment" "frontend" {
  target_group_arn = aws_lb_target_group.frontend.arn
  target_id        = aws_instance.frontend.id
  port             = 80
}

resource "aws_lb_target_group_attachment" "backend" {
  target_group_arn = aws_lb_target_group.backend.arn
  target_id        = aws_instance.backend.id
  port             = 8080
}
```

**Step 2: Commit**

```bash
git add infra/alb.tf
git commit -m "infra: add ALB with path-based routing for API/WS to backend"
```

---

### Task 1.4: EC2 Instances

**Files:**
- Create: `infra/ec2.tf`
- Create: `infra/scripts/backend-userdata.sh`
- Create: `infra/scripts/frontend-userdata.sh`

**Step 1: Create backend user data script**

```bash
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
curl -L "https://github.com/docker/compose/releases/download/v${DOCKER_COMPOSE_VERSION}/docker-compose-linux-$(uname -m)" \
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
      POSTGRES_PASSWORD: ${DB_PASSWORD}
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
      DATABASE_URL: postgresql://nm_user:${DB_PASSWORD}@postgres:5432/network_master
      NM_LISTEN_ADDR: 0.0.0.0:8080
      NM_LOG_LEVEL: info
      NM_JWT_SECRET: ${JWT_SECRET}

volumes:
  pgdata:
COMPOSE

# Write .env
cat > .env << ENV
DB_PASSWORD=${db_password}
JWT_SECRET=${jwt_secret}
ENV

echo "Backend setup complete"
```

**Step 2: Create frontend user data script**

```bash
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
```

**Step 3: Create ec2.tf**

```hcl
# infra/ec2.tf

data "aws_ami" "al2023" {
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

# Backend EC2 (private subnet)
resource "aws_instance" "backend" {
  ami                    = data.aws_ami.al2023.id
  instance_type          = var.backend_instance_type
  key_name               = var.ssh_key_name
  subnet_id              = aws_subnet.private.id
  vpc_security_group_ids = [aws_security_group.backend.id]

  root_block_device {
    volume_size = 30
    volume_type = "gp3"
    encrypted   = true
  }

  user_data = templatefile("${path.module}/scripts/backend-userdata.sh", {
    db_password = var.db_password
    jwt_secret  = var.jwt_secret
  })

  tags = { Name = "${var.project_name}-backend" }
}

# Frontend EC2 (public subnet)
resource "aws_instance" "frontend" {
  ami                         = data.aws_ami.al2023.id
  instance_type               = var.frontend_instance_type
  key_name                    = var.ssh_key_name
  subnet_id                   = aws_subnet.public_a.id
  vpc_security_group_ids      = [aws_security_group.frontend.id]
  associate_public_ip_address = true

  root_block_device {
    volume_size = 20
    volume_type = "gp3"
    encrypted   = true
  }

  user_data = templatefile("${path.module}/scripts/frontend-userdata.sh", {})

  tags = { Name = "${var.project_name}-frontend" }
}
```

**Step 4: Commit**

```bash
git add infra/ec2.tf infra/scripts/
git commit -m "infra: add EC2 instances with user data scripts for backend and frontend"
```

---

### Task 1.5: Outputs

**Files:**
- Create: `infra/outputs.tf`

**Step 1: Create outputs.tf**

```hcl
# infra/outputs.tf

output "alb_dns_name" {
  description = "ALB DNS name (access the app here)"
  value       = aws_lb.main.dns_name
}

output "frontend_public_ip" {
  description = "Frontend EC2 public IP (for SSH)"
  value       = aws_instance.frontend.public_ip
}

output "backend_private_ip" {
  description = "Backend EC2 private IP"
  value       = aws_instance.backend.private_ip
}

output "ssh_frontend" {
  description = "SSH to frontend"
  value       = "ssh -i ~/.ssh/${var.ssh_key_name}.pem ec2-user@${aws_instance.frontend.public_ip}"
}

output "ssh_backend" {
  description = "SSH to backend (via frontend bastion)"
  value       = "ssh -i ~/.ssh/${var.ssh_key_name}.pem -J ec2-user@${aws_instance.frontend.public_ip} ec2-user@${aws_instance.backend.private_ip}"
}

output "dashboard_url" {
  description = "Dashboard URL"
  value       = "http://${aws_lb.main.dns_name}"
}

output "agent_install_cmd" {
  description = "Agent install command"
  value       = "nm-agent.exe install --server ${aws_lb.main.dns_name}:80"
}

output "nat_gateway_id" {
  description = "NAT Gateway ID (for stop/start scripts)"
  value       = aws_nat_gateway.main.id
}

output "nat_eip_allocation_id" {
  description = "NAT EIP allocation ID (for stop/start scripts)"
  value       = aws_eip.nat.id
}

output "backend_instance_id" {
  description = "Backend EC2 instance ID"
  value       = aws_instance.backend.id
}

output "frontend_instance_id" {
  description = "Frontend EC2 instance ID"
  value       = aws_instance.frontend.id
}
```

**Step 2: Commit**

```bash
git add infra/outputs.tf
git commit -m "infra: add Terraform outputs for SSH, URLs, and instance IDs"
```

---

### Task 1.6: Terraform Config Files

**Files:**
- Create: `infra/terraform.tfvars.example`
- Create: `infra/.gitignore`

**Step 1: Create example tfvars**

```hcl
# infra/terraform.tfvars.example
# Copy to terraform.tfvars and fill in your values

aws_region            = "us-east-1"
project_name          = "network-master"
ssh_key_name          = "your-key-pair-name"
admin_cidr            = "YOUR_IP/32"
backend_instance_type = "t3.small"
frontend_instance_type = "t3.micro"
db_password           = "CHANGE_ME_strong_password_here"
jwt_secret            = "CHANGE_ME_random_64_char_string"
```

**Step 2: Create .gitignore**

```
# infra/.gitignore
*.tfstate
*.tfstate.*
.terraform/
.terraform.lock.hcl
terraform.tfvars
*.auto.tfvars
crash.log
```

**Step 3: Commit**

```bash
git add infra/terraform.tfvars.example infra/.gitignore
git commit -m "infra: add tfvars example and gitignore"
```

---

### Task 1.7: The `nm` CLI Wrapper

**Files:**
- Create: `nm` (root of repo)

**Step 1: Create the nm script**

```bash
#!/usr/bin/env bash
# nm - Network Master deployment CLI
set -euo pipefail

INFRA_DIR="$(cd "$(dirname "$0")/infra" && pwd)"
PROJECT_NAME="network-master"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()  { echo -e "${GREEN}[nm]${NC} $*"; }
warn() { echo -e "${YELLOW}[nm]${NC} $*"; }
err()  { echo -e "${RED}[nm]${NC} $*" >&2; }

get_tf_output() {
  terraform -chdir="$INFRA_DIR" output -raw "$1" 2>/dev/null || echo ""
}

cmd_deploy() {
  log "Deploying Network Master infrastructure..."
  cd "$INFRA_DIR"

  if [ ! -f terraform.tfvars ]; then
    err "Missing infra/terraform.tfvars. Copy from terraform.tfvars.example and fill in values."
    exit 1
  fi

  terraform init -upgrade
  terraform apply -auto-approve

  log "Infrastructure deployed!"
  echo ""
  echo -e "${CYAN}Dashboard:${NC} $(get_tf_output dashboard_url)"
  echo -e "${CYAN}SSH Frontend:${NC} $(get_tf_output ssh_frontend)"
  echo -e "${CYAN}SSH Backend:${NC} $(get_tf_output ssh_backend)"
  echo -e "${CYAN}Agent Install:${NC} $(get_tf_output agent_install_cmd)"
}

cmd_stop() {
  log "Stopping instances and NAT gateway to save costs..."

  BACKEND_ID=$(get_tf_output backend_instance_id)
  FRONTEND_ID=$(get_tf_output frontend_instance_id)
  NAT_GW_ID=$(get_tf_output nat_gateway_id)
  NAT_EIP_ID=$(get_tf_output nat_eip_allocation_id)
  REGION=$(cd "$INFRA_DIR" && terraform output -raw aws_region 2>/dev/null || echo "us-east-1")

  if [ -z "$BACKEND_ID" ] || [ -z "$FRONTEND_ID" ]; then
    err "No infrastructure found. Run 'nm deploy' first."
    exit 1
  fi

  # Stop EC2 instances
  log "Stopping EC2 instances..."
  aws ec2 stop-instances --instance-ids "$BACKEND_ID" "$FRONTEND_ID" --region "$REGION" > /dev/null
  aws ec2 wait instance-stopped --instance-ids "$BACKEND_ID" "$FRONTEND_ID" --region "$REGION"

  # Delete NAT Gateway (saves ~$0.045/hr)
  if [ -n "$NAT_GW_ID" ]; then
    log "Deleting NAT Gateway..."
    aws ec2 delete-nat-gateway --nat-gateway-id "$NAT_GW_ID" --region "$REGION" > /dev/null 2>&1 || true
    sleep 5
    # Release NAT EIP (avoids idle charge)
    if [ -n "$NAT_EIP_ID" ]; then
      aws ec2 release-address --allocation-id "$NAT_EIP_ID" --region "$REGION" > /dev/null 2>&1 || true
    fi
  fi

  log "All stopped. Estimated cost: ~\$0.01/hr (EBS + ALB idle only)"
  warn "Run 'nm start' to bring everything back up."
}

cmd_start() {
  log "Starting instances and recreating NAT gateway..."

  BACKEND_ID=$(get_tf_output backend_instance_id)
  FRONTEND_ID=$(get_tf_output frontend_instance_id)
  REGION=$(cd "$INFRA_DIR" && terraform output -raw aws_region 2>/dev/null || echo "us-east-1")

  if [ -z "$BACKEND_ID" ] || [ -z "$FRONTEND_ID" ]; then
    err "No infrastructure found. Run 'nm deploy' first."
    exit 1
  fi

  # Start EC2 instances
  log "Starting EC2 instances..."
  aws ec2 start-instances --instance-ids "$BACKEND_ID" "$FRONTEND_ID" --region "$REGION" > /dev/null
  aws ec2 wait instance-running --instance-ids "$BACKEND_ID" "$FRONTEND_ID" --region "$REGION"

  # Recreate NAT Gateway via Terraform
  log "Recreating NAT Gateway..."
  cd "$INFRA_DIR"
  terraform apply -auto-approve -target=aws_eip.nat -target=aws_nat_gateway.main

  log "All running!"
  echo -e "${CYAN}Dashboard:${NC} $(get_tf_output dashboard_url)"
}

cmd_teardown() {
  warn "This will DESTROY all infrastructure and data. Are you sure? [y/N]"
  read -r confirm
  if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
    log "Aborted."
    exit 0
  fi

  log "Tearing down all infrastructure..."
  cd "$INFRA_DIR"
  terraform destroy -auto-approve

  log "All destroyed. Cost: \$0.00/mo"
}

cmd_status() {
  BACKEND_ID=$(get_tf_output backend_instance_id)
  FRONTEND_ID=$(get_tf_output frontend_instance_id)
  REGION=$(cd "$INFRA_DIR" && terraform output -raw aws_region 2>/dev/null || echo "us-east-1")

  if [ -z "$BACKEND_ID" ]; then
    warn "No infrastructure found. Run 'nm deploy' first."
    exit 0
  fi

  echo -e "${CYAN}=== Network Master Status ===${NC}"
  echo ""

  # Instance states
  STATES=$(aws ec2 describe-instances \
    --instance-ids "$BACKEND_ID" "$FRONTEND_ID" \
    --region "$REGION" \
    --query 'Reservations[].Instances[].{Id:InstanceId,State:State.Name,Type:InstanceType,Name:Tags[?Key==`Name`].Value|[0]}' \
    --output table 2>/dev/null || echo "Could not query instances")

  echo "$STATES"
  echo ""

  # Cost estimate
  BACKEND_STATE=$(aws ec2 describe-instances --instance-ids "$BACKEND_ID" --region "$REGION" \
    --query 'Reservations[0].Instances[0].State.Name' --output text 2>/dev/null || echo "unknown")

  if [ "$BACKEND_STATE" = "running" ]; then
    echo -e "${GREEN}State: RUNNING${NC} - Estimated ~\$0.07/hr (~\$50/mo)"
  elif [ "$BACKEND_STATE" = "stopped" ]; then
    echo -e "${YELLOW}State: STOPPED${NC} - Estimated ~\$0.01/hr (~\$7/mo)"
  else
    echo -e "${RED}State: $BACKEND_STATE${NC}"
  fi

  echo ""
  echo -e "${CYAN}Dashboard:${NC} $(get_tf_output dashboard_url)"
}

cmd_ssh_frontend() {
  SSH_CMD=$(get_tf_output ssh_frontend)
  log "Connecting to frontend..."
  eval "$SSH_CMD"
}

cmd_ssh_backend() {
  SSH_CMD=$(get_tf_output ssh_backend)
  log "Connecting to backend (via frontend bastion)..."
  eval "$SSH_CMD"
}

cmd_build_deploy() {
  log "Building and deploying application..."

  FRONTEND_IP=$(get_tf_output frontend_public_ip)
  BACKEND_IP=$(get_tf_output backend_private_ip)
  SSH_KEY="$HOME/.ssh/$(cd "$INFRA_DIR" && terraform output -raw ssh_key_name 2>/dev/null).pem"

  # Build frontend
  log "Building frontend..."
  cd "$(dirname "$0")/frontend"
  npm ci
  VITE_API_BASE="" npm run build

  # Copy frontend to frontend EC2
  log "Deploying frontend..."
  scp -i "$SSH_KEY" -r dist/* "ec2-user@$FRONTEND_IP:/opt/network-master/static/"

  # Build backend Docker image and push
  log "Building and pushing backend..."
  cd "$(dirname "$0")"
  ssh -i "$SSH_KEY" -J "ec2-user@$FRONTEND_IP" "ec2-user@$BACKEND_IP" \
    "cd /opt/network-master && docker-compose up --build -d"

  log "Deployment complete!"
}

cmd_help() {
  echo -e "${CYAN}nm${NC} - Network Master Deployment CLI"
  echo ""
  echo "Usage: nm <command>"
  echo ""
  echo "Commands:"
  echo "  deploy        Create all AWS infrastructure (terraform apply)"
  echo "  stop          Stop EC2s + NAT GW (\$0.01/hr)"
  echo "  start         Restart EC2s + NAT GW"
  echo "  teardown      Destroy everything (\$0.00/mo)"
  echo "  status        Show instance states and cost estimate"
  echo "  build-deploy  Build and deploy app to running instances"
  echo "  ssh-frontend  SSH to frontend EC2"
  echo "  ssh-backend   SSH to backend EC2 (via bastion)"
  echo "  help          Show this help"
}

case "${1:-help}" in
  deploy)        cmd_deploy ;;
  stop)          cmd_stop ;;
  start)         cmd_start ;;
  teardown)      cmd_teardown ;;
  status)        cmd_status ;;
  build-deploy)  cmd_build_deploy ;;
  ssh-frontend)  cmd_ssh_frontend ;;
  ssh-backend)   cmd_ssh_backend ;;
  help|--help|-h) cmd_help ;;
  *)
    err "Unknown command: $1"
    cmd_help
    exit 1
    ;;
esac
```

**Step 2: Make executable**

```bash
chmod +x nm
```

**Step 3: Commit**

```bash
git add nm
git commit -m "feat: add nm CLI for deploy/stop/start/teardown with cost control"
```

---

### Task 1.8: Backend Dockerfile (Standalone)

**Files:**
- Create: `infra/backend/Dockerfile`
- Create: `infra/backend/docker-compose.yml`

**Step 1: Create backend Dockerfile**

```dockerfile
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
```

**Step 2: Create backend docker-compose.yml**

```yaml
# infra/backend/docker-compose.yml
services:
  postgres:
    image: postgres:16
    restart: unless-stopped
    environment:
      POSTGRES_DB: network_master
      POSTGRES_USER: nm_user
      POSTGRES_PASSWORD: ${DB_PASSWORD:-nm_secret}
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U nm_user -d network_master"]
      interval: 5s
      timeout: 5s
      retries: 5

  server:
    build:
      context: ../../
      dockerfile: infra/backend/Dockerfile
    restart: unless-stopped
    ports:
      - "8080:8080"
    depends_on:
      postgres:
        condition: service_healthy
    environment:
      DATABASE_URL: postgresql://nm_user:${DB_PASSWORD:-nm_secret}@postgres:5432/network_master
      NM_LISTEN_ADDR: 0.0.0.0:8080
      NM_LOG_LEVEL: ${NM_LOG_LEVEL:-info}
      NM_JWT_SECRET: ${JWT_SECRET:-change-me-in-production}

volumes:
  pgdata:
```

**Step 3: Commit**

```bash
git add infra/backend/
git commit -m "infra: add standalone backend Dockerfile and docker-compose"
```

---

### Task 1.9: Frontend Dockerfile (Standalone)

**Files:**
- Create: `infra/frontend/Dockerfile`
- Create: `infra/frontend/docker-compose.yml`
- Create: `infra/frontend/nginx.conf`

**Step 1: Create nginx config**

```nginx
# infra/frontend/nginx.conf
server {
    listen 80;
    server_name _;

    root /usr/share/nginx/html;
    index index.html;

    # SPA fallback
    location / {
        try_files $uri $uri/ /index.html;
    }

    # Cache static assets aggressively
    location /assets/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # Gzip
    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml;
    gzip_min_length 1000;
    gzip_vary on;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;
}
```

**Step 2: Create frontend Dockerfile**

```dockerfile
# infra/frontend/Dockerfile
FROM node:22-slim AS builder

WORKDIR /app
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY infra/frontend/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

**Step 3: Create frontend docker-compose.yml**

```yaml
# infra/frontend/docker-compose.yml
services:
  frontend:
    build:
      context: ../../
      dockerfile: infra/frontend/Dockerfile
    restart: unless-stopped
    ports:
      - "80:80"
```

**Step 4: Commit**

```bash
git add infra/frontend/
git commit -m "infra: add standalone frontend Dockerfile with nginx and docker-compose"
```

---

### Task 1.10: Add aws_region output + SSH key output

**Files:**
- Modify: `infra/outputs.tf`

**Step 1: Add missing outputs**

Add to `infra/outputs.tf`:

```hcl
output "aws_region" {
  description = "AWS region"
  value       = var.aws_region
}

output "ssh_key_name" {
  description = "SSH key pair name"
  value       = var.ssh_key_name
}
```

**Step 2: Commit**

```bash
git add infra/outputs.tf
git commit -m "infra: add aws_region and ssh_key_name outputs for nm CLI"
```

---

## Phase 2: Database Migrations

### Task 2.1: Users Table + RBAC Migration

**Files:**
- Create: `migrations/004_users_rbac.sql`

**Step 1: Write migration**

```sql
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
```

**Step 2: Commit**

```bash
git add migrations/004_users_rbac.sql
git commit -m "db: add users table with RBAC roles"
```

---

### Task 2.2: Workspaces + Session State + Named Configs Migration

**Files:**
- Create: `migrations/005_workspaces_sessions_configs.sql`

**Step 1: Write migration**

```sql
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
```

**Step 2: Commit**

```bash
git add migrations/005_workspaces_sessions_configs.sql
git commit -m "db: add workspaces, session states, timeline comments, summary screens"
```

---

### Task 2.3: Advanced Alerts Migration

**Files:**
- Create: `migrations/006_advanced_alerts.sql`

**Step 1: Write migration**

```sql
-- migrations/006_advanced_alerts.sql

-- Extend alert_rules with more condition types and action types
ALTER TABLE alert_rules ADD COLUMN condition_type VARCHAR(30) NOT NULL DEFAULT 'threshold'
    CHECK (condition_type IN (
        'latency_over_time', 'loss_over_time', 'latency_over_samples',
        'mos_threshold', 'route_changed', 'ip_in_route', 'timer'
    ));

ALTER TABLE alert_rules ADD COLUMN condition_params JSONB NOT NULL DEFAULT '{}';
ALTER TABLE alert_rules ADD COLUMN actions JSONB NOT NULL DEFAULT '[]';
ALTER TABLE alert_rules ADD COLUMN notify_on_start BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE alert_rules ADD COLUMN notify_on_end BOOLEAN NOT NULL DEFAULT FALSE;

-- LiveShare links
CREATE TABLE liveshare_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token VARCHAR(64) NOT NULL UNIQUE,
    target_id UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    label VARCHAR(200),
    notes TEXT,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_liveshare_token ON liveshare_links(token);

-- Discovered devices (Local Network Discovery)
CREATE TABLE discovered_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    ip_address VARCHAR(45) NOT NULL,
    mac_address VARCHAR(17),
    hostname VARCHAR(255),
    vendor VARCHAR(255),
    latency_us INTEGER,
    description TEXT,
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (agent_id, ip_address)
);

CREATE INDEX idx_devices_agent ON discovered_devices(agent_id);

-- Insights (automated analysis)
CREATE TABLE insights (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_id UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    analysis_period VARCHAR(10) NOT NULL CHECK (analysis_period IN ('24h', '48h', '7d')),
    overall_quality VARCHAR(10) NOT NULL CHECK (overall_quality IN ('good', 'fair', 'poor')),
    good_pct REAL NOT NULL DEFAULT 0,
    fair_pct REAL NOT NULL DEFAULT 0,
    poor_pct REAL NOT NULL DEFAULT 0,
    events JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_insights_target ON insights(target_id, created_at DESC);
```

**Step 2: Commit**

```bash
git add migrations/006_advanced_alerts.sql
git commit -m "db: add advanced alerts, liveshare, discovered devices, and insights tables"
```

---

### Task 2.4: MOS + Focus Time Support in Hop Stats

**Files:**
- Create: `migrations/007_mos_focus_time.sql`

**Step 1: Write migration**

```sql
-- migrations/007_mos_focus_time.sql

-- Add MOS score to hop_stats_hourly
ALTER TABLE hop_stats_hourly ADD COLUMN mos_score REAL;

-- Add MOS to running stats view (computed, not stored in samples)
-- Focus Time is computed at query time, no schema change needed

-- Add quality score threshold settings
CREATE TABLE user_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    setting_key VARCHAR(100) NOT NULL,
    setting_value JSONB NOT NULL,
    UNIQUE (user_id, setting_key)
);

-- Seed default display thresholds
INSERT INTO user_settings (user_id, setting_key, setting_value)
SELECT id, 'display_thresholds', '{"warning_ms": 200, "critical_ms": 500, "loss_warning_pct": 5, "loss_critical_pct": 15}'::jsonb
FROM users WHERE role = 'admin' LIMIT 1;
```

**Step 2: Commit**

```bash
git add migrations/007_mos_focus_time.sql
git commit -m "db: add MOS score to hourly stats, user settings table"
```

---

## Phase 3: Backend - Auth & RBAC

### Task 3.1: User Model + Auth Types

**Files:**
- Modify: `crates/nm-common/src/models.rs` (add after line 345)

**Step 1: Add user models to nm-common/src/models.rs**

Add at end of file:

```rust
// ── User & Auth ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: String,
    pub role: String,
    pub is_active: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserPublic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublic {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub password: String,
    pub display_name: String,
    #[serde(default = "default_viewer_role")]
    pub role: String,
}

fn default_viewer_role() -> String { "viewer".to_string() }

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid,
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

// ── Workspace ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub layout_json: serde_json::Value,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspace {
    pub name: String,
    pub layout_json: Option<serde_json::Value>,
}

// ── Timeline Comment ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TimelineComment {
    pub id: Uuid,
    pub target_id: Uuid,
    pub session_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub text: String,
    pub auto_generated: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTimelineComment {
    pub target_id: Uuid,
    pub session_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    pub text: String,
}

// ── Summary Screen ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SummaryScreen {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub focus_time_seconds: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSummaryScreen {
    pub name: String,
    #[serde(default = "default_focus_time")]
    pub focus_time_seconds: i32,
}

fn default_focus_time() -> i32 { 600 }

// ── LiveShare ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LiveShareLink {
    pub id: Uuid,
    pub token: String,
    pub target_id: Uuid,
    pub label: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLiveShareLink {
    pub target_id: Uuid,
    pub label: Option<String>,
    pub notes: Option<String>,
    pub expires_in_hours: Option<i64>,
}

// ── Insight ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Insight {
    pub id: Uuid,
    pub target_id: Uuid,
    pub analysis_period: String,
    pub overall_quality: String,
    pub good_pct: f32,
    pub fair_pct: f32,
    pub poor_pct: f32,
    pub events: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ── Discovered Device ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DiscoveredDevice {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub ip_address: String,
    pub mac_address: Option<String>,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub latency_us: Option<i32>,
    pub description: Option<String>,
    pub discovered_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}
```

**Step 2: Commit**

```bash
git add crates/nm-common/src/models.rs
git commit -m "feat: add User, Workspace, Comment, Summary, LiveShare, Insight models"
```

---

### Task 3.2: Auth Middleware + JWT

**Files:**
- Create: `crates/nm-server/src/auth.rs`

**Step 1: Create auth module**

```rust
// crates/nm-server/src/auth.rs

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde_json::json;
use uuid::Uuid;

use nm_common::models::JwtClaims;
use crate::state::AppState;

/// Create a JWT token for a user
pub fn create_token(
    user_id: Uuid,
    email: &str,
    role: &str,
    secret: &str,
    expiry_hours: i64,
) -> anyhow::Result<String> {
    let now = chrono::Utc::now();
    let claims = JwtClaims {
        sub: user_id,
        email: email.to_string(),
        role: role.to_string(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::hours(expiry_hours)).timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// Validate a JWT token and return claims
pub fn validate_token(token: &str, secret: &str) -> Result<JwtClaims, StatusCode> {
    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| StatusCode::UNAUTHORIZED)
}

/// Extract claims from request (used by handlers)
pub fn get_claims(request: &Request) -> Option<&JwtClaims> {
    request.extensions().get::<JwtClaims>()
}

/// Middleware: require valid JWT
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match token {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization header"})),
            )
                .into_response();
        }
    };

    match validate_token(token, &state.config.jwt_secret) {
        Ok(claims) => {
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
            .into_response(),
    }
}

/// Middleware: require admin role
pub async fn require_admin(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    // First check auth
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match token {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization header"})),
            )
                .into_response();
        }
    };

    match validate_token(token, &state.config.jwt_secret) {
        Ok(claims) => {
            if claims.role != "admin" {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Admin access required"})),
                )
                    .into_response();
            }
            let mut request = request;
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
            .into_response(),
    }
}

/// Middleware: require operator or admin role
pub async fn require_operator(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match token {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization header"})),
            )
                .into_response();
        }
    };

    match validate_token(token, &state.config.jwt_secret) {
        Ok(claims) => {
            if claims.role != "admin" && claims.role != "operator" {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({"error": "Operator access required"})),
                )
                    .into_response();
            }
            let mut request = request;
            request.extensions_mut().insert(claims);
            next.run(request).await
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid or expired token"})),
        )
            .into_response(),
    }
}
```

**Step 2: Register auth module in main.rs**

Add `pub mod auth;` to the module declarations in `crates/nm-server/src/main.rs`.

**Step 3: Commit**

```bash
git add crates/nm-server/src/auth.rs
git commit -m "feat: add JWT auth middleware with role-based access control"
```

---

### Task 3.3: Auth API Routes (Login, Register, Me)

**Files:**
- Create: `crates/nm-server/src/api/auth.rs`
- Create: `crates/nm-server/src/db/users.rs`

**Step 1: Create users DB module**

```rust
// crates/nm-server/src/db/users.rs

use nm_common::models::{CreateUser, User};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_by_email(pool: &PgPool, email: &str) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND is_active = true")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn create(pool: &PgPool, input: &CreateUser, password_hash: &str) -> anyhow::Result<User> {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (email, password_hash, display_name, role) VALUES ($1, $2, $3, $4) RETURNING *"
    )
        .bind(&input.email)
        .bind(password_hash)
        .bind(&input.display_name)
        .bind(&input.role)
        .fetch_one(pool)
        .await?;
    Ok(user)
}

pub async fn list(pool: &PgPool) -> anyhow::Result<Vec<User>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at")
        .fetch_all(pool)
        .await?;
    Ok(users)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_last_login(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
```

**Step 2: Create auth API routes**

```rust
// crates/nm-server/src/api/auth.rs

use axum::{
    extract::{Request, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;

use nm_common::models::{CreateUser, LoginRequest, LoginResponse, UserPublic};
use crate::{auth, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/register", post(register))
        .route("/auth/me", get(me))
}

async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user = crate::db::users::get_by_email(&state.pool, &input.email)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"}))))?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid credentials"}))))?;

    let valid = bcrypt::verify(&input.password, &user.password_hash)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Auth error"}))))?;

    if !valid {
        return Err((StatusCode::UNAUTHORIZED, Json(json!({"error": "Invalid credentials"}))));
    }

    let token = auth::create_token(
        user.id,
        &user.email,
        &user.role,
        &state.config.jwt_secret,
        state.config.jwt_expiry_hours,
    )
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Token error"}))))?;

    let _ = crate::db::users::update_last_login(&state.pool, user.id).await;

    Ok(Json(LoginResponse {
        token,
        user: UserPublic {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            role: user.role,
        },
    }))
}

async fn register(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserPublic>), (StatusCode, Json<serde_json::Value>)> {
    // Validate email format
    if !input.email.contains('@') || input.email.len() < 5 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid email"}))));
    }

    // Validate password strength
    if input.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Password must be at least 8 characters"}))));
    }

    // Validate role
    if !["admin", "operator", "viewer"].contains(&input.role.as_str()) {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Invalid role"}))));
    }

    let password_hash = bcrypt::hash(&input.password, 12)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Hash error"}))))?;

    let user = crate::db::users::create(&state.pool, &input, &password_hash)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate") {
                (StatusCode::CONFLICT, Json(json!({"error": "Email already exists"})))
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": "Database error"})))
            }
        })?;

    Ok((StatusCode::CREATED, Json(UserPublic {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        role: user.role,
    })))
}

async fn me(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<UserPublic>, StatusCode> {
    let claims = auth::get_claims(&request).ok_or(StatusCode::UNAUTHORIZED)?;

    let user = crate::db::users::get_by_id(&state.pool, claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(UserPublic {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        role: user.role,
    }))
}
```

**Step 3: Register in db/mod.rs and api/mod.rs**

Add `pub mod users;` to `crates/nm-server/src/db/mod.rs`.
Add `pub mod auth;` to `crates/nm-server/src/api/mod.rs` and merge `auth::router()` into the router function.

**Step 4: Commit**

```bash
git add crates/nm-server/src/api/auth.rs crates/nm-server/src/db/users.rs
git commit -m "feat: add login/register/me auth API endpoints"
```

---

### Task 3.4 - 3.8: Wire Auth Middleware to Existing Routes

These tasks involve adding `require_auth` middleware to all existing API routes and `require_operator` to mutation routes. The pattern is:

**For each route group in `crates/nm-server/src/api/mod.rs`:**

```rust
// Before (no auth):
.merge(agents::router())

// After (with auth):
.merge(agents::router().route_layer(axum::middleware::from_fn_with_state(
    state.clone(),
    auth::require_auth,
)))
```

Auth-free routes: `/auth/login`, `/auth/register`, `/share/{token}`, `/health`, `/ws/agent`

Each task is a separate route group. Commit after each.

---

## Phase 4: Backend - Core Features

### Task 4.1: Focus Time Query Parameter

**Files:**
- Modify: `crates/nm-server/src/api/traces.rs` (line 65-72, TimeseriesQuery)
- Modify: `crates/nm-server/src/db/samples.rs`

Add `focus_start` and `focus_end` parameters to all stats queries. When present, compute Avg/Min/Max/PL%/Jitter/MOS only within that window. Default to last 10 minutes.

**New query struct:**

```rust
#[derive(Debug, Deserialize)]
pub struct FocusQuery {
    pub focus_start: Option<DateTime<Utc>>,
    pub focus_end: Option<DateTime<Utc>>,
    #[serde(default = "default_focus_seconds")]
    pub focus_seconds: i64,
}

fn default_focus_seconds() -> i64 { 600 }
```

Add a `get_focus_stats` function to `db/samples.rs` that computes all stats within a focus window.

---

### Task 4.2: MOS Calculation in Rust

**Files:**
- Modify: `crates/nm-common/src/quality.rs`
- Modify: `crates/nm-server/src/state.rs` (RunningHopStats)

Add `compute_mos(avg_rtt_ms: f64, jitter_ms: f64, loss_pct: f64) -> f64` using the ITU-T E-model:

```rust
pub fn compute_mos(avg_rtt_ms: f64, jitter_ms: f64, loss_pct: f64) -> f64 {
    let r = 93.2f64;
    let effective_latency = avg_rtt_ms + (jitter_ms * 2.0) + 10.0;

    let latency_penalty = if effective_latency < 160.0 {
        effective_latency / 40.0
    } else {
        (effective_latency - 120.0) / 10.0
    };

    let loss_penalty = loss_pct * 2.5;
    let r = (r - latency_penalty - loss_penalty).clamp(0.0, 100.0);

    let mos = 1.0 + (0.035 * r) + (0.000007 * r * (r - 60.0) * (100.0 - r));
    mos.clamp(1.0, 4.5)
}
```

Add `mos` field to `RunningHopStats` and include in `LiveHopData`.

---

### Task 4.3: Named Configurations API

**Files:**
- Modify: `crates/nm-server/src/api/targets.rs` (add config_id support)
- Modify: `crates/nm-server/src/db/targets.rs` (add config_id to queries)

When a target is created/updated with a `config_id`, load the trace profile and push updated settings to the agent via WebSocket.

---

### Task 4.4: Session State Machine

**Files:**
- Modify: `crates/nm-server/src/db/sessions.rs`
- Create: `crates/nm-server/src/api/sessions.rs`

New endpoints:
- `POST /sessions/{id}/pause` - Set state to 'paused', tell agent to stop probing
- `POST /sessions/{id}/resume` - Set state to 'active', tell agent to restart
- `POST /sessions/{id}/archive` - Set state to 'archived'
- `GET /sessions` - List all sessions with state filtering

---

### Task 4.5: Timeline Comments API

**Files:**
- Create: `crates/nm-server/src/api/comments.rs`
- Create: `crates/nm-server/src/db/comments.rs`

Endpoints:
- `GET /targets/{id}/comments?from=&to=` - List comments in time range
- `POST /targets/{id}/comments` - Create comment
- `DELETE /comments/{id}` - Delete comment

Auto-generated comments on: alert triggers, route changes.

---

### Task 4.6: Summary Screens API

**Files:**
- Create: `crates/nm-server/src/api/summaries.rs`
- Create: `crates/nm-server/src/db/summaries.rs`

Endpoints:
- CRUD for summary screens
- `GET /summaries/{id}/data?focus_seconds=600` - Get aggregated final-hop stats for all targets

---

### Task 4.7: Workspaces API

**Files:**
- Create: `crates/nm-server/src/api/workspaces.rs`
- Create: `crates/nm-server/src/db/workspaces.rs`

Endpoints:
- CRUD for workspaces
- `POST /workspaces/{id}/load` - Returns full workspace state (targets, layout, focus times)
- `POST /workspaces/{id}/save` - Saves current state

---

### Task 4.8-4.14: Additional Core Backend Tasks

- **4.8**: WHOIS/ASN lookup background worker (RDAP client, cache in hops table)
- **4.9**: LiveShare endpoint (token-based WebSocket subscription)
- **4.10**: Insights background analyzer (cron every 6 hours)
- **4.11**: Local Network Discovery agent protocol messages
- **4.12**: Advanced alert condition evaluator (6 condition types)
- **4.13**: Alert action executor (11 action types with template variables)
- **4.14**: REST API key management (for external integrations)

---

## Phase 5: Backend - Advanced Features

### Tasks 5.1-5.12

- **5.1**: Template variable engine for alerts (`{{Host.IPAddress}}`, etc.)
- **5.2**: Alert action: Add Comment (auto-annotate timeline on alert)
- **5.3**: Alert action: REST webhook with configurable method/body
- **5.4**: Alert action: Modify Summary (add/remove target from summary)
- **5.5**: Route change alert condition
- **5.6**: IP in route alert condition
- **5.7**: Timer-based alert condition
- **5.8**: MOS threshold alert condition
- **5.9**: WHOIS lookup endpoint (`GET /hops/{id}/whois`)
- **5.10**: Export improvements (JSON export, image placeholder)
- **5.11**: User settings API (display thresholds, preferences)
- **5.12**: Agent health metrics (CPU, memory in heartbeat)

---

## Phase 6: Frontend - UI Overhaul

### Task 6.1: Dark Mode Theme System

**Files:**
- Modify: `frontend/src/styles/globals.css`
- Create: `frontend/src/stores/themeStore.ts`
- Create: `frontend/src/components/ui/ThemeToggle.tsx`

**Step 1: Create theme store**

```typescript
// frontend/src/stores/themeStore.ts
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

type Theme = 'dark' | 'light' | 'system';

interface ThemeState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  resolvedTheme: () => 'dark' | 'light';
}

export const useThemeStore = create<ThemeState>()(
  persist(
    (set, get) => ({
      theme: 'dark',
      setTheme: (theme) => {
        set({ theme });
        applyTheme(theme);
      },
      resolvedTheme: () => {
        const t = get().theme;
        if (t === 'system') {
          return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
        }
        return t;
      },
    }),
    { name: 'nm-theme' }
  )
);

function applyTheme(theme: Theme) {
  const resolved = theme === 'system'
    ? (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
    : theme;
  document.documentElement.classList.toggle('dark', resolved === 'dark');
  document.documentElement.classList.toggle('light', resolved === 'light');
}
```

**Step 2: Update globals.css with light/dark CSS variables**

```css
/* Light theme */
:root.light {
  --bg-primary: #ffffff;
  --bg-surface: #f4f4f5;
  --bg-elevated: #e4e4e7;
  --border-default: #d4d4d8;
  --text-primary: #09090b;
  --text-secondary: #71717a;
  --accent: #2563eb;
  --success: #059669;
  --warning: #d97706;
  --danger: #dc2626;
}

/* Dark theme (default) */
:root, :root.dark {
  --bg-primary: #09090b;
  --bg-surface: #18181b;
  --bg-elevated: #27272a;
  --border-default: #3f3f46;
  --text-primary: #fafafa;
  --text-secondary: #a1a1aa;
  --accent: #3b82f6;
  --success: #10b981;
  --warning: #f59e0b;
  --danger: #ef4444;
}
```

---

### Task 6.2: Responsive AppShell with Bottom Tab Bar

**Files:**
- Rewrite: `frontend/src/components/layout/AppShell.tsx`
- Rewrite: `frontend/src/components/layout/Sidebar.tsx`
- Rewrite: `frontend/src/components/layout/Header.tsx`

Mobile (< 768px): Bottom tab bar navigation, no sidebar.
Tablet (768-1024px): Collapsible sidebar, compact header.
Desktop (> 1024px): Full sidebar + header.

---

### Task 6.3: Login Page + Auth Context

**Files:**
- Create: `frontend/src/stores/authStore.ts`
- Create: `frontend/src/features/auth/LoginPage.tsx`
- Modify: `frontend/src/api/client.ts` (add auth header)
- Modify: `frontend/src/App.tsx` (protected routes)

---

### Task 6.4: Focus Time Control Component

**Files:**
- Create: `frontend/src/components/FocusTimeControl.tsx`
- Create: `frontend/src/stores/focusStore.ts`

Universal dropdown: 10min, 30min, 1hr, 6hr, 12hr, 24hr, 7d, custom. All stats queries use this value. Stored in Zustand and passed to API calls.

---

### Task 6.5: Trace Latency Graph (Color-Coded Bar Chart)

**Files:**
- Create: `frontend/src/features/trace/LatencyBarChart.tsx`

Canvas-based horizontal bar chart per hop, aligned with HopTable rows. Color coded: green < warning, yellow < critical, red >= critical. Min/max range lines. Packet loss as red fill overlay.

---

### Task 6.6: Timeline Graph (Pixel-Column Canvas)

**Files:**
- Create: `frontend/src/features/trace/TimelineCanvas.tsx`

The signature PingPlotter view. Canvas-based renderer where each pixel column = one sample. Color = latency severity. Features: horizontal scroll/pan, zoom, click-to-set-focus-period, red triangle comment markers, hover tooltips.

---

### Task 6.7-6.16: Additional Frontend Tasks

- **6.7**: Summary Screen page (multi-target overview grid)
- **6.8**: Session Manager page (state machine controls)
- **6.9**: Workspace Manager (save/load/switch)
- **6.10**: Advanced Alert Builder (condition + action JSONB editor)
- **6.11**: LiveShare page (token-based real-time view)
- **6.12**: WHOIS/ASN lookup panel (right-click hop context menu)
- **6.13**: Network Discovery page (scan controls + device list)
- **6.14**: Insights panel (automated analysis cards)
- **6.15**: User Management page (admin RBAC)
- **6.16**: Settings page overhaul (display thresholds, theme, preferences)

---

## Phase 7: Frontend - Feature Pages (Detailed)

### Tasks 7.1-7.18

Each task creates one page/component:

- **7.1**: DashboardPage overhaul (summary cards, agent grid, quality indicators)
- **7.2**: TracePage three-pane layout (grid + bar chart + timeline)
- **7.3**: Hop right-click context menu (WHOIS, copy IP, add to timeline, set alert)
- **7.4**: Comment annotation overlay on TimelineCanvas
- **7.5**: Focus Period interaction (click timeline to set, blue marker)
- **7.6**: SummaryScreenPage with uniform Focus Time
- **7.7**: SessionManagerPage with state controls
- **7.8**: WorkspaceSwitcher in header
- **7.9**: AlertBuilderPage with 6 condition types
- **7.10**: AlertActionEditor with 11 action types
- **7.11**: LiveSharePage with real-time WebSocket view
- **7.12**: NetworkDiscoveryPage with scan UI
- **7.13**: InsightsPanel with severity-ranked cards
- **7.14**: UserManagementPage (admin only)
- **7.15**: ReportsPage overhaul with date range picker
- **7.16**: Export improvements (PNG screenshot, JSON)
- **7.17**: TrafficPage sparklines and history charts
- **7.18**: Mobile-optimized trace view (touch pan/zoom)

---

## Phase 8: Integration & Deployment

### Task 8.1: Frontend API Base URL Configuration

**Files:**
- Modify: `frontend/src/api/client.ts`
- Modify: `frontend/vite.config.ts`

For the two-EC2 setup, the frontend needs to call the backend through the ALB. Set `VITE_API_BASE` to empty string (same origin via ALB routing).

---

### Task 8.2: WebSocket URL Configuration

**Files:**
- Modify: `frontend/src/ws/WebSocketProvider.tsx`

Derive WebSocket URL from `window.location` so it works through ALB:
```typescript
const wsUrl = `${window.location.protocol === 'https:' ? 'wss:' : 'ws:'}//${window.location.host}/ws/live`;
```

---

### Task 8.3: Docker Compose Health Checks

Ensure backend waits for PostgreSQL, frontend build includes proper env vars.

---

### Task 8.4: Terraform Validation

```bash
cd infra && terraform validate && terraform plan
```

---

### Task 8.5: End-to-End Smoke Test

1. `./nm deploy` - Create infrastructure
2. `./nm build-deploy` - Build and deploy app
3. Access ALB URL, verify login, create agent, add target
4. `./nm stop` - Verify instances stop
5. `./nm start` - Verify everything comes back
6. `./nm teardown` - Verify clean destroy

---

### Task 8.6: SSL-Ready Configuration

**Files:**
- Create: `infra/ssl.tf` (commented out, ready to uncomment)

```hcl
# Uncomment these resources when you have a domain name

# resource "aws_acm_certificate" "main" {
#   domain_name       = var.domain_name
#   validation_method = "DNS"
# }
#
# resource "aws_lb_listener" "https" {
#   load_balancer_arn = aws_lb.main.arn
#   port              = 443
#   protocol          = "HTTPS"
#   ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
#   certificate_arn   = aws_acm_certificate.main.arn
#
#   default_action {
#     type             = "forward"
#     target_group_arn = aws_lb_target_group.frontend.arn
#   }
# }
```

---

## Execution Order Summary

```
Phase 1 (Infra)     ─────►  Can start immediately, independent
Phase 2 (DB)        ─────►  Can start immediately, independent
Phase 3 (Auth)      ─────►  Depends on Phase 2
Phase 4 (Core BE)   ─────►  Depends on Phase 2, 3
Phase 5 (Adv BE)    ─────►  Depends on Phase 4
Phase 6 (FE Overhaul) ──►  Can start after Phase 3 (needs auth)
Phase 7 (FE Pages)  ─────►  Depends on Phase 4, 6
Phase 8 (Integration) ──►  Depends on all above
```

Phases 1+2 can run in parallel. Phase 6 can start in parallel with Phase 4.
