//! Persistence Layer
//!
//! This module provides database persistence for positions, trades, and audit logs.
//! Uses SQLite for local storage with async operations via sqlx.
//!
//! # Features
//! - Position tracking across restarts
//! - Trade history with full audit trail
//! - Performance metrics storage
//! - Automatic schema migrations
//!
//! # Database Schema
//!
//! ## Positions Table
//! - id: UUID
//! - symbol: Trading pair (e.g., "BTC-USD")
//! - exchange: Exchange name
//! - side: "long" or "short"
//! - entry_price: Decimal
//! - quantity: Decimal
//! - current_price: Decimal (updated)
//! - unrealized_pnl: Decimal
//! - status: "open", "closed"
//! - opened_at: Timestamp
//! - closed_at: Optional timestamp
//!
//! ## Trades Table
//! - id: UUID
//! - position_id: Foreign key to positions
//! - symbol: Trading pair
//! - exchange: Exchange name
//! - side: "buy" or "sell"
//! - price: Decimal
//! - quantity: Decimal
//! - fee: Decimal
//! - exchange_order_id: String
//! - executed_at: Timestamp
//! - strategy: Strategy name that generated the trade
//!
//! ## Audit Log Table
//! - id: Serial
//! - event_type: Event type (order_placed, order_filled, error, etc.)
//! - exchange: Exchange name
//! - symbol: Optional trading pair
//! - details: JSON details
//! - timestamp: Timestamp

pub mod models;
pub mod reconciliation_audit;
pub mod repository;
pub mod screening_repository;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::ConnectOptions;
use std::path::Path;
use std::str::FromStr;
use tracing::info;

/// Database connection pool
pub type DbPool = SqlitePool;

/// Database initialization error
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    ConnectionError(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Query error: {0}")]
    QueryError(String),
}

/// Initialize the database connection pool
///
/// # Arguments
/// - `database_url`: Path to SQLite database file (e.g., "sqlite://data/nzeza.db")
///
/// # Returns
/// Database connection pool ready for use
///
/// # Errors
/// Returns error if database connection fails or migrations fail
pub async fn init_database(database_url: &str) -> Result<DbPool, DatabaseError> {
    info!("Initializing database: {}", database_url);

    // Ensure data directory exists
    if let Some(db_path) = database_url.strip_prefix("sqlite://") {
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DatabaseError::ConnectionError(sqlx::Error::Configuration(Box::new(e)))
            })?;
        }
    }

    // Create connection options
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .log_statements(tracing::log::LevelFilter::Debug);

    // Create connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // Run migrations
    run_migrations(&pool).await?;

    info!("✓ Database initialized successfully");

    Ok(pool)
}

/// Run database migrations
async fn run_migrations(pool: &DbPool) -> Result<(), DatabaseError> {
    info!("Running database migrations...");

    // Create positions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS positions (
            id TEXT PRIMARY KEY,
            symbol TEXT NOT NULL,
            exchange TEXT NOT NULL,
            side TEXT NOT NULL CHECK(side IN ('long', 'short')),
            entry_price REAL NOT NULL,
            quantity REAL NOT NULL,
            current_price REAL NOT NULL,
            unrealized_pnl REAL NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('open', 'closed')),
            opened_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            closed_at DATETIME,
            stop_loss REAL,
            take_profit REAL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        DatabaseError::MigrationError(format!("Failed to create positions table: {}", e))
    })?;

    // Create trades table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trades (
            id TEXT PRIMARY KEY,
            position_id TEXT,
            symbol TEXT NOT NULL,
            exchange TEXT NOT NULL,
            side TEXT NOT NULL CHECK(side IN ('buy', 'sell')),
            price REAL NOT NULL,
            quantity REAL NOT NULL,
            fee REAL NOT NULL DEFAULT 0.0,
            exchange_order_id TEXT,
            executed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            strategy TEXT NOT NULL,
            signal_confidence REAL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (position_id) REFERENCES positions(id)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| DatabaseError::MigrationError(format!("Failed to create trades table: {}", e)))?;

    // Create audit log table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            event_type TEXT NOT NULL,
            exchange TEXT NOT NULL,
            symbol TEXT,
            details TEXT NOT NULL,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        DatabaseError::MigrationError(format!("Failed to create audit_log table: {}", e))
    })?;

    // Create reconciliation_audit table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS reconciliation_audit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            reconciliation_id TEXT NOT NULL UNIQUE,
            exchange_id TEXT NOT NULL,
            reconciliation_timestamp DATETIME NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            status TEXT NOT NULL,
            discrepancy_count INTEGER NOT NULL,
            local_balances_json TEXT NOT NULL,
            exchange_balances_json TEXT NOT NULL,
            discrepancies_json TEXT NOT NULL,
            recovery_attempted BOOLEAN DEFAULT 0,
            recovery_status TEXT,
            recovery_details_json TEXT,
            operator_notes TEXT,
            INDEX idx_exchange_time (exchange_id, reconciliation_timestamp),
            INDEX idx_status (status)
        )
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| {
        DatabaseError::MigrationError(format!(
            "Failed to create reconciliation_audit table: {}",
            e
        ))
    })?;

    // Add order_flags column if it doesn't exist (for databases migrated from older versions)
    let order_flags_exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM pragma_table_info('dydx_order_metadata') WHERE name='order_flags'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    if order_flags_exists.0 == 0 {
        sqlx::query("ALTER TABLE dydx_order_metadata ADD COLUMN order_flags INTEGER DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|e| {
                DatabaseError::MigrationError(format!("Failed to add order_flags column: {}", e))
            })?;
    }

    // Add clob_pair_id column if it doesn't exist (for databases migrated from older versions)
    let clob_pair_id_exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM pragma_table_info('dydx_order_metadata') WHERE name='clob_pair_id'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or((0,));

    if clob_pair_id_exists.0 == 0 {
        sqlx::query("ALTER TABLE dydx_order_metadata ADD COLUMN clob_pair_id INTEGER DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|e| {
                DatabaseError::MigrationError(format!("Failed to add clob_pair_id column: {}", e))
            })?;
    }

    // Create indexes for better query performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_positions_status ON positions(status)")
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::MigrationError(format!("Failed to create index: {}", e)))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_positions_symbol ON positions(symbol)")
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::MigrationError(format!("Failed to create index: {}", e)))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_position_id ON trades(position_id)")
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::MigrationError(format!("Failed to create index: {}", e)))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_trades_executed_at ON trades(executed_at)")
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::MigrationError(format!("Failed to create index: {}", e)))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)")
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::MigrationError(format!("Failed to create index: {}", e)))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_dydx_order_status ON dydx_order_metadata(status)")
        .execute(pool)
        .await
        .map_err(|e| DatabaseError::MigrationError(format!("Failed to create index: {}", e)))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_dydx_order_placed_at ON dydx_order_metadata(placed_at)",
    )
    .execute(pool)
    .await
    .map_err(|e| DatabaseError::MigrationError(format!("Failed to create index: {}", e)))?;

    info!("✓ Database migrations completed successfully");

    Ok(())
}

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// Database URL (e.g., "sqlite://data/nzeza.db")
    pub url: String,

    /// Maximum number of connections in the pool
    pub max_connections: u32,

    /// Enable query logging
    pub log_queries: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://data/nzeza.db".to_string(),
            max_connections: 5,
            log_queries: cfg!(debug_assertions),
        }
    }
}

impl DatabaseConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        let url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/nzeza.db".to_string());

        let max_connections = std::env::var("DATABASE_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);

        let log_queries = std::env::var("DATABASE_LOG_QUERIES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(cfg!(debug_assertions));

        Self {
            url,
            max_connections,
            log_queries,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_init() {
        let pool = init_database("sqlite::memory:").await;
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn test_migrations() {
        let pool = init_database("sqlite::memory:").await.unwrap();

        // Verify tables exist
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('positions', 'trades', 'audit_log')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(result.0, 3);
    }

    #[test]
    fn test_database_config_default() {
        let config = DatabaseConfig::default();
        assert_eq!(config.url, "sqlite://data/nzeza.db");
        assert_eq!(config.max_connections, 5);
    }
}
