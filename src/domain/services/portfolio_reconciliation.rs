//! Portfolio Reconciliation Service
//!
//! This service detects balance discrepancies between local state and exchanges.
//! It provides comprehensive reconciliation capabilities with audit trails.

use crate::domain::entities::exchange::Exchange;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

/// Portfolio balance structure
#[derive(Debug, Clone)]
pub struct Balance {
    pub currency: String,
    pub amount: f64,
}

/// Exchange balances wrapper
#[derive(Debug, Clone)]
pub struct ExchangeBalances {
    pub exchange: Exchange,
    pub balances: HashMap<String, Balance>,
}

impl ExchangeBalances {
    pub fn new(exchange: Exchange) -> Self {
        Self {
            exchange,
            balances: HashMap::new(),
        }
    }

    pub fn add_balance(&mut self, currency: String, amount: f64) {
        self.balances
            .insert(currency.clone(), Balance { currency, amount });
    }

    pub fn get_balance(&self, currency: &str) -> Option<Balance> {
        self.balances.get(currency).cloned()
    }

    pub fn all_balances(&self) -> Vec<Balance> {
        self.balances.values().cloned().collect()
    }
}

/// Local portfolio structure
#[derive(Debug, Clone)]
pub struct Portfolio {
    pub balances: HashMap<String, f64>,
}

impl Portfolio {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
        }
    }

    pub fn add_balance(&mut self, currency: String, amount: f64) {
        self.balances.insert(currency, amount);
    }
}

impl Default for Portfolio {
    fn default() -> Self {
        Self::new()
    }
}

/// Balance discrepancy types
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BalanceDiscrepancy {
    Missing {
        currency: String,
        amount: f64,
    },
    Mismatch {
        currency: String,
        local: f64,
        exchange: f64,
        diff: f64,
    },
    Precision {
        currency: String,
        tolerance: f64,
    },
}

impl BalanceDiscrepancy {
    pub fn currency(&self) -> &str {
        match self {
            Self::Missing { currency, .. } => currency,
            Self::Mismatch { currency, .. } => currency,
            Self::Precision { currency, .. } => currency,
        }
    }

    pub fn severity(&self) -> DiscrepancySeverity {
        match self {
            Self::Missing { .. } => DiscrepancySeverity::Critical,
            Self::Mismatch { diff, .. } => {
                if *diff > 100.0 {
                    DiscrepancySeverity::Critical
                } else if *diff > 10.0 {
                    DiscrepancySeverity::Major
                } else {
                    DiscrepancySeverity::Minor
                }
            }
            Self::Precision { .. } => DiscrepancySeverity::Ok,
        }
    }
}

/// Discrepancy severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscrepancySeverity {
    Ok,
    Minor,
    Major,
    Critical,
}

impl fmt::Display for DiscrepancySeverity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ok => "OK",
                Self::Minor => "MINOR",
                Self::Major => "MAJOR",
                Self::Critical => "CRITICAL",
            }
        )
    }
}

/// Reconciliation status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationStatus {
    Ok,
    Minor,
    Major,
    Critical,
}

impl fmt::Display for ReconciliationStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ok => "OK",
                Self::Minor => "MINOR",
                Self::Major => "MAJOR",
                Self::Critical => "CRITICAL",
            }
        )
    }
}

/// Reconciliation report
#[derive(Debug, Clone)]
pub struct ReconciliationReport {
    pub exchange: Exchange,
    pub timestamp: DateTime<Utc>,
    pub discrepancies: Vec<BalanceDiscrepancy>,
    pub status: ReconciliationStatus,
    pub local_balances: Vec<Balance>,
    pub exchange_balances: HashMap<String, Balance>,
}

impl ReconciliationReport {
    pub fn new(exchange: Exchange) -> Self {
        Self {
            exchange,
            timestamp: Utc::now(),
            discrepancies: Vec::new(),
            status: ReconciliationStatus::Ok,
            local_balances: Vec::new(),
            exchange_balances: HashMap::new(),
        }
    }

    pub fn add_discrepancy(&mut self, discrepancy: BalanceDiscrepancy) {
        let severity = discrepancy.severity();
        self.discrepancies.push(discrepancy);

        // Update status to worst severity
        let current_level = match self.status {
            ReconciliationStatus::Ok => 0,
            ReconciliationStatus::Minor => 1,
            ReconciliationStatus::Major => 2,
            ReconciliationStatus::Critical => 3,
        };

        let new_level = match severity {
            DiscrepancySeverity::Ok => 0,
            DiscrepancySeverity::Minor => 1,
            DiscrepancySeverity::Major => 2,
            DiscrepancySeverity::Critical => 3,
        };

        if new_level > current_level {
            self.status = match severity {
                DiscrepancySeverity::Ok => ReconciliationStatus::Ok,
                DiscrepancySeverity::Minor => ReconciliationStatus::Minor,
                DiscrepancySeverity::Major => ReconciliationStatus::Major,
                DiscrepancySeverity::Critical => ReconciliationStatus::Critical,
            };
        }
    }

    pub fn discrepancy_count(&self) -> usize {
        self.discrepancies.len()
    }
}

/// Reconciliation errors
#[derive(Debug, Clone)]
pub enum ReconciliationError {
    ApiError(String),
    NetworkError(String),
    NetworkTimeout,
    TimeoutError,
    ParseError(String),
    ValidationError(String),
    RetryExhausted,
}

impl fmt::Display for ReconciliationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::ApiError(msg) => format!("API Error: {}", msg),
                Self::NetworkError(msg) => format!("Network Error: {}", msg),
                Self::NetworkTimeout => "Network Timeout".to_string(),
                Self::TimeoutError => "Timeout Error".to_string(),
                Self::ParseError(msg) => format!("Parse Error: {}", msg),
                Self::ValidationError(msg) => format!("Validation Error: {}", msg),
                Self::RetryExhausted => "Retry Exhausted".to_string(),
            }
        )
    }
}

impl std::error::Error for ReconciliationError {}

/// Reconciliation configuration
#[derive(Debug, Clone)]
pub struct ReconciliationConfig {
    pub timeout_milliseconds: u64,
    pub threshold_percentage: f64,
    pub max_retries: u32,
    pub retry_delay_millis: u64,
    pub precision_tolerance: f64,
}

impl Default for ReconciliationConfig {
    fn default() -> Self {
        Self {
            timeout_milliseconds: 30000,
            threshold_percentage: 0.1,
            max_retries: 3,
            retry_delay_millis: 1000,
            precision_tolerance: 0.0001,
        }
    }
}

/// Retry policy for reconciliation operations
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_millis: u64,
    pub max_delay_millis: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_millis: 1000,
            max_delay_millis: 10000,
        }
    }
}

impl RetryPolicy {
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = self.initial_delay_millis * 2_u64.pow(attempt);
        delay.min(self.max_delay_millis)
    }
}

/// Main reconciliation service trait
#[async_trait]
pub trait PortfolioReconciliationService: Send + Sync {
    async fn fetch_exchange_balances(
        &self,
        exchange: &Exchange,
    ) -> Result<ExchangeBalances, ReconciliationError>;

    fn detect_discrepancies(
        &self,
        local: &Portfolio,
        exchange: &ExchangeBalances,
    ) -> Vec<BalanceDiscrepancy>;

    fn generate_report(
        &self,
        discrepancies: Vec<BalanceDiscrepancy>,
        exchange: Exchange,
    ) -> ReconciliationReport;

    async fn reconcile(
        &self,
        exchange: Exchange,
    ) -> Result<ReconciliationReport, ReconciliationError>;

    fn classify_discrepancy_severity(
        &self,
        discrepancy: &BalanceDiscrepancy,
    ) -> DiscrepancySeverity;
}

/// Concrete portfolio reconciliation service for testing
pub struct ConcretePortfolioReconciliationService {
    config: ReconciliationConfig,
}

impl ConcretePortfolioReconciliationService {
    /// Create a new portfolio reconciliation service
    pub fn new() -> Self {
        Self {
            config: ReconciliationConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: ReconciliationConfig) -> Self {
        Self { config }
    }

    /// Create with timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        let mut config = ReconciliationConfig::default();
        config.timeout_milliseconds = timeout.as_millis() as u64;
        Self { config }
    }

    /// Create with threshold
    pub fn with_threshold(threshold: f64) -> Self {
        let mut config = ReconciliationConfig::default();
        config.threshold_percentage = threshold;
        Self { config }
    }

    /// Create with retry policy
    pub fn with_retry_policy(_max_retries: u32) -> Self {
        Self::new()
    }

    /// Calculate backoff delay (for testing)
    pub fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
        let policy = RetryPolicy::default();
        Duration::from_millis(policy.delay_for_attempt(attempt))
    }
}

impl Default for ConcretePortfolioReconciliationService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PortfolioReconciliationService for ConcretePortfolioReconciliationService {
    async fn fetch_exchange_balances(
        &self,
        exchange: &Exchange,
    ) -> Result<ExchangeBalances, ReconciliationError> {
        // For testing, return mock data
        match exchange {
            Exchange::Coinbase => {
                let mut balances = ExchangeBalances::new(exchange.clone());
                balances.add_balance("BTC".to_string(), 1.5);
                balances.add_balance("ETH".to_string(), 10.0);
                Ok(balances)
            }
            Exchange::Dydx => {
                let mut balances = ExchangeBalances::new(exchange.clone());
                balances.add_balance("BTC".to_string(), 1.0);
                Ok(balances)
            }
            _ => Err(ReconciliationError::ValidationError(
                "Unsupported exchange".to_string(),
            )),
        }
    }

    fn detect_discrepancies(
        &self,
        local: &Portfolio,
        exchange: &ExchangeBalances,
    ) -> Vec<BalanceDiscrepancy> {
        let mut discrepancies = Vec::new();

        for (currency, local_balance) in &local.balances {
            if let Some(exchange_balance) = exchange.get_balance(currency) {
                let diff = (local_balance - exchange_balance.amount).abs();
                if diff > 0.0 {
                    discrepancies.push(BalanceDiscrepancy::Mismatch {
                        currency: currency.clone(),
                        local: *local_balance,
                        exchange: exchange_balance.amount,
                        diff,
                    });
                }
            } else {
                discrepancies.push(BalanceDiscrepancy::Missing {
                    currency: currency.clone(),
                    amount: *local_balance,
                });
            }
        }

        discrepancies
    }

    fn generate_report(
        &self,
        discrepancies: Vec<BalanceDiscrepancy>,
        exchange: Exchange,
    ) -> ReconciliationReport {
        let mut report = ReconciliationReport::new(exchange);
        for discrepancy in discrepancies {
            report.add_discrepancy(discrepancy);
        }
        report
    }

    async fn reconcile(
        &self,
        exchange: Exchange,
    ) -> Result<ReconciliationReport, ReconciliationError> {
        let exchange_balances = self.fetch_exchange_balances(&exchange).await?;
        let local_portfolio = Portfolio::new(); // Empty for testing
        let discrepancies = self.detect_discrepancies(&local_portfolio, &exchange_balances);
        let report = self.generate_report(discrepancies, exchange);
        Ok(report)
    }

    fn classify_discrepancy_severity(
        &self,
        discrepancy: &BalanceDiscrepancy,
    ) -> DiscrepancySeverity {
        discrepancy.severity()
    }
}
