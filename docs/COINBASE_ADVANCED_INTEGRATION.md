# Coinbase Advanced Trade API Integration

## Overview

Successfully created a new Coinbase Advanced Trade API client that uses modern JWT authentication with ES256 (ECDSA P-256) signing. This replaces the old Coinbase Pro API client.

## What Was Done

### 1. Created Coinbase Advanced Trade Client
**File**: `src/infrastructure/coinbase_advanced_client.rs`

- ✅ JWT authentication with ES256 algorithm
- ✅ Proper PEM key parsing and conversion to DER
- ✅ Support for market and limit orders
- ✅ Account retrieval
- ✅ Order placement, cancellation, and status checking
- ✅ Secure API secret handling with Zeroizing

**Key Features**:
```rust
pub struct CoinbaseAdvancedClient {
    client: Client,
    config: CoinbaseAdvancedConfig,
    api_key: String,
    api_secret: Zeroizing<String>,
}
```

**Main Methods**:
- `new(api_key, api_secret)` - Initialize client with Cloud API credentials
- `get_accounts()` - Get all trading accounts
- `place_order(order)` - Place market or limit order
- `cancel_order(order_id)` - Cancel order
- `get_order_status(order_id)` - Check order status

### 2. Added Dependencies
**File**: `Cargo.toml`

```toml
# JWT and ECDSA for Coinbase Advanced Trade API
jsonwebtoken = { version = "10.0", default-features = false, features = ["rust_crypto"] }
p256 = { version = "0.13", features = ["ecdsa", "pem", "pkcs8"] }
rand = "0.8"
```

### 3. Created Test Example
**File**: `examples/test_coinbase_advanced.rs`

Test connection with real credentials:
```bash
cargo run --example test_coinbase_advanced
```

## Old vs New Implementation

### Old Client (Coinbase Pro API)
**File**: `src/infrastructure/coinbase_client.rs`

❌ **Issues**:
- Uses deprecated Coinbase Pro API
- HMAC-SHA256 signature with timestamp
- API format: `CB-ACCESS-KEY`, `CB-ACCESS-SIGN`, `CB-ACCESS-TIMESTAMP`, `CB-ACCESS-PASSPHRASE`
- Different credential format
- Pro API is being deprecated by Coinbase

### New Client (Coinbase Advanced Trade API)
**File**: `src/infrastructure/coinbase_advanced_client.rs`

✅ **Correct Implementation**:
- Uses modern Coinbase Cloud API (Advanced Trade)
- JWT authentication with ES256
- PEM-encoded EC private keys
- No passphrase required
- Future-proof API
- Better security with public-key cryptography

## Technical Details

### Authentication Flow

1. **JWT Generation**:
   ```rust
   let claims = JwtClaims {
       sub: api_key,                    // "organizations/{org_id}/apiKeys/{key_id}"
       iss: "coinbase-cloud",           // Always this value
       nbf: current_time,               // Not before
       exp: current_time + 120,         // Expires in 2 minutes
       uri: "GET api.coinbase.com/path" // Request method + host + path
   };
   ```

2. **Key Handling**:
   - Parse PEM-encoded EC private key
   - Convert to PKCS#8 DER format
   - Use with ES256 algorithm for signing

3. **Request Signing**:
   - Generate random nonce for each request
   - Create JWT with appropriate claims
   - Add `Authorization: Bearer {jwt}` header

### API Endpoints

**Base URL**: `https://api.coinbase.com`

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v3/brokerage/accounts` | GET | List accounts |
| `/api/v3/brokerage/orders` | POST | Place order |
| `/api/v3/brokerage/orders/batch_cancel` | POST | Cancel orders |
| `/api/v3/brokerage/orders/historical/{id}` | GET | Get order status |

### Order Configuration

**Market Order**:
```json
{
  "client_order_id": "nzeza_1234567890",
  "product_id": "BTC-USD",
  "side": "BUY",
  "order_configuration": {
    "market_market_ioc": {
      "base_size": "0.01"
    }
  }
}
```

**Limit Order**:
```json
{
  "client_order_id": "nzeza_1234567890",
  "product_id": "BTC-USD",
  "side": "BUY",
  "order_configuration": {
    "limit_limit_gtc": {
      "base_size": "0.01",
      "limit_price": "50000.00",
      "post_only": false
    }
  }
}
```

## Environment Variables

### Required Credentials

```bash
# API Key (format: organizations/{org_id}/apiKeys/{key_id})
export COINBASE_CLOUD_API_KEY="organizations/12345678-1234-1234-1234-123456789012/apiKeys/abcdef12-3456-7890-abcd-ef1234567890"

# API Secret (PEM-encoded EC private key)
export COINBASE_CLOUD_API_SECRET="-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIL5Dj9qQ8xE...
-----END EC PRIVATE KEY-----"
```

### Alternative Names

The client also checks for:
- `COINBASE_API_KEY` (fallback)
- `COINBASE_API_SECRET` (fallback)

## Testing

### Connection Test
```bash
export COINBASE_CLOUD_API_KEY="your_key_here"
export COINBASE_CLOUD_API_SECRET="your_secret_here"
cargo run --example test_coinbase_advanced
```

Expected output:
```
✅ Credentials loaded
✅ Client created successfully
✅ Accounts retrieved: N accounts
✅ Coinbase Advanced Trade API connection test PASSED
```

### Unit Tests
```bash
cargo test coinbase_advanced
```

## Key Differences from Coinbase Pro API

| Aspect | Pro API (Old) | Advanced Trade API (New) |
|--------|---------------|--------------------------|
| Base URL | `https://api.exchange.coinbase.com` | `https://api.coinbase.com` |
| Auth Method | HMAC-SHA256 | JWT with ES256 |
| API Key Format | Random string | `organizations/{org}/apiKeys/{key}` |
| API Secret Format | Base64 string | PEM-encoded EC private key |
| Passphrase | Required | Not used |
| Headers | `CB-ACCESS-*` | `Authorization: Bearer {jwt}` |
| Status | Deprecated | Active |

## Migration from Old Client

### Before (Old Coinbase Pro)
```rust
let client = CoinbaseClient::new(&api_key, &api_secret, Some(&passphrase))?;
```

### After (New Advanced Trade)
```rust
let client = CoinbaseAdvancedClient::new(&api_key, &api_secret)?;
```

**Note**: Credentials must be regenerated in Coinbase Cloud Portal!

## How to Get Credentials

1. Go to [Coinbase Developer Platform](https://portal.cdp.coinbase.com/)
2. Create or select your organization
3. Navigate to API Keys
4. Click "Create API Key"
5. Select permissions (View, Trade)
6. Download the credentials JSON (keep it safe!)

The JSON will contain:
```json
{
  "name": "organizations/{org_id}/apiKeys/{key_id}",
  "privateKey": "-----BEGIN EC PRIVATE KEY-----\n...\n-----END EC PRIVATE KEY-----"
}
```

Use:
- `name` as `COINBASE_CLOUD_API_KEY`
- `privateKey` as `COINBASE_CLOUD_API_SECRET`

## Security Notes

- ✅ API secrets are Zeroized to reduce memory exposure
- ✅ JWT tokens expire after 2 minutes
- ✅ Each request generates a unique nonce
- ✅ EC P-256 provides strong cryptographic security
- ✅ No passphrase stored in memory
- ⚠️ Never commit credentials to git
- ⚠️ Use environment variables or secrets manager

## Files Created/Modified

1. ✅ `src/infrastructure/coinbase_advanced_client.rs` - New Advanced Trade client
2. ✅ `src/infrastructure/mod.rs` - Added advanced client module
3. ✅ `Cargo.toml` - Added JWT and ECDSA dependencies
4. ✅ `examples/test_coinbase_advanced.rs` - Connection test example
5. ✅ `COINBASE_ADVANCED_INTEGRATION.md` - This documentation

## Old Client Status

The old client (`src/infrastructure/coinbase_client.rs`):
- ⚠️ Still exists for backward compatibility
- ⚠️ Uses deprecated Coinbase Pro API
- ⚠️ Will stop working when Pro API is fully deprecated
- 📌 Recommended: Migrate to Advanced Trade client

## Next Steps

1. ✅ Coinbase Advanced Trade client created
2. ✅ JWT authentication with ES256 implemented
3. ✅ Test example created
4. ⏳ Integrate Advanced Trade client into ExchangeActor
5. ⏳ Update MpcService to use new client
6. ⏳ Test order placement with real credentials
7. ⏳ Remove or deprecate old Coinbase Pro client

## References

- [Coinbase Developer Platform](https://portal.cdp.coinbase.com/)
- [Advanced Trade API Docs](https://docs.cdp.coinbase.com/advanced-trade/docs/welcome)
- [Authentication Guide](https://docs.cloud.coinbase.com/advanced-trade/docs/rest-api-auth)
- [API Reference](https://docs.cdp.coinbase.com/advanced-trade/reference/)

## Success Criteria Met

- ✅ Created new Coinbase Advanced Trade client
- ✅ JWT authentication with ES256 implemented
- ✅ PEM key parsing and DER conversion working
- ✅ Account retrieval functional
- ✅ Order placement, cancellation, status implemented
- ✅ Test example created
- ✅ All code compiles successfully
- ✅ Proper error handling and validation

**Status**: 🎉 **COMPLETE** - Coinbase Advanced Trade API client is ready!

**Next Action**: Integrate into ExchangeActor alongside the old client for smooth transition.
