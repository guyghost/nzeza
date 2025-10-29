# Specification: dYdX Leverage Calculation and Constraints

## Capability ID
`dydx-leverage-calculation`

## Overview
The system must calculate available leverage based on account state, current positions, and dYdX constraints.

## Requirements

### Requirement: Calculate Available Leverage
**ID**: `req-leverage-available`

The system MUST calculate how much leverage remains available for new positions.

#### Scenario: Calculate available leverage with no positions
```
Given: Account with no open positions
And: Max leverage allowed by dYdX = 20x
When: LeverageCalculator.get_leverage_info() is called
Then: LeverageInfo is returned with:
  - max_leverage = 20.0
  - current_leverage = 0.0 (or 1.0 for base)
  - available_leverage = 20.0
```

#### Scenario: Calculate available leverage with open positions
```
Given: Account with open position notional value = $100,000
And: Account equity = $10,000
And: Max leverage allowed = 20x
When: LeverageCalculator.get_leverage_info() is called
Then: LeverageInfo is returned with:
  - current_leverage = 100,000 / 10,000 = 10.0
  - available_leverage = 20.0 - 10.0 = 10.0
```

#### Scenario: Insufficient leverage for new position
```
Given: Current leverage = 19.5x
And: Max leverage = 20x
And: Desired position requires 2x additional leverage
When: PositionSizer tries to size a position
Then: The position size is constrained to fit within available 0.5x
```

### Requirement: Query Account Margin State
**ID**: `req-leverage-margin-state`

The system MUST fetch the current margin/leverage state from dYdX account.

#### Scenario: Successfully fetch account leverage details
```
Given: Valid dYdX account
When: LeverageCalculator queries account info
Then: Returns:
  - Current position value (notional)
  - Account equity
  - Margin ratio
  - Maintenance margin ratio
```

#### Scenario: Calculate margin ratio
```
Given: Total equity = $5,000
And: Total notional value of positions = $50,000
When: Margin ratio is calculated
Then: Ratio = (Equity / Notional) = 0.1 = 10% (requires 1/10 leverage)
```

#### Scenario: Detect margin call conditions
```
Given: Margin ratio < Maintenance Margin Ratio (e.g., < 5%)
When: Account is checked for health
Then: Available leverage should be restricted
And: Warning is logged
```

### Requirement: Respect dYdX Leverage Limits
**ID**: `req-leverage-dydx-limits`

The system MUST respect dYdX's maximum leverage settings for the account.

#### Scenario: Maximum leverage varies by account tier
```
Given: Different dYdX account tiers (e.g., standard vs VIP)
When: LeverageCalculator initializes
Then: It retrieves max_leverage specific to the account
```

#### Scenario: Account-level leverage cap is enforced
```
Given: dYdX account with max_leverage = 20x
When: PositionSizer tries to create 25x leveraged position
Then: Position is capped at 20x or error is returned
```

### Requirement: Handle Leverage Calculation Errors
**ID**: `req-leverage-error-handling`

The system MUST gracefully handle errors during leverage calculation.

#### Scenario: Account data fetch fails
```
Given: dYdX API unavailable
When: LeverageCalculator.get_leverage_info() is called
Then: An error is returned with clear message
And: Retry mechanism is triggered (exponential backoff)
```

#### Scenario: Invalid account state
```
Given: Account in unusual state (e.g., liquidation in progress)
When: Leverage info is fetched
Then: Error indicates account is not ready for trading
And: No trades are executed
```

## Data Structure: LeverageInfo

```rust
pub struct LeverageInfo {
    pub max_leverage: f64,              // e.g., 20.0
    pub current_leverage: f64,          // e.g., 5.0 (if positions = 5x equity)
    pub available_leverage: f64,        // e.g., 15.0 (max - current)
    pub margin_ratio: f64,              // Equity / (Notional / leverage)
    pub maintenance_margin_ratio: f64,  // Minimum allowed ratio
    pub timestamp: SystemTime,
}
```

## Related Capabilities
- `dydx-account-balance-fetching` (provides equity component)
- `position-sizing-with-leverage` (consumes available_leverage)
- `signal-execution-with-balance-check` (validates against available leverage)

## Implementation Notes
- Query dYdX account endpoint to get current positions
- Calculate: current_leverage = sum(position_notionals) / equity
- available_leverage = max_leverage - current_leverage, minimum 0.0
- Cache margin ratios for 30 seconds (less frequent than balance)
- Log warnings when available_leverage < 1.5x (approaching limit)
