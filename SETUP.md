# Network Master Setup Guide

## Overview

Network Master is a network monitoring tool (similar to PingPlotter) with three components:

- **Server** — Rust backend + React frontend, served as a single Docker container
- **Agent** — Windows service that runs on each PC you want to monitor from
- **Database** — PostgreSQL (runs alongside the server in Docker)

```
┌─────────────┐       ┌──────────────────────┐       ┌─────────────┐
│   Agent PC  │──────▶│  Server (Docker)      │◀──────│  Browser    │
│  nm-agent   │  WS   │  :8080 API + Frontend │  HTTP │  Dashboard  │
└─────────────┘       │  PostgreSQL :5432      │       └─────────────┘
                      └──────────────────────┘
```

---

## 1. Deploy the Server

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/) and Docker Compose installed on the host machine
- Port **8080** accessible from agent PCs and your browser

### Quick Start

```bash
git clone <repo-url> network-master
cd network-master
docker compose up --build -d
```

That's it. This starts:

| Service  | Port | Description                          |
|----------|------|--------------------------------------|
| server   | 8080 | API + WebSocket + Frontend           |
| postgres | 5432 | Database (internal, not required to expose) |

The frontend is available at **http://\<server-ip\>:8080**.

### Verify It's Running

```bash
# Check containers are up
docker compose ps

# Check server health
curl http://localhost:8080/health
# Should return: OK
```

### Configuration

Set environment variables in a `.env` file next to `docker-compose.yml` or pass them directly:

| Variable               | Default                          | Description                    |
|------------------------|----------------------------------|--------------------------------|
| `DATABASE_URL`         | `postgresql://nm_user:nm_secret@postgres:5432/network_master` | PostgreSQL connection string |
| `NM_LISTEN_ADDR`       | `0.0.0.0:8080`                   | Server bind address            |
| `NM_LOG_LEVEL`         | `info`                           | Log level (debug, info, warn)  |
| `NM_JWT_SECRET`        | `change-me-in-production`        | JWT signing secret             |
| `NM_STATIC_DIR`        | `/app/static`                    | Frontend files path (Docker)   |

For production, change the JWT secret:

```bash
echo "NM_JWT_SECRET=$(openssl rand -hex 32)" > .env
docker compose up --build -d
```

### Optional: pgAdmin

pgAdmin is available but not started by default. To include it:

```bash
docker compose --profile tools up -d
```

pgAdmin will be at **http://\<server-ip\>:5050** (login: `admin@networkmaster.local` / `admin`).

### Updating

```bash
git pull
docker compose up --build -d
```

---

## 2. Install an Agent

Agents run on **Windows PCs** and connect to the server via WebSocket. Each agent monitors network paths and per-process traffic from its perspective.

### Prerequisites

- Windows 10 or later
- Administrator privileges (required for service installation)
- Network access to the server on port 8080

### Option A: Pre-Built Binary

If you already have `nm-agent.exe` (from a release or a previous build), copy it to the target PC and skip to the Install step.

### Option B: Build From Source

On a machine with [Rust](https://rustup.rs/) installed:

```bash
cargo build --release -p nm-agent
```

The binary is at `target\release\nm-agent.exe` (~7 MB). Copy it to the target PC.

### Install

Open an **elevated command prompt** (Run as Administrator) on the target PC:

```
nm-agent.exe install --server <server-ip>:8080
```

Example:

```
nm-agent.exe install --server 192.168.1.50:8080
```

This runs a 5-step process:
1. Creates `C:\Program Files\NetworkMaster\`
2. Copies the binary there
3. Registers with the server (gets a unique agent ID and API key)
4. Writes the config to `nm-agent.toml`
5. Installs and starts a Windows service (`NetworkMasterAgent`)

The agent will auto-start on boot.

### Verify

After installation, the agent should appear in the server dashboard at **http://\<server-ip\>:8080** within a few seconds.

You can also check locally:

```
sc query NetworkMasterAgent
```

### Agent File Locations

| File | Path |
|------|------|
| Binary | `C:\Program Files\NetworkMaster\nm-agent.exe` |
| Config | `C:\Program Files\NetworkMaster\nm-agent.toml` |
| Logs   | `C:\Program Files\NetworkMaster\nm-agent.log`  |

### Agent Commands

```bash
# Install as service and register with server
nm-agent.exe install --server <host>:<port>

# Run in foreground (for debugging, no service)
nm-agent.exe run

# Run with a custom config file
nm-agent.exe run --config path\to\config.toml

# Uninstall service and remove files
nm-agent.exe uninstall
```

### Managing the Service

```bash
# Stop the agent
sc stop NetworkMasterAgent

# Start the agent
sc start NetworkMasterAgent

# Check status
sc query NetworkMasterAgent
```

---

## 3. Using the Dashboard

Open **http://\<server-ip\>:8080** in your browser.

### Pages

| Page | Path | Description |
|------|------|-------------|
| Dashboard | `/` | Overview of agents, targets, alerts |
| Traffic | `/traffic` | Per-process network traffic for each agent |
| Alerts | `/alerts` | Alert rules and event history |
| Reports | `/reports` | Session history and CSV export |
| Settings | `/settings/agents` | Agent and target management |

### Adding a Target

1. Go to **Settings > Agents**
2. Select an online agent
3. Click **Add Target**
4. Enter a hostname or IP (e.g., `8.8.8.8`, `google.com`)
5. Choose probe method (ICMP, TCP, or UDP)
6. The agent will begin tracing immediately

### Viewing Traces

Click on a target from the dashboard to open the live trace view with:
- Hop-by-hop latency table
- Latency, loss, and jitter charts
- Route change detection

### Viewing Traffic

Go to the **Traffic** page, select an agent, and see real-time per-process network activity:
- Which processes are using the network
- Download/upload rates per process
- Active connections and remote endpoints

---

## 4. Network Requirements

### Ports

| Port | Direction | Protocol | Purpose |
|------|-----------|----------|---------|
| 8080 | Inbound to server | TCP | HTTP API + WebSocket + Frontend |
| 5432 | Internal only | TCP | PostgreSQL (server to DB) |

### Firewall

If the server is behind a firewall, open port 8080 for TCP inbound:

**Linux (ufw):**
```bash
sudo ufw allow 8080/tcp
```

**Windows Firewall:**
```powershell
New-NetFirewallRule -DisplayName "Network Master Server" -Direction Inbound -Port 8080 -Protocol TCP -Action Allow
```

**Cloud VM (Azure/AWS/GCP):** Add an inbound security rule for TCP port 8080 in your cloud console.

### Agents Behind NAT

Agents initiate outbound WebSocket connections to the server. They do **not** require any inbound ports. As long as the agent can reach `<server-ip>:8080` outbound, it will work.

---

## 5. Development Setup

For local development without Docker:

### Server

```bash
# Start PostgreSQL (via Docker or locally)
docker compose up postgres -d

# Run the server
cargo run -p nm-server
```

The server expects `DATABASE_URL` to be set. Create a `.env` file:

```
DATABASE_URL=postgresql://nm_user:nm_secret@localhost:5432/network_master
```

### Frontend

```bash
cd frontend
npm install
npm run dev
```

The Vite dev server runs on **http://localhost:5173** and proxies API/WebSocket requests to the server at `localhost:8080`.

### Agent

```bash
cargo run -p nm-agent -- run --config nm-agent.toml
```

---

## 6. Troubleshooting

### Agent can't connect to server

- Verify the server is running: `curl http://<server-ip>:8080/health`
- Check firewall rules on the server host
- On the agent PC, test connectivity: `curl http://<server-ip>:8080/health`
- Check agent logs: `C:\Program Files\NetworkMaster\nm-agent.log`

### Docker build fails

- Ensure Docker has enough memory (Rust compilation needs ~2 GB)
- Check that `Cargo.lock` and `frontend/package-lock.json` are committed
- Run `docker compose build --no-cache` for a clean build

### Agent shows offline in dashboard

- Check the Windows service is running: `sc query NetworkMasterAgent`
- Check network connectivity to the server
- The agent reconnects automatically with exponential backoff (up to 60s)

### Database reset

To wipe all data and start fresh:

```bash
docker compose down -v   # -v removes the pgdata volume
docker compose up --build -d
```
