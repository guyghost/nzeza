# Gestion de Portefeuille - Documentation Technique

## Vue d'Ensemble

Le système a été amélioré pour récupérer dynamiquement la valeur du portefeuille depuis **Coinbase Advanced Trade API** et ajuster automatiquement les calculs de taille de position en fonction de la valeur réelle du compte.

## Changements Apportés

### 1. Structure `PortfolioState` (mpc_service.rs:83-105)

Une nouvelle structure traque l'état du portefeuille en temps réel :

```rust
pub struct PortfolioState {
    /// Total portfolio value in USD (or base currency)
    pub total_value: f64,
    /// Cash available for trading (excluding positions)
    pub available_cash: f64,
    /// Value locked in open positions
    pub position_value: f64,
    /// Last update timestamp
    pub last_updated: SystemTime,
}
```

**Valeurs par défaut** :
- `total_value`: 10000.0 USD (fallback si Coinbase n'est pas disponible)
- `available_cash`: 10000.0 USD
- `position_value`: 0.0 USD

### 2. Récupération du Portefeuille depuis Coinbase

#### Méthode `fetch_and_update_portfolio_from_coinbase()` (mpc_service.rs:782-833)

**Fonctionnement** :
1. Récupère le premier trader disponible
2. Envoie un message `GetBalance` au `TraderActor`
3. Attend la réponse avec un timeout de 5 secondes
4. Met à jour `PortfolioState` avec la valeur en USD
5. Log le résultat : `"Updated portfolio value from Coinbase: $X.XX"`

**Gestion d'erreurs** :
- Si pas de trader disponible → `MpcError::InvalidConfiguration`
- Si timeout (5s) → `MpcError::Timeout`
- Si erreur Coinbase → Retourne la valeur cachée précédente

**Code** :
```rust
pub async fn fetch_and_update_portfolio_from_coinbase(&self) -> Result<f64, MpcError> {
    let trader_sender = traders.values().next()
        .ok_or_else(|| MpcError::InvalidConfiguration("No traders available"))?
        .clone();

    trader_sender.send(TraderMessage::GetBalance {
        currency: None,  // Get all balances
        reply: reply_tx,
    }).await?;

    let total_usd = timeout(5s, reply_rx.recv()).await??;

    portfolio_state.total_value = total_usd;
    portfolio_state.last_updated = SystemTime::now();

    Ok(total_usd)
}
```

#### Méthode `get_portfolio_value()` (mpc_service.rs:839-852)

**Fonctionnement** :
1. Retourne la valeur cachée de `PortfolioState`
2. Vérifie l'âge du cache (stale si > 5 minutes)
3. Log un warning si le cache est périmé

**Avantages** :
- Pas de latence (lecture du cache local)
- Détection automatique du cache périmé
- Thread-safe avec `Mutex`

### 3. Mise à Jour Automatique du Portefeuille

#### Après Ouverture de Position (mpc_service.rs:857-866)

```rust
pub async fn update_portfolio_after_position_open(&self, position_value: f64) {
    portfolio_state.available_cash -= position_value;
    portfolio_state.position_value += position_value;
}
```

**Appelé dans** : `execute_order_from_signal()` après création d'une position (ligne 1149-1159)

**Exemple** :
```
Position ouverte: BTC-USD, 0.001 BTC @ $50,000
position_value = 0.001 × 50000 = $50

Avant: available_cash = $10,000, position_value = $0
Après: available_cash = $9,950, position_value = $50
```

#### Après Fermeture de Position (mpc_service.rs:871-881)

```rust
pub async fn update_portfolio_after_position_close(&self, position_value: f64, realized_pnl: f64) {
    portfolio_state.available_cash += position_value + realized_pnl;
    portfolio_state.position_value -= position_value;
    portfolio_state.total_value += realized_pnl;
}
```

**Appelé dans** : `close_position()` lors de la fermeture (ligne 677-690)

**Exemple avec profit** :
```
Position fermée: 0.001 BTC, entry=$50,000, exit=$52,000
position_value = $50
realized_pnl = 0.001 × (52000 - 50000) = +$2

Avant: available_cash = $9,950, position_value = $50, total = $10,000
Après: available_cash = $10,002, position_value = $0, total = $10,002
```

**Exemple avec perte** :
```
Position fermée: 0.001 BTC, entry=$50,000, exit=$48,000
position_value = $50
realized_pnl = 0.001 × (48000 - 50000) = -$2

Avant: available_cash = $9,950, position_value = $50, total = $10,000
Après: available_cash = $9,998, position_value = $0, total = $9,998
```

### 4. Background Task de Rafraîchissement (main.rs:1259-1276)

Une tâche asynchrone met à jour le portefeuille toutes les 60 secondes :

```rust
async fn portfolio_refresh_task(app_state: AppState, interval_duration: Duration) {
    let mut interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;

        info!("💰 Refreshing portfolio value from Coinbase...");
        match app_state.mpc_service.fetch_and_update_portfolio_from_coinbase().await {
            Ok(portfolio_value) => {
                info!("✓ Portfolio updated: ${:.2}", portfolio_value);
            }
            Err(e) => {
                warn!("✗ Failed to refresh portfolio: {}", e);
            }
        }
    }
}
```

**Spawn dans main.rs ligne 286-290** :
```rust
let app_state_clone = app_state.clone();
tokio::spawn(async move {
    portfolio_refresh_task(app_state_clone, Duration::from_secs(60)).await;
});
```

### 5. API Endpoints

#### GET `/portfolio` - Consulter l'État du Portefeuille

**Réponse** :
```json
{
  "total_value": 10052.35,
  "available_cash": 9802.35,
  "position_value": 250.00,
  "last_updated": 1730810400,
  "cache_age_seconds": 45,
  "is_stale": false,
  "currency": "USD"
}
```

**Champs** :
- `total_value`: Valeur totale du portefeuille en USD
- `available_cash`: Liquidités disponibles pour trader
- `position_value`: Valeur bloquée dans les positions ouvertes
- `last_updated`: Timestamp Unix de la dernière mise à jour Coinbase
- `cache_age_seconds`: Âge du cache en secondes
- `is_stale`: `true` si le cache a plus de 5 minutes
- `currency`: Devise (toujours "USD")

#### POST `/portfolio/refresh` - Forcer un Rafraîchissement

**Réponse succès** :
```json
{
  "success": true,
  "portfolio_value": 10052.35,
  "message": "Portfolio refreshed successfully from Coinbase",
  "currency": "USD"
}
```

**Réponse erreur** :
```json
{
  "success": false,
  "error": "Timeout waiting for response"
}
```

**Usage** :
```bash
# Consulter le portfolio
curl -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:3000/portfolio

# Forcer un refresh
curl -X POST \
     -H "Authorization: Bearer YOUR_API_KEY" \
     http://localhost:3000/portfolio/refresh
```

## Calcul de Taille de Position

### Méthode `calculate_position_size()` (mpc_service.rs:916-866)

**Avant** :
```rust
// Valeur statique basée sur INITIAL_CAPITAL (défaut: $10,000)
let portfolio_value = 10000.0 + total_pnl;
```

**Maintenant** :
```rust
// Valeur dynamique depuis Coinbase
let portfolio_value = self.get_portfolio_value().await;
```

**Formule** :
```
position_value = portfolio_value × portfolio_percentage_per_position
quantity = position_value / current_price
```

**Exemple avec configuration par défaut** :
```
portfolio_value = $10,000 (depuis Coinbase)
portfolio_percentage_per_position = 0.02 (2%)
current_price = $50,000 (BTC-USD)

position_value = $10,000 × 0.02 = $200
quantity = $200 / $50,000 = 0.004 BTC
```

**Validations** :
- `portfolio_value > 0` (sinon `MpcError::InvalidInput`)
- `portfolio_value.is_finite()` (pas d'infini/NaN)
- `quantity.is_finite()`
- `quantity >= MIN_ORDER_QUANTITY` (0.0001)

## Configuration

### Variables d'Environnement

```env
# Coinbase Advanced Trade API
COINBASE_ADVANCED_API_KEY=organizations/abc123.../apiKeys/key456...
COINBASE_ADVANCED_API_SECRET=-----BEGIN EC PRIVATE KEY-----
...
-----END EC PRIVATE KEY-----

# Symboles à trader
COINBASE_SYMBOLS=BTC-USD,ETH-USD,SOL-USD

# Trading parameters
PORTFOLIO_PERCENTAGE_PER_POSITION=0.02  # 2% par position
MAX_POSITIONS_PER_SYMBOL=2
MAX_TOTAL_POSITIONS=5
MIN_CONFIDENCE_THRESHOLD=0.75

# Gestion du risque
STOP_LOSS_PERCENTAGE=0.03      # 3%
TAKE_PROFIT_PERCENTAGE=0.06    # 6%
```

### Paramètres de Cache

- **Intervalle de refresh** : 60 secondes (configurable dans `portfolio_refresh_task`)
- **Expiration du cache** : 5 minutes (warning si pas de refresh)
- **Timeout Coinbase** : 5 secondes par requête

## Flux de Données

```
1. Initialisation
   └─ PortfolioState par défaut ($10,000)

2. Premier Refresh (t=0s)
   ├─ fetch_and_update_portfolio_from_coinbase()
   ├─ TraderActor.GetBalance → Coinbase API
   └─ PortfolioState.total_value = $9,875.42

3. Génération de Signal (t=10s)
   ├─ SignalCombiner → TradingSignal(Buy, 0.85)
   └─ store_signal("BTC-USD", signal)

4. Exécution d'Ordre (t=30s)
   ├─ calculate_position_size()
   │   └─ Uses cached portfolio_value ($9,875.42)
   ├─ quantity = $9,875.42 × 0.02 / $50,000 = 0.00395 BTC
   ├─ place_order() → Coinbase
   └─ update_portfolio_after_position_open($197.50)

5. Refresh Automatique (t=60s)
   ├─ fetch_and_update_portfolio_from_coinbase()
   └─ PortfolioState updated avec nouvelle valeur

6. Fermeture de Position (t=5min)
   ├─ check_and_execute_stops()
   ├─ should_take_profit() = true
   ├─ close_position("pos_123")
   │   ├─ realized_pnl = +$11.85
   │   └─ update_portfolio_after_position_close($197.50, $11.85)
   └─ Available cash += $209.35
```

## Avantages de l'Implémentation

### 1. **Précision**
- Valeur réelle du portefeuille depuis Coinbase
- Pas de dérive entre calcul théorique et réalité
- Ajustement automatique après chaque trade

### 2. **Performance**
- Cache local pour éviter trop de requêtes API
- Timeout de 5s pour éviter les blocages
- Refresh asynchrone en arrière-plan

### 3. **Résilience**
- Fallback sur valeur cachée en cas d'erreur Coinbase
- Détection automatique du cache périmé
- Gestion d'erreurs complète avec types spécifiques

### 4. **Observabilité**
- Logs détaillés de chaque refresh
- API endpoint pour monitoring en temps réel
- Métriques d'âge du cache

### 5. **Scalabilité**
- Thread-safe avec `Arc<Mutex<PortfolioState>>`
- Pas de contention (refresh en background)
- Compatible avec multiple traders

## Limitations Actuelles

### 1. Conversion USD

Le système suppose que `trader.get_balance()` retourne directement un montant en USD. Pour une version complète :

```rust
// TODO: Implémenter conversion crypto → USD
let accounts = coinbase_client.get_accounts().await?;
let total_usd = accounts.iter()
    .map(|account| {
        let crypto_value = account.available;
        let crypto_currency = &account.currency;
        let usd_price = get_current_price(crypto_currency, "USD").await?;
        Ok(crypto_value * usd_price)
    })
    .sum::<Result<f64, _>>()?;
```

### 2. Multi-Exchange

Actuellement, seule Coinbase est utilisée pour la valeur du portefeuille. Pour un portefeuille multi-exchange :

```rust
// TODO: Agréger balances de tous les exchanges
let dydx_balance = dydx_client.get_balance().await?;
let coinbase_balance = coinbase_client.get_balance().await?;
let total = dydx_balance + coinbase_balance;
```

### 3. Positions Non Tracées

Si des positions sont ouvertes directement via Coinbase (hors système), le système ne les verra pas. Solution :

```rust
// TODO: Synchroniser positions depuis Coinbase
let open_positions = coinbase_client.get_open_positions().await?;
for position in open_positions {
    if !self.open_positions.contains(&position.id) {
        // Position ouverte ailleurs, l'ajouter au système
    }
}
```

## Monitoring et Debugging

### Logs Importants

```
INFO  💰 Refreshing portfolio value from Coinbase...
INFO  ✓ Portfolio updated: $10,052.35
INFO  Updated portfolio value from Coinbase: $10052.35

INFO  Portfolio updated after position open: available_cash=$9852.35, position_value=$200.00
INFO  Portfolio updated after position close: PnL=$11.85, total_value=$10064.20, available_cash=$10064.20

WARN  Portfolio cache is stale (age: 325s). Consider calling fetch_and_update_portfolio_from_coinbase()
WARN  ✗ Failed to refresh portfolio: Timeout waiting for response
```

### Métriques à Surveiller

1. **Cache Age** : Ne devrait jamais dépasser 5 minutes
2. **Refresh Failures** : Taux d'échec < 5%
3. **Available Cash** : Doit rester positif
4. **Position Value** : Somme des positions ouvertes

### Commandes de Debug

```bash
# Vérifier l'état du portfolio
curl -H "Authorization: Bearer $API_KEY" http://localhost:3000/portfolio | jq

# Forcer un refresh
curl -X POST -H "Authorization: Bearer $API_KEY" http://localhost:3000/portfolio/refresh | jq

# Voir les positions ouvertes
curl -H "Authorization: Bearer $API_KEY" http://localhost:3000/positions | jq

# Voir les métriques système
curl -H "Authorization: Bearer $API_KEY" http://localhost:3000/metrics | jq
```

## Tests

Les tests unitaires existants continuent de fonctionner avec les valeurs par défaut. Pour tester avec Coinbase réel :

```rust
#[tokio::test]
async fn test_portfolio_fetch_from_coinbase() {
    // Requires COINBASE_ADVANCED_API_KEY and COINBASE_ADVANCED_API_SECRET
    let config = TradingConfig::from_env();
    let mpc_service = MpcService::new(config);

    // Add trader with Coinbase client
    let client = ExchangeClientFactory::create(Exchange::Coinbase).await.unwrap();
    let trader = Trader::new("test", strategy, 0.01, 0.7).unwrap();
    trader.add_exchange(Exchange::Coinbase, client);
    mpc_service.add_trader("test", TraderActor::spawn(trader)).await;

    // Fetch portfolio
    let result = mpc_service.fetch_and_update_portfolio_from_coinbase().await;
    assert!(result.is_ok());
    assert!(result.unwrap() > 0.0);
}
```

## Conclusion

Le système est maintenant capable de :
- ✅ Récupérer la valeur réelle du portefeuille depuis Coinbase
- ✅ Ajuster automatiquement les tailles de position
- ✅ Tracker l'état du portefeuille en temps réel
- ✅ Gérer les erreurs et les timeouts
- ✅ Exposer des API pour monitoring
- ✅ Logger toutes les opérations importantes

La valeur du portefeuille est maintenant **dynamique** et non plus **statique**, ce qui permet un trading adapté à la réalité du compte.
