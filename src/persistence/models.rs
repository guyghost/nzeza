//! Database Models
//!
//! Persistent data structures for positions, trades, and audit logs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Position record in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PositionRecord {
    pub id: String,
    pub symbol: String,
    pub exchange: String,
    pub side: String, // "long" or "short"
    pub entry_price: f64,
    pub quantity: f64,
    pub current_price: f64,
    pub unrealized_pnl: f64,
    pub status: String, // "open" or "closed"
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Trade record in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TradeRecord {
    pub id: String,
    pub position_id: Option<String>,
    pub symbol: String,
    pub exchange: String,
    pub side: String, // "buy" or "sell"
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub exchange_order_id: Option<String>,
    pub executed_at: DateTime<Utc>,
    pub strategy: String,
    pub signal_confidence: Option<f64>,
    pub created_at: DateTime<Utc>,
}

/// Audit log record in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLogRecord {
    pub id: i64,
    pub event_type: String,
    pub exchange: String,
    pub symbol: Option<String>,
    pub details: String, // JSON string
    pub timestamp: DateTime<Utc>,
}

/// Create position input
#[derive(Debug, Clone)]
pub struct CreatePosition {
    pub id: String,
    pub symbol: String,
    pub exchange: String,
    pub side: String,
    pub entry_price: f64,
    pub quantity: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

/// Update position input
#[derive(Debug, Clone)]
pub struct UpdatePosition {
    pub current_price: f64,
    pub unrealized_pnl: f64,
}

/// Create trade input
#[derive(Debug, Clone)]
pub struct CreateTrade {
    pub id: String,
    pub position_id: Option<String>,
    pub symbol: String,
    pub exchange: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
    pub fee: f64,
    pub exchange_order_id: Option<String>,
    pub strategy: String,
    pub signal_confidence: Option<f64>,
}

/// Create audit log input
#[derive(Debug, Clone)]
pub struct CreateAuditLog {
    pub event_type: String,
    pub exchange: String,
    pub symbol: Option<String>,
    pub details: serde_json::Value,
}
