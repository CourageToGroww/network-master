use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::PathBuf;

use crate::{INSTALL_DIR, CONFIG_FILENAME, SERVICE_NAME};

#[derive(Deserialize)]
struct RegistrationResponse {
    agent: AgentInfo,
    api_key: String,
}

#[derive(Deserialize)]
struct AgentInfo {
    id: String,
}

/// Install the agent: copy binary, register with server, write config, create+start service.
pub fn install(server: &str) -> Result<()> {
    println!("Network Master Agent Installer");
    println!("==============================");

    // Normalize server address
    let server_addr = normalize_server_addr(server);
    let api_base = format!("http://{}/api/v1", server_addr);
    let ws_url = format!("ws://{}/ws/agent", server_addr);

    // 1. Create install directory
    println!("[1/5] Creating install directory...");
    let install_dir = PathBuf::from(INSTALL_DIR);
    std::fs::create_dir_all(&install_dir)
        .context("Failed to create install directory. Run as Administrator.")?;

    // 2. Copy binary to install dir
    println!("[2/5] Copying agent binary...");
    let current_exe = std::env::current_exe()?;
    let dest_exe = install_dir.join("nm-agent.exe");
    // Don't copy over ourselves if already running from install dir
    if current_exe != dest_exe {
        std::fs::copy(&current_exe, &dest_exe)
            .context("Failed to copy binary to install directory")?;
    }

    // 3. Register with server
    println!("[3/5] Registering with server at {}...", server_addr);
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let (agent_id, api_key) = register_with_server(&api_base, &hostname)?;
    println!("       Registered as: {} ({})", hostname, &agent_id[..8]);

    // 4. Write config
    println!("[4/5] Writing configuration...");
    let config_path = install_dir.join(CONFIG_FILENAME);
    let config_content = format!(
        r#"# Network Master Agent Configuration (auto-generated)
server_url = "{ws_url}"
agent_id = "{agent_id}"
api_key = "{api_key}"
log_level = "info"
default_timeout_ms = 2000
reconnect_max_delay_secs = 60
"#
    );
    std::fs::write(&config_path, config_content)
        .context("Failed to write config file")?;

    // 5. Install and start Windows service
    println!("[5/5] Installing Windows service...");
    install_windows_service(&dest_exe)?;

    println!();
    println!("Installation complete!");
    println!("  Service name: {}", SERVICE_NAME);
    println!("  Install dir:  {}", INSTALL_DIR);
    println!("  Server:       {}", server_addr);
    println!();
    println!("The agent is now running as a background service.");
    println!("Manage it with: sc stop/start {}", SERVICE_NAME);

    Ok(())
}

/// Uninstall: stop service, remove service, delete files.
pub fn uninstall() -> Result<()> {
    println!("Network Master Agent Uninstaller");
    println!("================================");

    // 1. Stop and remove service
    println!("[1/2] Removing Windows service...");
    remove_windows_service()?;

    // 2. Remove install directory
    println!("[2/2] Removing files...");
    let install_dir = PathBuf::from(INSTALL_DIR);
    if install_dir.exists() {
        // Give the service a moment to fully stop
        std::thread::sleep(std::time::Duration::from_secs(2));
        match std::fs::remove_dir_all(&install_dir) {
            Ok(_) => println!("       Removed {}", INSTALL_DIR),
            Err(e) => println!("       Warning: Could not fully remove directory: {}", e),
        }
    }

    println!();
    println!("Uninstall complete.");

    Ok(())
}

fn normalize_server_addr(server: &str) -> String {
    let addr = server
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_start_matches("ws://")
        .trim_start_matches("wss://")
        .trim_end_matches('/')
        .to_string();

    // Add default port if not specified
    if !addr.contains(':') {
        format!("{}:8080", addr)
    } else {
        addr
    }
}

fn register_with_server(api_base: &str, hostname: &str) -> Result<(String, String)> {
    let url = format!("{}/agents", api_base);
    let body = serde_json::json!({ "name": hostname });

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .context("Failed to connect to server. Is it running?")?;

    if !resp.status().is_success() {
        bail!(
            "Server returned error {}: {}",
            resp.status(),
            resp.text().unwrap_or_default()
        );
    }

    let reg: RegistrationResponse = resp.json().context("Invalid response from server")?;

    Ok((reg.agent.id, reg.api_key))
}

#[cfg(windows)]
fn install_windows_service(exe_path: &PathBuf) -> Result<()> {
    use std::ffi::OsString;
    use windows_service::{
        service::{
            ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType,
        },
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)
            .context("Failed to open Service Manager. Run as Administrator.")?;

    // Check if service already exists
    if let Ok(service) = manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS) {
        // Stop it if running
        let _ = service.stop();
        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = service.delete();
        std::thread::sleep(std::time::Duration::from_secs(1));
        drop(service);
    }

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from("Network Master Agent"),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe_path.clone(),
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };

    let service = manager
        .create_service(&service_info, ServiceAccess::START | ServiceAccess::QUERY_STATUS)
        .context("Failed to create service")?;

    // Set service description
    let _ = service.set_description("Network Master monitoring agent - collects network trace data");

    // Start it
    service
        .start::<String>(&[])
        .context("Failed to start service")?;

    println!("       Service installed and started.");
    Ok(())
}

#[cfg(not(windows))]
fn install_windows_service(_exe_path: &PathBuf) -> Result<()> {
    println!("       Windows service installation skipped (not on Windows).");
    println!("       Run with: nm-agent run");
    Ok(())
}

#[cfg(windows)]
fn remove_windows_service() -> Result<()> {
    use windows_service::{
        service::ServiceAccess,
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    let manager =
        ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
            .context("Failed to open Service Manager. Run as Administrator.")?;

    match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(service) => {
            let _ = service.stop();
            std::thread::sleep(std::time::Duration::from_secs(2));
            service.delete().context("Failed to delete service")?;
            println!("       Service removed.");
        }
        Err(_) => {
            println!("       Service not found (already removed).");
        }
    }

    Ok(())
}

#[cfg(not(windows))]
fn remove_windows_service() -> Result<()> {
    println!("       Not on Windows, nothing to remove.");
    Ok(())
}
