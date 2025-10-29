# Design Document: dYdX Signal Execution with Leverage and Balance

## Architecture Overview

### Component Diagram
```
┌─────────────────────────────────────────────────────────────┐
│  Strategy Layer                                             │
│  (FastScalping, MomentumScalping, etc.)                     │
│  → Emits: TradingSignal { signal: Buy/Sell, confidence }   │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│  Enhanced OrderExecutor                                     │
│  - Validates signal (threshold, rate limits)                │
│  - **NEW**: Fetches balance before execution                │
│  - **NEW**: Calculates leverage and position size           │
│  - Executes order on dYdX                                   │
│  - Logs execution metrics                                   │
└────────────────────┬────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│  New Services                                               │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ BalanceManager                                       │  │
│  │ - Fetch balance from dYdX account                    │  │
│  │ - Cache with TTL (5-10s)                             │  │
│  │ - Return: { total, available, locked }              │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ LeverageCalculator                                   │  │
│  │ - Get max leverage from account config               │  │
│  │ - Calculate used margin (current positions)          │  │
│  │ - Return: { max_leverage, available_leverage }       │  │
│  └──────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ PositionSizer                                        │  │
│  │ - Input: balance, current_price, leverage, limits   │  │
│  │ - Calculate: max_quantity = balance * leverage / price│ │
│  │ - Apply: portfolio exposure limits, min size         │  │
│  │ - Return: quantity to trade                          │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────────┐
│  dYdX V4 Client                                             │
│  - Place market/limit orders                                │
│  - Get account info                                         │
│  - Cancel orders                                            │
└────────────────────────────────────────────────────────────┘
```

## Data Model Changes

### New Structures

#### BalanceInfo
```rust
pub struct BalanceInfo {
    pub total_balance: f64,      // Total account balance
    pub available_balance: f64,  // Available for trading (not locked)
    pub locked_balance: f64,     // Locked in positions
    pub timestamp: SystemTime,   // When balance was fetched
}
```

#### LeverageInfo
```rust
pub struct LeverageInfo {
    pub max_leverage: f64,           // Max allowed leverage (e.g., 20x)
    pub current_leverage: f64,       // Current leverage used
    pub available_leverage: f64,     // Remaining leverage available
    pub margin_ratio: f64,           // Current margin ratio
    pub maintenance_margin_ratio: f64, // Minimum required ratio
}
```

#### PositionSizingRequest
```rust
pub struct PositionSizingRequest {
    pub symbol: String,
    pub available_balance: f64,
    pub leverage: f64,
    pub current_price: f64,
    pub max_portfolio_exposure: f64,  // e.g., 0.1 (10%)
    pub min_order_size: f64,
    pub max_order_size: f64,
}
```

#### PositionSizingResult
```rust
pub struct PositionSizingResult {
    pub quantity: f64,
    pub notional_value: f64,  // quantity * price
    pub margin_used: f64,     // notional / leverage
    pub reason: String,       // Why this size was chosen
}
```

## Service Layer

### 1. BalanceManager

**Responsibility**: Fetch and cache account balance from dYdX

```rust
pub struct BalanceManager {
    exchange_client: Arc<dyn ExchangeClient>,
    cache: Arc<Mutex<Option<CachedBalance>>>,
    cache_ttl: Duration,
}

impl BalanceManager {
    pub async fn get_balance(&self) -> Result<BalanceInfo, String>;
    pub async fn refresh_balance(&self) -> Result<BalanceInfo, String>;
}
```

**Key Points**:
- Cache balance for 5-10 seconds to reduce API calls
- `refresh_balance()` forces a new fetch from API
- Returns structured `BalanceInfo` with total, available, locked

### 2. LeverageCalculator

**Responsibility**: Calculate available leverage based on account state

```rust
pub struct LeverageCalculator {
    exchange_client: Arc<dyn ExchangeClient>,
}

impl LeverageCalculator {
    pub async fn get_leverage_info(&self, account: &Account) -> Result<LeverageInfo, String>;
    pub fn calculate_available_leverage(
        &self,
        max_leverage: f64,
        current_leverage: f64,
    ) -> f64;
}
```

**Key Points**:
- Query dYdX account to get current positions and leverage
- Calculate margin ratio = total_equity / (total_notional_value / max_leverage)
- Available leverage = max_leverage - current_leverage
- Handle edge cases (insufficient margin)

### 3. PositionSizer

**Responsibility**: Calculate appropriate order quantity

```rust
pub struct PositionSizer;

impl PositionSizer {
    pub fn size_position(req: &PositionSizingRequest) -> Result<PositionSizingResult, String> {
        // 1. Calculate max quantity based on balance + leverage
        let max_qty_from_balance = (req.available_balance * req.leverage) / req.current_price;
        
        // 2. Apply portfolio exposure limit
        let max_qty_from_exposure = (req.available_balance * req.max_portfolio_exposure) / req.current_price;
        
        // 3. Apply min/max order size limits
        let quantity = min(max_qty_from_balance, max_qty_from_exposure);
        let quantity = max(quantity, req.min_order_size);
        let quantity = min(quantity, req.max_order_size);
        
        // 4. Return result with justification
        Ok(PositionSizingResult {
            quantity,
            notional_value: quantity * req.current_price,
            margin_used: (quantity * req.current_price) / req.leverage,
            reason: /* explain why this size */,
        })
    }
}
```

**Key Points**:
- Conservative approach: always respect portfolio exposure limits
- Return justification for debugging
- Handle edge cases (zero quantity, insufficient balance)

## Order Execution Flow

### Enhanced Signal Execution

```
1. Signal Generated (Buy @ confidence 0.8)
   ↓
2. Validate Signal
   - Confidence >= threshold? ✓
   - Symbol supported? ✓
   - Rate limits ok? ✓
   ↓
3. **NEW: Fetch Account State**
   - Get balance via BalanceManager
   - Get leverage info via LeverageCalculator
   ↓
4. **NEW: Size Position**
   - Use PositionSizer to calculate order quantity
   - Validate quantity > 0
   ↓
5. Create Order
   - symbol: "BTC-USD"
   - side: Buy
   - quantity: calculated_quantity
   ↓
6. Execute on dYdX
   - Place market or limit order
   ↓
7. Log & Metrics
   - Record execution metrics
   - Update position tracking
```

## Modified Structures

### OrderExecutor Enhancement

```rust
pub struct OrderExecutor {
    // ... existing fields ...
    balance_manager: BalanceManager,        // NEW
    leverage_calculator: LeverageCalculator,  // NEW
    position_sizer: PositionSizer,          // NEW (stateless)
}

impl OrderExecutor {
    pub async fn execute_signal(
        &self,
        symbol: &str,
        signal: &TradingSignal,
        current_price: f64,
    ) -> Result<Order, String> {
        // 1. Validate signal
        self.validate_signal(symbol, signal)?;
        
        // 2. NEW: Fetch balance
        let balance = self.balance_manager.get_balance().await?;
        if balance.available_balance <= 0.0 {
            return Err("Insufficient balance".to_string());
        }
        
        // 3. NEW: Get leverage info
        let leverage_info = self.leverage_calculator.get_leverage_info(&account).await?;
        
        // 4. NEW: Size position
        let sizing_req = PositionSizingRequest {
            symbol: symbol.to_string(),
            available_balance: balance.available_balance,
            leverage: leverage_info.available_leverage,
            current_price,
            max_portfolio_exposure: self.config.portfolio_percentage,
            min_order_size: self.config.min_quantity,
            max_order_size: f64::INFINITY,
        };
        let sizing_result = PositionSizer::size_position(&sizing_req)?;
        
        // 5. Create order with calculated quantity
        let order = Order {
            symbol: symbol.to_string(),
            side: match signal.signal {
                Signal::Buy => OrderSide::Buy,
                Signal::Sell => OrderSide::Sell,
                _ => return Err("Hold signals should not execute".to_string()),
            },
            quantity: sizing_result.quantity,
            order_type: OrderType::Market,
            // ... other fields ...
        };
        
        // 6. Execute on exchange
        self.execute_order(&order).await
    }
}
```

## Error Handling

### New Error Variants

```rust
pub enum TradeExecutionError {
    InsufficientBalance {
        required: f64,
        available: f64,
    },
    InsufficientLeverage {
        required: f64,
        available: f64,
    },
    BalanceFetchFailed(String),
    LeverageCalculationFailed(String),
    PositionSizingFailed(String),
    // ... existing variants ...
}
```

## Testing Strategy

### Unit Tests
- `test_balance_manager_caching` - Verify TTL-based caching
- `test_leverage_calculation_*` - Various margin scenarios
- `test_position_sizer_*` - Edge cases (zero balance, extreme leverage, etc.)

### Integration Tests
- `test_execute_signal_with_balance_check` - Full flow with mock dYdX
- `test_insufficient_balance_prevents_execution`
- `test_leverage_limits_position_size`

### Property Tests
- Generate random balances/prices, verify quantity always respects limits

## Implementation Timeline

1. **Phase 1**: BalanceManager + BalanceInfo (2-3 days)
2. **Phase 2**: LeverageCalculator + LeverageInfo (2-3 days)
3. **Phase 3**: PositionSizer (1-2 days)
4. **Phase 4**: Integrate into OrderExecutor (1-2 days)
5. **Phase 5**: Full integration testing + documentation (1-2 days)

## Performance Considerations

- **Balance Caching**: 5-10 second TTL balances cost of ~1 API call per 10s, acceptable
- **Leverage Queries**: Can be bundled with balance fetch (same account endpoint)
- **Position Sizing**: Pure calculation, O(1), negligible overhead

## Security Considerations

1. **Balance Verification**: Always fresh check before large trades
2. **Leverage Limits**: Respect dYdX maximum leverage settings
3. **API Key Exposure**: BalanceManager only reads balance, never exposes keys
4. **Position Limits**: Enforce max portfolio exposure to prevent over-leverage

## Future Enhancements

- **Dynamic Leverage Adjustment**: Reduce leverage during high volatility
- **Multi-leg Positions**: Size two related positions simultaneously
- **Slippage Estimation**: Adjust position size based on expected slippage
- **Risk-Adjusted Sizing**: Kelly criterion or other advanced sizing models
