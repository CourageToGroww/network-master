use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "nm-cli", about = "Network Master CLI")]
struct Cli {
    /// Server URL
    #[arg(long, default_value = "http://localhost:8080", env = "NM_SERVER_URL")]
    server: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage agents
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Manage targets
    Target {
        #[command(subcommand)]
        action: TargetAction,
    },
    /// Show server status
    Status,
}

#[derive(Subcommand)]
enum AgentAction {
    /// List all agents
    List,
    /// Register a new agent
    Register {
        /// Agent name
        name: String,
    },
    /// Remove an agent
    Remove {
        /// Agent ID
        id: String,
    },
}

#[derive(Subcommand)]
enum TargetAction {
    /// List targets for an agent
    List {
        /// Agent ID
        agent_id: String,
    },
    /// Add a target to an agent
    Add {
        /// Agent ID
        agent_id: String,
        /// Target address (hostname or IP)
        address: String,
        /// Display name
        #[arg(long)]
        name: Option<String>,
        /// Probe method (icmp, tcp, udp)
        #[arg(long, default_value = "icmp")]
        method: String,
        /// Probe interval in ms
        #[arg(long, default_value = "2500")]
        interval: i32,
    },
    /// Remove a target
    Remove {
        /// Target ID
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    let base_url = cli.server.trim_end_matches('/');

    match cli.command {
        Commands::Status => {
            let resp: serde_json::Value = client
                .get(format!("{}/api/v1/dashboard/summary", base_url))
                .send()
                .await?
                .json()
                .await?;
            println!("Server Status:");
            println!("  Total Agents:   {}", resp["total_agents"]);
            println!("  Online Agents:  {}", resp["online_agents"]);
            println!("  Total Targets:  {}", resp["total_targets"]);
            println!("  Active Targets: {}", resp["active_targets"]);
            println!("  Active Alerts:  {}", resp["active_alerts"]);
        }

        Commands::Agent { action } => match action {
            AgentAction::List => {
                let agents: Vec<serde_json::Value> = client
                    .get(format!("{}/api/v1/agents", base_url))
                    .send()
                    .await?
                    .json()
                    .await?;

                println!("{:<38} {:<20} {:<10} {}", "ID", "Name", "Status", "Last Seen");
                println!("{}", "-".repeat(80));
                for agent in &agents {
                    println!(
                        "{:<38} {:<20} {:<10} {}",
                        agent["id"].as_str().unwrap_or(""),
                        agent["name"].as_str().unwrap_or(""),
                        if agent["is_online"].as_bool().unwrap_or(false) { "online" } else { "offline" },
                        agent["last_seen_at"].as_str().unwrap_or("never"),
                    );
                }
                println!("\n{} agents total", agents.len());
            }

            AgentAction::Register { name } => {
                #[derive(Serialize)]
                struct Req { name: String }

                let resp: serde_json::Value = client
                    .post(format!("{}/api/v1/agents", base_url))
                    .json(&Req { name })
                    .send()
                    .await?
                    .json()
                    .await?;

                println!("Agent registered successfully!");
                println!("  Agent ID: {}", resp["agent"]["id"]);
                println!("  API Key:  {}", resp["api_key"]);
                println!("\nSave the API key - it will not be shown again.");
            }

            AgentAction::Remove { id } => {
                client
                    .delete(format!("{}/api/v1/agents/{}", base_url, id))
                    .send()
                    .await?;
                println!("Agent {} removed", id);
            }
        },

        Commands::Target { action } => match action {
            TargetAction::List { agent_id } => {
                let targets: Vec<serde_json::Value> = client
                    .get(format!("{}/api/v1/agents/{}/targets", base_url, agent_id))
                    .send()
                    .await?
                    .json()
                    .await?;

                println!("{:<38} {:<30} {:<8} {:<8} {}", "ID", "Address", "Method", "Active", "Interval");
                println!("{}", "-".repeat(90));
                for target in &targets {
                    println!(
                        "{:<38} {:<30} {:<8} {:<8} {}ms",
                        target["id"].as_str().unwrap_or(""),
                        target["address"].as_str().unwrap_or(""),
                        target["probe_method"].as_str().unwrap_or(""),
                        target["is_active"].as_bool().unwrap_or(false),
                        target["interval_ms"].as_i64().unwrap_or(0),
                    );
                }
            }

            TargetAction::Add { agent_id, address, name, method, interval } => {
                #[derive(Serialize)]
                struct Req {
                    address: String,
                    display_name: Option<String>,
                    probe_method: String,
                    interval_ms: i32,
                    packet_size: i32,
                    max_hops: i32,
                }

                let resp: serde_json::Value = client
                    .post(format!("{}/api/v1/agents/{}/targets", base_url, agent_id))
                    .json(&Req {
                        address,
                        display_name: name,
                        probe_method: method,
                        interval_ms: interval,
                        packet_size: 64,
                        max_hops: 30,
                    })
                    .send()
                    .await?
                    .json()
                    .await?;

                println!("Target added: {}", resp["id"]);
            }

            TargetAction::Remove { id } => {
                client
                    .delete(format!("{}/api/v1/targets/{}", base_url, id))
                    .send()
                    .await?;
                println!("Target {} removed", id);
            }
        },
    }

    Ok(())
}
