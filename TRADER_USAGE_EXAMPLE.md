# Guide d'Utilisation des Traders

## Vue d'ensemble

Le système utilise maintenant une architecture découplée où les **traders** gèrent l'exécution des ordres de manière indépendante des **exchanges**.

## Architecture

```
┌─────────────────┐
│   MpcService    │
├─────────────────┤
│                 │
│ ┌─────────────┐ │     ┌──────────────┐
│ │   Traders   │─┼────▶│ TraderActor  │──┐
│ └─────────────┘ │     └──────────────┘  │
│                 │                        │
│ ┌─────────────┐ │     ┌──────────────┐  │    ┌─────────────────┐
│ │  Exchanges  │─┼────▶│ExchangeActor │  ├───▶│ ExchangeClient  │
│ └─────────────┘ │     └──────────────┘  │    │  (dYdX, etc)    │
└─────────────────┘                        │    └─────────────────┘
                                           │
                                  Exécution d'ordres
```

## Initialisation au Démarrage

### 1. Création des Exchange Clients

Les clients sont créés via `ExchangeClientFactory` à partir des variables d'environnement:

```rust
// Dans main.rs (déjà implémenté)
let exchange_clients = ExchangeClientFactory::create_all().await;
```

**Variables d'environnement requises:**

```bash
# dYdX v4
DYDX_MNEMONIC="your mnemonic phrase here"

# Coinbase Advanced Trade
COINBASE_ADVANCED_API_KEY="organizations/xxx/apiKeys/xxx"
COINBASE_ADVANCED_API_SECRET="-----BEGIN EC PRIVATE KEY-----\n...\n-----END EC PRIVATE KEY-----"

# Coinbase Pro (legacy)
COINBASE_API_KEY="your_api_key"
COINBASE_API_SECRET="your_api_secret"
COINBASE_PASSPHRASE="your_passphrase"
```

### 2. Création des Traders

Chaque trader est créé avec:
- Une stratégie de trading
- Une taille de position maximale
- Un seuil de confiance minimum

```rust
// Dans main.rs (déjà implémenté)
let trader = Trader::new(
    "trader_fastscalping".to_string(),
    Box::new(FastScalping::new()),
    0.01,  // Position size
    0.7    // Min confidence (70%)
)?;

// Ajouter les exchanges disponibles
for (exchange, client) in &exchange_clients {
    trader.add_exchange(exchange.clone(), client.clone());
}

// Spawner l'actor
let trader_sender = TraderActor::spawn(trader);

// Ajouter au service
mpc_service.add_trader("trader_fastscalping".to_string(), trader_sender).await;
```

## Utilisation via API

### Exécuter un Signal de Trading

Pour exécuter un signal via un trader spécifique:

```rust
// Obtenir le trader
let trader = mpc_service.get_trader("trader_fastscalping").await.unwrap();

// Créer le signal
let signal = TradingSignal {
    signal: Signal::Buy,
    confidence: 0.85,
};

// Envoyer la commande
let (reply_tx, mut reply_rx) = mpsc::channel(1);
trader.send(TraderMessage::ExecuteSignal {
    signal,
    symbol: "BTC-USD".to_string(),
    price: Price::new(50000.0)?,
    reply: reply_tx,
}).await?;

// Recevoir la réponse
let order_id = reply_rx.recv().await??;
println!("Order placed: {}", order_id);
```

### Placer un Ordre Manuel

```rust
let trader = mpc_service.get_trader("trader_fastscalping").await.unwrap();

let order = Order::new(
    "manual_order_123".to_string(),
    "ETH-USD".to_string(),
    OrderSide::Buy,
    OrderType::Market,
    None,
    0.1,
)?;

let (reply_tx, mut reply_rx) = mpsc::channel(1);
trader.send(TraderMessage::PlaceOrder {
    order,
    reply: reply_tx,
}).await?;

let order_id = reply_rx.recv().await??;
```

### Changer l'Exchange Actif

Un trader peut basculer entre exchanges dynamiquement:

```rust
let trader = mpc_service.get_trader("trader_fastscalping").await.unwrap();

let (reply_tx, mut reply_rx) = mpsc::channel(1);
trader.send(TraderMessage::SetActiveExchange {
    exchange: Exchange::Coinbase,
    reply: reply_tx,
}).await?;

reply_rx.recv().await??;
```

### Obtenir les Statistiques d'un Trader

```rust
let trader = mpc_service.get_trader("trader_fastscalping").await.unwrap();

let (reply_tx, mut reply_rx) = mpsc::channel(1);
trader.send(TraderMessage::GetStats {
    reply: reply_tx,
}).await?;

let stats = reply_rx.recv().await?;
println!("Trader ID: {}", stats.id);
println!("Total orders: {}", stats.total_orders);
println!("Success rate: {:.2}%",
    100.0 * stats.successful_orders as f64 / stats.total_orders as f64
);
```

## Traders Actuellement Configurés

Le système initialise automatiquement 3 traders au démarrage:

| Trader ID | Stratégie | Description |
|-----------|-----------|-------------|
| `trader_fastscalping` | FastScalping | Trading rapide, signaux fréquents |
| `trader_momentumscalping` | MomentumScalping | Suit les tendances fortes |
| `trader_conservativescalping` | ConservativeScalping | Approche prudente, seuil élevé |

## Endpoints API (à venir)

### Exécuter via un Trader

```bash
POST /api/traders/{trader_id}/execute
{
  "signal": "BUY",
  "symbol": "BTC-USD",
  "confidence": 0.85
}
```

### Obtenir les Stats d'un Trader

```bash
GET /api/traders/{trader_id}/stats
```

Réponse:
```json
{
  "id": "trader_fastscalping",
  "active_exchange": "dYdX",
  "total_orders": 150,
  "successful_orders": 142,
  "failed_orders": 8,
  "success_rate": 94.67,
  "available_exchanges": ["dYdX", "Coinbase"]
}
```

### Lister Tous les Traders

```bash
GET /api/traders
```

Réponse:
```json
{
  "traders": [
    {
      "id": "trader_fastscalping",
      "active_exchange": "dYdX",
      "total_orders": 150
    },
    {
      "id": "trader_momentumscalping",
      "active_exchange": "Coinbase",
      "total_orders": 89
    }
  ]
}
```

### Changer l'Exchange Actif

```bash
POST /api/traders/{trader_id}/exchange
{
  "exchange": "Coinbase"
}
```

## Avantages de cette Architecture

### 1. Flexibilité Multi-Exchange
Un trader peut utiliser plusieurs exchanges et basculer dynamiquement:

```rust
// Trader peut utiliser dYdX
trader.set_active_exchange(Exchange::Dydx)?;
// ... puis basculer vers Coinbase
trader.set_active_exchange(Exchange::Coinbase)?;
```

### 2. Tests Facilités
Utilisation de mocks pour les tests:

```rust
struct MockExchangeClient { /* ... */ }

impl ExchangeClient for MockExchangeClient {
    async fn place_order(&self, _: &Order) -> ExchangeResult<String> {
        Ok("mock_order_id".to_string())
    }
}

let mock = Arc::new(MockExchangeClient::new());
trader.add_exchange(Exchange::Binance, mock);
```

### 3. Isolation des Erreurs
Si un exchange est down, le trader peut basculer automatiquement:

```rust
let health = trader.check_health().await;
if !health[&Exchange::Dydx] {
    trader.set_active_exchange(Exchange::Coinbase)?;
}
```

### 4. Stratégies Indépendantes
Chaque trader a sa propre stratégie et peut opérer indépendamment:

```rust
// Trader conservatif sur dYdX
let conservative = Trader::new("conservative", strategy, 0.01, 0.9)?;

// Trader agressif sur Coinbase
let aggressive = Trader::new("aggressive", strategy, 0.1, 0.5)?;
```

## Évolutions Futures

### Smart Order Routing
```rust
impl Trader {
    async fn route_order(&self, order: &Order) -> Result<String, String> {
        // Sélectionner le meilleur exchange basé sur:
        // - Liquidité
        // - Frais
        // - Latence
        // - Disponibilité
        let best_exchange = self.select_best_exchange(&order).await?;
        // ...
    }
}
```

### Load Balancing
Distribuer les ordres sur plusieurs exchanges pour réduire l'impact:

```rust
// Split large order across multiple exchanges
let order_splits = trader.split_order(&large_order, 3)?;
for (exchange, partial_order) in order_splits {
    trader.place_on_exchange(exchange, partial_order).await?;
}
```

### Failover Automatique
Basculement automatique en cas de problème:

```rust
// Dans TraderActor::run()
if let Err(e) = trader.execute_signal(&signal).await {
    // Tenter sur un autre exchange
    trader.set_active_exchange(fallback_exchange)?;
    trader.execute_signal(&signal).await?;
}
```

## Logs et Monitoring

Le système génère des logs détaillés pour chaque trader:

```
[INFO] TraderActor spawned for trader: trader_fastscalping
[INFO] Trader 'trader_fastscalping' configured with dYdX
[INFO] Trader 'trader_fastscalping' configured with Coinbase
[INFO] Trader trader_fastscalping executing Buy order on dYdX for BTC-USD with confidence 0.85
[INFO] Trader trader_fastscalping successfully executed signal for BTC-USD: order_id=abc123
```

## Résumé

L'architecture avec traders découplés offre:
- ✅ Flexibilité multi-exchange
- ✅ Isolation et résilience
- ✅ Tests faciles avec mocks
- ✅ Scalabilité (N traders × M exchanges)
- ✅ Stratégies indépendantes
- ✅ Smart routing possible

Les traders sont **opérationnels** et prêts à utiliser dès le démarrage du serveur !
