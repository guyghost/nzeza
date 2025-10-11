# Nzeza

A Multi-Party Computation (MPC) server for secure crypto trading across multiple exchanges including dydx v4, hyperliquid, coinbase, binance, and kraken.

## Features

- **Multi-Symbol Trading**: Subscribe to multiple trading pairs across different exchanges
- **Real-time Price Updates**: Asynchronous actors using WebSockets for live price feeds
- **MPC Price Aggregation**: Aggregate prices from multiple exchanges for better price discovery
- **Candle Building**: Automatic conversion of price streams to OHLCV candles
- **Trading Signals**: Automated signal generation using combined strategies
- **Trading Tools**: Technical indicators (EMA, RSI, Bollinger Bands, MACD, Stochastic Oscillator, VWAP) and strategies (Fast Scalping, Momentum Scalping, Conservative Scalping)
- **Domain-Driven Design**: Clean architecture with entities, value objects, repositories, and services
- **REST API**: HTTP endpoints for prices, signals, and candle data

## Installation

Ensure you have Rust installed, then:

```bash
cargo build --release
```

## Usage

Run the server:

```bash
cargo run
```

The server will start on `http://127.0.0.1:3000` and automatically subscribe to configured symbols.

### API Endpoints

#### Health Check
```bash
GET /health
```

#### Prices
```bash
# Get all aggregated prices
GET /prices

# Get price for specific symbol
GET /prices/{symbol}
# Example: GET /prices/BTCUSDT
```

#### Trading Signals
```bash
# Get all trading signals
GET /signals

# Get signal for specific symbol
GET /signals/{symbol}
# Example: GET /signals/BTCUSDT
```

#### Candles (OHLCV Data)
```bash
# Get historical candles for a symbol
GET /candles/{symbol}
# Example: GET /candles/BTCUSDT
```

### Configuration

Edit `src/config.rs` to configure which symbols to track on each exchange:

```rust
pub fn default() -> Self {
    let mut symbols = HashMap::new();
    symbols.insert(Exchange::Binance, vec!["BTCUSDT".to_string()]);
    symbols.insert(Exchange::Coinbase, vec!["BTC-USD".to_string()]);
    // ...
}
```

#### Environment Variables

The server requires specific environment variables for authentication and exchange access:

**⚠️ REQUIRED - API Security:**
- `API_KEYS`: Comma-separated list of valid API keys for authentication
  - **The server will NOT start without this variable set**
  - Minimum recommended length: 32 characters per key
  - Example: `API_KEYS=your_secure_random_key_here_min_32_chars,another_key_if_needed`
  - Generate secure keys with: `openssl rand -hex 32`

**dYdX (Required for trading):**
- `DYDX_MNEMONIC`: Your dYdX wallet mnemonic phrase for signing orders
  - ⚠️ **WARNING**: Current dYdX implementation is INCOMPLETE and will NOT work with dYdX v4
  - See `src/infrastructure/dydx_client.rs` for details

**Portfolio Management:**
- `INITIAL_CAPITAL`: Starting capital for the trading bot in USD (default: 10000)
  - Example: `INITIAL_CAPITAL=50000`
  - Portfolio value is dynamically calculated as: Initial Capital + Total PnL

**Optional (for other exchanges):**
- Other exchanges may require API keys in the future
- `PORTFOLIO_PERCENTAGE_PER_POSITION`: Percentage of portfolio to risk per position (default configured in code)

**Production Security Example:**
```bash
# Generate a secure API key
export API_KEYS=$(openssl rand -hex 32)
export DYDX_MNEMONIC="your twelve word mnemonic phrase here"

# For production, store these in a secure secrets manager (e.g., HashiCorp Vault, AWS Secrets Manager)
cargo run
```

**Development Example:**
```bash
# Use a placeholder key for local development (NOT for production!)
export API_KEYS=dev_test_key_min_32_characters_long
cargo run
```

## Security

⚠️ **CRITICAL**: This trading bot runs on HTTP (localhost:3000) **without TLS encryption**.

### Production Deployment Requirements

**DO NOT expose this application directly to the internet.** For production use, you **MUST** implement one of the following:

#### Option 1: TLS Reverse Proxy (Recommended)
Use a reverse proxy with TLS termination:

```nginx
# Nginx example
server {
    listen 443 ssl http2;
    server_name trading.yourdomain.com;

    ssl_certificate /path/to/fullchain.pem;
    ssl_certificate_key /path/to/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Popular reverse proxy options:
- **Nginx**: Battle-tested, high performance
- **Caddy**: Automatic HTTPS with Let's Encrypt
- **Traefik**: Container-native with automatic TLS

#### Option 2: Private Network + VPN
- Deploy on a private VPC/subnet
- Access via VPN (WireGuard, OpenVPN, Tailscale)
- Never expose port 3000 to public internet

### Security Checklist

Before deploying to production:

- [ ] **TLS/HTTPS enabled** via reverse proxy or VPN
- [ ] **Strong API keys** (32+ characters, cryptographically random)
- [ ] **Secrets management** (Vault, AWS Secrets Manager, not .env files)
- [ ] **API key logging removed** ✅ (fixed in this version)
- [ ] **Firewall rules** limiting access to trading server
- [ ] **Monitor logs** for unauthorized access attempts
- [ ] **Rate limiting enabled** (100 req/min default)
- [ ] **Authentication required** for all protected endpoints

### Why TLS is Required

Without TLS:
- API keys transmitted in plaintext over the network
- Trading orders visible to network observers
- Vulnerable to man-in-the-middle attacks
- Portfolio data exposed

**See [SECURITY.md](SECURITY.md) for detailed security guidelines and deployment examples.**

## Development

This project follows Test-Driven Development (TDD) principles. All changes must include tests. See [AGENTS.md](AGENTS.md) for detailed development guidelines.

## License

[Specify license here]