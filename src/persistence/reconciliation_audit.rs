//! Reconciliation Audit Repository
//!
//! This repository handles persistence of reconciliation audit trails to the database.

use super::models::*;
use super::{DatabaseError, DbPool};
use crate::domain::entities::exchange::Exchange;
use crate::domain::services::reconciliation::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use tracing::{debug, error};

/// Reconciliation repository trait
#[async_trait]
pub trait ReconciliationRepository: Send + Sync {
    async fn save_reconciliation(
        &self,
        report: &ReconciliationReport,
    ) -> Result<(), ReconciliationError>;
    async fn get_last_reconciliation(
        &self,
        exchange: &Exchange,
    ) -> Result<Option<ReconciliationReport>, ReconciliationError>;
    async fn get_reconciliation_history(
        &self,
        exchange: &Exchange,
        days: u32,
    ) -> Result<Vec<ReconciliationReport>, ReconciliationError>;
}

/// SQLite implementation of reconciliation repository
pub struct SqliteReconciliationRepository {
    pool: DbPool,
}

impl SqliteReconciliationRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReconciliationRepository for SqliteReconciliationRepository {
    async fn save_reconciliation(
        &self,
        report: &ReconciliationReport,
    ) -> Result<(), ReconciliationError> {
        let exchange_str = match report.exchange {
            Exchange::Coinbase => "coinbase",
            Exchange::Dydx => "dydx",
            _ => "unknown",
        };

        let discrepancies_json = serde_json::to_string(&report.discrepancies)
            .map_err(|e| ReconciliationError::ParseError(e.to_string()))?;

        let status_str = match report.status {
            ReconciliationStatus::Ok => "OK",
            ReconciliationStatus::Minor => "MINOR",
            ReconciliationStatus::Major => "MAJOR",
            ReconciliationStatus::Critical => "CRITICAL",
        };

        sqlx::query(
            r#"
            INSERT INTO reconciliation_audit (
                reconciliation_id, exchange_id, reconciliation_timestamp,
                status, discrepancy_count, discrepancies_json, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(format!(
            "rec_{}_{}",
            exchange_str,
            report.timestamp.timestamp()
        ))
        .bind(exchange_str)
        .bind(report.timestamp)
        .bind(status_str)
        .bind(report.discrepancy_count() as i64)
        .bind(&discrepancies_json)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to save reconciliation audit: {}", e);
            ReconciliationError::ApiError(format!("Database error: {}", e))
        })?;

        debug!("Saved reconciliation audit for {:?}", report.exchange);
        Ok(())
    }

    async fn get_last_reconciliation(
        &self,
        exchange: &Exchange,
    ) -> Result<Option<ReconciliationReport>, ReconciliationError> {
        let exchange_str = match exchange {
            Exchange::Coinbase => "coinbase",
            Exchange::Dydx => "dydx",
            _ => "unknown",
        };

        let row = sqlx::query(
            r#"
            SELECT reconciliation_timestamp, status, discrepancy_count, discrepancies_json, exchange_id
            FROM reconciliation_audit
            WHERE exchange_id = ?1
            ORDER BY reconciliation_timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(exchange_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get last reconciliation for {:?}: {}", exchange, e);
            ReconciliationError::ApiError(format!("Database error: {}", e))
        })?;

        match row {
            Some(row) => {
                let timestamp: DateTime<Utc> = row.get("reconciliation_timestamp");
                let status_str: String = row.get("status");
                let discrepancy_count: i64 = row.get("discrepancy_count");
                let discrepancies_json: String = row.get("discrepancies_json");

                let status = match status_str.as_str() {
                    "OK" => ReconciliationStatus::Ok,
                    "MINOR" => ReconciliationStatus::Minor,
                    "MAJOR" => ReconciliationStatus::Major,
                    "CRITICAL" => ReconciliationStatus::Critical,
                    _ => ReconciliationStatus::Ok,
                };

                let discrepancies: Vec<BalanceDiscrepancy> =
                    serde_json::from_str(&discrepancies_json).unwrap_or_default();

                let report = ReconciliationReport {
                    exchange: exchange.clone(),
                    timestamp,
                    status,
                    discrepancies,
                    local_balances: Vec::new(),
                    exchange_balances: std::collections::HashMap::new(),
                };

                Ok(Some(report))
            }
            None => Ok(None),
        }
    }

    async fn get_reconciliation_history(
        &self,
        exchange: &Exchange,
        days: u32,
    ) -> Result<Vec<ReconciliationReport>, ReconciliationError> {
        let exchange_str = match exchange {
            Exchange::Coinbase => "coinbase",
            Exchange::Dydx => "dydx",
            _ => "unknown",
        };

        let cutoff = Utc::now() - chrono::Duration::days(days as i64);

        let rows = sqlx::query(
            r#"
            SELECT reconciliation_timestamp, status, discrepancy_count, discrepancies_json, exchange_id
            FROM reconciliation_audit
            WHERE exchange_id = ?1 AND reconciliation_timestamp >= ?2
            ORDER BY reconciliation_timestamp DESC
            "#,
        )
        .bind(exchange_str)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get reconciliation history for {:?}: {}", exchange, e);
            ReconciliationError::ApiError(format!("Database error: {}", e))
        })?;

        let mut reports = Vec::new();
        for row in rows {
            let timestamp: DateTime<Utc> = row.get("reconciliation_timestamp");
            let status_str: String = row.get("status");
            let discrepancies_json: String = row.get("discrepancies_json");

            let status = match status_str.as_str() {
                "OK" => ReconciliationStatus::Ok,
                "MINOR" => ReconciliationStatus::Minor,
                "MAJOR" => ReconciliationStatus::Major,
                "CRITICAL" => ReconciliationStatus::Critical,
                _ => ReconciliationStatus::Ok,
            };

            let discrepancies: Vec<BalanceDiscrepancy> =
                serde_json::from_str(&discrepancies_json).unwrap_or_default();

            let report = ReconciliationReport {
                exchange: exchange.clone(),
                timestamp,
                status,
                discrepancies,
                local_balances: Vec::new(),
                exchange_balances: std::collections::HashMap::new(),
            };

            reports.push(report);
        }

        Ok(reports)
    }
}

/// Audit entry for external use
#[derive(Debug, Clone)]
pub struct ReconciliationAuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
    pub status: ReconciliationStatus,
    pub discrepancy_count: usize,
}
