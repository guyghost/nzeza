mod domain;
mod application;
mod infrastructure;
mod config;
use axum::{routing::{get, post, delete}, Router, Json, extract::{State, Path, WebSocketUpgrade}};
use axum::response::Response;
use axum::extract::ws::{WebSocket, Message};
use tokio::sync::broadcast;
use std::net::SocketAddr;
use crate::application::services::mpc_service::MpcService;
use crate::domain::entities::exchange::Exchange;
use crate::infrastructure::adapters::exchange_actor::ExchangeActor;
use crate::domain::services::strategies::{FastScalping, MomentumScalping, ConservativeScalping, SignalCombiner, Strategy};
use tracing::{info, error, warn, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::collections::HashMap;

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

    // Load trading configuration and subscribe to symbols
    let mut config = crate::config::TradingConfig::from_env();
    
    // Disable automated trading if DYDX_MNEMONIC is not set
    if std::env::var("DYDX_MNEMONIC").is_err() {
        config.enable_automated_trading = false;
        warn!("DYDX_MNEMONIC not set - automated trading disabled");
    }
    
    info!("Configuration chargée depuis l'environnement:");
    info!("  Seuil de confiance minimum: {:.2}", config.min_confidence_threshold);
    info!("  Trading automatisé activé: {}", config.enable_automated_trading);
    info!("  Taille de position par défaut: {}", config.default_position_size);
    info!("  Positions max par symbole: {}", config.max_positions_per_symbol);
    info!("  Positions totales max: {}", config.max_total_positions);
    if let Some(sl) = config.stop_loss_percentage {
        info!("  Stop-loss: {:.1}%", sl * 100.0);
    }
    if let Some(tp) = config.take_profit_percentage {
        info!("  Take-profit: {:.1}%", tp * 100.0);
    }
    info!("  Portfolio % per position: {:.2}%", config.portfolio_percentage_per_position * 100.0);
    info!("  Max trades per hour: {}", config.max_trades_per_hour);
    info!("  Max trades per day: {}", config.max_trades_per_day);

    // Create MPC service and add senders
    let mut mpc_service = MpcService::new(config.clone());
    mpc_service.add_actor(Exchange::Binance, binance_sender);
    mpc_service.add_actor(Exchange::Dydx, dydx_sender);
    mpc_service.add_actor(Exchange::Hyperliquid, hyperliquid_sender);
    mpc_service.add_actor(Exchange::Coinbase, coinbase_sender);
    mpc_service.add_actor(Exchange::Kraken, kraken_sender);

    // Initialize signal combiner with strategies
    let strategies: Vec<Box<dyn Strategy + Send + Sync>> = vec![
        Box::new(FastScalping::new()),
        Box::new(MomentumScalping::new()),
        Box::new(ConservativeScalping::new()),
    ];
    let weights = vec![0.4, 0.4, 0.2];
    let combiner = SignalCombiner::new(strategies, weights)
        .expect("Failed to create signal combiner");
    mpc_service.set_signal_combiner(combiner).await;

    for (exchange, symbols) in &config.symbols {
        info!("Souscription à {} symboles sur {}", symbols.len(), get_exchange_name(exchange));
        for symbol in symbols {
            match mpc_service.subscribe(exchange, symbol).await {
                Ok(_) => info!("✓ Souscrit à {} sur {}", symbol, get_exchange_name(exchange)),
                Err(e) => error!("✗ Échec de souscription à {} sur {}: {}", symbol, get_exchange_name(exchange), e),
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

    // Spawn order execution task
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        order_execution_task(app_state_clone).await;
    });

    // Spawn metrics broadcasting task
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        metrics_broadcast_task(app_state_clone, metrics_tx).await;
    });

    let app = Router::new()
        .route("/", get(|| async { "MPC Trading Server with Indicators and Strategies is running!" }))
        .route("/health", get(health_check))
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
        .route("/orders/status/:order_id", get(get_order_status))
        .route("/positions", get(get_positions))
        .route("/positions/pnl", get(get_total_pnl))
        .route("/config", get(get_config))
        .route("/candles/:symbol", get(get_symbol_candles))
        .with_state(app_state.clone());

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
            app_state.mpc_service.update_exchange_connection(
                format!("{:?}", exchange),
                *is_healthy
            ).await;
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
            if let Ok(aggregated_price) = app_state.mpc_service.get_aggregated_price(&normalized_symbol).await {
                info!("Prix agrégé pour {}: {:.2}", normalized_symbol, aggregated_price.value());
                // Update candle builder
                app_state.mpc_service.update_candle(normalized_symbol.clone(), aggregated_price).await;

                // Try to generate signal
                if let Ok(signal) = app_state.mpc_service.generate_signal_for_symbol(&normalized_symbol).await {
                    info!(
                        "Signal for {}: {:?} (confidence: {:.2})",
                        normalized_symbol, signal.signal, signal.confidence
                    );

                    // Store the signal for automated execution
                    app_state.mpc_service.store_signal(normalized_symbol.clone(), signal).await;
                }

                // Update position prices and metrics
                if let Err(e) = app_state.mpc_service.update_position_prices().await {
                    debug!("Failed to update position prices: {}", e);
                }

                // Update trading metrics
                let positions = app_state.mpc_service.get_open_positions().await;
                let total_unrealized_pnl = app_state.mpc_service.get_total_unrealized_pnl().await;
                app_state.mpc_service.update_unrealized_pnl(total_unrealized_pnl).await;

                // Update system health with position count
                app_state.mpc_service.update_trading_status(
                    positions.len() as u32,
                    0 // TODO: track pending orders
                ).await;
            } else {
                debug!("Impossible d'obtenir le prix agrégé pour {}", normalized_symbol);
            }
        }
    }
}

/// Background task for order execution based on signals
async fn order_execution_task(app_state: AppState) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30)); // Check every 30 seconds

    loop {
        interval.tick().await;

        info!("🔍 Checking for orders to execute...");
        let results = app_state.mpc_service.check_and_execute_orders().await;

        let successful_orders = results.iter().filter(|r| r.is_ok()).count();
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
        if let Ok(price) = app_state.mpc_service.get_aggregated_price(&normalized_symbol).await {
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
    State(app_state): State<AppState>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);
    match app_state.mpc_service.get_aggregated_price(&normalized_symbol).await {
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
        if let Ok(signal) = mpc_service.generate_signal_for_symbol(&normalized_symbol).await {
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
        Ok(signal) => Json(serde_json::json!({
            "symbol": symbol,
            "normalized_symbol": normalized_symbol,
            "signal": format!("{:?}", signal.signal),
            "confidence": signal.confidence
        })),
        Err(e) => Json(serde_json::json!({
            "error": format!("Failed to generate signal: {}", e)
        }))
    }
}

/// Execute pending orders for all symbols
async fn execute_pending_orders(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
) -> Json<serde_json::Value> {
    let results = mpc_service.check_and_execute_orders().await;

    let successful: Vec<String> = results.iter()
        .filter_map(|r| r.as_ref().ok())
        .cloned()
        .collect();

    let failed: Vec<String> = results.iter()
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
    State(mpc_service): State<std::sync::Arc<MpcService>>,
    Path(symbol): Path<String>,
) -> Json<serde_json::Value> {
    let normalized_symbol = crate::config::TradingConfig::normalize_symbol(&symbol);

    match mpc_service.generate_signal_for_symbol(&normalized_symbol).await {
        Ok(signal) => {
            match mpc_service.execute_order_from_signal(&normalized_symbol, &signal).await {
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
                }))
            }
        },
        Err(e) => Json(serde_json::json!({
            "success": false,
            "symbol": symbol,
            "normalized_symbol": normalized_symbol,
            "error": format!("Failed to generate signal: {}", e)
        }))
    }
}

/// Place a manual order
async fn place_manual_order(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    use crate::domain::entities::order::{Order, OrderSide, OrderType};

    // Parse the payload
    let symbol = payload["symbol"].as_str()
        .ok_or((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Missing symbol field"}))))?;

    let side_str = payload["side"].as_str()
        .ok_or((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Missing side field"}))))?;

    let quantity = payload["quantity"].as_f64()
        .ok_or((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Missing or invalid quantity field"}))))?;

    let order_type_str = payload.get("order_type").and_then(|v| v.as_str()).unwrap_or("market");
    let price = payload.get("price").and_then(|v| v.as_f64());

    // Parse side
    let side = match side_str.to_uppercase().as_str() {
        "BUY" => OrderSide::Buy,
        "SELL" => OrderSide::Sell,
        _ => return Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid side. Must be 'BUY' or 'SELL'"})))),
    };

    // Parse order type
    let order_type = match order_type_str.to_uppercase().as_str() {
        "MARKET" => OrderType::Market,
        "LIMIT" => OrderType::Limit,
        _ => return Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": "Invalid order_type. Must be 'MARKET' or 'LIMIT'"})))),
    };

    // Generate order ID
    let order_id = format!("manual_order_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());

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
        Err(e) => return Err((axum::http::StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": format!("Failed to create order: {}", e)})))),
    };

    // Place order on dYdX
    match mpc_service.place_order(&crate::domain::entities::exchange::Exchange::Dydx, order).await {
        Ok(order_id) => Ok(Json(serde_json::json!({
            "success": true,
            "order_id": order_id,
            "message": "Order placed successfully on dYdX"
        }))),
        Err(e) => Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "success": false,
            "error": e
        })))),
    }
}

/// Cancel an order
async fn cancel_order(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
    Path(order_id): Path<String>,
) -> Json<serde_json::Value> {
    match mpc_service.cancel_order(&crate::domain::entities::exchange::Exchange::Dydx, &order_id).await {
        Ok(()) => Json(serde_json::json!({
            "success": true,
            "message": format!("Order {} cancelled successfully", order_id)
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e
        }))
    }
}

/// Get order status
async fn get_order_status(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
    Path(order_id): Path<String>,
) -> Json<serde_json::Value> {
    match mpc_service.get_order_status(&crate::domain::entities::exchange::Exchange::Dydx, &order_id).await {
        Ok(status) => Json(serde_json::json!({
            "success": true,
            "order_id": order_id,
            "status": status
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e
        }))
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

/// Get all open positions
async fn get_positions(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
) -> Json<serde_json::Value> {
    let positions = mpc_service.get_open_positions().await;
    let position_data: HashMap<String, serde_json::Value> = positions.iter().map(|(id, position)| {
        (id.clone(), serde_json::json!({
            "symbol": position.symbol,
            "side": format!("{:?}", position.side),
            "quantity": position.quantity.value(),
            "entry_price": position.entry_price.value(),
            "current_price": position.current_price.map(|p| p.value()),
            "unrealized_pnl": position.unrealized_pnl().map(|p| p.value()),
            "entry_time": position.entry_time.to_rfc3339(),
            "stop_loss_price": position.stop_loss_price.map(|p| p.value()),
            "take_profit_price": position.take_profit_price.map(|p| p.value())
        }))
    }).collect();

    Json(serde_json::json!({
        "positions": position_data,
        "count": positions.len()
    }))
}

/// Get total unrealized PnL across all positions
async fn get_total_pnl(
    State(mpc_service): State<std::sync::Arc<MpcService>>,
) -> Json<serde_json::Value> {
    let total_pnl = mpc_service.get_total_unrealized_pnl().await;

    Json(serde_json::json!({
        "total_unrealized_pnl": total_pnl.value(),
        "currency": "USD"
    }))
}

/// Get current configuration
async fn get_config(
    State(app_state): State<AppState>,
) -> Json<serde_json::Value> {
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
async fn get_metrics(
    State(app_state): State<AppState>,
) -> Json<serde_json::Value> {
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
    if let Ok(metrics_json) = serde_json::to_string(&get_metrics(State(app_state.clone())).await.0) {
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
async fn metrics_broadcast_task(app_state: AppState, tx: broadcast::Sender<String>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5)); // Update every 5 seconds

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
