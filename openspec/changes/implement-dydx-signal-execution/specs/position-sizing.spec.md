# Specification: Position Sizing with Leverage and Balance

## Capability ID
`position-sizing-with-leverage`

## Overview
The system must calculate appropriate position sizes based on available balance, leverage, and risk constraints.

## Requirements

### Requirement: Calculate Position Size from Balance and Leverage
**ID**: `req-sizing-balance-leverage`

The system MUST calculate the maximum position size given available balance and leverage.

#### Scenario: Basic position sizing calculation
```
Given: Available balance = $1,000
And: Current price = $50,000 (BTC)
And: Available leverage = 5x
When: PositionSizer.size_position() is called
Then: Max quantity = (1000 * 5) / 50,000 = 0.1 BTC
And: Notional value = 0.1 * 50,000 = $5,000
And: Margin used = 5,000 / 5 = $1,000
```

#### Scenario: Position sizing with no leverage
```
Given: Available balance = $1,000
And: Current price = $50,000
And: Available leverage = 1.0 (no leverage allowed)
When: PositionSizer.size_position() is called
Then: Max quantity = 1,000 / 50,000 = 0.02 BTC
And: Notional value = $1,000 (can only use full balance)
```

#### Scenario: Insufficient balance for minimum order
```
Given: Available balance = $10
And: Current price = $50,000
And: Min order size = $100
When: PositionSizer.size_position() is called
Then: Quantity = 0 (rounded down to min)
And: An error or warning is returned
```

### Requirement: Apply Portfolio Exposure Limits
**ID**: `req-sizing-portfolio-limits`

The system MUST limit position size based on maximum portfolio exposure percentage.

#### Scenario: Portfolio exposure limits position size
```
Given: Total portfolio value = $10,000
And: Max portfolio exposure = 10% per trade
And: Available balance = $10,000
And: Current price = $50,000
And: Available leverage = 5x
When: PositionSizer.size_position() is called
Then: Position size is limited by exposure:
  - Max exposure = 10,000 * 0.1 = $1,000
  - Max quantity = 1,000 / 50,000 = 0.02 BTC
  - (Not 0.1 BTC which would be 50% exposure if balance were 100% available)
```

#### Scenario: Conservative sizing with multiple positions
```
Given: Portfolio has max exposure of 5% per trade
And: Already have 2 open positions at 5% each (10% total)
And: Max portfolio limit = 50%
When: Trying to open 3rd position
Then: Position sizes are recalculated or trade is rejected
```

### Requirement: Apply Minimum and Maximum Order Sizes
**ID**: `req-sizing-order-limits`

The system MUST constrain position size to configured limits.

#### Scenario: Size respects minimum order size
```
Given: Min order size = $500
And: Calculated quantity notional = $300
When: PositionSizer applies min constraint
Then: Position is not executed (quantity = 0 or error)
```

#### Scenario: Size respects maximum order size
```
Given: Max order size = $5,000
And: Calculated quantity notional = $8,000
When: PositionSizer applies max constraint
Then: Position is capped at $5,000 notional
And: Quantity adjusted accordingly
```

#### Scenario: Minimum and maximum bounds converge correctly
```
Given: Min size = 0.01 BTC ($500)
And: Max size = 0.05 BTC ($2,500)
And: Calculated optimal = 0.03 BTC ($1,500)
When: PositionSizer returns result
Then: Quantity = 0.03 BTC (within bounds)
```

### Requirement: Provide Sizing Justification
**ID**: `req-sizing-justification`

The system MUST explain why a particular position size was chosen.

#### Scenario: Return sizing reason
```
Given: Position sizing has been calculated
When: PositionSizingResult is returned
Then: It includes a 'reason' field explaining:
  - Why this size was chosen
  - Which constraint was binding (balance, exposure, min, max)
  - Examples: "Limited by portfolio exposure (5%)", 
             "Limited by min order size",
             "Full balance used with available leverage"
```

### Requirement: Handle Edge Cases
**ID**: `req-sizing-edge-cases`

The system MUST handle edge cases gracefully.

#### Scenario: Zero balance
```
Given: Available balance = $0
When: PositionSizer.size_position() is called
Then: Quantity = 0
And: Error message explains insufficient balance
```

#### Scenario: Very high price
```
Given: Current price = $1,000,000
And: Available balance = $500
When: PositionSizer.size_position() is called
Then: Quantity = 0 or very small (< 0.001)
And: Respects minimum order size
```

#### Scenario: Very low price
```
Given: Current price = $0.01
And: Available balance = $1,000
And: Max order size = 10,000,000 units
When: PositionSizer.size_position() is called
Then: Quantity is constrained by max order size
And: Does not exceed maximum quantity
```

#### Scenario: Leverage = 1.0 (no margin)
```
Given: Available leverage = 1.0
And: Available balance = $5,000
When: PositionSizer.size_position() is called
Then: Treats as no-margin trading (size = balance / price)
```

## Data Structure: PositionSizingRequest

```rust
pub struct PositionSizingRequest {
    pub symbol: String,
    pub available_balance: f64,
    pub leverage: f64,                    // Available leverage (1.0 = none, 5.0 = 5x)
    pub current_price: f64,
    pub max_portfolio_exposure: f64,      // e.g., 0.1 for 10%
    pub min_order_size: f64,              // USD notional
    pub max_order_size: f64,              // USD notional
}
```

## Data Structure: PositionSizingResult

```rust
pub struct PositionSizingResult {
    pub quantity: f64,
    pub notional_value: f64,              // quantity * price
    pub margin_used: f64,                 // notional_value / leverage
    pub reason: String,                   // Explanation
}
```

## Implementation Algorithm

```
FUNCTION PositionSizer.size_position(request) -> PositionSizingResult:
    1. Calculate max_qty_from_balance:
       max_qty = (request.available_balance * request.leverage) / request.current_price
    
    2. Calculate max_qty_from_exposure:
       max_qty_exposure = (request.available_balance * request.max_portfolio_exposure) / request.current_price
    
    3. Apply both constraints:
       max_qty = MIN(max_qty_from_balance, max_qty_from_exposure)
    
    4. Apply order size constraints:
       max_notional = request.max_order_size
       max_qty = MIN(max_qty, max_notional / request.current_price)
    
       min_notional = request.min_order_size
       if (max_qty * request.current_price) < min_notional:
           return ERROR("Below minimum order size")
    
    5. Return PositionSizingResult:
       - quantity = max_qty
       - notional_value = quantity * request.current_price
       - margin_used = notional_value / request.leverage
       - reason = explain which constraint was binding
```

## Related Capabilities
- `dydx-account-balance-fetching` (provides available_balance)
- `dydx-leverage-calculation` (provides leverage)
- `signal-execution-with-balance-check` (uses calculated quantity)

## Implementation Notes
- Pure calculation, no I/O needed (stateless service)
- All quantities in base currency units (e.g., BTC, ETH)
- All monetary values in USD or account currency
- Always round quantity DOWN (conservative sizing)
- Handle floating point precision issues (e.g., 0.99999 < 1.0)
