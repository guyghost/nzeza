use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::domain::entities::symbol_screening::{RecommendationCategory, SymbolScreeningResult};

/// Repository for persisting and retrieving screening results
pub struct ScreeningRepository {
    pool: SqlitePool,
}

impl ScreeningRepository {
    pub fn new(pool: SqlitePool) -> Self {
        ScreeningRepository { pool }
    }

    /// Initialize database tables
    pub async fn init(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS symbol_screening_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                exchange TEXT NOT NULL,
                volatility_score REAL NOT NULL,
                volume_score REAL NOT NULL,
                spread_score REAL NOT NULL,
                momentum_score REAL NOT NULL,
                overall_score REAL NOT NULL,
                recommendation TEXT NOT NULL,
                screened_at TIMESTAMP NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for common queries
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbol_exchange ON symbol_screening_results(symbol, exchange)")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_screened_at ON symbol_screening_results(screened_at)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Persist screening result to database
    pub async fn save(&self, result: &SymbolScreeningResult) -> Result<i64, sqlx::Error> {
        let recommendation = format!("{:?}", result.recommendation);

        let row = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO symbol_screening_results
            (symbol, exchange, volatility_score, volume_score, spread_score, momentum_score,
             overall_score, recommendation, screened_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&result.symbol)
        .bind(&result.exchange)
        .bind(result.volatility_score)
        .bind(result.volume_score)
        .bind(result.spread_score)
        .bind(result.momentum_score)
        .bind(result.overall_score)
        .bind(&recommendation)
        .bind(result.screened_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Retrieve recent screening results
    pub async fn get_recent(&self, limit: i64) -> Result<Vec<SymbolScreeningResult>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT symbol, exchange, volatility_score, volume_score, spread_score,
                   momentum_score, overall_score, recommendation, screened_at
            FROM symbol_screening_results
            ORDER BY screened_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let results = rows
            .into_iter()
            .map(|row| {
                let symbol = row.get::<String, _>("symbol");
                let exchange = row.get::<String, _>("exchange");
                let volatility_score = row.get::<f64, _>("volatility_score");
                let volume_score = row.get::<f64, _>("volume_score");
                let spread_score = row.get::<f64, _>("spread_score");
                let momentum_score = row.get::<f64, _>("momentum_score");
                let overall_score = row.get::<f64, _>("overall_score");
                let recommendation_str = row.get::<String, _>("recommendation");
                let screened_at = row.get::<DateTime<Utc>, _>("screened_at");

                let recommendation = match recommendation_str.as_str() {
                    "BestCandidate" => RecommendationCategory::BestCandidate,
                    "GoodCandidate" => RecommendationCategory::GoodCandidate,
                    "FairCandidate" => RecommendationCategory::FairCandidate,
                    _ => RecommendationCategory::Avoid,
                };

                SymbolScreeningResult {
                    symbol,
                    exchange,
                    volatility_score,
                    volume_score,
                    spread_score,
                    momentum_score,
                    overall_score,
                    recommendation,
                    screened_at,
                }
            })
            .collect();

        Ok(results)
    }

    /// Retrieve historical screening scores by symbol
    pub async fn get_by_symbol(
        &self,
        symbol: &str,
        limit: i64,
    ) -> Result<Vec<SymbolScreeningResult>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT symbol, exchange, volatility_score, volume_score, spread_score,
                   momentum_score, overall_score, recommendation, screened_at
            FROM symbol_screening_results
            WHERE symbol = ?
            ORDER BY screened_at DESC
            LIMIT ?
            "#,
        )
        .bind(symbol)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let results = rows
            .into_iter()
            .map(|row| {
                let symbol = row.get::<String, _>("symbol");
                let exchange = row.get::<String, _>("exchange");
                let volatility_score = row.get::<f64, _>("volatility_score");
                let volume_score = row.get::<f64, _>("volume_score");
                let spread_score = row.get::<f64, _>("spread_score");
                let momentum_score = row.get::<f64, _>("momentum_score");
                let overall_score = row.get::<f64, _>("overall_score");
                let recommendation_str = row.get::<String, _>("recommendation");
                let screened_at = row.get::<DateTime<Utc>, _>("screened_at");

                let recommendation = match recommendation_str.as_str() {
                    "BestCandidate" => RecommendationCategory::BestCandidate,
                    "GoodCandidate" => RecommendationCategory::GoodCandidate,
                    "FairCandidate" => RecommendationCategory::FairCandidate,
                    _ => RecommendationCategory::Avoid,
                };

                SymbolScreeningResult {
                    symbol,
                    exchange,
                    volatility_score,
                    volume_score,
                    spread_score,
                    momentum_score,
                    overall_score,
                    recommendation,
                    screened_at,
                }
            })
            .collect();

        Ok(results)
    }

    /// Delete old screening records (older than days)
    pub async fn delete_old(&self, older_than_days: i64) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM symbol_screening_results
            WHERE screened_at < datetime('now', ? || ' days')
            "#,
        )
        .bind(-older_than_days)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Query by symbol and exchange
    pub async fn get_by_symbol_and_exchange(
        &self,
        symbol: &str,
        exchange: &str,
        limit: i64,
    ) -> Result<Vec<SymbolScreeningResult>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT symbol, exchange, volatility_score, volume_score, spread_score,
                   momentum_score, overall_score, recommendation, screened_at
            FROM symbol_screening_results
            WHERE symbol = ? AND exchange = ?
            ORDER BY screened_at DESC
            LIMIT ?
            "#,
        )
        .bind(symbol)
        .bind(exchange)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let results = rows
            .into_iter()
            .map(|row| {
                let symbol = row.get::<String, _>("symbol");
                let exchange = row.get::<String, _>("exchange");
                let volatility_score = row.get::<f64, _>("volatility_score");
                let volume_score = row.get::<f64, _>("volume_score");
                let spread_score = row.get::<f64, _>("spread_score");
                let momentum_score = row.get::<f64, _>("momentum_score");
                let overall_score = row.get::<f64, _>("overall_score");
                let recommendation_str = row.get::<String, _>("recommendation");
                let screened_at = row.get::<DateTime<Utc>, _>("screened_at");

                let recommendation = match recommendation_str.as_str() {
                    "BestCandidate" => RecommendationCategory::BestCandidate,
                    "GoodCandidate" => RecommendationCategory::GoodCandidate,
                    "FairCandidate" => RecommendationCategory::FairCandidate,
                    _ => RecommendationCategory::Avoid,
                };

                SymbolScreeningResult {
                    symbol,
                    exchange,
                    volatility_score,
                    volume_score,
                    spread_score,
                    momentum_score,
                    overall_score,
                    recommendation,
                    screened_at,
                }
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_screening_repository_initialization() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let repo = ScreeningRepository::new(pool);
        repo.init().await.unwrap();
    }

    #[tokio::test]
    async fn test_screening_repository_persist_and_retrieve() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let repo = ScreeningRepository::new(pool);
        repo.init().await.unwrap();

        let result = SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            0.8,
            0.7,
            0.9,
            0.6,
        );

        let id = repo.save(&result).await.unwrap();
        assert!(id > 0);

        let retrieved = repo.get_recent(1).await.unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].symbol, "BTC-USD");
    }

    #[tokio::test]
    async fn test_screening_repository_get_by_symbol() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let repo = ScreeningRepository::new(pool);
        repo.init().await.unwrap();

        for i in 0..5 {
            let result = SymbolScreeningResult::new(
                "BTC-USD".to_string(),
                "dydx".to_string(),
                0.8 - i as f64 * 0.05,
                0.7,
                0.9,
                0.6,
            );
            repo.save(&result).await.unwrap();
        }

        let retrieved = repo.get_by_symbol("BTC-USD", 10).await.unwrap();
        assert_eq!(retrieved.len(), 5);
    }

    #[tokio::test]
    async fn test_screening_repository_get_by_symbol_and_exchange() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let repo = ScreeningRepository::new(pool);
        repo.init().await.unwrap();

        let result = SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            0.8,
            0.7,
            0.9,
            0.6,
        );
        repo.save(&result).await.unwrap();

        let retrieved = repo
            .get_by_symbol_and_exchange("BTC-USD", "dydx", 10)
            .await
            .unwrap();
        assert_eq!(retrieved.len(), 1);
    }

    #[tokio::test]
    async fn test_screening_repository_delete_old() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let repo = ScreeningRepository::new(pool);
        repo.init().await.unwrap();

        let result = SymbolScreeningResult::new(
            "BTC-USD".to_string(),
            "dydx".to_string(),
            0.8,
            0.7,
            0.9,
            0.6,
        );
        repo.save(&result).await.unwrap();

        // Delete records older than 0 days
        repo.delete_old(0).await.unwrap();

        let retrieved = repo.get_recent(10).await.unwrap();
        assert_eq!(retrieved.len(), 0);
    }
}
