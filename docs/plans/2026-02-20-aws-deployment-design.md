# AWS Deployment Design — Network Master

Date: 2026-02-20

## Goal

Deploy Network Master (PingPlotter Pro clone) to an AWS t2.micro instance with:
- 1-command deploy (`make up`)
- Stable public IP (Elastic IP)
- Datto-like Windows agent that self-enrolls via `nm-agent.exe install --server <ip>:80`
- Infrastructure pattern matching prodactive-sm

## Approach

**Cross-compile binary locally + Docker for PostgreSQL only**

Cross-compile `nm-server` to a static musl Linux binary locally (solves the 2GB RAM Rust
compilation problem on t2.micro). Build the React frontend locally. SCP both to EC2. Run
`nm-server` as a systemd service. PostgreSQL runs as a Docker container managed by the
setup script. nginx sits in front as a reverse proxy (WebSocket-aware for agents).

This mirrors the prodactive-sm pattern exactly.

## Repository Structure

```
network-master/
  infra/
    main.tf                   # provider, VPC, subnet, IGW, route table
    ec2.tf                    # SG, IAM role, EC2 instance, Elastic IP
    variables.tf              # all input variables
    outputs.tf                # server_public_ip, ssh_command, dashboard_url
    setup.sh                  # EC2 user_data: Docker, nginx, postgres, systemd
    deploy.sh                 # 1-command: compile -> build -> scp -> restart
    terraform.tfvars.example  # template for local secrets
  Makefile                    # make up / make deploy / make destroy
```

## Infrastructure (Terraform)

### Networking (main.tf)
- VPC `10.0.0.0/16` with DNS enabled
- Public subnet `10.0.1.0/24` in `<region>a`
- Internet Gateway + route table (default route to IGW)

### Compute (ec2.tf)
- Security Group: TCP 22 (admin IP only), TCP 80 (all), TCP 8080 (all — direct agent fallback)
- IAM role: no AWS service access needed (no S3/ECR — binary delivered via SCP)
- EC2: Amazon Linux 2023, t2.micro, 20GB gp3 encrypted root
- Elastic IP attached to instance
- user_data: `templatefile("setup.sh", { ... })` with secrets injected

### Variables (variables.tf)
```
aws_region      string   default "us-east-1"
ssh_key_name    string   (no default)
admin_ip        string   CIDR for SSH, e.g. "1.2.3.4/32"
environment     string   default "prod"
instance_type   string   default "t2.micro"
jwt_secret      string   sensitive
db_password     string   sensitive
admin_password  string   sensitive
domain_name     string   default "" (IP-only if blank)
```

### Outputs (outputs.tf)
```
server_public_ip  — Elastic IP address
ssh_command       — ready-to-paste SSH command
dashboard_url     — http://<ip> or https://<domain>
agent_install_cmd — nm-agent.exe install --server <ip>:80
```

## EC2 First-Boot (setup.sh)

Runs once via user_data on instance creation:

1. `dnf update -y` + install Docker, nginx, certbot
2. Create `nm` system user, `/opt/nm/{static,data,updates}` directories
3. Start PostgreSQL container:
   ```
   docker run -d --name nm-postgres --restart always \
     -e POSTGRES_DB=network_master \
     -e POSTGRES_USER=nm_user \
     -e POSTGRES_PASSWORD=<db_password> \
     -v nm-pgdata:/var/lib/postgresql/data \
     -p 127.0.0.1:5432:5432 \
     postgres:16
   ```
4. Write `/opt/nm/.env` (permissions 600, owned by nm):
   - `DATABASE_URL`, `NM_JWT_SECRET`, `NM_LISTEN_ADDR=127.0.0.1:8080`, `NM_STATIC_DIR`, `NM_LOG_LEVEL`
5. Create `/etc/systemd/system/nm-server.service` (User=nm, EnvironmentFile, Restart=always)
6. Configure nginx:
   - Listen 80, proxy to 127.0.0.1:8080
   - WebSocket upgrade headers (for agent connections)
   - `client_max_body_size 60M` (for agent binary uploads)
7. If `domain_name` is set: run certbot for HTTPS + auto-renewal cron
8. `systemctl enable nginx nm-server` (service starts when binary is deployed)

## Deploy Script (deploy.sh)

Runs on every deploy from the developer's machine:

```
[1/4] cargo build --release --target x86_64-unknown-linux-musl -p nm-server
[2/4] npm run build  (in frontend/)
[3/4] scp binary to /opt/nm/nm-server
      scp -r frontend/dist/* to /opt/nm/static/
[4/4] ssh: sudo systemctl restart nm-server
      ssh: sudo systemctl status nm-server
```

Reads `SERVER_IP` from `terraform output -raw server_public_ip`.
Reads `SSH_KEY` from `terraform.tfvars`.

## Makefile (project root)

```makefile
up:      # terraform apply -auto-approve && ./infra/deploy.sh
deploy:  # ./infra/deploy.sh  (infra already provisioned)
destroy: # terraform destroy (with confirmation prompt)
infra:   # terraform apply -auto-approve only
ssh:     # ssh -i ~/.ssh/<key>.pem ec2-user@<ip>
```

## Agent (Datto-like)

The existing `nm-agent` is already Datto-like:
- Self-registers with the server on first install
- Installs as a Windows service (auto-starts on boot)
- WebSocket connection to server through nginx on port 80
- Reconnects automatically with exponential backoff

**User-facing install flow:**
1. Download `nm-agent.exe` from the dashboard
2. Run as Administrator: `nm-agent.exe install --server <public-ip>:80`
3. Agent appears in dashboard within seconds

**OTA updates:** Server serves agent binary at `/api/agent/download`. Agent binary stored at
`/opt/nm/updates/nm-agent.exe`, uploaded separately via SCP or through the dashboard.

## Prerequisites (developer machine)

- Terraform >= 1.5
- AWS CLI configured with credentials
- Rust with musl target: `rustup target add x86_64-unknown-linux-musl`
- `musl-tools` installed: `sudo apt install musl-tools`  (Linux) or cross-rs (Mac)
- Node.js >= 18 + npm
- SSH key pair created in AWS, `.pem` file at `~/.ssh/<key-name>.pem`

## First Deploy Sequence

```bash
cd network-master/infra
cp terraform.tfvars.example terraform.tfvars
# fill in terraform.tfvars with your values
make up    # provisions infra + deploys app
```

Output:
```
dashboard_url     = "http://54.x.x.x"
agent_install_cmd = "nm-agent.exe install --server 54.x.x.x:80"
ssh_command       = "ssh -i ~/.ssh/my-key.pem ec2-user@54.x.x.x"
```
