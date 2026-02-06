use anyhow::Result;
use clap::Parser;

mod config;
mod connection;
mod probe;
mod resolver;
mod scheduler;
mod system_info;
mod trace_manager;

#[cfg(windows)]
mod service;

#[derive(Parser)]
#[command(name = "nm-agent", about = "Network Master Windows Agent")]
struct Args {
    /// Run in foreground mode (not as a Windows service)
    #[arg(long)]
    foreground: bool,

    /// Path to config file
    #[arg(long, default_value = "nm-agent.toml")]
    config: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.foreground {
        run_foreground(args.config)
    } else {
        #[cfg(windows)]
        {
            service::run_service()?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            eprintln!("Windows service mode is only available on Windows. Use --foreground.");
            run_foreground(args.config)
        }
    }
}

fn run_foreground(config_path: String) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let config = config::load(&config_path)?;

        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&config.log_level)),
            )
            .init();

        tracing::info!("Network Master Agent starting in foreground mode");

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
    let (target_tx, target_rx) = tokio::sync::mpsc::channel::<nm_common::protocol::TargetConfig>(64);

    // Spawn connection manager
    let conn_config = config.clone();
    let conn_outgoing_rx = outgoing_rx;
    let conn_target_tx = target_tx;
    let connection_task = tokio::spawn(async move {
        connection::run(conn_config, conn_outgoing_rx, conn_target_tx).await;
    });

    // Spawn probe scheduler
    let sched_config = config.clone();
    let sched_outgoing_tx = outgoing_tx.clone();
    let scheduler_task = tokio::spawn(async move {
        scheduler::run(sched_config, target_rx, sched_outgoing_tx).await;
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
    }

    Ok(())
}
