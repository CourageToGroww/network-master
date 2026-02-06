use anyhow::Result;
use nm_common::config::ServerConfig;

pub fn load() -> Result<ServerConfig> {
    let mut config = ServerConfig::default();

    // Override from environment variables
    if let Ok(v) = std::env::var("NM_LISTEN_ADDR") {
        config.listen_addr = v;
    }
    if let Ok(v) = std::env::var("NM_LOG_LEVEL") {
        config.log_level = v;
    }
    if let Ok(v) = std::env::var("DATABASE_URL") {
        config.database_url = v;
    }
    if let Ok(v) = std::env::var("NM_DB_MAX_CONNECTIONS") {
        config.db_max_connections = v.parse().unwrap_or(20);
    }
    if let Ok(v) = std::env::var("NM_JWT_SECRET") {
        config.jwt_secret = v;
    }
    if let Ok(v) = std::env::var("NM_JWT_EXPIRY_HOURS") {
        config.jwt_expiry_hours = v.parse().unwrap_or(24);
    }

    Ok(config)
}
