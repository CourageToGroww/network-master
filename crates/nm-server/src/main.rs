use std::sync::Arc;

use anyhow::Result;
use axum::{Router, routing::get};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod api;
pub mod auth;
mod config;
mod db;
mod engine;
mod state;
mod ws;

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if present
    dotenvy::dotenv().ok();

    // Load config
    let config = config::load()?;

    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .json()
        .init();

    tracing::info!("Starting Network Master server");

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .connect(&config.database_url)
        .await?;

    tracing::info!("Connected to PostgreSQL");

    // Run migrations
    sqlx::migrate!("../../migrations").run(&pool).await?;
    tracing::info!("Database migrations applied");

    // Broadcast channels for real-time fan-out
    let (live_tx, _) = broadcast::channel::<nm_common::protocol::LiveTraceUpdate>(10_000);
    let (alert_tx, _) = broadcast::channel::<nm_common::protocol::AlertFiredNotification>(1_000);
    let (update_tx, _) = broadcast::channel::<nm_common::protocol::UpdateProgressReport>(100);
    let (traffic_tx, _) = broadcast::channel::<nm_common::protocol::LiveProcessTrafficUpdate>(500);
    let (agent_status_tx, _) = broadcast::channel::<nm_common::protocol::AgentOnlineStatusChange>(100);

    // Create update directory
    let update_dir = std::path::PathBuf::from("data/updates");
    tokio::fs::create_dir_all(&update_dir).await?;
    tracing::info!("Update directory ready at {:?}", update_dir);

    // Build app state
    let state = AppState {
        pool,
        live_tx,
        alert_tx,
        update_tx,
        traffic_tx,
        agent_status_tx,
        agent_registry: ws::connection_mgr::AgentRegistry::new(),
        config: Arc::new(config.clone()),
        hop_stats: Arc::new(dashmap::DashMap::new()),
        route_cache: Arc::new(dashmap::DashMap::new()),
        update_dir,
    };

    // Spawn background tasks
    let state_clone = state.clone();
    tokio::spawn(async move {
        engine::stats_aggregator::run(state_clone).await;
    });

    let state_clone = state.clone();
    tokio::spawn(async move {
        engine::update_watcher::run(state_clone).await;
    });

    // SPA static file fallback (serves frontend, returns index.html for client-side routes)
    let spa_fallback = ServeDir::new(&config.static_dir)
        .not_found_service(ServeFile::new(format!("{}/index.html", &config.static_dir)));

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .merge(api::download_router())
        .nest("/api/v1", api::router(state.clone()))
        .route("/ws/agent", get(ws::agent_handler::handle))
        .route("/ws/live", get(ws::frontend_handler::handle))
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
        .fallback_service(spa_fallback);

    // Bind and serve
    let listener = tokio::net::TcpListener::bind(&config.listen_addr).await?;
    tracing::info!("Listening on {}", config.listen_addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}
