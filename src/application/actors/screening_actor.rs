use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::{error, info};

use crate::domain::entities::symbol_screening::SymbolScreeningResult;
use crate::domain::services::indicators::Candle;
use crate::domain::services::symbol_screening::{SymbolMarketData, SymbolScreeningService};
use crate::persistence::screening_repository::ScreeningRepository;

/// Message for the screening actor
pub enum ScreeningMessage {
    /// Trigger screening for all symbols
    ScreenAll,
    /// Trigger screening for specific symbols
    ScreenSymbols(Vec<String>),
    /// Shutdown the actor
    Shutdown,
}

/// Results from screening
#[derive(Debug, Clone)]
pub struct ScreeningResults {
    pub exchange: String,
    pub results: Vec<SymbolScreeningResult>,
}

/// Actor that periodically screens symbols for scalping potential
pub struct ScreeningActor {
    service: SymbolScreeningService,
    repository: Option<ScreeningRepository>,
    exchange: String,
    screening_interval: Duration,
    tx: mpsc::Sender<ScreeningResults>,
}

impl ScreeningActor {
    pub fn new(
        exchange: String,
        screening_interval: Duration,
        tx: mpsc::Sender<ScreeningResults>,
    ) -> Self {
        ScreeningActor {
            service: SymbolScreeningService::with_default_cache_ttl(),
            repository: None,
            exchange,
            screening_interval,
            tx,
        }
    }

    pub fn with_repository(mut self, repository: ScreeningRepository) -> Self {
        self.repository = Some(repository);
        self
    }

    /// Run the screening actor
    pub async fn run(mut self, mut rx: mpsc::Receiver<ScreeningMessage>) {
        info!(
            "Starting screening actor for {} with interval {:?}",
            self.exchange, self.screening_interval
        );

        let mut ticker = interval(self.screening_interval);

        loop {
            tokio::select! {
                // Periodic screening
                _ = ticker.tick() => {
                    self.perform_screening(None).await;
                }

                // Handle incoming messages
                msg = rx.recv() => {
                    match msg {
                        Some(ScreeningMessage::ScreenAll) => {
                            self.perform_screening(None).await;
                        }
                        Some(ScreeningMessage::ScreenSymbols(symbols)) => {
                            self.perform_screening(Some(symbols)).await;
                        }
                        Some(ScreeningMessage::Shutdown) => {
                            info!("Screening actor shutting down");
                            break;
                        }
                        None => {
                            error!("Screening actor message channel closed");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Perform screening (to be called with actual market data)
    async fn perform_screening(&self, _symbols_filter: Option<Vec<String>>) {
        // In production, this would:
        // 1. Get current market data from exchanges
        // 2. Screen symbols
        // 3. Persist results
        // 4. Send via tx channel

        // For now, this is a placeholder for the full implementation
        // that would integrate with exchange clients
    }
}

/// Spawn a screening actor for an exchange
pub async fn spawn_screening_actor(
    exchange: String,
    repository: Option<ScreeningRepository>,
    screening_interval: Duration,
) -> (
    mpsc::Sender<ScreeningMessage>,
    mpsc::Receiver<ScreeningResults>,
) {
    let (tx, rx) = mpsc::channel::<ScreeningResults>(100);
    let (msg_tx, msg_rx) = mpsc::channel::<ScreeningMessage>(50);

    let mut actor = ScreeningActor::new(exchange, screening_interval, tx);
    if let Some(repo) = repository {
        actor = actor.with_repository(repo);
    }

    tokio::spawn(async move {
        actor.run(msg_rx).await;
    });

    (msg_tx, rx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_screening_actor_spawning() {
        let (msg_tx, mut results_rx) =
            spawn_screening_actor("dydx".to_string(), None, Duration::from_secs(1)).await;

        // Verify actor is running by having a channel
        assert!(!msg_tx.is_closed());

        // Shutdown
        msg_tx.send(ScreeningMessage::Shutdown).await.unwrap();

        // Give actor time to shutdown
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(msg_tx.is_closed());
    }

    #[tokio::test]
    async fn test_screening_actor_message_handling() {
        let (msg_tx, _results_rx) = spawn_screening_actor(
            "dydx".to_string(),
            None,
            Duration::from_secs(10), // Long interval
        )
        .await;

        // Send ScreenAll message
        let result = msg_tx.send(ScreeningMessage::ScreenAll).await;
        assert!(result.is_ok());

        // Send ScreenSymbols message
        let result = msg_tx
            .send(ScreeningMessage::ScreenSymbols(vec!["BTC-USD".to_string()]))
            .await;
        assert!(result.is_ok());

        // Shutdown
        msg_tx.send(ScreeningMessage::Shutdown).await.unwrap();
    }

    #[tokio::test]
    async fn test_screening_actor_shutdown_and_cleanup() {
        let (msg_tx, _results_rx) =
            spawn_screening_actor("dydx".to_string(), None, Duration::from_secs(10)).await;

        assert!(!msg_tx.is_closed());

        msg_tx.send(ScreeningMessage::Shutdown).await.unwrap();

        // Give actor time to shutdown
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Channel should be closed
        assert!(msg_tx.is_closed());
    }

    #[tokio::test]
    async fn test_screening_actor_periodic_evaluation_trigger() {
        let (msg_tx, mut results_rx) = spawn_screening_actor(
            "dydx".to_string(),
            None,
            Duration::from_millis(100), // Fast interval for testing
        )
        .await;

        // Give actor time to start
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Shutdown
        msg_tx.send(ScreeningMessage::Shutdown).await.unwrap();

        // Give actor time to shutdown
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_screening_actor_dydx_symbol_discovery() {
        let (msg_tx, _results_rx) =
            spawn_screening_actor("dydx".to_string(), None, Duration::from_secs(10)).await;

        // Send discovery message
        msg_tx.send(ScreeningMessage::ScreenAll).await.unwrap();

        msg_tx.send(ScreeningMessage::Shutdown).await.unwrap();
    }
}
