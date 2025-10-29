# Proposal: Execute Trading Signals on dYdX with Leverage and Balance Management

## ID
`implement-dydx-signal-execution`

## Overview
Implement the ability to automatically execute trading orders on dYdX when signals are detected, taking into account available balance and leverage. This enhances the trading pipeline from signal generation → signal validation → order execution with proper risk management.

## Problem Statement
Currently, the system:
- ✅ Generates trading signals (Buy/Sell/Hold) from technical indicators and strategies
- ✅ Validates signals against thresholds and confidence levels
- ❌ Cannot execute trades on dYdX with proper balance and leverage verification
- ❌ Lacks position sizing logic based on available balance
- ❌ Doesn't calculate maximum position based on margin/leverage constraints

This prevents the trading system from closing the loop: signal → execution.

## Scope
This proposal covers:
1. **Balance Management**: Retrieve account balance from dYdX and validate sufficient funds
2. **Leverage Calculation**: Determine available leverage and calculate appropriate position size
3. **Position Sizing**: Calculate order quantity based on:
   - Available balance (free collateral)
   - Maximum leverage allowed
   - Risk management parameters (max portfolio exposure)
4. **Signal-to-Trade Flow**: Connect signal generation to dYdX trade execution
5. **Error Handling**: Gracefully handle balance/leverage insufficient errors

This proposal does NOT cover:
- Backtesting with leverage
- Leverage adjustment during live trading
- Complex multi-leg positions
- Cross-collateral calculations

## Architecture Decisions
1. **Separation of Concerns**:
   - `BalanceManager`: Fetches and caches account balance
   - `LeverageCalculator`: Computes available leverage and position limits
   - `PositionSizer`: Calculates order quantities based on balance + leverage
   - Enhanced `OrderExecutor`: Integrates balance/leverage checks before execution

2. **Flow**:
   ```
   Signal Generated
      ↓
   Signal Validation (confidence, symbol, rate limits)
      ↓
   Fetch Account Balance
      ↓
   Calculate Available Leverage
      ↓
   Calculate Position Size
      ↓
   Validate Position Fits Constraints
      ↓
   Execute Trade on dYdX
   ```

3. **Caching**: Cache balance for a short TTL (e.g., 5-10 seconds) to reduce API calls while maintaining freshness

## Acceptance Criteria
- [ ] `BalanceManager` successfully fetches account balance from dYdX
- [ ] `LeverageCalculator` correctly computes available leverage
- [ ] `PositionSizer` calculates order quantity respecting balance and leverage
- [ ] Signal execution flow includes balance/leverage validation
- [ ] Insufficient balance/leverage returns clear error, prevents trade
- [ ] All tests pass (unit + integration)
- [ ] Code follows DDD and actor patterns from the project

## References
- Related: `feature_balance_and_trading.md` (balance retrieval foundation)
- DDD Services: `src/domain/services/order_executor.rs`
- dYdX Client: `src/infrastructure/dydx_v4_client.rs`
- Position Entity: `src/domain/entities/position.rs`

## Capabilities
This proposal defines the following capabilities:
1. **dydx-account-balance-fetching** - Retrieve and cache account balance
2. **dydx-leverage-calculation** - Calculate available leverage and constraints
3. **position-sizing-with-leverage** - Size positions based on balance + leverage
4. **signal-execution-with-balance-check** - Execute trades with pre-execution balance validation
