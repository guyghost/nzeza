# Nzeza

A Multi-Party Computation (MPC) server for secure crypto trading across multiple exchanges including dydx v4, hyperliquid, coinbase, binance, and kraken.

## Features

- **Real-time Price Updates**: Asynchronous actors using WebSockets for live price feeds
- **MPC Computations**: Price aggregation, order matching, and portfolio optimization
- **Trading Tools**: Technical indicators (EMA, RSI, Bollinger Bands, MACD, Stochastic Oscillator, VWAP) and strategies (Fast Scalping, Momentum Scalping, Conservative Scalping)
- **Domain-Driven Design**: Clean architecture with entities, value objects, repositories, and services

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

## Development

This project follows Test-Driven Development (TDD) principles. All changes must include tests. See [AGENTS.md](AGENTS.md) for detailed development guidelines.

## License

[Specify license here]