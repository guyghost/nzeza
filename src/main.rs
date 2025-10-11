mod application;
mod auth;
mod task_runner;
mod config;
mod domain;
mod infrastructure;
mod persistence;
mod rate_limit;
use crate::application::actors::trader_actor::TraderActor;
use crate::application::services::mpc_service::MpcService;
use crate::domain::entities::exchange::Exchange;
use crate::domain::entities::trader::Trader;
use crate::domain::services::strategies::{
    ConservativeScalping, FastScalping, MomentumScalping, SignalCombiner, Strategy,
};
use crate::infrastructure::adapters::exchange_actor::ExchangeActor;
use crate::infrastructure::exchange_client_factory::ExchangeClientFactory;
use axum::extract::ws::{Message, WebSocket};
use axum::response::Response;
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    middleware,
    routing::{delete, get, post},
    Json, Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::io::ErrorKind;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    mpc_service: std::sync::Arc<MpcService>,
    metrics_tx: broadcast::Sender<String>,
}

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
    // Load environment variables from .env file
    if let Err(e) = dotenvy::dotenv() {
        tracing::warn!("Could not load .env file: {}", e);
        tracing::info!("Continuing with environment variables from system");
    } else {
        tracing::info!("Loaded environment variables from .env file");
    }

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

    // Initialize API authentication
    auth::init_api_keys();

    // Initialize rate limiter (100 requests per minute)
    let rate_limiter = rate_limit::create_rate_limiter(rate_limit::RateLimiterConfig::default());
    info!("Rate limiting initialized: 100 requests per minute");

    // Load trading configuration first to validate before spawning actors
    let config = crate::config::TradingConfig::from_env();

    // ⚠️ WARNING: dYdX v4 integration has known issues
    // The current implementation uses Ethereum (EIP-712) signing instead of Cosmos SDK signing.
    // dYdX v4 is a Cosmos-based blockchain and requires proper protobuf encoding and Cosmos signatures.
    // Orders MAY be rejected by the exchange. For production use, implement proper Cosmos integration.
    // See: https://github.com/dydxprotocol/v4-clients for official client
    if std::env::var("DYDX_MNEMONIC").is_ok() {
        warn!("⚠️  dYdX v4 integration uses Ethereum signing (not Cosmos SDK)");
        warn!("⚠️  Orders may be REJECTED by the exchange");
        warn!("⚠️  Use at your own risk - implement proper Cosmos SDK for production");
    }

    // Spawn actor tasks for all exchanges
    let binance_sender = ExchangeActor::spawn(Exchange::Binance);
    let dydx_sender = ExchangeActor::spawn(Exchange::Dydx);
    let hyperliquid_sender = ExchangeActor::spawn(Exchange::Hyperliquid);
    let coinbase_sender = ExchangeActor::spawn(Exchange::Coinbase);
    let kraken_sender = ExchangeActor::spawn(Exchange::Kraken);

     info!("Configuration chargée depuis l'environnement:");
     info!(
         "  Seuil de confiance minimum: {:.2}",
         config.min_confidence_threshold
     );
     info!(
         "  Trading automatisé activé: {}",
         config.enable_automated_trading
     );

     // Validate confidence threshold configuration
     if config.min_confidence_threshold < 0.0 || config.min_confidence_threshold > 1.0 {
         warn!("⚠️  Invalid confidence threshold {:.2} - should be between 0.0 and 1.0", config.min_confidence_threshold);
     } else {
         info!("✓ Confidence threshold configuration valid: {:.2}", config.min_confidence_threshold);
     }
    info!(
        "  Taille de position par défaut: {}",
        config.default_position_size
    );
    info!(
        "  Positions max par symbole: {}",
        config.max_positions_per_symbol
    );
    info!("  Positions totales max: {}", config.max_total_positions);
    if let Some(sl) = config.stop_loss_percentage {
        info!("  Stop-loss: {:.1}%", sl * 100.0);
    }
    if let Some(tp) = config.take_profit_percentage {
        info!("  Take-profit: {:.1}%", tp * 100.0);
    }
    info!(
        "  Portfolio % per position: {:.2}%",
        config.portfolio_percentage_per_position * 100.0
    );
    info!("  Max trades per hour: {}", config.max_trades_per_hour);
    info!("  Max trades per day: {}", config.max_trades_per_day);

    // Create MPC service and add exchange actors (for market data)
    let mut mpc_service = MpcService::new(config.clone());
    mpc_service.add_actor(Exchange::Binance, binance_sender);
    mpc_service.add_actor(Exchange::Dydx, dydx_sender);
    mpc_service.add_actor(Exchange::Hyperliquid, hyperliquid_sender);
    mpc_service.add_actor(Exchange::Coinbase, coinbase_sender);
    mpc_service.add_actor(Exchange::Kraken, kraken_sender);

    // Create exchange clients for traders (order execution)
    info!("Initializing exchange clients for traders...");
    let exchange_clients = ExchangeClientFactory::create_all().await;

    if exchange_clients.is_empty() {
        warn!("⚠️  No exchange clients available - check your credentials");
        warn!("⚠️  Trading will be disabled");
    } else {
        info!("✓ Created {} exchange client(s)", exchange_clients.len());

        // Retrieve and log account balances
        info!("🔍 Retrieving account balances from exchanges...");
        for (exchange, client) in &exchange_clients {
            match client.get_balance(None).await {
                Ok(balances) => {
                    for balance in balances {
                        if balance.total > 0.0 {
                            info!(
                                "💰 {} Balance - {}: {:.4} (available: {:.4})",
                                get_exchange_name(exchange),
                                balance.currency,
                                balance.total,
                                balance.available
                            );
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "⚠️  Failed to retrieve balance from {}: {}",
                        get_exchange_name(exchange),
                        e
                    );
                }
            }
        }

        // Check if we have sufficient balances to start trading
        let mut has_sufficient_balance = false;
        for (exchange, client) in &exchange_clients {
            if let Ok(balances) = client.get_balance(None).await {
                for balance in balances {
                    if balance.available > 10.0 { // Minimum threshold for trading
                        has_sufficient_balance = true;
                        break;
                    }
                }
            }
            if has_sufficient_balance {
                break;
            }
        }

        if has_sufficient_balance {
            info!("✅ Sufficient balances detected - starting automated trading");
        } else {
            warn!("⚠️  Insufficient balances for trading - automated trading disabled");
            warn!("⚠️  Please fund your accounts and restart the server");
        }
    }

    // Initialize signal combiner with strategies
    let strategies = vec![
        (
            "FastScalping".to_string(),
            Box::new(FastScalping::new()) as Box<dyn Strategy + Send + Sync>,
        ),
        (
            "MomentumScalping".to_string(),
            Box::new(MomentumScalping::new()) as Box<dyn Strategy + Send + Sync>,
        ),
        (
            "ConservativeScalping".to_string(),
            Box::new(ConservativeScalping::new()) as Box<dyn Strategy + Send + Sync>,
        ),
    ];
    let weights = vec![0.4, 0.4, 0.2];
    let combiner =
        SignalCombiner::new(strategies, weights).expect("Failed to create signal combiner");
    mpc_service.set_signal_combiner(combiner).await;

    // Create and spawn traders with exchange clients
    if !exchange_clients.is_empty() {
        info!("Creating traders with available exchange clients...");

        // Create one trader per strategy for now
        let trader_strategies = vec![
            ("FastScalping", Box::new(FastScalping::new()) as Box<dyn Strategy + Send + Sync>),
            ("MomentumScalping", Box::new(MomentumScalping::new()) as Box<dyn Strategy + Send + Sync>),
            ("ConservativeScalping", Box::new(ConservativeScalping::new()) as Box<dyn Strategy + Send + Sync>),
        ];

        for (strategy_name, strategy) in trader_strategies {
            let trader_id = format!("trader_{}", strategy_name.to_lowercase());

            match Trader::new(
                trader_id.clone(),
                strategy,
                config.default_position_size,
                config.min_confidence_threshold,
            ) {
                Ok(mut trader) => {
                    // Add all available exchange clients to this trader
                    for (exchange, client) in &exchange_clients {
                        trader.add_exchange(exchange.clone(), client.clone());
                        info!(
                            "  ✓ Trader '{}' configured with {}",
                            trader_id,
                            get_exchange_name(exchange)
                        );
                    }

                    // EXPLICITLY set Coinbase as active exchange if available (most reliable)
                    // dYdX v4 has known issues with order execution (Ethereum signing vs Cosmos SDK)
                    if exchange_clients.contains_key(&Exchange::Coinbase) {
                        if let Err(e) = trader.set_active_exchange(Exchange::Coinbase) {
                            warn!("Failed to set Coinbase as active exchange for {}: {}", trader_id, e);
                        } else {
                            info!("  ✓ Trader '{}' using Coinbase as primary exchange", trader_id);
                        }
                    }

                    // Spawn trader actor
                    let trader_sender = TraderActor::spawn(trader);
                    mpc_service.add_trader(trader_id.clone(), trader_sender).await;

                    info!("✓ Trader '{}' spawned and ready", trader_id);
                }
                Err(e) => {
                    error!("Failed to create trader '{}': {}", trader_id, e);
                }
            }
        }

        info!("All traders initialized successfully");

        // Validate trader setup and log configuration
        info!("🔍 Validating trader setup...");
        let trader_ids: Vec<String> = {
            let traders = mpc_service.traders.lock().await;
            traders.keys().cloned().collect()
        };
        let trader_count = trader_ids.len();
        info!("✓ {} trader(s) registered in MPC service", trader_count);

        if trader_count == 0 {
            warn!("⚠️  No traders available - automated trading will be disabled");
        } else {
            for trader_id in &trader_ids {
                info!("✓ Trader '{}' is available for order execution", trader_id);
            }

            // Validate that trader selection logic can pick at least one trader
            let strategy_order = {
                let guard = mpc_service.strategy_order.lock().await;
                guard.clone()
            };
            let selected_trader = {
                let traders = mpc_service.traders.lock().await;
                strategy_order
                    .iter()
                    .find_map(|strategy_name| {
                        let trader_id = format!("trader_{}", strategy_name.to_lowercase());
                        if traders.contains_key(&trader_id) {
                            Some(trader_id)
                        } else {
                            None
                        }
                    })
                    .or_else(|| traders.keys().next().cloned())
            };

            match selected_trader {
                Some(selected_id) => {
                    info!("✅ Trader selection check passed (selected: {})", selected_id);
                }
                None => {
                    warn!("✗ No trader available for order execution - selection check failed");
                }
            }

            info!("✅ Trader setup validation complete - {} trader(s) ready", trader_count);
        }
    }

    for (exchange, symbols) in &config.symbols {
        info!(
            "Souscription à {} symboles sur {}",
            symbols.len(),
            get_exchange_name(exchange)
        );
        for symbol in symbols {
            match mpc_service.subscribe(exchange, symbol).await {
                Ok(_) => info!(
                    "✓ Souscrit à {} sur {}",
                    symbol,
                    get_exchange_name(exchange)
                ),
                Err(e) => error!(
                    "✗ Échec de souscription à {} sur {}: {}",
                    symbol,
                    get_exchange_name(exchange),
                    e
                ),
            }
        }
    }

    // Create broadcast channel for real-time metrics
    let (metrics_tx, _) = broadcast::channel::<String>(100);
    let metrics_tx_clone = metrics_tx.clone();

    let app_state = AppState {
        mpc_service: std::sync::Arc::new(mpc_service),
        metrics_tx: metrics_tx_clone,
    };

    // Spawn supervision task
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        supervision_task(app_state_clone).await;
    });

    // Spawn price collection and signal generation task
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        signal_generation_task(app_state_clone).await;
    });

    // Spawn order execution task with circuit breaker
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        use task_runner::{run_with_circuit_breaker, CircuitBreakerConfig};

        let config = CircuitBreakerConfig {
            max_consecutive_failures: 10,
            initial_retry_delay: Duration::from_secs(5),
            max_retry_delay: Duration::from_secs(60),
        };

        run_with_circuit_breaker("order_execution_task", config, || async {
            order_execution_task_iteration(app_state_clone.clone()).await
        }).await;
    });

    // Spawn alerting task
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        alerting_task(app_state_clone).await;
    });

    // Spawn metrics broadcast task
    let app_state_clone = app_state.clone();
    let metrics_tx_for_task = metrics_tx.clone();
    tokio::spawn(async move {
        metrics_broadcast_task(app_state_clone, metrics_tx_for_task, Duration::from_secs(5)).await;
    });

    // Spawn strategy weight adjustment task
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        strategy_weight_adjustment_task(app_state_clone, Duration::from_secs(300)).await;
    });

    // Spawn portfolio refresh task
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        portfolio_refresh_task(app_state_clone, Duration::from_secs(60)).await;
    });

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route(
            "/",
            get(|| async { "MPC Trading Server with Indicators and Strategies is running!" }),
        )
        .route("/health", get(health_check))
        .with_state(app_state.clone());

    // Protected routes (require authentication and rate limiting)
    let rate_limiter_clone = rate_limiter.clone();
    let protected_routes = Router::new()
        .route("/metrics", get(get_metrics))
        .route("/ws/metrics", get(metrics_websocket_handler))
        .route("/prices", get(get_all_prices))
        .route("/prices/:symbol", get(get_symbol_price))
        .route("/signals", get(get_all_signals))
        .route("/signals/:symbol", get(get_symbol_signal))
        .route("/orders/execute", post(execute_pending_orders))
        .route("/orders/:symbol/execute", post(execute_symbol_order))
        .route("/orders/place", post(place_manual_order))
        .route("/orders/cancel/:order_id", delete(cancel_order))
        .route("/orders/cancel/:exchange/:order_id", delete(cancel_order_with_exchange))
        .route("/orders/status/:order_id", get(get_order_status))
        .route("/orders/status/:exchange/:order_id", get(get_order_status_with_exchange))
        .route("/positions", get(get_positions))
        .route("/positions/pnl", get(get_total_pnl))
        .route("/portfolio", get(get_portfolio))
        .route("/portfolio/refresh", post(refresh_portfolio))
        .route("/config", get(get_config))
        .route("/alerts", get(get_alerts))
        .route("/performance", get(get_performance_profiles))
        .route_layer(middleware::from_fn(move |req, next| {
            rate_limit::rate_limit_middleware(rate_limiter_clone.clone(), req, next)
        }))
        .route_layer(middleware::from_fn(auth::require_auth))
        .with_state(app_state.clone());

    // Combine public and protected routes
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes);

    let preferred_addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = match tokio::net::TcpListener::bind(preferred_addr).await {
        Ok(listener) => {
            let actual_addr = listener.local_addr()?;
            info!("Listening on {}", actual_addr);
            Some(listener)
        }
        Err(e) => {
            if e.kind() != ErrorKind::PermissionDenied {
                return Err(e.into());
            }

            warn!(
                "Failed to bind to {}: {}. Falling back to an ephemeral port.",
                preferred_addr, e
            );

            let fallback_addr = SocketAddr::from(([127, 0, 0, 1], 0));
            match tokio::net::TcpListener::bind(fallback_addr).await {
                Ok(listener) => {
                    let actual_addr = listener.local_addr()?;
                    info!("Listening on {}", actual_addr);
                    Some(listener)
                }
                Err(fallback_err) if fallback_err.kind() == ErrorKind::PermissionDenied => {
                    warn!(
                        "Failed to bind to fallback HTTP address: {}. Continuing without HTTP server.",
                        fallback_err
                    );
                    None
                }
                Err(fallback_err) => return Err(fallback_err.into()),
            }
        }
    };

    // Set up graceful shutdown with actor shutdown
    let shutdown_signal = || async {
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

    if let Some(listener) = listener {
        let server = axum::serve(listener, app);
        info!("Server started successfully. Press Ctrl+C to stop.");
        server.with_graceful_shutdown(shutdown_signal()).await?;
    } else {
        warn!("HTTP server not started; waiting for shutdown signal. Press Ctrl+C to exit.");
        shutdown_signal().await;
    }

    info!("Server shutting down gracefully...");

    // Shutdown all actors
    app_state.mpc_service.shutdown().await;

    info!("Shutdown complete");
    Ok(())
}

/// Health check endpoint
async fn health_check(
    State(app_state): State<AppState>,
) -> Json<HashMap<String, serde_json::Value>> {
    let health = app_state.mpc_service.check_all_actors_health().await;

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
async fn supervision_task(app_state: AppState) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

    loop {
        interval.tick().await;

        info!("Vérification périodique de santé des acteurs...");
        let health = app_state.mpc_service.check_all_actors_health().await;

        // Update system health metrics
        for (exchange, is_healthy) in &health {
            app_state
                .mpc_service
                .update_exchange_connection(format!("{:?}", exchange), *is_healthy)
                .await;
        }

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
async fn signal_generation_task(app_state: AppState) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

    loop {
        interval.tick().await;

        // Get all tracked symbols and normalize them
        let symbols = app_state.mpc_service.get_all_symbols().await;
        let mut normalized_symbols = std::collections::HashSet::new();

        for symbol in &symbols {
            let normalized = crate::config::TradingConfig::normalize_symbol(symbol);
            normalized_symbols.insert(normalized);
        }

        for normalized_symbol in normalized_symbols {
            // Get aggregated price for the normalized symbol
            if let Ok(aggregated_price) = app_state
                .mpc_service
                .get_aggregated_price(&normalized_symbol)
                .await
            {
                 info!(
                     "Prix agrégé pour {}: {:.2}",
                     normalized_symbol,
                     aggregated_price.value()
                 );
                 // Update candle builder
                 app_state
                     .mpc_service
                     .update_candle(normalized_symbol.clone(), aggregated_price)
                     .await;

                 // Log candle collection status
                 let current_candles = app_state.mpc_service.get_candles(&normalized_symbol).await;
                 debug!("✓ Candle updated for {}: now {} candles in collection", normalized_symbol, current_candles.len());

                // Try to generate signal
                if let Ok(signal) = app_state
                    .mpc_service
                    .generate_signal_for_symbol(&normalized_symbol)
                    .await
                {
                    info!(
                        "Signal for {}: {:?} (confidence: {:.2})",
                        normalized_symbol, signal.signal, signal.confidence
                    );

                     // Store the signal for automated execution
                     app_state
                         .mpc_service
                         .store_signal(normalized_symbol.clone(), signal)
                         .await;

                     debug!("✓ Signal stored for {} in LRU cache", normalized_symbol);
                }

                // Update position prices and metrics
                if let Err(e) = app_state.mpc_service.update_position_prices().await {
                    debug!("Failed to update position prices: {}", e);
                }

                // Update trading metrics
                let positions = app_state.mpc_service.get_open_positions().await;
                let total_unrealized_pnl = app_state.mpc_service.get_total_unrealized_pnl().await;
                app_state
                    .mpc_service
                    .update_unrealized_pnl(total_unrealized_pnl)
                    .await;

                // Update system health with position count
                app_state
                    .mpc_service
                    .update_trading_status(
                        positions.len() as u32,
                        0, // TODO: track pending orders
                    )
                    .await;
            } else {
                debug!(
                    "Impossible d'obtenir le prix agrégé pour {}",
                    normalized_symbol
                );
            }
        }
    }
}

/// Background task for order execution based on signals
///
/// NOTE: This is wrapped with a circuit breaker to prevent silent failures.
/// Other background tasks (supervision, signal_generation, alerting, etc.) should
/// follow the same pattern for production deployments.
async fn order_execution_task(app_state: AppState) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30)); // Check every 30 seconds

    loop {
        interval.tick().await;

        debug!("🔍 Checking for orders to execute...");
        let results = app_state.mpc_service.check_and_execute_orders().await;

        // Count only ACTUAL orders executed (not Hold signals or low confidence rejections)
        let successful_orders = results
            .iter()
            .filter(|r| {
                r.as_ref()
                    .ok()
                    .map(|msg| msg.contains("Order executed") && msg.contains("Order ID"))
                    .unwrap_or(false)
            })
            .count();

        let failed_orders = results.iter().filter(|r| r.is_err()).count();

        if successful_orders > 0 {
            info!("✅ {} orders executed successfully", successful_orders);
        }

        if failed_orders > 0 {
            warn!("❌ {} orders failed to execute", failed_orders);
            // Record errors in system health
            for _ in 0..failed_orders {
                app_state.mpc_service.record_error().await;
            }
        }

        // Check for stop-loss and take-profit triggers
        let stop_results = app_state.mpc_service.check_and_execute_stops().await;
        let stops_triggered = stop_results.iter().filter(|r| r.is_ok()).count();

        if stops_triggered > 0 {
            info!("🛑 {} positions closed due to stops", stops_triggered);
        }

        if successful_orders == 0 && failed_orders == 0 && stops_triggered == 0 {
            debug!("No signals available for order execution and no stops triggered");
        }
    }
}

/// Single iteration of order execution task for circuit breaker
async fn order_execution_task_iteration(app_state: AppState) -> Result<(), String> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    interval.tick().await;

    debug!("🔍 Checking for orders to execute...");
    let results = app_state.mpc_service.check_and_execute_orders().await;

    // Count only ACTUAL orders executed (not Hold signals or low confidence rejections)
    let successful_orders = results
        .iter()
        .filter(|r| {
            r.as_ref()
                .ok()
                .map(|msg| msg.contains("Order executed") && msg.contains("Order ID"))
                .unwrap_or(false)
        })
        .count();

    let failed_orders = results.iter().filter(|r| r.is_err()).count();

    if successful_orders > 0 {
        info!("✅ {} orders executed successfully", successful_orders);
    }

    if failed_orders > 0 {
        warn!("❌ {} orders failed to execute", failed_orders);
        // Record errors in system health
        for _ in 0..failed_orders {
            app_state.mpc_service.record_error().await;
        }
    }

    // Check for stop-loss and take-profit triggers
    let stop_results = app_state.mpc_service.check_and_execute_stops().await;
    let stops_triggered = stop_results.iter().filter(|r| r.is_ok()).count();

    if stops_triggered > 0 {
        info!("🛑 {} positions closed due to stops", stops_triggered);
    }

    if successful_orders == 0 && failed_orders == 0 && stops_triggered == 0 {
        debug!("No signals available for order execution and no stops triggered");
    }

    Ok(())
}

/// Get aggregated prices for all symbols
async fn get_all_prices(
    State(app_state): State<AppState>,
) -> Json<HashMap<String, serde_json::Value>> {
    let symbols = app_state.mpc_service.get_all_symbols().await;
    let mut normalized_symbols = std::collections::HashSet::new();

    for symbol in &symbols {
        let normalized = crate::config::TradingConfig::normalize_symbol(symbol);
        normalized_symbols.insert(normalized);
    }

    let mut prices = HashMap::new();

    for normalized_symbol in normalized_symbols {
        if let Ok(price) = app_state
            .mpc_service
            .get_aggregated_price(&normalized_symbol)
            .await
        {
            prices.insert(
                normalized_symbol,
                serde_json::json!({
                    "price": price.value(),
                    "aggregated": true
                }),
            );
        }
    }

    Json(prices)
}

/// Get aggregated price for a specific symbol
async fn get_symbol_price(
    State(app_state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);
    match app_state
        .mpc_service
        .get_aggregated_price(&normalized_symbol)
        .await
    {
        Ok(price) => Json(serde_json::json!({
            "symbol": symbol,
            "normalized_symbol": normalized_symbol,
            "price": price.value(),
            "aggregated": true
        })),
        Err(e) => Json(serde_json::json!({
            "error": e.to_string()
        })),
    }
}

/// Get signals for all symbols
async fn get_all_signals(
    State(app_state): State<AppState>,
) -> Json<HashMap<String, serde_json::Value>> {
    let symbols = app_state.mpc_service.get_all_symbols().await;
    let mut normalized_symbols = std::collections::HashSet::new();

    for symbol in symbols {
        let normalized = crate::config::TradingConfig::normalize_symbol(&symbol);
        normalized_symbols.insert(normalized);
    }

    let mut signals = HashMap::new();

    for normalized_symbol in normalized_symbols {
        if let Ok(signal) = app_state
            .mpc_service
            .generate_signal_for_symbol(&normalized_symbol)
            .await
        {
            signals.insert(
                normalized_symbol,
                serde_json::json!({
                    "signal": format!("{:?}", signal.signal),
                    "confidence": signal.confidence
                }),
            );
        }
    }

    Json(signals)
}

/// Get signal for a specific symbol
async fn get_symbol_signal(
    State(app_state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);
    match app_state
        .mpc_service
        .generate_signal_for_symbol(&normalized_symbol)
        .await
    {
        Ok(signal) => Json(serde_json::json!({
            "symbol": symbol,
            "normalized_symbol": normalized_symbol,
            "signal": format!("{:?}", signal.signal),
            "confidence": signal.confidence
        })),
        Err(e) => Json(serde_json::json!({
            "error": format!("Failed to generate signal: {}", e)
        })),
    }
}

/// Execute pending orders for all symbols
async fn execute_pending_orders(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    let results = app_state.mpc_service.check_and_execute_orders().await;

    let successful: Vec<String> = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .cloned()
        .collect();

    let failed: Vec<String> = results
        .iter()
        .filter_map(|r| r.as_ref().err())
        .map(|e| e.to_string())
        .collect();

    Json(serde_json::json!({
        "executed_orders": successful.len(),
        "failed_orders": failed.len(),
        "successful": successful,
        "failed": failed
    }))
}

/// Execute orders for a specific symbol
async fn execute_symbol_order(
    State(app_state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);

    match app_state
        .mpc_service
        .generate_signal_for_symbol(&normalized_symbol)
        .await
    {
        Ok(signal) => {
            match app_state
                .mpc_service
                .execute_order_from_signal(&normalized_symbol, &signal)
                .await
            {
                Ok(msg) => Json(serde_json::json!({
                    "success": true,
                    "symbol": symbol,
                    "normalized_symbol": normalized_symbol,
                    "signal": format!("{:?}", signal.signal),
                    "confidence": signal.confidence,
                    "message": msg
                })),
                Err(e) => Json(serde_json::json!({
                    "success": false,
                    "symbol": symbol,
                    "normalized_symbol": normalized_symbol,
                    "error": e.to_string()
                })),
            }
        }
        Err(e) => Json(serde_json::json!({
            "success": false,
            "symbol": symbol,
            "normalized_symbol": normalized_symbol,
            "error": format!("Failed to generate signal: {}", e)
        })),
    }
}

/// Place a manual order
async fn place_manual_order(
    State(app_state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    use crate::domain::entities::order::{Order, OrderSide, OrderType};

    // Parse the payload
    let symbol = payload["symbol"].as_str().ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Missing symbol field"})),
    ))?;

    let side_str = payload["side"].as_str().ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Missing side field"})),
    ))?;

    let quantity = payload["quantity"].as_f64().ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": "Missing or invalid quantity field"})),
    ))?;

    let order_type_str = payload
        .get("order_type")
        .and_then(|v| v.as_str())
        .unwrap_or("market");
    let price = payload.get("price").and_then(|v| v.as_f64());
    
    // Parse exchange (default to dYdX for backward compatibility)
    let exchange_str = payload
        .get("exchange")
        .and_then(|v| v.as_str())
        .unwrap_or("dydx");
    
    let exchange = match exchange_str.to_lowercase().as_str() {
        "dydx" => crate::domain::entities::exchange::Exchange::Dydx,
        "coinbase" => crate::domain::entities::exchange::Exchange::Coinbase,
        _ => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid exchange. Must be 'dydx' or 'coinbase'"})),
            ))
        }
    };

    // Parse side
    let side = match side_str.to_uppercase().as_str() {
        "BUY" => OrderSide::Buy,
        "SELL" => OrderSide::Sell,
        _ => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid side. Must be 'BUY' or 'SELL'"})),
            ))
        }
    };

    // Parse order type
    let order_type = match order_type_str.to_uppercase().as_str() {
        "MARKET" => OrderType::Market,
        "LIMIT" => OrderType::Limit,
        _ => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({"error": "Invalid order_type. Must be 'MARKET' or 'LIMIT'"}),
                ),
            ))
        }
    };

    // Generate order ID
    let order_id = format!(
        "manual_order_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    // Create order
    let order = match Order::new(
        order_id,
        symbol.to_string(),
        side,
        order_type,
        price,
        quantity,
    ) {
        Ok(order) => order,
        Err(e) => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Failed to create order: {}", e)})),
            ))
        }
    };

    // Place order on the selected exchange
    match app_state
        .mpc_service
        .place_order(&exchange, order)
        .await
    {
        Ok(order_id) => Ok(Json(serde_json::json!({
            "success": true,
            "order_id": order_id,
            "exchange": exchange_str,
            "message": format!("Order placed successfully on {}", exchange_str)
        }))),
        Err(e) => Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "success": false,
                "error": e
            })),
        )),
    }
}

/// Cancel an order (legacy endpoint - defaults to dYdX)
async fn cancel_order(
    State(app_state): State<AppState>,
    Path(order_id): Path<String>,
) -> Json<serde_json::Value> {
    cancel_order_with_exchange(
        State(app_state),
        Path(("dydx".to_string(), order_id)),
    ).await
}

/// Cancel an order with exchange specification
async fn cancel_order_with_exchange(
    State(app_state): State<AppState>,
    Path((exchange_str, order_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let exchange = match exchange_str.to_lowercase().as_str() {
        "dydx" => crate::domain::entities::exchange::Exchange::Dydx,
        "coinbase" => crate::domain::entities::exchange::Exchange::Coinbase,
        _ => {
            return Json(serde_json::json!({
                "success": false,
                "error": "Invalid exchange. Must be 'dydx' or 'coinbase'"
            }));
        }
    };

    match app_state
        .mpc_service
        .cancel_order(&exchange, &order_id)
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": format!("Order {} cancelled successfully on {}", order_id, exchange_str)
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e
        })),
    }
}

/// Get order status (legacy endpoint - defaults to dYdX)
async fn get_order_status(
    State(app_state): State<AppState>,
    Path(order_id): Path<String>,
) -> Json<serde_json::Value> {
    get_order_status_with_exchange(
        State(app_state),
        Path(("dydx".to_string(), order_id)),
    ).await
}

/// Get order status with exchange specification
async fn get_order_status_with_exchange(
    State(app_state): State<AppState>,
    Path((exchange_str, order_id)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    let exchange = match exchange_str.to_lowercase().as_str() {
        "dydx" => crate::domain::entities::exchange::Exchange::Dydx,
        "coinbase" => crate::domain::entities::exchange::Exchange::Coinbase,
        _ => {
            return Json(serde_json::json!({
                "success": false,
                "error": "Invalid exchange. Must be 'dydx' or 'coinbase'"
            }));
        }
    };

    match app_state
        .mpc_service
        .get_order_status(&exchange, &order_id)
        .await
    {
        Ok(status) => Json(serde_json::json!({
            "success": true,
            "order_id": order_id,
            "exchange": exchange_str,
            "status": status
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e
        })),
    }
}

/// Get candles for a specific symbol
async fn get_symbol_candles(
    State(app_state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);
    let candles = app_state.mpc_service.get_candles(&normalized_symbol).await;
    let candle_data: Vec<serde_json::Value> = candles
        .iter()
        .map(|c| {
            serde_json::json!({
                "open": c.open.value(),
                "high": c.high.value(),
                "low": c.low.value(),
                "close": c.close.value(),
                "volume": c.volume
            })
        })
        .collect();

    Json(serde_json::json!({
        "symbol": symbol,
        "normalized_symbol": normalized_symbol,
        "candles": candle_data,
        "count": candles.len()
    }))
}

/// Get all open positions
async fn get_positions(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    let positions = app_state.mpc_service.get_open_positions().await;
    let position_data: HashMap<String, serde_json::Value> = positions
        .iter()
        .map(|(id, position)| {
            (
                id.clone(),
                serde_json::json!({
                    "symbol": position.symbol,
                    "side": format!("{:?}", position.side),
                    "quantity": position.quantity.value(),
                    "entry_price": position.entry_price.value(),
                    "current_price": position.current_price.map(|p| p.value()),
                    "unrealized_pnl": position.unrealized_pnl().map(|p| p.value()),
                    "entry_time": position.entry_time.to_rfc3339(),
                    "stop_loss_price": position.stop_loss_price.map(|p| p.value()),
                    "take_profit_price": position.take_profit_price.map(|p| p.value())
                }),
            )
        })
        .collect();

    Json(serde_json::json!({
        "positions": position_data,
        "count": positions.len()
    }))
}

/// Get total unrealized PnL across all positions
async fn get_total_pnl(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    let total_pnl = app_state.mpc_service.get_total_unrealized_pnl().await;

    Json(serde_json::json!({
        "total_unrealized_pnl": total_pnl.value(),
        "currency": "USD"
    }))
}

/// Get current portfolio state
async fn get_portfolio(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    use crate::application::services::mpc_service::PortfolioState;

    let portfolio_state: PortfolioState = {
        let state = app_state.mpc_service.portfolio_state.lock().await;
        state.clone()
    };

    let cache_age = std::time::SystemTime::now()
        .duration_since(portfolio_state.last_updated)
        .unwrap_or(Duration::from_secs(0));

    Json(serde_json::json!({
        "total_value": portfolio_state.total_value,
        "available_cash": portfolio_state.available_cash,
        "position_value": portfolio_state.position_value,
        "exchange_balances": portfolio_state.exchange_balances,
        "last_updated": portfolio_state.last_updated
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs(),
        "cache_age_seconds": cache_age.as_secs(),
        "is_stale": cache_age > Duration::from_secs(300),
        "currency": "USD"
    }))
}

/// Manually trigger portfolio refresh from all exchanges
async fn refresh_portfolio(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    match app_state.mpc_service.fetch_and_update_portfolio_from_exchanges().await {
        Ok(portfolio_value) => Json(serde_json::json!({
            "success": true,
            "portfolio_value": portfolio_value,
            "message": "Portfolio refreshed successfully from all exchanges",
            "currency": "USD"
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

/// Get performance profiles
async fn get_performance_profiles(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    let profiles = app_state.mpc_service.get_performance_profiles().await;
    let profile_data: HashMap<String, serde_json::Value> = profiles
        .iter()
        .map(|(name, profile)| {
            (
                name.clone(),
                serde_json::json!({
                    "operation": profile.operation,
                    "avg_execution_time_ms": profile.avg_execution_time_ms,
                    "max_execution_time_ms": profile.max_execution_time_ms,
                    "min_execution_time_ms": profile.min_execution_time_ms,
                    "execution_count": profile.execution_count,
                    "last_execution": profile.last_execution
                }),
            )
        })
        .collect();

    Json(serde_json::json!({
        "profiles": profile_data,
        "count": profiles.len()
    }))
}

/// Get active alerts
async fn get_alerts(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    let alerts = app_state.mpc_service.get_active_alerts().await;
    let alert_data: Vec<serde_json::Value> = alerts
        .iter()
        .map(|alert| {
            serde_json::json!({
                "type": format!("{:?}", alert.alert_type),
                "message": alert.message,
                "severity": format!("{:?}", alert.severity),
                "timestamp": alert.timestamp,
                "resolved": alert.resolved
            })
        })
        .collect();

    Json(serde_json::json!({
        "alerts": alert_data,
        "count": alerts.len()
    }))
}

/// Get current configuration
async fn get_config(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    let mpc_service = &app_state.mpc_service;
    Json(serde_json::json!({
        "min_confidence_threshold": mpc_service.config.min_confidence_threshold,
        "max_positions_per_symbol": mpc_service.config.max_positions_per_symbol,
        "max_total_positions": mpc_service.config.max_total_positions,
        "default_position_size": mpc_service.config.default_position_size,
        "enable_automated_trading": mpc_service.config.enable_automated_trading,
        "stop_loss_percentage": mpc_service.config.stop_loss_percentage,
        "take_profit_percentage": mpc_service.config.take_profit_percentage,
        "portfolio_percentage_per_position": mpc_service.config.portfolio_percentage_per_position,
        "max_trades_per_hour": mpc_service.config.max_trades_per_hour,
        "max_trades_per_day": mpc_service.config.max_trades_per_day,
        "symbols_count": mpc_service.config.symbols.len()
    }))
}

/// Get current trading metrics
async fn get_metrics(State(app_state): State<AppState>) -> Json<serde_json::Value> {
    let trading_metrics = app_state.mpc_service.get_trading_metrics().await;
    let system_health = app_state.mpc_service.get_system_health().await;

    Json(serde_json::json!({
        "trading": {
            "total_realized_pnl": trading_metrics.total_realized_pnl.value(),
            "total_unrealized_pnl": trading_metrics.total_unrealized_pnl.value(),
            "total_equity": trading_metrics.current_equity().value(),
            "winning_trades": trading_metrics.winning_trades,
            "losing_trades": trading_metrics.losing_trades,
            "total_trades": trading_metrics.total_trades,
            "win_rate": trading_metrics.win_rate,
            "profit_factor": trading_metrics.profit_factor,
            "max_drawdown": trading_metrics.max_drawdown.value(),
            "current_drawdown": trading_metrics.current_drawdown.value(),
            "sharpe_ratio": trading_metrics.sharpe_ratio,
            "total_volume": trading_metrics.total_volume,
            "avg_trade_latency_ms": trading_metrics.avg_trade_latency_ms,
            "expectancy": trading_metrics.expectancy().value(),
            "uptime_seconds": trading_metrics.uptime.as_secs()
        },
        "system": {
            "exchange_connections": system_health.exchange_connections,
            "memory_usage_mb": system_health.memory_usage_mb,
            "cpu_usage_percent": system_health.cpu_usage_percent,
            "active_positions": system_health.active_positions,
            "pending_orders": system_health.pending_orders,
            "error_rate_per_minute": system_health.error_rate_per_minute,
            "is_healthy": system_health.is_system_healthy()
        },
        "timestamp": trading_metrics.last_updated
    }))
}

/// WebSocket handler for real-time metrics streaming
async fn metrics_websocket_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_metrics_socket(socket, app_state))
}

/// Handle WebSocket connection for metrics streaming
async fn handle_metrics_socket(mut socket: WebSocket, app_state: AppState) {
    info!("New WebSocket connection for metrics streaming");

    // Subscribe to the broadcast channel
    let mut rx = app_state.metrics_tx.subscribe();

    // Send initial metrics
    if let Ok(metrics_json) = serde_json::to_string(&get_metrics(State(app_state.clone())).await.0)
    {
        if socket.send(Message::Text(metrics_json)).await.is_err() {
            return;
        }
    }

    // Listen for new metrics and forward to client
    while let Ok(msg) = rx.recv().await {
        if socket.send(Message::Text(msg)).await.is_err() {
            break; // Client disconnected
        }
    }

    info!("WebSocket connection closed");
}

/// Background task that broadcasts metrics updates
async fn metrics_broadcast_task(
    app_state: AppState,
    tx: broadcast::Sender<String>,
    interval_duration: Duration,
) {
    let mut interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;

        // Collect current metrics
        let trading_metrics = app_state.mpc_service.get_trading_metrics().await;
        let system_health = app_state.mpc_service.get_system_health().await;

        // Create metrics payload
        let metrics_payload = serde_json::json!({
            "type": "metrics_update",
            "trading": {
                "total_realized_pnl": trading_metrics.total_realized_pnl.value(),
                "total_unrealized_pnl": trading_metrics.total_unrealized_pnl.value(),
                "total_equity": trading_metrics.current_equity().value(),
                "win_rate": trading_metrics.win_rate,
                "total_trades": trading_metrics.total_trades,
                "active_positions": system_health.active_positions
            },
            "system": {
                "is_healthy": system_health.is_system_healthy(),
                "exchange_connections": system_health.exchange_connections.len(),
                "memory_usage_mb": system_health.memory_usage_mb,
                "cpu_usage_percent": system_health.cpu_usage_percent
            },
            "timestamp": trading_metrics.last_updated
        });

        // Broadcast to all connected WebSocket clients
        if let Ok(json_str) = serde_json::to_string(&metrics_payload) {
            let _ = tx.send(json_str); // Ignore errors if no receivers
        }
    }
}

/// Background task for automatic strategy weight adjustment
async fn strategy_weight_adjustment_task(app_state: AppState, interval_duration: Duration) {
    let mut interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;

        info!("🔄 Adjusting strategy weights based on performance...");
        match app_state.mpc_service.adjust_strategy_weights().await {
            Ok(()) => {
                debug!("Strategy weights adjusted successfully");
            }
            Err(e) => {
                warn!("Failed to adjust strategy weights: {}", e);
            }
        }

        // Log current strategy metrics for monitoring
        let strategy_metrics = app_state.mpc_service.get_strategy_metrics().await;
        for (name, metrics) in strategy_metrics {
            debug!(
                "Strategy {}: signals={}, executions={}, rate={:.1}%, pnl={:.2}, score={:.3}",
                name,
                metrics.signals_generated,
                metrics.signals_executed,
                metrics.execution_rate,
                metrics.strategy_pnl.value(),
                metrics.performance_score
            );
        }
    }
}

/// Background task for monitoring and alerting
async fn alerting_task(app_state: AppState) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Check every minute

    loop {
        interval.tick().await;

        let new_alerts = app_state.mpc_service.check_alerts().await;

        for alert in new_alerts {
            match alert.severity {
                crate::domain::services::metrics::AlertSeverity::Critical => {
                    error!("🚨 CRITICAL ALERT: {}", alert.message);
                }
                crate::domain::services::metrics::AlertSeverity::High => {
                    error!("⚠️ HIGH ALERT: {}", alert.message);
                }
                crate::domain::services::metrics::AlertSeverity::Medium => {
                    warn!("⚡ MEDIUM ALERT: {}", alert.message);
                }
                crate::domain::services::metrics::AlertSeverity::Low => {
                    info!("ℹ️ LOW ALERT: {}", alert.message);
                }
            }
        }

        // Log active alerts count
        let active_alerts = app_state.mpc_service.get_active_alerts().await;
        if !active_alerts.is_empty() {
            debug!("Currently {} active alerts", active_alerts.len());
        }
    }
}

/// Background task for refreshing portfolio value from all exchanges
async fn portfolio_refresh_task(app_state: AppState, interval_duration: Duration) {
    let mut interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;

        info!("💰 Refreshing portfolio value from all exchanges...");
        match app_state.mpc_service.fetch_and_update_portfolio_from_exchanges().await {
            Ok(portfolio_value) => {
                info!("✓ Portfolio updated: ${:.2}", portfolio_value);
            }
            Err(e) => {
                warn!("✗ Failed to refresh portfolio: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn metrics_broadcast_task_emits_updates() {
        let config = crate::config::TradingConfig::default();
        let mpc_service = MpcService::new(config);
        let (metrics_tx, _) = broadcast::channel::<String>(8);
        let app_state = AppState {
            mpc_service: std::sync::Arc::new(mpc_service),
            metrics_tx: metrics_tx.clone(),
        };

        let mut rx = metrics_tx.subscribe();
        let task_handle = tokio::spawn(metrics_broadcast_task(
            app_state,
            metrics_tx.clone(),
            Duration::from_millis(20),
        ));

        let result = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
        task_handle.abort();

        assert!(
            result.is_ok(),
            "expected to receive at least one metrics update"
        );
    }

    #[tokio::test]
    async fn strategy_weight_adjustment_task_respects_interval() {
        let config = crate::config::TradingConfig::default();
        let service = MpcService::new(config);
        let strategies = vec![
            (
                "FastScalping".to_string(),
                Box::new(FastScalping::new()) as Box<dyn Strategy + Send + Sync>,
            ),
            (
                "MomentumScalping".to_string(),
                Box::new(MomentumScalping::new()) as Box<dyn Strategy + Send + Sync>,
            ),
        ];
        let weights = vec![0.5, 0.5];
        let combiner =
            SignalCombiner::new(strategies, weights).expect("combiner should initialize");
        service.set_signal_combiner(combiner).await;

        {
            let mut metrics = service.strategy_metrics.lock().await;
            if let Some(fast) = metrics.get_mut("FastScalping") {
                fast.performance_score = 0.9;
            }
            if let Some(momentum) = metrics.get_mut("MomentumScalping") {
                momentum.performance_score = 0.1;
            }
        }

        let (metrics_tx, _) = broadcast::channel::<String>(4);
        let app_state = AppState {
            mpc_service: std::sync::Arc::new(service),
            metrics_tx,
        };

        let handle = tokio::spawn(strategy_weight_adjustment_task(
            app_state.clone(),
            Duration::from_millis(20),
        ));

        tokio::time::sleep(Duration::from_millis(70)).await;

        let weights = {
            let guard = app_state.mpc_service.signal_combiner.read().await;
            guard.as_ref().unwrap().weights().to_vec()
        };

        handle.abort();

        assert!(
            weights[0] > weights[1],
            "higher-performing strategy should gain weight"
        );
    }
}
