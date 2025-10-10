# Feature: Account Balance Retrieval and Trading Initiation

## Overview
This feature adds the capability to retrieve account balances from dYdX and Coinbase exchanges, log the balance information, and initiate trading operations based on the retrieved balances.

## Requirements
- Retrieve account balances from dYdX v4 API using authenticated endpoints.
- Retrieve account balances from Coinbase API using authenticated endpoints.
- Log balance information at INFO level using tracing.
- Start trading strategies only after successful balance retrieval and validation.
- Handle errors gracefully if balance retrieval fails (e.g., invalid credentials, network issues).

## Implementation Details
- **dYdX Balance Retrieval**: Use the `/accounts` endpoint with EIP-712 signature authentication.
- **Coinbase Balance Retrieval**: Use the `/accounts` endpoint with HMAC-SHA256 authentication.
- **Logging**: Use `tracing::info!` to log balances in a structured format.
- **Trading Initiation**: After balance check, spawn trading actors or enable trading modes if balances are sufficient.
- **Error Handling**: Return errors for failed retrievals, log warnings, and prevent trading if critical.

## API Endpoints
- dYdX: GET /v4/accounts (authenticated)
- Coinbase: GET /accounts (authenticated)

## Security Considerations
- Ensure API keys and secrets are securely stored (environment variables).
- Do not log sensitive information like full API keys.
- Validate balances before proceeding to trading to avoid invalid operations.

## Testing
- Unit tests for balance parsing and error handling.
- Integration tests with mock APIs for balance retrieval.
- Ensure trading does not start without valid balances.