mod domain;
mod application;
mod infrastructure;
use axum::{routing::get, Router, Json, extract::State};
use std::net::SocketAddr;
use crate::application::services::mpc_service::MpcService;
use crate::domain::entities::exchange::Exchange;
use crate::infrastructure::adapters::exchange_actor::ExchangeActor;
use crate::domain::services::strategies::{FastScalping, MomentumScalping, ConservativeScalping, SignalCombiner};
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "nzeza=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("MPC Trading Server starting with actors and strategies...");

    // Spawn actor tasks for all exchanges
    let binance_sender = ExchangeActor::spawn(Exchange::Binance);
    let dydx_sender = ExchangeActor::spawn(Exchange::Dydx);
    let hyperliquid_sender = ExchangeActor::spawn(Exchange::Hyperliquid);
    let coinbase_sender = ExchangeActor::spawn(Exchange::Coinbase);
    let kraken_sender = ExchangeActor::spawn(Exchange::Kraken);

    // Create MPC service and add senders
    let mut mpc_service = MpcService::new();
    mpc_service.add_actor(Exchange::Binance, binance_sender);
    mpc_service.add_actor(Exchange::Dydx, dydx_sender);
    mpc_service.add_actor(Exchange::Hyperliquid, hyperliquid_sender);
    mpc_service.add_actor(Exchange::Coinbase, coinbase_sender);
    mpc_service.add_actor(Exchange::Kraken, kraken_sender);

    // Set up signal combiner with strategies
    let strategies: Vec<Box<dyn crate::domain::services::strategies::Strategy + Send + Sync>> = vec![
        Box::new(FastScalping::new()),
        Box::new(MomentumScalping::new()),
        Box::new(ConservativeScalping::new()),
    ];
    let weights = vec![0.4, 0.4, 0.2]; // Weighted combination
    let combiner = SignalCombiner::new(strategies, weights)
        .expect("Failed to create signal combiner with valid strategies and weights");
    mpc_service.set_signal_combiner(combiner);

    // Wrap mpc_service in Arc for sharing
    let mpc_service = std::sync::Arc::new(mpc_service);
    let mpc_service_shutdown = mpc_service.clone();
    let mpc_service_supervision = mpc_service.clone();

    // Spawn supervision task
    tokio::spawn(async move {
        supervision_task(mpc_service_supervision).await;
    });

    let app = Router::new()
        .route("/", get(|| async { "MPC Trading Server with Indicators and Strategies is running!" }))
        .route("/health", get(health_check))
        .with_state(mpc_service.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    // Set up graceful shutdown with actor shutdown
    let shutdown_signal = async move {
        let ctrl_c = async {
            match tokio::signal::ctrl_c().await {
                Ok(()) => info!("Received Ctrl+C signal"),
                Err(e) => error!("Failed to install Ctrl+C handler: {}", e),
            }
        };

        #[cfg(unix)]
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut sig) => {
                    sig.recv().await;
                    info!("Received SIGTERM signal");
                }
                Err(e) => error!("Failed to install SIGTERM handler: {}", e),
            }
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
    };

    info!("Server started successfully. Press Ctrl+C to stop.");
    server.with_graceful_shutdown(shutdown_signal).await?;

    info!("Server shutting down gracefully...");

    // Shutdown all actors
    mpc_service_shutdown.shutdown().await;

    info!("Shutdown complete");
    Ok(())
}

/// Health check endpoint
async fn health_check(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
) -> Json<HashMap<String, serde_json::Value>> {
    let health = mpc_service.check_all_actors_health().await;

    let mut response = HashMap::new();
    response.insert("status".to_string(), serde_json::json!("running"));

    let actors_health: HashMap<String, bool> = health
        .iter()
        .map(|(exchange, is_healthy)| (format!("{:?}", exchange), *is_healthy))
        .collect();

    response.insert("actors".to_string(), serde_json::json!(actors_health));

    let all_healthy = health.values().all(|&v| v);
    response.insert("all_healthy".to_string(), serde_json::json!(all_healthy));

    Json(response)
}

/// Background supervision task that periodically checks actor health
async fn supervision_task(mpc_service: std::sync::Arc<MpcService>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        interval.tick().await;

        info!("Running periodic health check...");
        let health = mpc_service.check_all_actors_health().await;

        let unhealthy_count = health.values().filter(|&&v| !v).count();
        if unhealthy_count > 0 {
            warn!("{} actors are unhealthy", unhealthy_count);
        } else {
            info!("All actors are healthy");
        }
    }
}
