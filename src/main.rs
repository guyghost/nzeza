mod domain;
mod application;
mod infrastructure;
use axum::{routing::get, Router};
use std::net::SocketAddr;
use crate::application::services::mpc_service::MpcService;
use crate::domain::entities::exchange::Exchange;
use crate::infrastructure::adapters::exchange_actor::ExchangeActor;
use crate::domain::services::strategies::{FastScalping, MomentumScalping, ConservativeScalping, SignalCombiner};
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    let combiner = SignalCombiner::new(strategies, weights);
    mpc_service.set_signal_combiner(combiner);

    let app = Router::new()
        .route("/", get(|| async { "MPC Trading Server with Indicators and Strategies is running!" }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    // Set up graceful shutdown
    let shutdown_signal = async {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C signal");
            },
            _ = terminate => {
                info!("Received SIGTERM signal");
            },
        }
    };

    info!("Server started successfully. Press Ctrl+C to stop.");
    server.with_graceful_shutdown(shutdown_signal).await?;

    info!("Server shutting down gracefully...");
    Ok(())
}
