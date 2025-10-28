# Gestion de Portefeuille Multi-Exchange - Documentation Technique

## Vue d'Ensemble

Le système a été amélioré pour récupérer dynamiquement la valeur du portefeuille depuis **tous les exchanges configurés** (Coinbase Advanced Trade, dYdX v4, etc.) et ajuster automatiquement les calculs de taille de position en fonction de la valeur réelle agrégée des comptes.

## Changements Apportés

### 1. Structure `PortfolioState` (mpc_service.rs:83-108)

Une nouvelle structure traque l'état du portefeuille en temps réel avec support multi-exchange :

```rust
pub struct PortfolioState {
    /// Total portfolio value in USD (or base currency)
    pub total_value: f64,
    /// Cash available for trading (excluding positions)
    pub available_cash: f64,
    /// Value locked in open positions
    pub position_value: f64,
    /// Balance per exchange (trader_id -> balance_usd)
    pub exchange_balances: HashMap<String, f64>,
    /// Last update timestamp
    pub last_updated: SystemTime,
}
```

**Valeurs par défaut** :
- `total_value`: 10000.0 USD (fallback si Coinbase n'est pas disponible)
- `available_cash`: 10000.0 USD
- `position_value`: 0.0 USD

### 2. Récupération du Portefeuille depuis Tous les Exchanges

#### Méthode `fetch_and_update_portfolio_from_exchanges()` (mpc_service.rs:795-877)

**Fonctionnement** :
1. Récupère **tous** les traders disponibles avec leurs IDs
2. Envoie un message `GetBalance` à **chaque** `TraderActor`
3. Attend la réponse de chaque trader avec un timeout de 5 secondes
4. **Agrège** toutes les balances en USD
5. Stocke les balances individuelles par trader dans `exchange_balances`
6. Met à jour `PortfolioState` avec la valeur totale
7. Log le résultat : `"✓ Updated portfolio value from N exchange(s): $X.XX"`

**Gestion d'erreurs** :
- Si pas de trader disponible → `MpcError::InvalidConfiguration`
- Si timeout (5s) par trader → Continue avec les autres traders
- Si un trader échoue → Log warning et continue
- Si **tous** les traders échouent → Retourne la valeur cachée précédente

**Exemple de logs** :
```
INFO  ✓ Trader 'trader_fastscalping' balance: $5,432.10
INFO  ✓ Trader 'trader_momentumscalping' balance: $3,210.55
INFO  ✓ Trader 'trader_conservative' balance: $1,357.35
INFO  ✓ Updated portfolio value from 3 exchange(s): $10,000.00
DEBUG   - trader_fastscalping: $5,432.10
DEBUG   - trader_momentumscalping: $3,210.55
DEBUG   - trader_conservative: $1,357.35
```

**Code** :
```rust
pub async fn fetch_and_update_portfolio_from_exchanges(&self) -> Result<f64, MpcError> {
    let trader_info: Vec<(String, Sender)> = traders.iter()
        .map(|(id, sender)| (id.clone(), sender.clone()))
        .collect();

    let mut total_usd = 0.0;
    let mut exchange_balances = HashMap::new();

    for (trader_id, trader_sender) in trader_info.iter() {
        match timeout(5s, trader_sender.send(...)).await {
            Ok(balance_usd) => {
                total_usd += balance_usd;
                exchange_balances.insert(trader_id.clone(), balance_usd);
            }
            Err(e) => warn!("Trader '{}' failed: {}", trader_id, e),
        }
    }

    portfolio_state.total_value = total_usd;
    portfolio_state.exchange_balances = exchange_balances;
    portfolio_state.last_updated = SystemTime::now();

    Ok(total_usd)
}
```

#### Méthode `fetch_and_update_portfolio_from_coinbase()` (DEPRECATED)

Cette méthode est conservée pour compatibilité arrière mais délègue maintenant à `fetch_and_update_portfolio_from_exchanges()`:

```rust
#[deprecated(since = "0.2.0", note = "Use fetch_and_update_portfolio_from_exchanges instead")]
pub async fn fetch_and_update_portfolio_from_coinbase(&self) -> Result<f64, MpcError> {
    self.fetch_and_update_portfolio_from_exchanges().await
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

### 4. Background Task de Rafraîchissement (main.rs:1304-1321)

Une tâche asynchrone met à jour le portefeuille depuis **tous les exchanges** toutes les 60 secondes :

```rust
async fn portfolio_refresh_task(app_state: AppState, interval_duration: Duration) {
    let mut interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;

        info!("💰 Refreshing portfolio value from all exchanges...");
        match app_state.mpc_service.fetch_and_update_portfolio_from_exchanges().await {
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
  "exchange_balances": {
    "trader_fastscalping": 5432.10,
    "trader_momentumscalping": 3210.55,
    "trader_conservative": 1409.70
  },
  "last_updated": 1730810400,
  "cache_age_seconds": 45,
  "is_stale": false,
  "currency": "USD"
}
```

**Champs** :
- `total_value`: Valeur totale du portefeuille en USD (somme de tous les exchanges)
- `available_cash`: Liquidités disponibles pour trader
- `position_value`: Valeur bloquée dans les positions ouvertes
- `exchange_balances`: Balance par trader/exchange (trader_id → balance USD)
- `last_updated`: Timestamp Unix de la dernière mise à jour
- `cache_age_seconds`: Âge du cache en secondes
- `is_stale`: `true` si le cache a plus de 5 minutes
- `currency`: Devise (toujours "USD")

#### POST `/portfolio/refresh` - Forcer un Rafraîchissement

**Réponse succès** :
```json
{
  "success": true,
  "portfolio_value": 10052.35,
  "message": "Portfolio refreshed successfully from all exchanges",
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

2. Premier Refresh (t=0s) - MULTI-EXCHANGE
   ├─ fetch_and_update_portfolio_from_exchanges()
   ├─ TraderActor[0].GetBalance → Coinbase API → $5,432.10
   ├─ TraderActor[1].GetBalance → dYdX API → $3,210.55
   ├─ TraderActor[2].GetBalance → Coinbase API → $1,232.77
   ├─ Agrégation: $5,432.10 + $3,210.55 + $1,232.77 = $9,875.42
   └─ PortfolioState.total_value = $9,875.42
       PortfolioState.exchange_balances = {
         "trader_fastscalping": 5432.10,
         "trader_momentumscalping": 3210.55,
         "trader_conservative": 1232.77
       }

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
   ├─ fetch_and_update_portfolio_from_exchanges()
   ├─ Query tous les traders/exchanges
   └─ PortfolioState updated avec nouvelles valeurs agrégées

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

## Exchanges Supportés

### 1. Coinbase Advanced Trade

- **API**: REST API avec authentification JWT ES256
- **Balance**: Retourne USDC/USD directement
- **Conversion**: Non requise (déjà en USD)
- **Status**: ✅ Production-ready

### 2. dYdX v4

- **API**: Indexer API + Node Client
- **Balance**: Retourne equity en USDC
- **Conversion**: Non requise (USDC = USD)
- **Status**: ⚠️  Signature Ethereum (EIP-712) au lieu de Cosmos SDK

### 3. Autres Exchanges (Binance, Kraken, Hyperliquid)

- **Status**: 🚧 Pas encore implémentés
- **TODO**: Ajouter clients + conversion crypto → USD

## Limitations Actuelles

### 1. Conversion Crypto → USD

Le système suppose que les balances sont en USD/USDC. Pour les cryptos natives (BTC, ETH, etc.), la conversion n'est pas encore implémentée:

```rust
// TODO: Implémenter conversion pour autres cryptos
let accounts = exchange_client.get_balance().await?;
let total_usd = accounts.iter()
    .map(|account| {
        if account.currency == "USD" || account.currency == "USDC" {
            Ok(account.available)
        } else {
            let usd_price = get_current_price(&account.currency, "USD").await?;
            Ok(account.available * usd_price)
        }
    })
    .sum::<Result<f64, _>>()?;
```

### 2. Positions Non Tracées

Si des positions sont ouvertes directement via un exchange (hors système), le système ne les verra pas. Solution :

```rust
// TODO: Synchroniser positions depuis tous les exchanges
for (trader_id, client) in exchange_clients.iter() {
    let open_positions = client.get_open_positions().await?;
    for position in open_positions {
        if !self.open_positions.contains(&position.id) {
            // Position ouverte ailleurs, l'ajouter au système
            warn!("Detected external position on {}: {}", trader_id, position.id);
        }
    }
}
```

### 3. Duplication de Traders sur Même Exchange

Si plusieurs traders utilisent le **même exchange** (ex: trader_fastscalping et trader_conservative sur Coinbase), le système va **compter deux fois** la même balance. Solutions:

**Option A**: Un trader par exchange unique
```rust
// S'assurer qu'un seul trader par exchange
let mut exchange_usage = HashMap::new();
for (trader_id, trader) in traders {
    let exchange = trader.active_exchange;
    if exchange_usage.contains_key(&exchange) {
        warn!("Exchange {:?} already used by trader {}, skipping {}",
              exchange, exchange_usage[&exchange], trader_id);
        continue;
    }
    exchange_usage.insert(exchange, trader_id);
}
```

**Option B**: Détecter et ne compter qu'une fois
```rust
// Déduplication par exchange client
let mut seen_exchanges = HashSet::new();
for (trader_id, trader_sender) in traders {
    let exchange = trader.active_exchange;
    if seen_exchanges.contains(&exchange) {
        debug!("Skipping {} (duplicate exchange {:?})", trader_id, exchange);
        continue;
    }
    seen_exchanges.insert(exchange);
    // Query balance
}
```

**Status**: ⚠️  À implémenter si plusieurs traders utilisent le même exchange

## Monitoring et Debugging

### Logs Importants

```
INFO  💰 Refreshing portfolio value from all exchanges...
INFO  ✓ Trader 'trader_fastscalping' balance: $5,432.10
INFO  ✓ Trader 'trader_momentumscalping' balance: $3,210.55
INFO  ✓ Trader 'trader_conservative' balance: $1,409.70
INFO  ✓ Updated portfolio value from 3 exchange(s): $10,052.35
DEBUG   - trader_fastscalping: $5,432.10
DEBUG   - trader_momentumscalping: $3,210.55
DEBUG   - trader_conservative: $1,409.70
INFO  ✓ Portfolio updated: $10,052.35

INFO  Portfolio updated after position open: available_cash=$9852.35, position_value=$200.00
INFO  Portfolio updated after position close: PnL=$11.85, total_value=$10064.20, available_cash=$10064.20

WARN  Portfolio cache is stale (age: 325s). Consider calling fetch_and_update_portfolio_from_exchanges()
WARN  ✗ Trader 'trader_dydx' failed to get balance: Timeout waiting for response
WARN  Failed to fetch balance from all exchanges, using cached value
```

### Métriques à Surveiller

1. **Cache Age** : Ne devrait jamais dépasser 5 minutes
2. **Refresh Failures** : Taux d'échec < 5% par exchange
3. **Exchange Coverage**: Tous les exchanges devraient répondre
4. **Available Cash** : Doit rester positif
5. **Position Value** : Somme des positions ouvertes
6. **Balance Consistency**: `total_value` ≈ Σ `exchange_balances`

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
- ✅ Récupérer la valeur réelle du portefeuille depuis **tous les exchanges** (Coinbase, dYdX)
- ✅ **Agréger** les balances de multiples exchanges en temps réel
- ✅ Tracker la balance **par exchange** pour une visibilité complète
- ✅ Ajuster automatiquement les tailles de position
- ✅ Gérer les erreurs et les timeouts par exchange (failover)
- ✅ Continuer si un exchange échoue (résilience)
- ✅ Exposer des API pour monitoring détaillé
- ✅ Logger toutes les opérations par exchange

La valeur du portefeuille est maintenant **dynamique, multi-exchange et agrégée** et non plus **statique sur un seul exchange**, ce qui permet un trading adapté à la réalité complète du portefeuille cross-exchange.
