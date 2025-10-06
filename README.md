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

⚠️ **IMPORTANT**: This bot currently runs on HTTP only (no TLS/HTTPS).

**For production deployment, you MUST:**
1. Use a reverse proxy (Nginx, Caddy) with TLS certificates
2. OR deploy on a private network with VPN access
3. Set strong API keys (32+ characters)
4. Store secrets securely (not in .env files)

**See [SECURITY.md](SECURITY.md) for complete security guidelines and deployment best practices.**

## Development

This project follows Test-Driven Development (TDD) principles. All changes must include tests. See [AGENTS.md](AGENTS.md) for detailed development guidelines.

## License

[Specify license here]