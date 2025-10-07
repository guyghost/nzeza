//! Database Repository
//!
//! Data access layer for positions, trades, and audit logs.

use super::models::*;
use super::{DatabaseError, DbPool};
use chrono::Utc;
use sqlx::Row;
use tracing::{debug, error};

/// Position repository
pub struct PositionRepository {
    pool: DbPool,
}

impl PositionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new position
    pub async fn create(&self, position: CreatePosition) -> Result<PositionRecord, DatabaseError> {
        let now = Utc::now();
        let record = sqlx::query_as::<_, PositionRecord>(
            r#"
            INSERT INTO positions (
                id, symbol, exchange, side, entry_price, quantity,
                current_price, unrealized_pnl, status, opened_at,
                stop_loss, take_profit, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5, 0.0, 'open', ?7, ?8, ?9, ?7, ?7)
            RETURNING *
            "#,
        )
        .bind(&position.id)
        .bind(&position.symbol)
        .bind(&position.exchange)
        .bind(&position.side)
        .bind(position.entry_price)
        .bind(position.quantity)
        .bind(now)
        .bind(position.stop_loss)
        .bind(position.take_profit)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to create position: {}", e);
            DatabaseError::QueryError(format!("Failed to create position: {}", e))
        })?;

        debug!("Created position: {} for {}", record.id, record.symbol);
        Ok(record)
    }

    /// Get position by ID
    pub async fn get(&self, id: &str) -> Result<Option<PositionRecord>, DatabaseError> {
        let record = sqlx::query_as::<_, PositionRecord>(
            "SELECT * FROM positions WHERE id = ?1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get position {}: {}", id, e);
            DatabaseError::QueryError(format!("Failed to get position: {}", e))
        })?;

        Ok(record)
    }

    /// Update position price and PnL
    pub async fn update(&self, id: &str, update: UpdatePosition) -> Result<(), DatabaseError> {
        let now = Utc::now();
        let rows_affected = sqlx::query(
            r#"
            UPDATE positions
            SET current_price = ?1, unrealized_pnl = ?2, updated_at = ?3
            WHERE id = ?4
            "#,
        )
        .bind(update.current_price)
        .bind(update.unrealized_pnl)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to update position {}: {}", id, e);
            DatabaseError::QueryError(format!("Failed to update position: {}", e))
        })?
        .rows_affected();

        if rows_affected == 0 {
            return Err(DatabaseError::QueryError(format!(
                "Position not found: {}",
                id
            )));
        }

        debug!("Updated position: {}", id);
        Ok(())
    }

    /// Close a position
    pub async fn close(&self, id: &str, final_price: f64, realized_pnl: f64) -> Result<(), DatabaseError> {
        let now = Utc::now();
        let rows_affected = sqlx::query(
            r#"
            UPDATE positions
            SET status = 'closed', current_price = ?1, unrealized_pnl = ?2,
                closed_at = ?3, updated_at = ?3
            WHERE id = ?4 AND status = 'open'
            "#,
        )
        .bind(final_price)
        .bind(realized_pnl)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to close position {}: {}", id, e);
            DatabaseError::QueryError(format!("Failed to close position: {}", e))
        })?
        .rows_affected();

        if rows_affected == 0 {
            return Err(DatabaseError::QueryError(format!(
                "Position not found or already closed: {}",
                id
            )));
        }

        debug!("Closed position: {}", id);
        Ok(())
    }

    /// Get all open positions
    pub async fn get_open_positions(&self) -> Result<Vec<PositionRecord>, DatabaseError> {
        let records = sqlx::query_as::<_, PositionRecord>(
            "SELECT * FROM positions WHERE status = 'open' ORDER BY opened_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get open positions: {}", e);
            DatabaseError::QueryError(format!("Failed to get open positions: {}", e))
        })?;

        Ok(records)
    }

    /// Get positions by symbol
    pub async fn get_by_symbol(&self, symbol: &str) -> Result<Vec<PositionRecord>, DatabaseError> {
        let records = sqlx::query_as::<_, PositionRecord>(
            "SELECT * FROM positions WHERE symbol = ?1 ORDER BY opened_at DESC"
        )
        .bind(symbol)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get positions for {}: {}", symbol, e);
            DatabaseError::QueryError(format!("Failed to get positions: {}", e))
        })?;

        Ok(records)
    }

    /// Get open positions count by symbol
    pub async fn count_open_by_symbol(&self, symbol: &str) -> Result<i64, DatabaseError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM positions WHERE symbol = ?1 AND status = 'open'"
        )
        .bind(symbol)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to count open positions for {}: {}", symbol, e);
            DatabaseError::QueryError(format!("Failed to count positions: {}", e))
        })?;

        let count: i64 = row.get("count");
        Ok(count)
    }
}

/// Trade repository
pub struct TradeRepository {
    pool: DbPool,
}

impl TradeRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new trade
    pub async fn create(&self, trade: CreateTrade) -> Result<TradeRecord, DatabaseError> {
        let now = Utc::now();
        let record = sqlx::query_as::<_, TradeRecord>(
            r#"
            INSERT INTO trades (
                id, position_id, symbol, exchange, side, price, quantity,
                fee, exchange_order_id, executed_at, strategy, signal_confidence, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?10)
            RETURNING *
            "#,
        )
        .bind(&trade.id)
        .bind(&trade.position_id)
        .bind(&trade.symbol)
        .bind(&trade.exchange)
        .bind(&trade.side)
        .bind(trade.price)
        .bind(trade.quantity)
        .bind(trade.fee)
        .bind(&trade.exchange_order_id)
        .bind(now)
        .bind(&trade.strategy)
        .bind(trade.signal_confidence)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to create trade: {}", e);
            DatabaseError::QueryError(format!("Failed to create trade: {}", e))
        })?;

        debug!("Created trade: {} for {}", record.id, record.symbol);
        Ok(record)
    }

    /// Get trade by ID
    pub async fn get(&self, id: &str) -> Result<Option<TradeRecord>, DatabaseError> {
        let record = sqlx::query_as::<_, TradeRecord>(
            "SELECT * FROM trades WHERE id = ?1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get trade {}: {}", id, e);
            DatabaseError::QueryError(format!("Failed to get trade: {}", e))
        })?;

        Ok(record)
    }

    /// Get trades by position ID
    pub async fn get_by_position(&self, position_id: &str) -> Result<Vec<TradeRecord>, DatabaseError> {
        let records = sqlx::query_as::<_, TradeRecord>(
            "SELECT * FROM trades WHERE position_id = ?1 ORDER BY executed_at DESC"
        )
        .bind(position_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get trades for position {}: {}", position_id, e);
            DatabaseError::QueryError(format!("Failed to get trades: {}", e))
        })?;

        Ok(records)
    }

    /// Get recent trades (last N)
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<TradeRecord>, DatabaseError> {
        let records = sqlx::query_as::<_, TradeRecord>(
            "SELECT * FROM trades ORDER BY executed_at DESC LIMIT ?1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get recent trades: {}", e);
            DatabaseError::QueryError(format!("Failed to get recent trades: {}", e))
        })?;

        Ok(records)
    }

    /// Get trades by symbol and date range
    pub async fn get_by_symbol_range(
        &self,
        symbol: &str,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<Vec<TradeRecord>, DatabaseError> {
        let records = sqlx::query_as::<_, TradeRecord>(
            r#"
            SELECT * FROM trades
            WHERE symbol = ?1 AND executed_at >= ?2 AND executed_at <= ?3
            ORDER BY executed_at DESC
            "#
        )
        .bind(symbol)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get trades for {} in date range: {}", symbol, e);
            DatabaseError::QueryError(format!("Failed to get trades: {}", e))
        })?;

        Ok(records)
    }
}

/// Audit log repository
pub struct AuditLogRepository {
    pool: DbPool,
}

impl AuditLogRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new audit log entry
    pub async fn create(&self, log: CreateAuditLog) -> Result<AuditLogRecord, DatabaseError> {
        let now = Utc::now();
        let details_json = serde_json::to_string(&log.details)
            .map_err(|e| DatabaseError::QueryError(format!("Failed to serialize details: {}", e)))?;

        let record = sqlx::query_as::<_, AuditLogRecord>(
            r#"
            INSERT INTO audit_log (event_type, exchange, symbol, details, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5)
            RETURNING *
            "#,
        )
        .bind(&log.event_type)
        .bind(&log.exchange)
        .bind(&log.symbol)
        .bind(&details_json)
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to create audit log: {}", e);
            DatabaseError::QueryError(format!("Failed to create audit log: {}", e))
        })?;

        debug!("Created audit log: {} for {}", record.event_type, record.exchange);
        Ok(record)
    }

    /// Get recent audit logs
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<AuditLogRecord>, DatabaseError> {
        let records = sqlx::query_as::<_, AuditLogRecord>(
            "SELECT * FROM audit_log ORDER BY timestamp DESC LIMIT ?1"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get recent audit logs: {}", e);
            DatabaseError::QueryError(format!("Failed to get audit logs: {}", e))
        })?;

        Ok(records)
    }

    /// Get audit logs by event type
    pub async fn get_by_event_type(&self, event_type: &str, limit: i64) -> Result<Vec<AuditLogRecord>, DatabaseError> {
        let records = sqlx::query_as::<_, AuditLogRecord>(
            "SELECT * FROM audit_log WHERE event_type = ?1 ORDER BY timestamp DESC LIMIT ?2"
        )
        .bind(event_type)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to get audit logs for event type {}: {}", event_type, e);
            DatabaseError::QueryError(format!("Failed to get audit logs: {}", e))
        })?;

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::init_database;

    #[tokio::test]
    async fn test_position_crud() {
        let pool = init_database("sqlite::memory:").await.unwrap();
        let repo = PositionRepository::new(pool);

        // Create position
        let position = CreatePosition {
            id: "test-pos-1".to_string(),
            symbol: "BTC-USD".to_string(),
            exchange: "binance".to_string(),
            side: "long".to_string(),
            entry_price: 50000.0,
            quantity: 0.1,
            stop_loss: Some(49000.0),
            take_profit: Some(52000.0),
        };

        let created = repo.create(position).await.unwrap();
        assert_eq!(created.symbol, "BTC-USD");
        assert_eq!(created.status, "open");

        // Get position
        let fetched = repo.get(&created.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, created.id);

        // Update position
        let update = UpdatePosition {
            current_price: 51000.0,
            unrealized_pnl: 100.0,
        };
        repo.update(&created.id, update).await.unwrap();

        // Close position
        repo.close(&created.id, 52000.0, 200.0).await.unwrap();
        let closed = repo.get(&created.id).await.unwrap().unwrap();
        assert_eq!(closed.status, "closed");
    }

    #[tokio::test]
    async fn test_trade_crud() {
        let pool = init_database("sqlite::memory:").await.unwrap();

        // Create a position first (required for foreign key)
        let position_repo = PositionRepository::new(pool.clone());
        let position = CreatePosition {
            id: "test-pos-1".to_string(),
            symbol: "BTC-USD".to_string(),
            exchange: "binance".to_string(),
            side: "long".to_string(),
            entry_price: 50000.0,
            quantity: 0.1,
            stop_loss: None,
            take_profit: None,
        };
        position_repo.create(position).await.unwrap();

        // Now create trade
        let repo = TradeRepository::new(pool);
        let trade = CreateTrade {
            id: "test-trade-1".to_string(),
            position_id: Some("test-pos-1".to_string()),
            symbol: "BTC-USD".to_string(),
            exchange: "binance".to_string(),
            side: "buy".to_string(),
            price: 50000.0,
            quantity: 0.1,
            fee: 5.0,
            exchange_order_id: Some("binance-123".to_string()),
            strategy: "fast_scalping".to_string(),
            signal_confidence: Some(0.85),
        };

        let created = repo.create(trade).await.unwrap();
        assert_eq!(created.symbol, "BTC-USD");
        assert_eq!(created.strategy, "fast_scalping");

        // Get trade
        let fetched = repo.get(&created.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, created.id);

        // Get recent trades
        let recent = repo.get_recent(10).await.unwrap();
        assert_eq!(recent.len(), 1);
    }
}
