use std::path::PathBuf;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::state::AppState;

/// Background task that watches the release binary for changes and auto-pushes
/// updates to all connected agents.
pub async fn run(state: AppState) {
    let binary_path = find_release_binary();
    let cargo_toml_path = find_agent_cargo_toml();

    tracing::info!(
        path = %binary_path.display(),
        "Update watcher started — monitoring release binary"
    );

    let mut last_hash: Option<String> = None;

    // Check if we already have a latest.json to seed the initial hash
    let info_path = state.update_dir.join("latest.json");
    if let Ok(data) = tokio::fs::read_to_string(&info_path).await {
        if let Ok(info) = serde_json::from_str::<serde_json::Value>(&data) {
            if let Some(hash) = info.get("sha256").and_then(|v| v.as_str()) {
                last_hash = Some(hash.to_string());
                tracing::info!(sha256 = %hash, "Seeded watcher with existing binary hash");
            }
        }
    }

    let mut interval = tokio::time::interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        // Check if binary exists
        let metadata = match tokio::fs::metadata(&binary_path).await {
            Ok(m) => m,
            Err(_) => continue, // Binary not built yet
        };

        // Read and hash the binary
        let data = match tokio::fs::read(&binary_path).await {
            Ok(d) => d,
            Err(e) => {
                tracing::debug!("Failed to read release binary: {}", e);
                continue;
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let new_hash = hex::encode(hasher.finalize());

        // Compare with last known hash
        if last_hash.as_deref() == Some(&new_hash) {
            continue; // No change
        }

        tracing::info!(
            sha256 = %new_hash,
            size = metadata.len(),
            "New release binary detected!"
        );

        // Read version from Cargo.toml
        let version = read_agent_version(&cargo_toml_path).await;

        // Copy binary to update directory
        let dest = state.update_dir.join("nm-agent.exe");
        if let Err(e) = tokio::fs::write(&dest, &data).await {
            tracing::error!("Failed to copy binary to update dir: {}", e);
            continue;
        }

        // Write latest.json
        let info = serde_json::json!({
            "version": version,
            "sha256": new_hash,
            "size_bytes": metadata.len(),
            "uploaded_at": chrono::Utc::now().to_rfc3339(),
        });
        let info_json = serde_json::to_string_pretty(&info).unwrap();
        if let Err(e) = tokio::fs::write(&info_path, &info_json).await {
            tracing::error!("Failed to write latest.json: {}", e);
            continue;
        }

        last_hash = Some(new_hash.clone());

        tracing::info!(
            version = %version,
            "Binary staged — pushing update to all connected agents"
        );

        // Push UpdateCommand to ALL connected agents
        let cmd = nm_common::protocol::UpdateCommand {
            version: version.clone(),
            download_url: "/api/v1/update/binary".to_string(),
            sha256: new_hash,
        };

        let agent_ids = state.agent_registry.online_agent_ids();
        let mut pushed = 0;

        for agent_id in &agent_ids {
            let envelope = nm_common::protocol::WsEnvelope::new(
                nm_common::protocol::WsPayload::UpdateCommand(cmd.clone()),
            );
            if state.agent_registry.send_to_agent(agent_id, envelope).await.is_ok() {
                pushed += 1;
            }
        }

        tracing::info!(
            version = %version,
            agents = pushed,
            total_online = agent_ids.len(),
            "Update pushed to agents"
        );
    }
}

/// Find the release binary path relative to the server's working directory.
fn find_release_binary() -> PathBuf {
    // Try relative to workspace root (standard dev layout)
    let candidates = [
        PathBuf::from("target/release/nm-agent.exe"),
        PathBuf::from("../target/release/nm-agent.exe"),
        PathBuf::from("../../target/release/nm-agent.exe"),
    ];

    for path in &candidates {
        if path.exists() {
            return path.clone();
        }
    }

    // Default — will be polled until it exists
    PathBuf::from("target/release/nm-agent.exe")
}

/// Find the agent's Cargo.toml to read its version.
fn find_agent_cargo_toml() -> PathBuf {
    let candidates = [
        PathBuf::from("crates/nm-agent/Cargo.toml"),
        PathBuf::from("../crates/nm-agent/Cargo.toml"),
        PathBuf::from("../../crates/nm-agent/Cargo.toml"),
    ];

    for path in &candidates {
        if path.exists() {
            return path.clone();
        }
    }

    PathBuf::from("crates/nm-agent/Cargo.toml")
}

/// Read version from the agent's Cargo.toml.
async fn read_agent_version(cargo_toml: &PathBuf) -> String {
    match tokio::fs::read_to_string(cargo_toml).await {
        Ok(content) => {
            // Simple parse: find `version = "x.y.z"`
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("version") && trimmed.contains('=') {
                    if let Some(ver) = trimmed.split('"').nth(1) {
                        return ver.to_string();
                    }
                }
            }
            "unknown".to_string()
        }
        Err(_) => "unknown".to_string(),
    }
}
