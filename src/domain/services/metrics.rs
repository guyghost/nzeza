use crate::domain::value_objects::price::Price;
use std::time::{Duration, SystemTime};
use std::collections::HashMap;

/// Performance metrics for the trading system
#[derive(Debug, Clone)]
pub struct TradingMetrics {
    /// Total realized PnL across all closed positions
    pub total_realized_pnl: Price,
    /// Total unrealized PnL across all open positions
    pub total_unrealized_pnl: Price,
    /// Number of winning trades
    pub winning_trades: u32,
    /// Number of losing trades
    pub losing_trades: u32,
    /// Total number of trades executed
    pub total_trades: u32,
    /// Win rate as percentage (0.0 to 100.0)
    pub win_rate: f64,
    /// Average profit per winning trade
    pub avg_win: Price,
    /// Average loss per losing trade
    pub avg_loss: Price,
    /// Profit factor (gross profit / gross loss)
    pub profit_factor: f64,
    /// Maximum drawdown experienced
    pub max_drawdown: Price,
    /// Current drawdown
    pub current_drawdown: Price,
    /// Sharpe ratio (risk-adjusted returns)
    pub sharpe_ratio: f64,
    /// Total trading volume in base currency
    pub total_volume: f64,
    /// Average trade execution latency in milliseconds
    pub avg_trade_latency_ms: f64,
    /// System uptime
    pub uptime: Duration,
    /// Last update timestamp
    pub last_updated: SystemTime,
}

impl Default for TradingMetrics {
    fn default() -> Self {
        Self {
            total_realized_pnl: Price::new(0.0).unwrap(),
            total_unrealized_pnl: Price::new(0.0).unwrap(),
            winning_trades: 0,
            losing_trades: 0,
            total_trades: 0,
            win_rate: 0.0,
            avg_win: Price::new(0.0).unwrap(),
            avg_loss: Price::new(0.0).unwrap(),
            profit_factor: 0.0,
            max_drawdown: Price::new(0.0).unwrap(),
            current_drawdown: Price::new(0.0).unwrap(),
            sharpe_ratio: 0.0,
            total_volume: 0.0,
            avg_trade_latency_ms: 0.0,
            uptime: Duration::from_secs(0),
            last_updated: SystemTime::now(),
        }
    }
}

impl TradingMetrics {
    /// Create new trading metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Update metrics with a new trade result
    pub fn record_trade(&mut self, pnl: Price, volume: f64, latency_ms: f64) {
        self.total_trades += 1;
        self.total_volume += volume;
        self.total_realized_pnl = Price::new(self.total_realized_pnl.value() + pnl.value()).unwrap();

        // Update trade counts and averages
        if pnl.value() > 0.0 {
            self.winning_trades += 1;
            let total_win_pnl = self.avg_win.value() * (self.winning_trades - 1) as f64 + pnl.value();
            self.avg_win = Price::new(total_win_pnl / self.winning_trades as f64).unwrap();
        } else if pnl.value() < 0.0 {
            self.losing_trades += 1;
            let total_loss_pnl = self.avg_loss.value() * (self.losing_trades - 1) as f64 + pnl.value().abs();
            self.avg_loss = Price::new(total_loss_pnl / self.losing_trades as f64).unwrap();
        }

        // Update win rate
        if self.total_trades > 0 {
            self.win_rate = (self.winning_trades as f64 / self.total_trades as f64) * 100.0;
        }

        // Update profit factor
        if self.avg_loss.value() > 0.0 {
            let gross_profit = self.avg_win.value() * self.winning_trades as f64;
            let gross_loss = self.avg_loss.value() * self.losing_trades as f64;
            if gross_loss > 0.0 {
                self.profit_factor = gross_profit / gross_loss;
            }
        }

        // Update latency (rolling average)
        if self.total_trades == 1 {
            self.avg_trade_latency_ms = latency_ms;
        } else {
            self.avg_trade_latency_ms = (self.avg_trade_latency_ms * (self.total_trades - 1) as f64 + latency_ms) / self.total_trades as f64;
        }

        self.last_updated = SystemTime::now();
    }

    /// Update unrealized PnL
    pub fn update_unrealized_pnl(&mut self, unrealized_pnl: Price) {
        self.total_unrealized_pnl = unrealized_pnl;
        self.last_updated = SystemTime::now();
    }

    /// Update drawdown metrics
    pub fn update_drawdown(&mut self, current_drawdown: Price, max_drawdown: Price) {
        self.current_drawdown = current_drawdown;
        if max_drawdown.value() > self.max_drawdown.value() {
            self.max_drawdown = max_drawdown;
        }
        self.last_updated = SystemTime::now();
    }

    /// Update Sharpe ratio
    pub fn update_sharpe_ratio(&mut self, sharpe_ratio: f64) {
        self.sharpe_ratio = sharpe_ratio;
        self.last_updated = SystemTime::now();
    }

    /// Update system uptime
    pub fn update_uptime(&mut self, uptime: Duration) {
        self.uptime = uptime;
    }

    /// Get current equity (realized + unrealized PnL)
    pub fn current_equity(&self) -> Price {
        Price::new(self.total_realized_pnl.value() + self.total_unrealized_pnl.value()).unwrap()
    }

    /// Get expectancy (average win * win rate - average loss * loss rate)
    pub fn expectancy(&self) -> Price {
        let win_rate_decimal = self.win_rate / 100.0;
        let loss_rate_decimal = 1.0 - win_rate_decimal;
        let expectancy_value = self.avg_win.value() * win_rate_decimal - self.avg_loss.value() * loss_rate_decimal;
        Price::new(expectancy_value).unwrap()
    }
}

/// Strategy performance metrics
#[derive(Debug, Clone)]
pub struct StrategyMetrics {
    /// Strategy name
    pub strategy_name: String,
    /// Number of signals generated
    pub signals_generated: u32,
    /// Number of signals that led to trades
    pub signals_executed: u32,
    /// Execution rate as percentage
    pub execution_rate: f64,
    /// Strategy-specific PnL
    pub strategy_pnl: Price,
    /// Strategy weight in signal combiner
    pub current_weight: f64,
    /// Performance score (0.0 to 1.0)
    pub performance_score: f64,
    /// Last update timestamp
    pub last_updated: SystemTime,
}

impl StrategyMetrics {
    pub fn new(strategy_name: String) -> Self {
        Self {
            strategy_name,
            signals_generated: 0,
            signals_executed: 0,
            execution_rate: 0.0,
            strategy_pnl: Price::new(0.0).unwrap(),
            current_weight: 0.0,
            performance_score: 0.5, // Start neutral
            last_updated: SystemTime::now(),
        }
    }

    pub fn record_signal(&mut self) {
        self.signals_generated += 1;
        self.update_execution_rate();
        self.last_updated = SystemTime::now();
    }

    pub fn record_execution(&mut self, pnl: Price) {
        self.signals_executed += 1;
        self.strategy_pnl = Price::new(self.strategy_pnl.value() + pnl.value()).unwrap();
        self.update_execution_rate();
        self.update_performance_score();
        self.last_updated = SystemTime::now();
    }

    pub fn update_weight(&mut self, new_weight: f64) {
        self.current_weight = new_weight;
        self.last_updated = SystemTime::now();
    }

    fn update_execution_rate(&mut self) {
        if self.signals_generated > 0 {
            self.execution_rate = (self.signals_executed as f64 / self.signals_generated as f64) * 100.0;
        }
    }

    fn update_performance_score(&mut self) {
        // Simple performance score based on PnL and execution rate
        let pnl_score = if self.strategy_pnl.value() > 0.0 {
            0.6 + (self.strategy_pnl.value().min(1000.0) / 1000.0) * 0.4
        } else {
            0.4 - (self.strategy_pnl.value().abs().min(1000.0) / 1000.0) * 0.4
        };

        let execution_score = (self.execution_rate / 100.0) * 0.4;

        self.performance_score = pnl_score + execution_score;
        self.performance_score = self.performance_score.max(0.0).min(1.0);
    }
}

/// System health metrics
#[derive(Debug, Clone)]
pub struct SystemHealthMetrics {
    /// Exchange connection status
    pub exchange_connections: HashMap<String, bool>,
    /// WebSocket connection health
    pub websocket_health: HashMap<String, Duration>, // Time since last message
    /// Memory usage (approximate)
    pub memory_usage_mb: f64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Active positions count
    pub active_positions: u32,
    /// Pending orders count
    pub pending_orders: u32,
    /// Error rate (errors per minute)
    pub error_rate_per_minute: f64,
    /// Last health check timestamp
    pub last_health_check: SystemTime,
}

impl Default for SystemHealthMetrics {
    fn default() -> Self {
        Self {
            exchange_connections: HashMap::new(),
            websocket_health: HashMap::new(),
            memory_usage_mb: 0.0,
            cpu_usage_percent: 0.0,
            active_positions: 0,
            pending_orders: 0,
            error_rate_per_minute: 0.0,
            last_health_check: SystemTime::now(),
        }
    }
}

impl SystemHealthMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_exchange_connection(&mut self, exchange: String, connected: bool) {
        self.exchange_connections.insert(exchange, connected);
        self.last_health_check = SystemTime::now();
    }

    pub fn update_websocket_health(&mut self, exchange: String, time_since_last_message: Duration) {
        self.websocket_health.insert(exchange, time_since_last_message);
        self.last_health_check = SystemTime::now();
    }

    pub fn update_system_resources(&mut self, memory_mb: f64, cpu_percent: f64) {
        self.memory_usage_mb = memory_mb;
        self.cpu_usage_percent = cpu_percent;
        self.last_health_check = SystemTime::now();
    }

    pub fn update_trading_status(&mut self, active_positions: u32, pending_orders: u32) {
        self.active_positions = active_positions;
        self.pending_orders = pending_orders;
        self.last_health_check = SystemTime::now();
    }

    pub fn record_error(&mut self) {
        // Simple error counting - in production, you'd want time-windowed error rates
        self.error_rate_per_minute += 1.0;
        self.last_health_check = SystemTime::now();
    }

    pub fn is_system_healthy(&self) -> bool {
        // Basic health check criteria
        let connected_exchanges = self.exchange_connections.values().filter(|&&connected| connected).count();
        let total_exchanges = self.exchange_connections.len();

        // At least 50% of exchanges connected
        let exchange_health = connected_exchanges >= (total_exchanges + 1) / 2;

        // Memory usage under 1GB
        let memory_health = self.memory_usage_mb < 1000.0;

        // CPU usage under 90%
        let cpu_health = self.cpu_usage_percent < 90.0;

        exchange_health && memory_health && cpu_health
    }
}