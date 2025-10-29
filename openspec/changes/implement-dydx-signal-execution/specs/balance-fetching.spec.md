# Specification: dYdX Account Balance Fetching and Caching

## Capability ID
`dydx-account-balance-fetching`

## Overview
The system must be able to fetch account balance information from dYdX and cache it for efficient repeated access.

## Requirements

### Requirement: Fetch Account Balance from dYdX API
**ID**: `req-balance-fetch-dydx`

The system MUST fetch the account balance (total available funds) from dYdX v4 using the authenticated account endpoint.

#### Scenario: Successfully fetch balance
```
Given: A valid dYdX client with authenticated account
When: BalanceManager.get_balance() is called
Then: A BalanceInfo struct is returned with:
  - total_balance > 0.0
  - available_balance > 0.0
  - locked_balance >= 0.0
  - timestamp is current
```

#### Scenario: Handle authentication error
```
Given: dYdX client with invalid credentials
When: BalanceManager.get_balance() is called
Then: An error is returned with message indicating authentication failure
```

#### Scenario: Handle network error
```
Given: dYdX API is unreachable
When: BalanceManager.get_balance() is called
Then: An error is returned with message indicating network failure
```

### Requirement: Cache Balance with TTL
**ID**: `req-balance-cache-ttl`

The system MUST cache the fetched balance for a configurable TTL to reduce API calls.

#### Scenario: Return cached balance within TTL
```
Given: BalanceManager has fetched balance 2 seconds ago
And: Cache TTL is 10 seconds
And: No refresh is requested
When: BalanceManager.get_balance() is called again
Then: The cached BalanceInfo is returned immediately
And: No new API call is made
```

#### Scenario: Fetch fresh balance after TTL expires
```
Given: BalanceManager has cached balance 15 seconds ago
And: Cache TTL is 10 seconds
When: BalanceManager.get_balance() is called
Then: A new balance is fetched from dYdX API
And: The cache is updated with new value
```

#### Scenario: Force refresh bypasses cache
```
Given: Cached balance exists
When: BalanceManager.refresh_balance() is called
Then: Balance is fetched from dYdX API regardless of TTL
And: Cache is updated with fresh value
```

### Requirement: Return Structured Balance Information
**ID**: `req-balance-structure`

The system MUST return balance information in a consistent structure containing all relevant balance components.

#### Scenario: BalanceInfo contains required fields
```
Given: Balance has been fetched
When: BalanceInfo is returned
Then: It contains:
  - total_balance: f64 (>= 0.0)
  - available_balance: f64 (>= 0.0, <= total_balance)
  - locked_balance: f64 (>= 0.0, <= total_balance)
  - timestamp: SystemTime (approximately current)
```

#### Scenario: Balance invariants are maintained
```
Given: BalanceInfo has been populated from dYdX
Then: total_balance == available_balance + locked_balance
And: total_balance > 0.0 (for active trading accounts)
```

### Requirement: Handle Insufficient Balance
**ID**: `req-balance-insufficient`

The system MUST distinguish between "no balance" and "balance retrieval failure".

#### Scenario: Account has zero balance
```
Given: Account has no balance on dYdX
When: BalanceManager.get_balance() is called
Then: BalanceInfo is returned with available_balance = 0.0
And: No error is raised (this is a valid state)
```

#### Scenario: Detect insufficient balance for trading
```
Given: BalanceInfo has available_balance < min_trade_amount
When: Caller checks balance before execution
Then: Can determine that balance is insufficient for a given trade
```

## Related Capabilities
- `dydx-leverage-calculation` (consumes balance info to calculate leverage)
- `signal-execution-with-balance-check` (uses balance info for validation)

## Implementation Notes
- Use dYdX v4 API endpoint: `/accounts` or equivalent from official client
- Implement with async/await for non-blocking I/O
- Thread-safe caching using Arc<Mutex>
- Cache key = account_address (inferred from wallet)
- Default TTL = 10 seconds (configurable)
