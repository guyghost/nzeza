use crate::application::actors::trader_actor::TraderMessage;
use crate::config::TradingConfig;
use crate::domain::entities::exchange::Exchange;
use crate::domain::entities::order::Order;
use crate::domain::entities::position::{Position, PositionSide};
use crate::domain::errors::MpcError;
use crate::domain::services::candle_builder::CandleBuilder;
use crate::domain::services::metrics::{
    AlertConfig, PerformanceProfiler, StrategyMetrics, SystemAlert, SystemHealthMetrics,
    TradingMetrics,
};
use crate::domain::services::strategies::{SignalCombiner, TradingSignal};
use crate::domain::value_objects::price::Price;
use crate::domain::value_objects::quantity::Quantity;
use crate::domain::value_objects::pnl::PnL;
use crate::infrastructure::adapters::exchange_actor::ExchangeMessage;
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Channel reply timeout duration (5 seconds)
const CHANNEL_REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// Candle interval in seconds (10 seconds for faster signal generation)
const CANDLE_INTERVAL_SECS: u64 = 10;

/// Number of candles to keep in history
const CANDLE_HISTORY_SIZE: usize = 100;

/// LRU cache capacity for last signals
const SIGNAL_CACHE_CAPACITY: usize = 1000;

/// Minimum number of candles required for signal generation
const MIN_CANDLES_FOR_SIGNAL: usize = 5;

/// Minimum quantity to avoid dust orders
const MIN_ORDER_QUANTITY: f64 = 0.0001;

/// ## Lock Ordering Convention (to prevent deadlocks)
///
/// Always acquire locks in this order:
/// 1. signal_combiner (RwLock)
/// 2. strategy_order (Mutex)
/// 3. strategy_metrics (Mutex)
/// 4. traders (Mutex)
/// 5. Other Mutexes (alphabetically: active_alerts, candle_builder, last_signals,
///    open_positions, performance_profiler, system_health, trade_history, trading_metrics)
///
/// IMPORTANT: Release locks as soon as possible to minimize contention.
/// Clone data if needed to release locks early.

/// Validate trading symbol format
///
/// Symbols must be:
/// - Non-empty and max 20 characters
/// - Contain only alphanumeric, hyphens, underscores, or slashes
fn validate_symbol(symbol: &str) -> Result<(), MpcError> {
    if symbol.is_empty() || symbol.len() > 20 {
        return Err(MpcError::InvalidInput(format!(
            "Invalid symbol length: '{}' (must be 1-20 characters)",
            symbol
        )));
    }

    if !symbol
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/')
    {
        return Err(MpcError::InvalidInput(format!(
            "Invalid symbol format: '{}' (only alphanumeric, -, _, / allowed)",
            symbol
        )));
    }

    Ok(())
}

/// Portfolio state tracking
#[derive(Debug, Clone)]
pub struct PortfolioState {
    /// Total portfolio value in USD (or base currency)
    pub total_value: f64,
    /// Cash available for trading (excluding positions)
    pub available_cash: f64,
    /// Value locked in open positions
    pub position_value: f64,
    /// Last update timestamp
    pub last_updated: SystemTime,
}

impl Default for PortfolioState {
    fn default() -> Self {
        Self {
            total_value: 10000.0, // Default fallback
            available_cash: 10000.0,
            position_value: 0.0,
            last_updated: SystemTime::now(),
        }
    }
}

pub struct MpcService {
    pub senders: Arc<HashMap<Exchange, mpsc::Sender<ExchangeMessage>>>, // Exchange actors for market data
    pub traders: Arc<Mutex<HashMap<String, mpsc::Sender<TraderMessage>>>>, // Trader actors for execution
    pub signal_combiner: Arc<RwLock<Option<SignalCombiner>>>, // RwLock for better concurrency
    pub candle_builder: Arc<Mutex<CandleBuilder>>,
    pub last_signals: Arc<Mutex<LruCache<String, TradingSignal>>>, // LRU cache to prevent unbounded growth
    pub open_positions: Arc<Mutex<HashMap<String, Position>>>,
    pub config: TradingConfig,
    pub trade_history: Arc<Mutex<Vec<(SystemTime, String)>>>, // (timestamp, symbol) for rate limiting
    pub trading_metrics: Arc<Mutex<TradingMetrics>>,
    pub system_health: Arc<Mutex<SystemHealthMetrics>>,
    pub strategy_metrics: Arc<Mutex<HashMap<String, StrategyMetrics>>>,
    pub strategy_order: Arc<Mutex<Vec<String>>>,
    pub alert_config: AlertConfig,
    pub active_alerts: Arc<Mutex<Vec<SystemAlert>>>,
    pub performance_profiler: Arc<Mutex<PerformanceProfiler>>,
    pub portfolio_state: Arc<Mutex<PortfolioState>>, // Real-time portfolio tracking
}

impl MpcService {
    pub fn new(config: TradingConfig) -> Self {
        // Initialize candle builder with configured interval and history size
        let candle_builder = Arc::new(Mutex::new(CandleBuilder::new(
            Duration::from_secs(CANDLE_INTERVAL_SECS),
            CANDLE_HISTORY_SIZE,
        )));

        // LRU cache capacity for signal storage
        let cache_capacity =
            NonZeroUsize::new(SIGNAL_CACHE_CAPACITY).expect("Cache capacity must be non-zero");

        Self {
            senders: Arc::new(HashMap::new()),
            traders: Arc::new(Mutex::new(HashMap::new())),
            signal_combiner: Arc::new(RwLock::new(None)),
            candle_builder,
            last_signals: Arc::new(Mutex::new(LruCache::new(cache_capacity))),
            open_positions: Arc::new(Mutex::new(HashMap::new())),
            config,
            trade_history: Arc::new(Mutex::new(Vec::new())),
            trading_metrics: Arc::new(Mutex::new(TradingMetrics::new())),
            system_health: Arc::new(Mutex::new(SystemHealthMetrics::new())),
            strategy_metrics: Arc::new(Mutex::new(HashMap::new())),
            strategy_order: Arc::new(Mutex::new(Vec::new())),
            alert_config: AlertConfig::default(),
            active_alerts: Arc::new(Mutex::new(Vec::new())),
            performance_profiler: Arc::new(Mutex::new(PerformanceProfiler::new())),
            portfolio_state: Arc::new(Mutex::new(PortfolioState::default())),
        }
    }

    /// Add a trader actor
    pub async fn add_trader(&self, trader_id: String, sender: mpsc::Sender<TraderMessage>) {
        let mut traders = self.traders.lock().await;
        traders.insert(trader_id.clone(), sender);
        info!("Added trader: {}", trader_id);
    }

    /// Get a trader by ID
    pub async fn get_trader(&self, trader_id: &str) -> Option<mpsc::Sender<TraderMessage>> {
        let traders = self.traders.lock().await;
        traders.get(trader_id).cloned()
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

    /// Add an exchange actor to the service
    ///
    /// # Errors
    /// Returns an error if the service has already been shared (Arc has multiple references)
    ///
    /// # Important
    /// This must be called during initialization before the service is wrapped in Arc and shared
    pub fn add_actor(&mut self, exchange: Exchange, sender: mpsc::Sender<ExchangeMessage>)
        -> Result<(), String> {
        // Get mutable access to the inner HashMap via Arc::get_mut
        // This only succeeds if there's exactly one Arc reference (during initialization)
        Arc::get_mut(&mut self.senders)
            .ok_or_else(|| {
                "Cannot add actors after service is shared. \
                Call add_actor during initialization only, before wrapping in Arc."
                    .to_string()
            })?
            .insert(exchange, sender);
        Ok(())
    }

    pub async fn set_signal_combiner(&self, combiner: SignalCombiner) {
        let mut signal_combiner_guard = self.signal_combiner.write().await;
        *signal_combiner_guard = Some(combiner);

        // Initialize strategy metrics
        let mut strategy_metrics = self.strategy_metrics.lock().await;
        if let Some(combiner) = signal_combiner_guard.as_ref() {
            let strategy_names = combiner.get_strategy_names();

            {
                let mut order = self.strategy_order.lock().await;
                *order = strategy_names.clone();
            }

            strategy_metrics.clear();
            for name in strategy_names {
                strategy_metrics.insert(name.clone(), StrategyMetrics::new(name));
            }
        }
    }

    /// Check health of a specific actor
    pub async fn check_actor_health(&self, exchange: &Exchange) -> Result<bool, MpcError> {
        if let Some(sender) = self.senders.as_ref().get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);

            sender
                .send(ExchangeMessage::HealthCheck { reply: reply_tx })
                .await
                .map_err(|_| {
                    MpcError::ChannelSendError("Failed to send health check".to_string())
                })?;

            timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
                .await
                .map_err(|_| MpcError::Timeout)?
                .ok_or(MpcError::NoResponse)
        } else {
            Err(MpcError::ActorNotFound(exchange.clone()))
        }
    }

    /// Check health of all actors
    pub async fn check_all_actors_health(&self) -> HashMap<Exchange, bool> {
        let mut health_status = HashMap::new();

        for exchange in self.senders.as_ref().keys() {
            match self.check_actor_health(exchange).await {
                Ok(is_healthy) => {
                    health_status.insert(exchange.clone(), is_healthy);
                    if !is_healthy {
                        warn!("Actor {:?} is unhealthy", exchange);
                    }
                }
                Err(e) => {
                    error!("Failed to check health of {:?}: {}", exchange, e);
                    health_status.insert(exchange.clone(), false);
                }
            }
        }

        info!("Health check complete: {:?}", health_status);
        health_status
    }

    /// Shutdown all actors gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down all exchange actors...");

        for (exchange, sender) in self.senders.as_ref().iter() {
            info!("Sending shutdown signal to {:?}", exchange);
            if let Err(e) = sender.send(ExchangeMessage::Shutdown).await {
                error!("Failed to send shutdown to {:?}: {}", exchange, e);
            }
        }

        // Give actors time to shutdown gracefully
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        info!("All actors shutdown complete");
    }


    pub async fn get_price(&self, exchange: &Exchange, symbol: &str) -> Result<Price, MpcError> {
        if let Some(sender) = self.senders.as_ref().get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::GetPrice {
                symbol: symbol.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await?;
            timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
                .await
                .map_err(|_| MpcError::Timeout)?
                .ok_or(MpcError::NoResponse)?
                .map_err(|e| MpcError::AggregationFailed(e))
        } else {
            Err(MpcError::ActorNotFound(exchange.clone()))
        }
    }


    pub async fn subscribe(&self, exchange: &Exchange, symbol: &str) -> Result<(), MpcError> {
        if let Some(sender) = self.senders.as_ref().get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::Subscribe {
                symbol: symbol.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await?;
            timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
                .await
                .map_err(|_| MpcError::Timeout)?
                .ok_or(MpcError::NoResponse)?
                .map_err(|e| MpcError::AggregationFailed(e))
        } else {
            Err(MpcError::ActorNotFound(exchange.clone()))
        }
    }


    pub async fn unsubscribe(&self, exchange: &Exchange, symbol: &str) -> Result<(), MpcError> {
        if let Some(sender) = self.senders.as_ref().get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::Unsubscribe {
                symbol: symbol.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await?;
            timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
                .await
                .map_err(|_| MpcError::Timeout)?
                .ok_or(MpcError::NoResponse)?
                .map_err(|e| MpcError::AggregationFailed(e))
        } else {
            Err(MpcError::ActorNotFound(exchange.clone()))
        }
    }


    pub async fn get_subscriptions(&self, exchange: &Exchange) -> Result<Vec<String>, MpcError> {
        if let Some(sender) = self.senders.as_ref().get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::GetSubscriptions { reply: reply_tx };
            sender.send(msg).await?;
            timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
                .await
                .map_err(|_| MpcError::Timeout)?
                .ok_or(MpcError::NoResponse)
        } else {
            Err(MpcError::ActorNotFound(exchange.clone()))
        }
    }

    // Pure method: aggregate prices from multiple exchanges

    pub fn aggregate_prices(prices: &[Price]) -> Result<Price, MpcError> {
        if prices.is_empty() {
            return Err(MpcError::NoPricesAvailable {
                symbol: "unknown".to_string(),
            });
        }
        let sum: f64 = prices.iter().map(|p| p.value()).sum();
        let avg = sum / prices.len() as f64;
        Price::new(avg).map_err(|e| MpcError::AggregationFailed(e.to_string()))
    }

    // Generate trading signal using combined strategies

    pub async fn generate_trading_signal(
        &self,
        candles: &[crate::domain::services::indicators::Candle],
    ) -> Result<TradingSignal, MpcError> {
        let signal_combiner_guard = self.signal_combiner.read().await;
        signal_combiner_guard
            .as_ref()
            .and_then(|combiner| combiner.combine_signals(candles))
            .ok_or(MpcError::SignalCombinerNotInitialized)
    }

    /// Generate trading signal and track individual strategy signals

    pub async fn generate_trading_signal_with_tracking(
        &self,
        candles: &[crate::domain::services::indicators::Candle],
    ) -> Result<TradingSignal, MpcError> {
        let signal_combiner_guard = self.signal_combiner.read().await;
        if let Some(combiner) = signal_combiner_guard.as_ref() {
            // Record signals for each strategy
            for _strategy in &combiner.strategies {
                // We can't easily identify which strategy this is without modifying the trait
                // For now, we'll record signals when they're actually used in execution
            }

            combiner
                .combine_signals(candles)
                .ok_or(MpcError::SignalCombinerNotInitialized)
        } else {
            Err(MpcError::SignalCombinerNotInitialized)
        }
    }

    /// Get aggregated price for a symbol across all exchanges (using normalized symbols)

    pub async fn get_aggregated_price(&self, symbol: &str) -> Result<Price, MpcError> {
        use crate::config::TradingConfig;
        let normalized_symbol = TradingConfig::normalize_symbol(symbol);

        let mut prices = Vec::new();

        for (exchange, sender) in self.senders.as_ref().iter() {
            // Get subscriptions for this exchange
            let (sub_tx, mut sub_rx) = mpsc::channel(1);
            sender
                .send(ExchangeMessage::GetSubscriptions { reply: sub_tx })
                .await?;

            if let Some(subs) = sub_rx.recv().await {
                // Check if any of the subscribed symbols normalize to our target symbol
                for sub_symbol in &subs {
                    let normalized_sub = TradingConfig::normalize_symbol(sub_symbol);
                    if normalized_sub == normalized_symbol {
                        // Get price for this symbol on this exchange
                        let (price_tx, mut price_rx) = mpsc::channel(1);
                        sender
                            .send(ExchangeMessage::GetPrice {
                                symbol: sub_symbol.to_string(),
                                reply: price_tx,
                            })
                            .await?;

                        if let Some(Ok(price)) = price_rx.recv().await {
                            debug!(
                                "Price obtained {:.2} from {} for {} (normalized: {})",
                                price.value(),
                                Self::get_exchange_name(exchange),
                                sub_symbol,
                                normalized_symbol
                            );
                            prices.push(price);
                        } else {
                            debug!(
                                "No price available from {} for {}",
                                Self::get_exchange_name(exchange),
                                sub_symbol
                            );
                        }
                        break; // Found a matching symbol, no need to check others
                    }
                }
            }
        }

        if prices.is_empty() {
            return Err(MpcError::NoPricesAvailable {
                symbol: symbol.to_string(),
            });
        }

        let aggregated = Self::aggregate_prices(&prices)?;
        debug!(
            "Aggregated price calculated {:.2} for {} (based on {} exchanges)",
            aggregated.value(),
            normalized_symbol,
            prices.len()
        );
        Ok(aggregated)
    }

    /// Get all tracked symbols across all exchanges

    pub async fn get_all_symbols(&self) -> Vec<String> {
        let mut all_symbols = std::collections::HashSet::new();

        for sender in self.senders.as_ref().values() {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            if sender
                .send(ExchangeMessage::GetSubscriptions { reply: reply_tx })
                .await
                .is_ok()
            {
                if let Some(symbols) = reply_rx.recv().await {
                    for symbol in symbols {
                        all_symbols.insert(symbol);
                    }
                }
            }
        }

        all_symbols.into_iter().collect()
    }

    /// Update candle builder with new price

    pub async fn update_candle(&self, symbol: String, price: Price) {
        let mut builder = self.candle_builder.lock().await;
        builder.add_price(symbol.clone(), price);
        debug!(
            "Candle updated for {} with price {:.2}",
            symbol,
            price.value()
        );
    }

    /// Get candles for a symbol

    pub async fn get_candles(
        &self,
        symbol: &str,
    ) -> Vec<crate::domain::services::indicators::Candle> {
        let builder = self.candle_builder.lock().await;
        builder.get_candles(symbol)
    }

    /// Generate trading signal for a specific symbol

    /// Generate trading signal for a specific symbol
    ///
    /// Requires at least MIN_CANDLES_FOR_SIGNAL candles to generate a signal
    pub async fn generate_signal_for_symbol(
        &self,
        symbol: &str,
    ) -> Result<TradingSignal, MpcError> {
        let candles = self.get_candles(symbol).await;
        debug!("Symbol {}: {} candles available", symbol, candles.len());
        if candles.len() >= MIN_CANDLES_FOR_SIGNAL {
            self.generate_trading_signal(&candles).await
        } else {
            debug!(
                "Not enough candles for {}: need {}, have {}",
                symbol,
                MIN_CANDLES_FOR_SIGNAL,
                candles.len()
            );
            Err(MpcError::NoSignalsAvailable)
        }
    }

    /// Place an order on a specific exchange

    pub async fn place_order(&self, exchange: &Exchange, order: Order) -> Result<String, MpcError> {
        if let Some(sender) = self.senders.as_ref().get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::PlaceOrder {
                order,
                reply: reply_tx,
            };
            sender.send(msg).await?;
            timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
                .await
                .map_err(|_| MpcError::Timeout)?
                .ok_or(MpcError::NoResponse)?
                .map_err(|e| MpcError::OrderPlacementFailed(e))
        } else {
            Err(MpcError::ActorNotFound(exchange.clone()))
        }
    }

    /// Cancel an order on a specific exchange

    pub async fn cancel_order(&self, exchange: &Exchange, order_id: &str) -> Result<(), MpcError> {
        if let Some(sender) = self.senders.as_ref().get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::CancelOrder {
                order_id: order_id.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await?;
            timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
                .await
                .map_err(|_| MpcError::Timeout)?
                .ok_or(MpcError::NoResponse)?
                .map_err(|e| MpcError::OrderPlacementFailed(e))
        } else {
            Err(MpcError::ActorNotFound(exchange.clone()))
        }
    }

    /// Get order status from a specific exchange

    pub async fn get_order_status(
        &self,
        exchange: &Exchange,
        order_id: &str,
    ) -> Result<String, MpcError> {
        if let Some(sender) = self.senders.as_ref().get(exchange) {
            let (reply_tx, mut reply_rx) = mpsc::channel(1);
            let msg = ExchangeMessage::GetOrderStatus {
                order_id: order_id.to_string(),
                reply: reply_tx,
            };
            sender.send(msg).await?;
            timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
                .await
                .map_err(|_| MpcError::Timeout)?
                .ok_or(MpcError::NoResponse)?
                .map_err(|e| MpcError::OrderPlacementFailed(e))
        } else {
            Err(MpcError::ActorNotFound(exchange.clone()))
        }
    }

    /// Store last signal for a symbol (using LRU cache)

    pub async fn store_signal(&self, symbol: String, signal: TradingSignal) {
        let mut last_signals = self.last_signals.lock().await;
        last_signals.put(symbol, signal); // LRU cache uses .put() instead of .insert()
    }

    /// Get all last signals from LRU cache

    pub async fn get_all_last_signals(&self) -> HashMap<String, TradingSignal> {
        let last_signals = self.last_signals.lock().await;
        last_signals
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Open a new position

    pub async fn open_position(
        &self,
        symbol: &str,
        side: PositionSide,
        quantity: Quantity,
        entry_price: Price,
    ) -> Result<String, MpcError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| MpcError::InvalidConfiguration("System time error".to_string()))?
            .as_millis();

        let position_id = format!("pos_{}_{}", symbol, timestamp);

        let position = Position::new_with_stops(
            position_id.clone(),
            symbol.to_string(),
            side.clone(),
            quantity,
            entry_price,
            self.config.stop_loss_percentage,
            self.config.take_profit_percentage,
        )
        .map_err(|e| {
            MpcError::InvalidConfiguration(format!("Failed to create position: {}", e))
        })?;

        let mut positions = self.open_positions.lock().await;
        positions.insert(position_id.clone(), position);

        let stop_info = if let (Some(sl), Some(tp)) = (
            self.config.stop_loss_percentage,
            self.config.take_profit_percentage,
        ) {
            format!(" (SL: {:.1}%, TP: {:.1}%)", sl * 100.0, tp * 100.0)
        } else {
            "".to_string()
        };

        info!(
            "Opened position: {} {} {} @ {}{}",
            position_id,
            side,
            quantity.value(),
            entry_price.value(),
            stop_info
        );
        Ok(position_id)
    }

    /// Close a position

    pub async fn close_position(&self, position_id: &str) -> Result<(), MpcError> {
        let mut positions = self.open_positions.lock().await;
        if let Some(position) = positions.remove(position_id) {
            let pnl = position.unrealized_pnl().unwrap_or(PnL::zero());
            let entry_value = position.quantity.value() * position.entry_price.value();

            info!(
                "Closed position: {} (PnL: {:?})",
                position_id,
                pnl
            );

            // Release lock before updating portfolio
            drop(positions);

            // Update portfolio after closing position
            self.update_portfolio_after_position_close(entry_value, pnl.value()).await;

            Ok(())
        } else {
            Err(MpcError::InvalidConfiguration(format!(
                "Position {} not found",
                position_id
            )))
        }
    }

    /// Update position prices with current market prices
    ///
    /// This method minimizes lock contention by:
    /// 1. Collecting symbols without holding the lock
    /// 2. Fetching prices asynchronously (no lock held)
    /// 3. Updating positions with a short final lock
    pub async fn update_position_prices(&self) -> Result<(), MpcError> {
        // Step 1: Collect symbols that need price updates (short lock)
        let symbols_to_update: Vec<String> = {
            let positions = self.open_positions.lock().await;
            positions
                .values()
                .map(|p| p.symbol.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect()
        };

        // Step 2: Fetch all prices concurrently (no lock held)
        let mut prices = HashMap::new();
        for symbol in &symbols_to_update {
            if let Ok(current_price) = self.get_aggregated_price(symbol).await {
                prices.insert(symbol.clone(), current_price);
            }
        }

        // Step 3: Update positions with fetched prices (short lock)
        let mut positions = self.open_positions.lock().await;
        for (symbol, price) in prices {
            for position in positions.values_mut() {
                if position.symbol == symbol {
                    position.update_price(price);
                }
            }
        }

        Ok(())
    }

    /// Get all open positions

    pub async fn get_open_positions(&self) -> HashMap<String, Position> {
        let positions = self.open_positions.lock().await;
        positions.clone()
    }

    /// Get total unrealized PnL across all positions
    ///
    /// Returns the sum of all unrealized PnL across open positions.
    /// The result can be positive (net profit) or negative (net loss).
    /// Get total unrealized PnL across all positions
    ///
    /// Returns the sum of all unrealized PnL across open positions.
    /// The result can be positive (net profit) or negative (net loss).
    ///
    /// If the total PnL is invalid (NaN/Infinity), logs detailed error information
    /// and returns zero as a safe fallback.
    pub async fn get_total_unrealized_pnl(&self) -> PnL {
        let positions = self.open_positions.lock().await;
        let mut total_pnl = 0.0;
        let mut position_pnls = Vec::new();

        for (position_id, position) in positions.iter() {
            if let Some(pnl) = position.unrealized_pnl() {
                let pnl_value = pnl.value();
                position_pnls.push((position_id.clone(), pnl_value));
                total_pnl += pnl_value;
            }
        }

        // PnL can be negative, which is valid for losses
        match PnL::new(total_pnl) {
            Ok(pnl) => pnl,
            Err(e) => {
                error!(
                    "Failed to create PnL from total: {} - Error: {}. \
                     Position PnLs: {:?}. Using zero PnL as fallback.",
                    total_pnl, e, position_pnls
                );
                PnL::zero()
            }
        }
    }

    /// Fetch portfolio value from Coinbase and update internal state
    ///
    /// This fetches the real-time balance from Coinbase Advanced Trade API
    /// and updates the portfolio state. It converts all crypto balances to USD equivalent.
    ///
    /// # Returns
    /// Updated portfolio value in USD
    pub async fn fetch_and_update_portfolio_from_coinbase(&self) -> Result<f64, MpcError> {
        use crate::application::actors::trader_actor::TraderMessage;

        // Get the first trader that has Coinbase as active exchange
        let traders = self.traders.lock().await;
        let trader_sender = traders.values().next()
            .ok_or_else(|| MpcError::InvalidConfiguration("No traders available".to_string()))?
            .clone();
        drop(traders); // Release lock early

        // Get balances from Coinbase
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        trader_sender
            .send(TraderMessage::GetBalance {
                currency: None, // Get all balances
                reply: reply_tx,
            })
            .await
            .map_err(|e| MpcError::ChannelSendError(format!("Failed to send GetBalance: {}", e)))?;

        let balance_result = timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
            .await
            .map_err(|_| MpcError::Timeout)?
            .ok_or_else(|| MpcError::ChannelSendError("GetBalance reply channel closed".to_string()))?;

        let total_usd = match balance_result {
            Ok(balance_usd) => balance_usd,
            Err(e) => {
                warn!("Failed to get balance from Coinbase: {}", e);
                // Fallback to cached value
                let state = self.portfolio_state.lock().await;
                return Ok(state.total_value);
            }
        };

        // Update portfolio state
        let mut portfolio_state = self.portfolio_state.lock().await;
        portfolio_state.total_value = total_usd;
        portfolio_state.available_cash = total_usd;
        portfolio_state.last_updated = SystemTime::now();

        info!("Updated portfolio value from Coinbase: ${:.2}", total_usd);

        Ok(total_usd)
    }

    /// Get current portfolio value
    ///
    /// Returns the cached portfolio value from the most recent fetch from Coinbase.
    /// If the cache is stale (older than 5 minutes), this should trigger a refresh.
    pub async fn get_portfolio_value(&self) -> f64 {
        let portfolio_state = self.portfolio_state.lock().await;

        // Check if cache is stale (older than 5 minutes)
        let cache_age = SystemTime::now()
            .duration_since(portfolio_state.last_updated)
            .unwrap_or(Duration::from_secs(0));

        if cache_age > Duration::from_secs(300) {
            warn!("Portfolio cache is stale (age: {:?}). Consider calling fetch_and_update_portfolio_from_coinbase()", cache_age);
        }

        portfolio_state.total_value
    }

    /// Update portfolio after position is opened
    ///
    /// This adjusts the available cash and position value when a new position is created.
    pub async fn update_portfolio_after_position_open(&self, position_value: f64) {
        let mut portfolio_state = self.portfolio_state.lock().await;
        portfolio_state.available_cash -= position_value;
        portfolio_state.position_value += position_value;

        debug!(
            "Portfolio updated after position open: available_cash=${:.2}, position_value=${:.2}",
            portfolio_state.available_cash, portfolio_state.position_value
        );
    }

    /// Update portfolio after position is closed
    ///
    /// This releases the position value back to available cash, adjusted by PnL.
    pub async fn update_portfolio_after_position_close(&self, position_value: f64, realized_pnl: f64) {
        let mut portfolio_state = self.portfolio_state.lock().await;
        portfolio_state.available_cash += position_value + realized_pnl;
        portfolio_state.position_value -= position_value;
        portfolio_state.total_value += realized_pnl;

        info!(
            "Portfolio updated after position close: PnL=${:.2}, total_value=${:.2}, available_cash=${:.2}",
            realized_pnl, portfolio_state.total_value, portfolio_state.available_cash
        );
    }

    /// Calculate position size based on portfolio percentage and current price
    ///
    /// # Arguments
    /// * `_symbol` - The trading symbol (reserved for future risk management per symbol)
    /// * `current_price` - Current market price for the asset
    ///
    /// # Returns
    /// The quantity to trade, calculated as a percentage of current portfolio value
    /// Calculate position size based on portfolio percentage and current price
    ///
    /// # Arguments
    /// * `_symbol` - The trading symbol (reserved for future risk management per symbol)
    /// * `current_price` - Current market price for the asset
    ///
    /// # Returns
    /// The quantity to trade, calculated as a percentage of current portfolio value
    ///
    /// # Errors
    /// Returns an error if:
    /// - Portfolio value is zero or negative
    /// - Calculation results in overflow
    /// - Quantity is not finite
    pub async fn calculate_position_size(
        &self,
        _symbol: &str,
        current_price: Price,
    ) -> Result<f64, MpcError> {
        let portfolio_value = self.get_portfolio_value().await;

        // Ensure we have positive portfolio value
        if portfolio_value <= 0.0 {
            return Err(MpcError::InvalidInput(
                "Portfolio value is zero or negative. Cannot calculate position size.".to_string()
            ));
        }

        if !portfolio_value.is_finite() {
            return Err(MpcError::InvalidInput(format!(
                "Portfolio value is not finite: {}",
                portfolio_value
            )));
        }

        // Calculate position value (f64 doesn't have checked_mul, but we validate result)
        let position_value = portfolio_value * self.config.portfolio_percentage_per_position;

        if !position_value.is_finite() {
            return Err(MpcError::InvalidInput(format!(
                "Position value calculation resulted in non-finite value: {} * {} = {}",
                portfolio_value, self.config.portfolio_percentage_per_position, position_value
            )));
        }

        // Calculate quantity: position_value / current_price
        let quantity = position_value / current_price.value();

        // Validation: ensure quantity is finite and positive
        if !quantity.is_finite() {
            return Err(MpcError::InvalidInput(format!(
                "Invalid quantity calculated: {} (portfolio_value: {}, price: {})",
                quantity,
                portfolio_value,
                current_price.value()
            )));
        }

        if quantity < 0.0 {
            return Err(MpcError::InvalidInput(format!(
                "Negative quantity calculated: {}",
                quantity
            )));
        }

        // Ensure quantity is reasonable (not too small)
        let final_quantity = quantity.max(MIN_ORDER_QUANTITY);

        Ok(final_quantity)
    }

    /// Check and execute stop-loss and take-profit orders

    pub async fn check_and_execute_stops(&self) -> Vec<Result<String, MpcError>> {
        let mut results = Vec::new();
        let mut positions_to_close = Vec::new();

        {
            let positions = self.open_positions.lock().await;
            for (position_id, position) in positions.iter() {
                if position.should_stop_loss() {
                    positions_to_close.push((position_id.clone(), "stop-loss"));
                } else if position.should_take_profit() {
                    positions_to_close.push((position_id.clone(), "take-profit"));
                }
            }
        }

        for (position_id, reason) in positions_to_close {
            match self.close_position(&position_id).await {
                Ok(()) => {
                    results.push(Ok(format!(
                        "Position {} closed due to {}",
                        position_id, reason
                    )));
                }
                Err(e) => {
                    results.push(Err(e));
                }
            }
        }

        results
    }

    /// Execute order based on signal

    pub async fn execute_order_from_signal(
        &self,
        symbol: &str,
        signal: &TradingSignal,
    ) -> Result<String, MpcError> {
        use crate::domain::entities::order::{Order, OrderSide, OrderType};

        // Validate symbol format
        validate_symbol(symbol)?;

        // Check if automated trading is enabled
        if !self.config.enable_automated_trading {
            return Ok("Automated trading is disabled".to_string());
        }

        // Check trading limits before proceeding
        self.check_trading_limits().await?;

        // Get current price (for logging purposes)
        let current_price = self.get_aggregated_price(symbol).await?;

        // Determine order side and calculate position size based on signal
        let (order_side, position_side) = match signal.signal {
            crate::domain::services::strategies::Signal::Buy => {
                (OrderSide::Buy, PositionSide::Long)
            }
            crate::domain::services::strategies::Signal::Sell => {
                (OrderSide::Sell, PositionSide::Short)
            }
            crate::domain::services::strategies::Signal::Hold => {
                return Ok("No order executed - signal is HOLD".to_string());
            }
        };

        // Calculate position size based on portfolio percentage
        let quantity = self.calculate_position_size(symbol, current_price).await?;
        let quantity = Quantity::new(quantity).map_err(|e| {
            MpcError::InvalidConfiguration(format!("Invalid quantity calculation: {}", e))
        })?;

        // Check position limits
        let positions = self.open_positions.lock().await;
        let symbol_positions: Vec<_> = positions.values().filter(|p| p.symbol == symbol).collect();

        if symbol_positions.len() >= self.config.max_positions_per_symbol {
            return Ok(format!(
                "Maximum positions per symbol ({}) reached for {}",
                self.config.max_positions_per_symbol, symbol
            ));
        }

        if positions.len() >= self.config.max_total_positions {
            return Ok(format!(
                "Maximum total positions ({}) reached",
                self.config.max_total_positions
            ));
        }
        drop(positions); // Release the lock

        // Only execute if confidence is high enough
        if signal.confidence < self.config.min_confidence_threshold {
            return Ok(format!(
                "Signal confidence {:.2} too low for execution (minimum {:.2})",
                signal.confidence, self.config.min_confidence_threshold
            ));
        }

        // Generate unique order ID
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| MpcError::InvalidConfiguration("System time error".to_string()))?
            .as_millis();
        let order_id = format!("order_{}_{}", timestamp, symbol);

        // Create the order
        let order = Order::new(
            order_id.clone(),
            symbol.to_string(),
            order_side.clone(),
            OrderType::Market,
            None, // Market order has no price
            quantity.value(),
        )
        .map_err(|e| MpcError::InvalidConfiguration(format!("Failed to create order: {}", e)))?;

        let (trader_id, trader_sender) = self
            .select_trader_sender()
            .await
            .ok_or_else(|| {
                MpcError::InvalidConfiguration(
                    "No trader available to execute orders. Ensure traders are initialized."
                        .to_string(),
                )
            })?;

        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        trader_sender
            .send(TraderMessage::PlaceOrder {
                order,
                reply: reply_tx,
            })
            .await
            .map_err(|_| {
                MpcError::ChannelSendError(format!(
                    "Failed to dispatch order to trader {}",
                    trader_id
                ))
            })?;

        match timeout(CHANNEL_REPLY_TIMEOUT, reply_rx.recv())
            .await
            .map_err(|_| {
                error!("Timeout waiting for order response from trader {}", trader_id);
                MpcError::Timeout
            })?
            .ok_or(MpcError::NoResponse)?
            .map_err(|e| MpcError::OrderPlacementFailed(e))
        {
            Ok(order_id) => {
                // Record the trade in history
                {
                    let mut trade_history = self.trade_history.lock().await;
                    trade_history.push((SystemTime::now(), symbol.to_string()));

                    // Clean up old trades (keep only last 24 hours)
                    let one_day_ago = SystemTime::now()
                        .checked_sub(Duration::from_secs(86400))
                        .unwrap_or(SystemTime::now());
                    trade_history.retain(|(timestamp, _)| *timestamp >= one_day_ago);
                }

                // Record strategy execution for all strategies (simplified approach)
                // In a real implementation, we'd track which strategies contributed to the signal
                let strategy_names = {
                    let combiner_guard = self.signal_combiner.read().await;
                    combiner_guard
                        .as_ref()
                        .map(|c| c.get_strategy_names())
                        .unwrap_or_default()
                };

                for strategy_name in strategy_names {
                    // Calculate proportional PnL based on strategy weight
                    // For simplicity, assume equal contribution for now
                    // Safe: 0.0 is always a valid price
                    let proportional_pnl = Price::new(0.0).expect("Zero price should always be valid");
                    self.record_strategy_execution(&strategy_name, proportional_pnl)
                        .await;
                }

                // Calculate position value for portfolio tracking
                let position_value = quantity.value() * current_price.value();

                // Open position after successful order execution
                let position_result = self
                    .open_position(symbol, position_side, quantity, current_price)
                    .await;

                match position_result {
                    Ok(position_id) => {
                        // Update portfolio after opening position
                        self.update_portfolio_after_position_open(position_value).await;

                        info!("ORDER EXECUTED & POSITION OPENED via trader {}: {:?} {} {} (confidence: {:.2}) - Order ID: {}, Position ID: {}, Position Value: ${:.2}",
                              trader_id, order_side, quantity, symbol, signal.confidence, order_id, position_id, position_value);
                        Ok(format!("Order executed by trader {}: {:?} {} {} - Order ID: {}, Position ID: {}",
                                  trader_id, order_side, quantity, symbol, order_id, position_id))
                    }
                    Err(e) => {
                        warn!("Order executed by trader {}: {:?} {} {} - Order ID: {} (position tracking failed: {})",
                              trader_id, order_side, quantity, symbol, order_id, e);
                        Ok(format!("Order executed by trader {}: {:?} {} {} - Order ID: {} (position tracking failed: {})",
                                  trader_id, order_side, quantity, symbol, order_id, e))
                    }
                }
            }
            Err(e) => {
                error!("Failed to execute order via trader {}: {}", trader_id, e);
                Err(e)
            }
        }
    }

    /// Select a trader sender following lock ordering convention
    ///
    /// # Lock Ordering
    /// 1. strategy_order (clone and release)
    /// 2. traders (use cloned strategy_order)
    async fn select_trader_sender(
        &self,
    ) -> Option<(String, mpsc::Sender<TraderMessage>)> {
        // Acquire strategy_order lock, clone, and release immediately
        let strategy_names = {
            let guard = self.strategy_order.lock().await;
            guard.clone()
        };

        // Now acquire traders lock using the cloned data
        let traders = self.traders.lock().await;

        // Try to find trader matching strategy order
        for strategy_name in &strategy_names {
            let trader_id = format!("trader_{}", strategy_name.to_lowercase());
            if let Some(sender) = traders.get(&trader_id) {
                return Some((trader_id, sender.clone()));
            }
        }

        // Fallback to first available trader
        traders
            .iter()
            .next()
            .map(|(id, sender)| (id.clone(), sender.clone()))
    }

    /// Check and execute orders for all symbols with signals

    pub async fn check_and_execute_orders(&self) -> Vec<Result<String, MpcError>> {
        let mut results = Vec::new();
        let last_signals = self.get_all_last_signals().await;

        for (symbol, signal) in last_signals {
            match self.execute_order_from_signal(&symbol, &signal).await {
                Ok(msg) => {
                    results.push(Ok(msg));
                }
                Err(e) => {
                    warn!("Failed to execute order for {}: {}", symbol, e);
                    results.push(Err(e));
                }
            }
        }

        results
    }

    /// Check if trading limits are respected

    pub async fn check_trading_limits(&self) -> Result<(), MpcError> {
        let trade_history = self.trade_history.lock().await;
        let now = SystemTime::now();

        // Count trades in the last hour
        let one_hour_ago = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);
        let trades_last_hour = trade_history
            .iter()
            .filter(|(timestamp, _)| *timestamp >= one_hour_ago)
            .count();

        if trades_last_hour >= self.config.max_trades_per_hour {
            return Err(MpcError::InvalidConfiguration(format!(
                "Hourly trade limit exceeded: {} trades in last hour (max: {})",
                trades_last_hour, self.config.max_trades_per_hour
            )));
        }

        // Count trades in the last 24 hours
        let one_day_ago = now.checked_sub(Duration::from_secs(86400)).unwrap_or(now);
        let trades_last_day = trade_history
            .iter()
            .filter(|(timestamp, _)| *timestamp >= one_day_ago)
            .count();

        if trades_last_day >= self.config.max_trades_per_day {
            return Err(MpcError::InvalidConfiguration(format!(
                "Daily trade limit exceeded: {} trades in last 24 hours (max: {})",
                trades_last_day, self.config.max_trades_per_day
            )));
        }

        Ok(())
    }

    /// Get current trading metrics

    pub async fn get_trading_metrics(&self) -> TradingMetrics {
        let metrics = self.trading_metrics.lock().await;
        metrics.clone()
    }

    /// Get current system health metrics

    pub async fn get_system_health(&self) -> SystemHealthMetrics {
        let health = self.system_health.lock().await;
        health.clone()
    }

    /// Record a completed trade in metrics

    pub async fn record_trade(&self, pnl: PnL, volume: f64, latency_ms: f64) {
        let mut metrics = self.trading_metrics.lock().await;
        metrics.record_trade(pnl, volume, latency_ms);
    }

    /// Update unrealized PnL in metrics

    pub async fn update_unrealized_pnl(&self, unrealized_pnl: PnL) {
        let mut metrics = self.trading_metrics.lock().await;
        metrics.update_unrealized_pnl(unrealized_pnl);
    }

    /// Update drawdown metrics

    pub async fn update_drawdown(&self, current_drawdown: Price, max_drawdown: Price) {
        let mut metrics = self.trading_metrics.lock().await;
        metrics.update_drawdown(current_drawdown, max_drawdown);
    }

    /// Update system uptime

    pub async fn update_uptime(&self, uptime: Duration) {
        let mut metrics = self.trading_metrics.lock().await;
        metrics.update_uptime(uptime);
    }

    /// Update exchange connection status

    pub async fn update_exchange_connection(&self, exchange: String, connected: bool) {
        let mut health = self.system_health.lock().await;
        health.update_exchange_connection(exchange, connected);
    }

    /// Update WebSocket health for an exchange

    pub async fn update_websocket_health(
        &self,
        exchange: String,
        time_since_last_message: Duration,
    ) {
        let mut health = self.system_health.lock().await;
        health.update_websocket_health(exchange, time_since_last_message);
    }

    /// Update system resource usage

    pub async fn update_system_resources(&self, memory_mb: f64, cpu_percent: f64) {
        let mut health = self.system_health.lock().await;
        health.update_system_resources(memory_mb, cpu_percent);
    }

    /// Update trading status (active positions, pending orders)

    pub async fn update_trading_status(&self, active_positions: u32, pending_orders: u32) {
        let mut health = self.system_health.lock().await;
        health.update_trading_status(active_positions, pending_orders);
    }

    /// Record an error for health monitoring

    pub async fn record_error(&self) {
        let mut health = self.system_health.lock().await;
        health.record_error();
    }

    /// Check if system is healthy

    pub async fn is_system_healthy(&self) -> bool {
        let health = self.system_health.lock().await;
        health.is_system_healthy()
    }

    /// Record a signal generated by a specific strategy

    pub async fn record_strategy_signal(&self, strategy_name: &str) {
        let mut strategy_metrics = self.strategy_metrics.lock().await;
        if let Some(metrics) = strategy_metrics.get_mut(strategy_name) {
            metrics.record_signal();
        }
    }

    /// Record an execution from a specific strategy with PnL

    pub async fn record_strategy_execution(&self, strategy_name: &str, pnl: Price) {
        let mut strategy_metrics = self.strategy_metrics.lock().await;
        if let Some(metrics) = strategy_metrics.get_mut(strategy_name) {
            metrics.record_execution(pnl);
        }
    }

    /// Adjust strategy weights based on performance metrics

    pub async fn adjust_strategy_weights(&self) -> Result<(), String> {
        let strategy_metrics = {
            let order = {
                let order_guard = self.strategy_order.lock().await;
                order_guard.clone()
            };
            let metrics_guard = self.strategy_metrics.lock().await;
            order
                .iter()
                .map(|name| {
                    metrics_guard
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| StrategyMetrics::new(name.clone()))
                })
                .collect::<Vec<_>>()
        };

        let mut signal_combiner_guard = self.signal_combiner.write().await;
        if let Some(combiner) = signal_combiner_guard.as_mut() {
            combiner.adjust_weights(&strategy_metrics)?;
            info!("Strategy weights adjusted based on performance metrics");
        }

        Ok(())
    }

    /// Get current strategy metrics

    pub async fn get_strategy_metrics(&self) -> HashMap<String, StrategyMetrics> {
        let strategy_metrics = self.strategy_metrics.lock().await;
        strategy_metrics.clone()
    }

    /// Check for new alerts and update active alerts

    pub async fn check_alerts(&self) -> Vec<SystemAlert> {
        let trading_metrics = self.trading_metrics.lock().await;
        let system_health = self.system_health.lock().await;

        let mut new_alerts = Vec::new();

        // Check trading alerts
        new_alerts.extend(trading_metrics.check_alerts(&self.alert_config));

        // Check system health alerts
        new_alerts.extend(system_health.check_alerts(&self.alert_config));

        // Filter out already active alerts (avoid duplicates)
        let mut active_alerts = self.active_alerts.lock().await;
        let existing_alert_types: std::collections::HashSet<_> = active_alerts
            .iter()
            .filter(|alert| !alert.resolved)
            .map(|alert| std::mem::discriminant(&alert.alert_type))
            .collect();

        let filtered_new_alerts: Vec<SystemAlert> = new_alerts
            .into_iter()
            .filter(|alert| {
                !existing_alert_types.contains(&std::mem::discriminant(&alert.alert_type))
            })
            .collect();

        // Add new alerts to active alerts
        for alert in &filtered_new_alerts {
            active_alerts.push(alert.clone());
        }

        filtered_new_alerts
    }

    /// Get active alerts

    pub async fn get_active_alerts(&self) -> Vec<SystemAlert> {
        let active_alerts = self.active_alerts.lock().await;
        active_alerts
            .iter()
            .filter(|alert| !alert.resolved)
            .cloned()
            .collect()
    }

    /// Get performance profiles

    pub async fn get_performance_profiles(
        &self,
    ) -> HashMap<String, crate::domain::services::metrics::PerformanceProfile> {
        let profiler = self.performance_profiler.lock().await;
        profiler.get_all_profiles().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::strategies::{ConservativeScalping, FastScalping, MomentumScalping, Strategy};

    #[test]
    fn test_mpc_service_new() {
        let config = TradingConfig::default();
        let service = MpcService::new(config);
        assert!(service.senders.is_empty());
    }

    #[test]
    fn test_aggregate_prices_single() {
        let prices = vec![Price::new(100.0).unwrap()];
        let avg = MpcService::aggregate_prices(&prices).unwrap();
        assert_eq!(avg.value(), 100.0);
    }

    #[tokio::test]
    async fn test_generate_trading_signal() {
        use crate::domain::services::indicators::Candle;
        use crate::domain::services::strategies::{FastScalping, MomentumScalping};

        let config = TradingConfig::default();
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
            SignalCombiner::new(strategies, weights).expect("Failed to create signal combiner");
        service.set_signal_combiner(combiner).await;

        let candles = vec![
            Candle::new(100.0, 105.0, 95.0, 102.0, 1000.0).unwrap(),
            Candle::new(102.0, 108.0, 98.0, 105.0, 1100.0).unwrap(),
            Candle::new(105.0, 110.0, 100.0, 108.0, 1200.0).unwrap(),
            Candle::new(108.0, 112.0, 103.0, 106.0, 1300.0).unwrap(),
            Candle::new(106.0, 111.0, 102.0, 109.0, 1400.0).unwrap(),
            Candle::new(109.0, 115.0, 105.0, 112.0, 1500.0).unwrap(),
            Candle::new(112.0, 118.0, 108.0, 115.0, 1600.0).unwrap(),
            Candle::new(115.0, 120.0, 110.0, 117.0, 1700.0).unwrap(),
            Candle::new(117.0, 122.0, 112.0, 119.0, 1800.0).unwrap(),
            Candle::new(119.0, 125.0, 115.0, 122.0, 1900.0).unwrap(),
            Candle::new(122.0, 128.0, 118.0, 125.0, 2000.0).unwrap(),
            Candle::new(125.0, 130.0, 120.0, 127.0, 2100.0).unwrap(),
            Candle::new(127.0, 132.0, 122.0, 129.0, 2200.0).unwrap(),
            Candle::new(129.0, 135.0, 125.0, 132.0, 2300.0).unwrap(),
            Candle::new(132.0, 138.0, 128.0, 135.0, 2400.0).unwrap(),
            Candle::new(135.0, 140.0, 130.0, 137.0, 2500.0).unwrap(),
            Candle::new(137.0, 142.0, 132.0, 139.0, 2600.0).unwrap(),
            Candle::new(139.0, 144.0, 134.0, 141.0, 2700.0).unwrap(),
            Candle::new(141.0, 146.0, 136.0, 143.0, 2800.0).unwrap(),
            Candle::new(143.0, 148.0, 138.0, 145.0, 2900.0).unwrap(),
            Candle::new(145.0, 150.0, 140.0, 147.0, 3000.0).unwrap(),
            Candle::new(147.0, 152.0, 142.0, 149.0, 3100.0).unwrap(),
            Candle::new(149.0, 154.0, 144.0, 151.0, 3200.0).unwrap(),
            Candle::new(151.0, 156.0, 146.0, 153.0, 3300.0).unwrap(),
            Candle::new(153.0, 158.0, 148.0, 155.0, 3400.0).unwrap(),
            Candle::new(155.0, 160.0, 150.0, 157.0, 3500.0).unwrap(),
            Candle::new(157.0, 162.0, 152.0, 159.0, 3600.0).unwrap(),
            Candle::new(159.0, 164.0, 154.0, 161.0, 3700.0).unwrap(),
            Candle::new(161.0, 166.0, 156.0, 163.0, 3800.0).unwrap(),
            Candle::new(163.0, 168.0, 158.0, 165.0, 3900.0).unwrap(),
        ];

        let signal = service.generate_trading_signal(&candles).await;
        assert!(signal.is_ok());
        let s = signal.unwrap();
        assert!(s.confidence >= 0.0 && s.confidence <= 1.0);
    }

    #[tokio::test]
    async fn test_mpc_service_with_mock_actor() {
        let config = TradingConfig::default();
        let mut service = MpcService::new(config);

        // Use mock actor for predictable testing
        let mock_price = crate::domain::value_objects::price::Price::new(55000.0).unwrap();
        let sender = crate::infrastructure::adapters::exchange_actor::MockExchangeActor::spawn(
            crate::domain::entities::exchange::Exchange::Binance,
            mock_price.clone(),
        );
        service.add_actor(crate::domain::entities::exchange::Exchange::Binance, sender)
            .expect("Failed to add actor");

        // Get price from mock actor
        let result = service
            .get_price(
                &crate::domain::entities::exchange::Exchange::Binance,
                "BTCUSDT",
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value(), 55000.0);
    }

    #[tokio::test]
    async fn test_subscribe_unsubscribe() {
        let config = TradingConfig::default();
        let mut service = MpcService::new(config);

        let mock_price = crate::domain::value_objects::price::Price::new(50000.0).unwrap();
        let sender = crate::infrastructure::adapters::exchange_actor::MockExchangeActor::spawn(
            crate::domain::entities::exchange::Exchange::Binance,
            mock_price,
        );
        service.add_actor(crate::domain::entities::exchange::Exchange::Binance, sender)
            .expect("Failed to add actor");

        // Test subscribe
        let result = service
            .subscribe(
                &crate::domain::entities::exchange::Exchange::Binance,
                "BTCUSDT",
            )
            .await;
        assert!(result.is_ok());

        // Test unsubscribe
        let result = service
            .unsubscribe(
                &crate::domain::entities::exchange::Exchange::Binance,
                "BTCUSDT",
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_subscriptions() {
        let config = TradingConfig::default();
        let mut service = MpcService::new(config);

        let mock_price = crate::domain::value_objects::price::Price::new(50000.0).unwrap();
        let sender = crate::infrastructure::adapters::exchange_actor::MockExchangeActor::spawn(
            crate::domain::entities::exchange::Exchange::Binance,
            mock_price,
        );
        service.add_actor(crate::domain::entities::exchange::Exchange::Binance, sender)
            .expect("Failed to add actor");

        // Get subscriptions
        let result = service
            .get_subscriptions(&crate::domain::entities::exchange::Exchange::Binance)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0); // Mock actor returns empty list
    }

    #[tokio::test]
    async fn test_adjust_strategy_weights_respects_strategy_order() {
        let config = TradingConfig::default();
        let service = MpcService::new(config);

        let strategies: Vec<(String, Box<dyn Strategy + Send + Sync>)> = vec![
            ("FastScalping".to_string(), Box::new(FastScalping::new())),
            (
                "MomentumScalping".to_string(),
                Box::new(MomentumScalping::new()),
            ),
            (
                "ConservativeScalping".to_string(),
                Box::new(ConservativeScalping::new()),
            ),
        ];
        let weights = vec![0.4, 0.4, 0.2];
        let combiner =
            SignalCombiner::new(strategies, weights).expect("Failed to create signal combiner");
        service.set_signal_combiner(combiner).await;

        {
            let mut metrics = service.strategy_metrics.lock().await;
            let mut reordered = std::collections::HashMap::new();
            if let Some(conservative) = metrics.remove("ConservativeScalping") {
                reordered.insert("ConservativeScalping".to_string(), conservative);
            }
            if let Some(momentum) = metrics.remove("MomentumScalping") {
                reordered.insert("MomentumScalping".to_string(), momentum);
            }
            if let Some(fast) = metrics.remove("FastScalping") {
                reordered.insert("FastScalping".to_string(), fast);
            }
            for metrics in reordered.values_mut() {
                metrics.performance_score = 0.1;
            }
            if let Some(fast) = reordered.get_mut("FastScalping") {
                fast.performance_score = 0.9;
            }
            if let Some(momentum) = reordered.get_mut("MomentumScalping") {
                momentum.performance_score = 0.3;
            }
            *metrics = reordered;
        }

        service
            .adjust_strategy_weights()
            .await
            .expect("weight adjustment should succeed");

        let weights = {
            let guard = service.signal_combiner.read().await;
            guard.as_ref().unwrap().weights().to_vec()
        };

        assert!(
            weights[0] > weights[1],
            "FastScalping weight should be higher than MomentumScalping"
        );
        assert!(
            weights[1] > weights[2],
            "MomentumScalping weight should be higher than ConservativeScalping"
        );
    }
}
