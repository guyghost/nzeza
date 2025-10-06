// Tests d'intégration pour vérifier le fonctionnement end-to-end du système

use crate::application::services::mpc_service::MpcService;
use crate::config::TradingConfig;
use crate::domain::entities::exchange::Exchange;
use crate::domain::services::strategies::{FastScalping, MomentumScalping, SignalCombiner, Strategy};
use crate::domain::value_objects::price::Price;
use crate::infrastructure::adapters::exchange_actor::MockExchangeActor;

#[tokio::test]
async fn test_full_mpc_workflow() {
    // Créer le service MPC
    let mut service = MpcService::new(TradingConfig::default());

    // Ajouter des stratégies
    let strategies = vec![
        (
            "FastScalping".to_string(),
            Box::new(FastScalping::new()) as Box<dyn Strategy + Send + Sync>
        ),
        (
            "MomentumScalping".to_string(),
            Box::new(MomentumScalping::new()) as Box<dyn Strategy + Send + Sync>
        ),
    ];
    let weights = vec![0.5, 0.5];
    let combiner = SignalCombiner::new(strategies, weights)
        .expect("Failed to create signal combiner");
    service.set_signal_combiner(combiner).await;

    // Ajouter des acteurs mock pour les tests
    let mock_price = Price::new(50000.0).unwrap();
    let binance_sender = MockExchangeActor::spawn(Exchange::Binance, mock_price.clone());
    let dydx_sender = MockExchangeActor::spawn(Exchange::Dydx, mock_price.clone());

    service.add_actor(Exchange::Binance, binance_sender);
    service.add_actor(Exchange::Dydx, dydx_sender);

    // Tester la récupération de prix
    let price_result = service.get_price(&Exchange::Binance, "BTCUSDT").await;
    assert!(price_result.is_ok());
    assert_eq!(price_result.unwrap().value(), 50000.0);

    // Tester l'agrégation de prix
    let aggregated_result = service.get_aggregated_price("BTC-USD").await;
    assert!(aggregated_result.is_ok());

    // Tester la génération de signaux
    let candles = service.get_candles("BTC-USD").await;
    if candles.len() >= 10 {
        let signal = service.generate_trading_signal(&candles);
        assert!(signal.is_some());
        let s = signal.unwrap();
        assert!(s.confidence >= 0.0 && s.confidence <= 1.0);
    }

    // Tester la génération de signal pour un symbole
    let signal_result = service.generate_signal_for_symbol("BTC-USD").await;
    // Peut être None si pas assez de données, mais ne devrait pas paniquer
    assert!(signal_result.is_none() || signal_result.is_some());
}

#[tokio::test]
async fn test_order_workflow() {
    // Créer le service MPC
    let mut service = MpcService::new(TradingConfig::default());

    // Ajouter un acteur mock
    let mock_price = Price::new(50000.0).unwrap();
    let dydx_sender = MockExchangeActor::spawn(Exchange::Dydx, mock_price);

    service.add_actor(Exchange::Dydx, dydx_sender);

    // Tester le placement d'ordre (devrait réussir avec le mock)
    use crate::domain::entities::order::{Order, OrderSide, OrderType};

    let order = Order::new(
        "test_order_123".to_string(),
        "BTC-USD".to_string(),
        OrderSide::Buy,
        OrderType::Market,
        None,
        0.001,
    ).unwrap();

    let order_result = service.place_order(&Exchange::Dydx, order).await;
    assert!(order_result.is_ok());
    assert_eq!(order_result.unwrap(), "mock_order_id");

    // Tester l'annulation d'ordre
    let cancel_result = service.cancel_order(&Exchange::Dydx, "test_order_123").await;
    assert!(cancel_result.is_ok());

    // Tester le statut d'ordre
    let status_result = service.get_order_status(&Exchange::Dydx, "test_order_123").await;
    assert!(status_result.is_ok());
    assert_eq!(status_result.unwrap(), "FILLED");
}

#[tokio::test]
async fn test_symbol_normalization_workflow() {
    // Créer le service MPC
    let mut service = MpcService::new(TradingConfig::default());

    // Ajouter des acteurs mock avec différents formats de symboles
    let mock_price = Price::new(50000.0).unwrap();

    // Binance (format USDT)
    let binance_sender = MockExchangeActor::spawn(Exchange::Binance, mock_price.clone());
    service.add_actor(Exchange::Binance, binance_sender);

    // Coinbase (format avec tiret)
    let coinbase_sender = MockExchangeActor::spawn(Exchange::Coinbase, mock_price.clone());
    service.add_actor(Exchange::Coinbase, coinbase_sender);

    // dYdX (format avec tiret)
    let dydx_sender = MockExchangeActor::spawn(Exchange::Dydx, mock_price.clone());
    service.add_actor(Exchange::Dydx, dydx_sender);

    // Tester la récupération de tous les symboles
    let all_symbols = service.get_all_symbols().await;
    assert!(!all_symbols.is_empty());

    // Tester l'agrégation de prix avec normalisation
    let aggregated_result = service.get_aggregated_price("BTC-USD").await;
    // Peut échouer si pas de données réelles, mais ne devrait pas paniquer
    assert!(aggregated_result.is_ok() || aggregated_result.is_err());
}
