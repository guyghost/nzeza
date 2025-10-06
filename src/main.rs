mod domain;
mod application;
mod infrastructure;
mod presentation;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use crate::application::services::mpc_service::MpcService;
use crate::domain::entities::exchange::Exchange;
use crate::infrastructure::adapters::exchange_actor::ExchangeActor;

#[tokio::main]
async fn main() {
    println!("MPC Server starting with actor-like tasks for all exchanges...");

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

    let app = Router::new()
        .route("/", get(|| async { "MPC Server with All Exchanges is running!" }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
