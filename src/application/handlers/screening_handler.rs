use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::domain::entities::symbol_screening::RecommendationCategory;

/// Query parameters for screening endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct ScreeningQuery {
    /// Filter by recommendation level (BestCandidate, GoodCandidate, FairCandidate, Avoid)
    pub level: Option<String>,
    /// Pagination page number (default 1)
    pub page: Option<u32>,
    /// Results per page (default 10, max 100)
    pub limit: Option<u32>,
}

/// Single screening result in API response
#[derive(Debug, Serialize, Deserialize)]
pub struct ScreeningResultResponse {
    pub symbol: String,
    pub exchange: String,
    pub volatility_score: f64,
    pub volume_score: f64,
    pub spread_score: f64,
    pub momentum_score: f64,
    pub overall_score: f64,
    pub recommendation: String,
    pub screened_at: String,
}

/// API response for screening results
#[derive(Debug, Serialize, Deserialize)]
pub struct ScreeningResponse {
    pub exchange: String,
    pub results: Vec<ScreeningResultResponse>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
}

/// Error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Get ranked symbols for an exchange
pub async fn get_screening_results(
    Path(exchange): Path<String>,
    Query(params): Query<ScreeningQuery>,
) -> Result<Json<ScreeningResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Validate exchange
    let valid_exchanges = vec!["dydx", "coinbase", "binance", "kraken", "hyperliquid"];
    if !valid_exchanges.contains(&exchange.as_str()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Unknown exchange: {}", exchange),
            }),
        ));
    }

    // Parse pagination
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).min(100).max(1);

    // For now, return empty response (to be filled by actual screening service)
    Ok(Json(ScreeningResponse {
        exchange,
        results: vec![],
        total: 0,
        page,
        limit,
    }))
}

/// Get screening result details for a specific symbol
pub async fn get_symbol_screening_details(
    Path((exchange, symbol)): Path<(String, String)>,
) -> Result<Json<Vec<ScreeningResultResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Validate exchange
    let valid_exchanges = vec!["dydx", "coinbase", "binance", "kraken", "hyperliquid"];
    if !valid_exchanges.contains(&exchange.as_str()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Unknown exchange: {}", exchange),
            }),
        ));
    }

    // For now, return empty response (to be filled by actual screening service)
    Ok(Json(vec![]))
}

/// Get screening history for a symbol
pub async fn get_screening_history(
    Path((exchange, symbol)): Path<(String, String)>,
    Query(params): Query<ScreeningQuery>,
) -> Result<Json<Vec<ScreeningResultResponse>>, (StatusCode, Json<ErrorResponse>)> {
    // Validate exchange
    let valid_exchanges = vec!["dydx", "coinbase", "binance", "kraken", "hyperliquid"];
    if !valid_exchanges.contains(&exchange.as_str()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Unknown exchange: {}", exchange),
            }),
        ));
    }

    // For now, return empty response
    Ok(Json(vec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_endpoint_get_ranked_symbols() {
        let result = get_screening_results(
            Path("dydx".to_string()),
            Query(ScreeningQuery {
                level: None,
                page: None,
                limit: None,
            }),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.exchange, "dydx");
    }

    #[tokio::test]
    async fn test_api_endpoint_error_handling() {
        let result = get_screening_results(
            Path("invalid_exchange".to_string()),
            Query(ScreeningQuery {
                level: None,
                page: None,
                limit: None,
            }),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(result.err().unwrap().0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_api_endpoint_pagination() {
        let result = get_screening_results(
            Path("dydx".to_string()),
            Query(ScreeningQuery {
                level: None,
                page: Some(2),
                limit: Some(20),
            }),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.page, 2);
        assert_eq!(response.limit, 20);
    }

    #[tokio::test]
    async fn test_api_endpoint_pagination_limits() {
        let result = get_screening_results(
            Path("dydx".to_string()),
            Query(ScreeningQuery {
                level: None,
                page: Some(0),    // Should be clamped to 1
                limit: Some(200), // Should be clamped to 100
            }),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap().0;
        assert_eq!(response.page, 1);
        assert_eq!(response.limit, 100);
    }
}
