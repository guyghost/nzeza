//! Reconciliation Actor
//!
//! This actor manages portfolio reconciliation operations asynchronously.
//! It handles scheduling, retries, and coordination between different exchanges.

use crate::domain::entities::exchange::Exchange;
use crate::domain::services::reconciliation::*;
use crate::infrastructure::coinbase_client::CoinbaseClient;
use crate::infrastructure::dydx_client::DydxClient;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// Channel capacity for reconciliation actor messages
const RECONCILIATION_CHANNEL_CAPACITY: usize = 100;

/// Messages that can be sent to the reconciliation actor
#[derive(Debug)]
pub enum ReconciliationMessage {
    /// Trigger reconciliation for a specific exchange
    ReconcileExchange {
        exchange: Exchange,
        reply: mpsc::Sender<Result<ReconciliationReport, ReconciliationError>>,
    },

    /// Trigger reconciliation for all configured exchanges
    ReconcileAll {
        reply: mpsc::Sender<Vec<ReconciliationReport>>,
    },

    /// Get status of last reconciliation
    GetStatus {
        reply: mpsc::Sender<HashMap<Exchange, Option<ReconciliationReport>>>,
    },

    /// Shutdown the actor
    Shutdown,
}

/// Reconciliation actor statistics
#[derive(Debug, Clone)]
pub struct ReconciliationStats {
    pub total_reconciliations: u64,
    pub successful_reconciliations: u64,
    pub failed_reconciliations: u64,
    pub last_reconciliation: Option<DateTime<Utc>>,
}

/// Reconciliation Actor
pub struct ReconciliationActor {
    coinbase_client: Option<Arc<CoinbaseClient>>,
    dydx_client: Option<Arc<DydxClient>>,
    config: ReconciliationConfig,
    stats: ReconciliationStats,
    last_reports: HashMap<Exchange, ReconciliationReport>,
}

impl ReconciliationActor {
    /// Create a new reconciliation actor
    pub fn new(
        coinbase_client: Option<Arc<CoinbaseClient>>,
        dydx_client: Option<Arc<DydxClient>>,
        config: ReconciliationConfig,
    ) -> Self {
        Self {
            coinbase_client,
            dydx_client,
            config,
            stats: ReconciliationStats {
                total_reconciliations: 0,
                successful_reconciliations: 0,
                failed_reconciliations: 0,
                last_reconciliation: None,
            },
            last_reports: HashMap::new(),
        }
    }

    /// Spawn a new reconciliation actor
    pub fn spawn(
        coinbase_client: Option<Arc<CoinbaseClient>>,
        dydx_client: Option<Arc<DydxClient>>,
        config: ReconciliationConfig,
    ) -> mpsc::Sender<ReconciliationMessage> {
        let (tx, rx) = mpsc::channel(RECONCILIATION_CHANNEL_CAPACITY);

        let actor = Self::new(coinbase_client, dydx_client, config);

        tokio::spawn(async move {
            actor.run(rx).await;
        });

        info!("ReconciliationActor spawned");
        tx
    }

    /// Main actor loop
    async fn run(mut self, mut rx: mpsc::Receiver<ReconciliationMessage>) {
        info!("ReconciliationActor started");

        while let Some(msg) = rx.recv().await {
            match msg {
                ReconciliationMessage::ReconcileExchange { exchange, reply } => {
                    debug!(
                        "ReconciliationActor received ReconcileExchange for {:?}",
                        exchange
                    );
                    let result = self.reconcile_exchange(&exchange).await;
                    self.update_stats(&result);
                    if let Err(e) = reply.send(result).await {
                        error!("Failed to send ReconcileExchange reply: {:?}", e);
                    }
                }

                ReconciliationMessage::ReconcileAll { reply } => {
                    debug!("ReconciliationActor received ReconcileAll");
                    let reports = self.reconcile_all().await;
                    for report in &reports {
                        self.last_reports
                            .insert(report.exchange.clone(), report.clone());
                    }
                    if let Err(e) = reply.send(reports).await {
                        error!("Failed to send ReconcileAll reply: {:?}", e);
                    }
                }

                ReconciliationMessage::GetStatus { reply } => {
                    debug!("ReconciliationActor received GetStatus");
                    let status: HashMap<Exchange, Option<ReconciliationReport>> = self
                        .last_reports
                        .iter()
                        .map(|(exchange, report)| (exchange.clone(), Some(report.clone())))
                        .collect();
                    if let Err(e) = reply.send(status).await {
                        error!("Failed to send GetStatus reply: {:?}", e);
                    }
                }

                ReconciliationMessage::Shutdown => {
                    info!("ReconciliationActor received shutdown signal");
                    break;
                }
            }
        }

        info!("ReconciliationActor stopped");
    }

    /// Reconcile a specific exchange
    async fn reconcile_exchange(
        &self,
        exchange: &Exchange,
    ) -> Result<ReconciliationReport, ReconciliationError> {
        let service = self.create_service_for_exchange(exchange)?;

        // Apply timeout
        let timeout_duration = Duration::from_millis(self.config.timeout_milliseconds);
        let result = timeout(timeout_duration, service.reconcile(exchange.clone())).await;

        match result {
            Ok(Ok(report)) => {
                info!("Successfully reconciled {:?}", exchange);
                Ok(report)
            }
            Ok(Err(e)) => {
                warn!("Reconciliation failed for {:?}: {:?}", exchange, e);
                Err(e)
            }
            Err(_) => {
                warn!("Reconciliation timed out for {:?}", exchange);
                Err(ReconciliationError::NetworkTimeout)
            }
        }
    }

    /// Reconcile all configured exchanges
    async fn reconcile_all(&self) -> Vec<ReconciliationReport> {
        let mut reports = Vec::new();
        let exchanges = vec![Exchange::Coinbase, Exchange::Dydx];

        for exchange in exchanges {
            match self.reconcile_exchange(&exchange).await {
                Ok(report) => reports.push(report),
                Err(e) => {
                    warn!("Failed to reconcile {:?}: {:?}", exchange, e);
                    // Create error report
                    let mut error_report = ReconciliationReport::new(exchange);
                    error_report.status = ReconciliationStatus::Critical;
                    reports.push(error_report);
                }
            }
        }

        reports
    }

    /// Create appropriate service for exchange
    fn create_service_for_exchange(
        &self,
        exchange: &Exchange,
    ) -> Result<Box<dyn PortfolioReconciliationService>, ReconciliationError> {
        match exchange {
            Exchange::Coinbase => {
                if let Some(client) = &self.coinbase_client {
                    let reconciler = CoinbaseReconciler::new(client.clone(), self.config.clone());
                    Ok(Box::new(reconciler))
                } else {
                    Err(ReconciliationError::ApiError(
                        "Coinbase client not configured".to_string(),
                    ))
                }
            }
            Exchange::Dydx => {
                if let Some(client) = &self.dydx_client {
                    let reconciler = DydxReconciler::new(client.clone(), self.config.clone());
                    Ok(Box::new(reconciler))
                } else {
                    Err(ReconciliationError::ApiError(
                        "dYdX client not configured".to_string(),
                    ))
                }
            }
            Exchange::Hyperliquid | Exchange::Binance | Exchange::Kraken => {
                Err(ReconciliationError::ApiError(format!(
                    "{:?} exchange not yet supported for reconciliation",
                    exchange
                )))
            }
        }
    }

    /// Update actor statistics
    fn update_stats(&mut self, result: &Result<ReconciliationReport, ReconciliationError>) {
        self.stats.total_reconciliations += 1;
        self.stats.last_reconciliation = Some(Utc::now());

        match result {
            Ok(report) => {
                self.stats.successful_reconciliations += 1;
                self.last_reports
                    .insert(report.exchange.clone(), report.clone());
            }
            Err(_) => {
                self.stats.failed_reconciliations += 1;
            }
        }
    }
}
