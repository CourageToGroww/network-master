use anyhow::Result;
use clap::{Parser, Subcommand};

mod config;
mod connection;
mod installer;
mod probe;
mod resolver;
mod scheduler;
mod system_info;
mod trace_manager;
mod traffic_monitor;
mod updater;

#[cfg(windows)]
mod service;

const INSTALL_DIR: &str = r"C:\Program Files\NetworkMaster";
const CONFIG_FILENAME: &str = "nm-agent.toml";
const SERVICE_NAME: &str = "NetworkMasterAgent";

#[derive(Parser)]
#[command(name = "nm-agent", about = "Network Master Agent")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Run in foreground mode (legacy flag, same as `run` subcommand)
    #[arg(long)]
    foreground: bool,

    /// Path to config file (for foreground/run mode)
    #[arg(long, default_value = "nm-agent.toml")]
    config: String,
}

#[derive(Subcommand)]
enum Command {
    /// Install as a Windows service and auto-register with the server
    Install {
        /// Server address (IP or hostname, optionally with port)
        #[arg(long)]
        server: String,
    },
    /// Uninstall the Windows service and remove files
    Uninstall,
    /// Run in foreground (not as a service)
    Run,
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Command::Install { server }) => {
            installer::install(&server)?;
        }
        Some(Command::Uninstall) => {
            installer::uninstall()?;
        }
        Some(Command::Run) => {
            run_foreground(args.config)?;
        }
        None if args.foreground => {
            run_foreground(args.config)?;
        }
        None => {
            // No subcommand: if --foreground was the old way, or running as service
            // Try to run as Windows service first, fall back to foreground
            #[cfg(windows)]
            {
                // Check if we're being launched by the service manager
                // by trying to dispatch. If it fails, run foreground.
                match service::run_service() {
                    Ok(()) => return Ok(()),
                    Err(_) => {
                        // Not launched by SCM — run foreground
                        run_foreground(args.config)?;
                    }
                }
            }
            #[cfg(not(windows))]
            {
                run_foreground(args.config)?;
            }
        }
    }

    Ok(())
}

fn run_foreground(config_path: String) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Try install dir config first, then local
        let cfg_path = if std::path::Path::new(&config_path).exists() {
            config_path
        } else {
            let install_cfg = format!("{}\\{}", INSTALL_DIR, CONFIG_FILENAME);
            if std::path::Path::new(&install_cfg).exists() {
                install_cfg
            } else {
                config_path
            }
        };

        let config = config::load(&cfg_path)?;

        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level)),
            )
            .init();

        tracing::info!("Network Master Agent starting in foreground mode");

        // Clean up old binary from previous update
        updater::cleanup_old_binaries();

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Handle Ctrl+C
        let ctrl_c = tokio::spawn(async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Received Ctrl+C, shutting down...");
            let _ = shutdown_tx.send(());
        });

        run_agent(config, shutdown_rx).await?;
        ctrl_c.abort();
        Ok(())
    })
}

pub async fn run_agent(
    config: nm_common::config::AgentConfig,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<()> {
    let agent_id: uuid::Uuid = config.agent_id.parse()?;

    tracing::info!(agent_id = %agent_id, server = %config.server_url, "Connecting to server");

    // Create channels for inter-task communication
    let (outgoing_tx, outgoing_rx) = tokio::sync::mpsc::channel::<nm_common::protocol::WsEnvelope>(1024);
    let (target_tx, target_rx) = tokio::sync::mpsc::channel::<scheduler::TargetCommand>(64);

    // Spawn connection manager
    let conn_config = config.clone();
    let conn_outgoing_rx = outgoing_rx;
    let conn_target_tx = target_tx;
    let conn_outgoing_tx = outgoing_tx.clone();
    let connection_task = tokio::spawn(async move {
        connection::run(conn_config, conn_outgoing_rx, conn_target_tx, conn_outgoing_tx).await;
    });

    // Spawn probe scheduler
    let sched_config = config.clone();
    let sched_outgoing_tx = outgoing_tx.clone();
    let scheduler_task = tokio::spawn(async move {
        scheduler::run(sched_config, target_rx, sched_outgoing_tx).await;
    });

    // Spawn traffic monitor
    let traffic_outgoing_tx = outgoing_tx.clone();
    let traffic_task = tokio::spawn(async move {
        traffic_monitor::run(agent_id, traffic_outgoing_tx).await;
    });

    // Wait for shutdown
    tokio::select! {
        _ = &mut shutdown_rx => {
            tracing::info!("Shutdown signal received");
        }
        _ = connection_task => {
            tracing::error!("Connection manager exited unexpectedly");
        }
        _ = scheduler_task => {
            tracing::error!("Scheduler exited unexpectedly");
        }
        _ = traffic_task => {
            tracing::error!("Traffic monitor exited unexpectedly");
        }
    }

    Ok(())
}
