mod domain;
mod application;
mod infrastructure;
mod config;
use axum::{routing::get, Router, Json, extract::{State, Path}};
use std::net::SocketAddr;
use crate::application::services::mpc_service::MpcService;
use crate::domain::entities::exchange::Exchange;
use crate::infrastructure::adapters::exchange_actor::ExchangeActor;
use crate::domain::services::strategies::{FastScalping, MomentumScalping, ConservativeScalping, SignalCombiner};
use tracing::{info, error, warn, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::collections::HashMap;

fn get_exchange_name(exchange: &Exchange) -> &'static str {
    match exchange {
        Exchange::Binance => "Binance",
        Exchange::Dydx => "dYdX",
        Exchange::Hyperliquid => "Hyperliquid",
        Exchange::Coinbase => "Coinbase",
        Exchange::Kraken => "Kraken",
    }
}

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

    info!("MPC Trading Server démarrage avec acteurs et stratégies...");
    info!("Échanges supportés: Binance, dYdX, Hyperliquid, Coinbase, Kraken");
    info!("Stratégies: Fast Scalping, Momentum Scalping, Conservative Scalping");

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

    // Load trading configuration and subscribe to symbols
    let config = crate::config::TradingConfig::default();
    info!("Souscription aux symboles configurés...");
    info!("Configuration chargée: {} échanges", config.symbols.len());

    for (exchange, symbols) in &config.symbols {
        info!("Souscription à {} symboles sur {}", symbols.len(), get_exchange_name(exchange));
        for symbol in symbols {
            match mpc_service.subscribe(exchange, symbol).await {
                Ok(_) => info!("✓ Souscrit à {} sur {}", symbol, get_exchange_name(exchange)),
                Err(e) => error!("✗ Échec de souscription à {} sur {}: {}", symbol, get_exchange_name(exchange), e),
            }
        }
    }

    // Wrap mpc_service in Arc for sharing
    let mpc_service = std::sync::Arc::new(mpc_service);
    let mpc_service_shutdown = mpc_service.clone();
    let mpc_service_supervision = mpc_service.clone();

    // Spawn supervision task
    tokio::spawn(async move {
        supervision_task(mpc_service_supervision).await;
    });

    // Spawn price collection and signal generation task
    let mpc_service_signals = mpc_service.clone();
    tokio::spawn(async move {
        signal_generation_task(mpc_service_signals).await;
    });

    let app = Router::new()
        .route("/", get(|| async { "MPC Trading Server with Indicators and Strategies is running!" }))
        .route("/health", get(health_check))
        .route("/prices", get(get_all_prices))
        .route("/prices/:symbol", get(get_symbol_price))
        .route("/signals", get(get_all_signals))
        .route("/signals/:symbol", get(get_symbol_signal))
        .route("/candles/:symbol", get(get_symbol_candles))
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

        info!("Vérification périodique de santé des acteurs...");
        let health = mpc_service.check_all_actors_health().await;

        let unhealthy_count = health.values().filter(|&&v| !v).count();
        if unhealthy_count > 0 {
            warn!("{} acteurs sont défaillants", unhealthy_count);
        } else {
            info!("Tous les acteurs sont opérationnels");
        }

        // Afficher le statut détaillé de chaque acteur
        for (exchange, is_healthy) in &health {
            if *is_healthy {
                info!("✓ {} : opérationnel", get_exchange_name(exchange));
            } else {
                warn!("✗ {} : défaillant", get_exchange_name(exchange));
            }
        }
    }
}

/// Background task for price collection and signal generation
async fn signal_generation_task(mpc_service: std::sync::Arc<MpcService>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

    loop {
        interval.tick().await;

        // Get all tracked symbols and normalize them
        let symbols = mpc_service.get_all_symbols().await;
        let mut normalized_symbols = std::collections::HashSet::new();
        
        for symbol in symbols {
            let normalized = crate::config::TradingConfig::normalize_symbol(&symbol);
            normalized_symbols.insert(normalized);
        }

        for normalized_symbol in normalized_symbols {
            // Get aggregated price for the normalized symbol
            if let Ok(aggregated_price) = mpc_service.get_aggregated_price(&normalized_symbol).await {
                info!("Prix agrégé pour {}: {:.2}", normalized_symbol, aggregated_price.value());
                // Update candle builder
                mpc_service.update_candle(normalized_symbol.clone(), aggregated_price).await;

                // Try to generate signal
                if let Some(signal) = mpc_service.generate_signal_for_symbol(&normalized_symbol).await {
                    info!(
                        "Signal for {}: {:?} (confidence: {:.2})",
                        normalized_symbol, signal.signal, signal.confidence
                    );
                }
            } else {
                debug!("Impossible d'obtenir le prix agrégé pour {}", normalized_symbol);
            }
        }
    }
}

/// Get aggregated prices for all symbols
async fn get_all_prices(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
) -> Json<HashMap<String, serde_json::Value>> {
    let symbols = mpc_service.get_all_symbols().await;
    let mut normalized_symbols = std::collections::HashSet::new();
    
    for symbol in symbols {
        let normalized = crate::config::TradingConfig::normalize_symbol(&symbol);
        normalized_symbols.insert(normalized);
    }
    
    let mut prices = HashMap::new();

    for normalized_symbol in normalized_symbols {
        if let Ok(price) = mpc_service.get_aggregated_price(&normalized_symbol).await {
            prices.insert(normalized_symbol, serde_json::json!({
                "price": price.value(),
                "aggregated": true
            }));
        }
    }

    Json(prices)
}

/// Get aggregated price for a specific symbol
async fn get_symbol_price(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);
    match mpc_service.get_aggregated_price(&normalized_symbol).await {
        Ok(price) => Json(serde_json::json!({
            "symbol": symbol,
            "normalized_symbol": normalized_symbol,
            "price": price.value(),
            "aggregated": true
        })),
        Err(e) => Json(serde_json::json!({
            "error": e
        })),
    }
}

/// Get signals for all symbols
async fn get_all_signals(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
) -> Json<HashMap<String, serde_json::Value>> {
    let symbols = mpc_service.get_all_symbols().await;
    let mut normalized_symbols = std::collections::HashSet::new();
    
    for symbol in symbols {
        let normalized = crate::config::TradingConfig::normalize_symbol(&symbol);
        normalized_symbols.insert(normalized);
    }
    
    let mut signals = HashMap::new();

    for normalized_symbol in normalized_symbols {
        if let Some(signal) = mpc_service.generate_signal_for_symbol(&normalized_symbol).await {
            signals.insert(normalized_symbol, serde_json::json!({
                "signal": format!("{:?}", signal.signal),
                "confidence": signal.confidence
            }));
        }
    }

    Json(signals)
}

/// Get signal for a specific symbol
async fn get_symbol_signal(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);
    match mpc_service.generate_signal_for_symbol(&normalized_symbol).await {
        Some(signal) => Json(serde_json::json!({
            "symbol": symbol,
            "normalized_symbol": normalized_symbol,
            "signal": format!("{:?}", signal.signal),
            "confidence": signal.confidence
        })),
        None => Json(serde_json::json!({
            "error": "Not enough data to generate signal"
        })),
    }
}

/// Get candles for a specific symbol
async fn get_symbol_candles(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);
    let candles = mpc_service.get_candles(&normalized_symbol).await;
    let candle_data: Vec<serde_json::Value> = candles.iter().map(|c| {
        serde_json::json!({
            "open": c.open.value(),
            "high": c.high.value(),
            "low": c.low.value(),
            "close": c.close.value(),
            "volume": c.volume
        })
    }).collect();

    Json(serde_json::json!({
        "symbol": symbol,
        "normalized_symbol": normalized_symbol,
        "candles": candle_data,
        "count": candles.len()
    }))
}
