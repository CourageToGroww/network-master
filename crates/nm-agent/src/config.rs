use anyhow::Result;
use nm_common::config::AgentConfig;

pub fn load(path: &str) -> Result<AgentConfig> {
    let mut config = AgentConfig::default();

    // Try to load from file
    if !std::path::Path::new(path).exists() {
        eprintln!("WARNING: Config file not found at '{}'. Using defaults (random agent_id, empty api_key). Authentication will fail.", path);
    }
    if std::path::Path::new(path).exists() {
        let content = std::fs::read_to_string(path)?;
        // Simple TOML-like parsing for key = "value" pairs
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                match key {
                    "server_url" | "url" => config.server_url = value.to_string(),
                    "agent_id" => config.agent_id = value.to_string(),
                    "api_key" => config.api_key = value.to_string(),
                    "log_level" => config.log_level = value.to_string(),
                    "log_file" => config.log_file = Some(value.to_string()),
                    "reconnect_max_delay_secs" => {
                        config.reconnect_max_delay_secs = value.parse().unwrap_or(60);
                    }
                    "default_timeout_ms" => {
                        config.default_timeout_ms = value.parse().unwrap_or(2000);
                    }
                    _ => {}
                }
            }
        }
    }

    // Override from environment
    if let Ok(v) = std::env::var("NM_SERVER_URL") {
        config.server_url = v;
    }
    if let Ok(v) = std::env::var("NM_AGENT_ID") {
        config.agent_id = v;
    }
    if let Ok(v) = std::env::var("NM_API_KEY") {
        config.api_key = v;
    }
    if let Ok(v) = std::env::var("NM_LOG_LEVEL") {
        config.log_level = v;
    }

    Ok(config)
}
