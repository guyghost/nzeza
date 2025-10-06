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

## Development

This project follows Test-Driven Development (TDD) principles. All changes must include tests. See [AGENTS.md](AGENTS.md) for detailed development guidelines.

## License

[Specify license here]