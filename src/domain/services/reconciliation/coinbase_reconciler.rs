//! Coinbase-specific balance reconciler

use crate::domain::entities::exchange::Exchange;
use crate::domain::services::portfolio_reconciliation::*;
use crate::infrastructure::coinbase_client::CoinbaseClient;
use async_trait::async_trait;
use std::sync::Arc;

/// Coinbase reconciler implementation
pub struct CoinbaseReconciler {
    client: Arc<CoinbaseClient>,
    config: ReconciliationConfig,
}

impl CoinbaseReconciler {
    pub fn new(client: Arc<CoinbaseClient>, config: ReconciliationConfig) -> Self {
        Self { client, config }
    }
}

#[async_trait]
impl PortfolioReconciliationService for CoinbaseReconciler {
    async fn fetch_exchange_balances(
        &self,
        exchange: &Exchange,
    ) -> Result<ExchangeBalances, ReconciliationError> {
        match exchange {
            Exchange::Coinbase => {
                let accounts = self
                    .client
                    .get_accounts()
                    .await
                    .map_err(|e| ReconciliationError::ApiError(e))?;

                let mut balances = ExchangeBalances::new(exchange.clone());
                for account in accounts {
                    if let Ok(amount) = account.balance.parse::<f64>() {
                        balances.add_balance(account.currency, amount);
                    }
                }

                Ok(balances)
            }
            _ => Err(ReconciliationError::ApiError(
                "CoinbaseReconciler only supports Coinbase exchange".to_string(),
            )),
        }
    }

    fn detect_discrepancies(
        &self,
        local: &Portfolio,
        exchange: &ExchangeBalances,
    ) -> Vec<BalanceDiscrepancy> {
        let mut discrepancies = Vec::new();

        // Check local balances against exchange
        for (currency, local_balance) in &local.balances {
            match exchange.get_balance(currency) {
                Some(exchange_balance) => {
                    let diff = (local_balance - exchange_balance.amount).abs();
                    if diff > 0.0 {
                        let difference_percentage = if exchange_balance.amount != 0.0 {
                            (local_balance - exchange_balance.amount).abs()
                                / exchange_balance.amount.abs()
                                * 100.0
                        } else {
                            100.0
                        };

                        if difference_percentage >= self.config.threshold_percentage * 100.0 {
                            discrepancies.push(BalanceDiscrepancy::Mismatch {
                                currency: currency.clone(),
                                local: *local_balance,
                                exchange: exchange_balance.amount,
                                diff,
                            });
                        } else if diff <= self.config.precision_tolerance {
                            discrepancies.push(BalanceDiscrepancy::Precision {
                                currency: currency.clone(),
                                tolerance: self.config.precision_tolerance,
                            });
                        }
                    }
                }
                None => {
                    discrepancies.push(BalanceDiscrepancy::Missing {
                        currency: currency.clone(),
                        amount: *local_balance,
                    });
                }
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

        // For now, create an empty local portfolio - in real implementation this would come from position manager
        let local_portfolio = Portfolio::new();

        let discrepancies = self.detect_discrepancies(&local_portfolio, &exchange_balances);
        let mut report = self.generate_report(discrepancies, exchange);

        // Convert balances to Balance structs for the report
        report.local_balances = local_portfolio
            .balances
            .iter()
            .map(|(currency, amount)| Balance {
                currency: currency.clone(),
                amount: *amount,
            })
            .collect();

        report.exchange_balances = exchange_balances.balances;

        Ok(report)
    }

    fn classify_discrepancy_severity(
        &self,
        discrepancy: &BalanceDiscrepancy,
    ) -> DiscrepancySeverity {
        discrepancy.severity()
    }
}
