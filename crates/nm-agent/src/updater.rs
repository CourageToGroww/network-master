use std::path::PathBuf;

use nm_common::protocol::{
    UpdateCommand, UpdateProgressReport, UpdateStatus, WsEnvelope, WsPayload,
};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use uuid::Uuid;

/// Perform a self-update: download, verify, swap binary, and restart.
pub async fn perform_update(
    cmd: UpdateCommand,
    server_base_url: String,
    agent_id: Uuid,
    outgoing_tx: mpsc::Sender<WsEnvelope>,
) {
    tracing::info!(version = %cmd.version, "Starting OTA update");

    if let Err(e) = do_update(&cmd, &server_base_url, agent_id, &outgoing_tx).await {
        tracing::error!("Update failed: {}", e);
        send_progress(&outgoing_tx, agent_id, UpdateStatus::Failed, 0, Some(e.to_string())).await;
    }
}

async fn do_update(
    cmd: &UpdateCommand,
    server_base_url: &str,
    agent_id: Uuid,
    outgoing_tx: &mpsc::Sender<WsEnvelope>,
) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;
    let exe_dir = current_exe.parent().unwrap_or_else(|| std::path::Path::new("."));
    let new_path = exe_dir.join("nm-agent.exe.new");
    let old_path = exe_dir.join("nm-agent.exe.old");

    // --- Download ---
    send_progress(outgoing_tx, agent_id, UpdateStatus::Downloading, 0, None).await;

    // Build download URL from the server's base URL
    let download_url = build_download_url(server_base_url, &cmd.download_url);
    tracing::info!(url = %download_url, "Downloading update binary");

    let client = reqwest::Client::new();
    let resp = client.get(&download_url).send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("Download failed with status {}", resp.status());
    }

    let bytes = resp.bytes().await?;
    tokio::fs::write(&new_path, &bytes).await?;

    tracing::info!(size = bytes.len(), "Download complete");
    send_progress(outgoing_tx, agent_id, UpdateStatus::Downloading, 70, None).await;

    // --- Verify SHA256 ---
    send_progress(outgoing_tx, agent_id, UpdateStatus::Verifying, 80, None).await;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let computed_hash = hex::encode(hasher.finalize());

    if computed_hash != cmd.sha256 {
        // Clean up downloaded file
        let _ = tokio::fs::remove_file(&new_path).await;
        anyhow::bail!(
            "SHA256 mismatch: expected {}, got {}",
            cmd.sha256,
            computed_hash
        );
    }

    tracing::info!("SHA256 verified successfully");

    // --- Install (swap binary) ---
    send_progress(outgoing_tx, agent_id, UpdateStatus::Installing, 90, None).await;

    // Remove any leftover .old file
    let _ = tokio::fs::remove_file(&old_path).await;

    // Rename current exe → .old
    tokio::fs::rename(&current_exe, &old_path).await?;

    // Rename .new → current exe name
    tokio::fs::rename(&new_path, &current_exe).await?;

    tracing::info!("Binary swap complete");

    // --- Restart ---
    send_progress(outgoing_tx, agent_id, UpdateStatus::Restarting, 100, None).await;

    // Small delay to let the progress message get sent
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    restart_self(&current_exe)?;

    Ok(())
}

fn restart_self(exe_path: &PathBuf) -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();

    tracing::info!(exe = %exe_path.display(), args = ?&args[1..], "Restarting with new binary");

    std::process::Command::new(exe_path)
        .args(&args[1..])
        .spawn()?;

    // Exit current process
    std::process::exit(0);
}

fn build_download_url(server_base_url: &str, download_path: &str) -> String {
    // server_base_url is like "ws://host:port/ws/agent" or "wss://host:port/ws/agent"
    // We need to extract the HTTP base from it
    let base = server_base_url
        .replace("ws://", "http://")
        .replace("wss://", "https://");

    // Strip the /ws/agent path
    if let Some(idx) = base.find("/ws/") {
        format!("{}{}", &base[..idx], download_path)
    } else if let Some(idx) = base.rfind('/') {
        format!("{}{}", &base[..idx], download_path)
    } else {
        format!("{}{}", base, download_path)
    }
}

async fn send_progress(
    tx: &mpsc::Sender<WsEnvelope>,
    agent_id: Uuid,
    status: UpdateStatus,
    progress_pct: u8,
    error: Option<String>,
) {
    let report = UpdateProgressReport {
        agent_id,
        status,
        progress_pct,
        error,
    };
    let envelope = WsEnvelope::new(WsPayload::UpdateProgress(report));
    let _ = tx.send(envelope).await;
}

/// Clean up leftover .old files from a previous update.
pub fn cleanup_old_binaries() {
    if let Ok(current_exe) = std::env::current_exe() {
        let exe_dir = current_exe.parent().unwrap_or_else(|| std::path::Path::new("."));
        let old_path = exe_dir.join("nm-agent.exe.old");
        if old_path.exists() {
            match std::fs::remove_file(&old_path) {
                Ok(_) => tracing::info!("Cleaned up old binary: {:?}", old_path),
                Err(e) => tracing::warn!("Failed to clean up old binary: {}", e),
            }
        }
    }
}
