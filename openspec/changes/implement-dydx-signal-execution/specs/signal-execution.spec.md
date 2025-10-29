# Specification: Signal Execution with Balance and Leverage Check

## Capability ID
`signal-execution-with-balance-check`

## Overview
The system must execute trading signals on dYdX after validating and calculating appropriate positions based on balance and leverage.

## Requirements

### Requirement: Validate Signal Before Execution
**ID**: `req-signal-exec-validation`

The system MUST validate trading signals before attempting execution.

#### Scenario: Signal meets confidence threshold
```
Given: Trading signal with confidence = 0.85
And: Minimum confidence threshold = 0.70
When: OrderExecutor.execute_signal() is called
Then: Signal passes validation
And: Execution proceeds to balance check
```

#### Scenario: Signal below confidence threshold
```
Given: Trading signal with confidence = 0.40
And: Minimum confidence threshold = 0.70
When: OrderExecutor.execute_signal() is called
Then: Signal fails validation
And: No trade is executed
And: Error indicates "Low confidence signal"
```

#### Scenario: Hold signals are skipped
```
Given: Trading signal = Signal::Hold
When: OrderExecutor attempts to execute
Then: Signal is recognized as non-actionable
And: No trade is placed
```

### Requirement: Fetch Account Balance Before Execution
**ID**: `req-signal-exec-fetch-balance`

The system MUST fetch fresh account balance before executing any trade.

#### Scenario: Successfully fetch balance for execution
```
Given: Valid dYdX authenticated session
And: Signal has passed validation
When: OrderExecutor.execute_signal() is called
Then: BalanceManager.get_balance() is invoked
And: BalanceInfo is returned with available_balance > 0
And: Execution continues to leverage check
```

#### Scenario: Insufficient balance prevents execution
```
Given: Account balance = $50
And: Min trade size = $100
When: OrderExecutor.execute_signal() is called
Then: Balance check fails
And: Error is returned: "Insufficient balance"
And: No trade is executed
```

#### Scenario: Balance fetch fails
```
Given: dYdX API is unreachable
When: OrderExecutor.execute_signal() is called
Then: Error is returned: "Failed to fetch balance"
And: Trade execution is aborted
And: Retry mechanism may attempt later (not specified here)
```

### Requirement: Calculate Leverage Availability Before Execution
**ID**: `req-signal-exec-leverage-check`

The system MUST verify available leverage before sizing and executing the position.

#### Scenario: Available leverage allows execution
```
Given: Available leverage = 10.0x
And: Desired position would require 3.0x leverage
When: OrderExecutor calculates position
Then: Leverage check passes
And: Position is sized accordingly
```

#### Scenario: Insufficient leverage prevents execution
```
Given: Available leverage = 1.5x
And: Minimum required leverage = 2.0x for desired position
When: OrderExecutor.execute_signal() is called
Then: Leverage check fails
And: Error is returned: "Insufficient leverage"
And: No trade is executed
```

### Requirement: Size Position Based on Balance and Leverage
**ID**: `req-signal-exec-position-sizing`

The system MUST calculate appropriate position size using PositionSizer.

#### Scenario: Position is sized correctly
```
Given: Available balance = $2,000
And: Available leverage = 5.0x
And: Current price = $40,000 (BTC)
And: Max portfolio exposure = 10%
When: OrderExecutor.execute_signal() calls PositionSizer
Then: Calculated quantity = min(
       (2000 * 5) / 40,000,      # balance + leverage
       (2000 * 0.1) / 40,000     # exposure limit
       ) = 0.005 BTC
```

#### Scenario: Position respects minimum order size
```
Given: Calculated position notional = $200
And: Minimum order size = $500
When: PositionSizer returns result
Then: Position is rejected or quantity = 0
And: Error indicates "Below minimum order size"
```

### Requirement: Execute Order on dYdX
**ID**: `req-signal-exec-place-order`

The system MUST place the calculated order on dYdX.

#### Scenario: Successfully place market order
```
Given: Position has been sized and validated
And: Symbol = "BTC-USD"
And: Side = Buy
And: Quantity = 0.05 BTC
When: OrderExecutor.execute_order() is called
Then: Market order is placed on dYdX
And: Order confirmation is received
And: Order ID is recorded
```

#### Scenario: Handle order placement failure
```
Given: Position is ready to execute
And: dYdX returns: "Insufficient margin"
When: OrderExecutor attempts order placement
Then: Error is caught
And: Error message explains dYdX rejection reason
And: Position is not partially executed
```

#### Scenario: Order execution with limit price
```
Given: Signal indicates Buy at current price
And: Config allows limit orders at market price
When: OrderExecutor creates order
Then: Order is placed as market order (immediate execution)
```

### Requirement: Log Execution Metrics
**ID**: `req-signal-exec-metrics`

The system MUST record execution metrics for monitoring and analysis.

#### Scenario: Successful execution is logged
```
Given: Order has been executed successfully
When: Metrics are recorded
Then: Log entry contains:
  - Signal type (Buy/Sell)
  - Confidence level
  - Quantity executed
  - Entry price
  - Margin used
  - Timestamp
  - Status: "Executed"
```

#### Scenario: Failed execution is logged
```
Given: Order execution failed
When: Error is logged
Then: Log entry contains:
  - Signal details
  - Reason for failure (e.g., "Insufficient balance")
  - Timestamp
  - Status: "Failed"
```

### Requirement: Handle Edge Cases and Errors
**ID**: `req-signal-exec-error-handling`

The system MUST gracefully handle errors and edge cases.

#### Scenario: Zero quantity after sizing
```
Given: Available balance < minimum order size
When: PositionSizer returns quantity = 0
Then: OrderExecutor recognizes this as "no trade"
And: Does not attempt to place order
And: Returns clear error message
```

#### Scenario: Concurrent signal race condition
```
Given: Two signals for same symbol arrive simultaneously
And: First signal consumes available balance
When: Second signal is processed
Then: Fresh balance is fetched
And: Second signal is sized against remaining balance
And: No double-spending of balance
```

#### Scenario: Signal during liquidation risk
```
Given: Margin ratio approaching maintenance level
When: Signal arrives
Then: LeverageCalculator detects risk
And: New position is rejected or severely sized down
And: Warning is logged
```

## Data Flow: Complete Signal Execution

```
Signal Generated (Buy, confidence=0.8)
  ↓
[Signal Validation]
  - Confidence >= 0.7? ✓
  - Symbol in allowed list? ✓
  - Rate limits ok? ✓
  ↓
[Fetch Account State]
  - BalanceManager.get_balance()
    → BalanceInfo { available: $2000, ... }
  - LeverageCalculator.get_leverage_info()
    → LeverageInfo { available_leverage: 5.0, ... }
  ↓
[Position Sizing]
  - Call PositionSizer.size_position() with:
    { symbol: "BTC-USD", balance: $2000, leverage: 5.0, price: $40k, ... }
  - Returns: PositionSizingResult { quantity: 0.05 BTC, notional: $2000, ... }
  ↓
[Pre-Execution Checks]
  - Quantity > 0? ✓
  - Notional >= min? ✓
  - Margin <= available? ✓
  ↓
[Create Order]
  - Type: Market
  - Side: Buy
  - Symbol: BTC-USD
  - Quantity: 0.05 BTC
  ↓
[Execute on dYdX]
  - Call dYdX client.place_order()
  - Receive: Order { id: "123abc", status: Filled, ... }
  ↓
[Record Metrics]
  - Log execution with all details
  - Update position tracking
  - Record fill price
  ↓
✓ Trade Complete
```

## Error Handling: Decision Tree

```
Signal arrives
  ├─ Confidence < threshold?
  │  └─ REJECT: "Low confidence signal"
  │
  ├─ Fetch balance fails?
  │  └─ REJECT: "Failed to fetch account balance"
  │
  ├─ Leverage info fails?
  │  └─ REJECT: "Failed to get leverage info"
  │
  ├─ Available balance <= 0?
  │  └─ REJECT: "Insufficient balance"
  │
  ├─ Available leverage <= 0?
  │  └─ REJECT: "Insufficient leverage"
  │
  ├─ Sized quantity <= 0?
  │  └─ REJECT: "Below minimum order size"
  │
  ├─ Order placement fails?
  │  └─ REJECT: "dYdX order placement failed: {reason}"
  │
  └─ ✓ All checks pass
     └─ EXECUTE order
```

## Related Capabilities
- `dydx-account-balance-fetching` (provides balance data)
- `dydx-leverage-calculation` (provides leverage constraints)
- `position-sizing-with-leverage` (calculates position quantity)

## Implementation Notes
- Execute all validations before any API calls to dYdX (except balance/leverage fetch)
- Cache signal validation results to avoid redundant checks
- Always fetch fresh balance/leverage before sizing (don't use cached signal)
- Order execution should be atomic (either fully succeeds or fully fails)
- Log all decisions (accepted/rejected signals) for auditing
- Integrate with existing OrderExecutor in `src/domain/services/order_executor.rs`
