# Project Context

## Purpose
The purpose of this project is to develop a secure Multi-Party Computation (MPC) server for cryptocurrency trading that enables collaborative trading strategies while preserving participant privacy. The server aims to:
- Aggregate and analyze market data (prices, volumes, order books) from multiple cryptocurrency exchanges (Binance, Kraken, Coinbase, dYdX, Hyperliquid) using MPC protocols.
- Execute trading strategies (e.g., scalping, momentum, arbitrage) with low latency while maintaining confidentiality.
- Ensure robust error handling, logging, and monitoring for reliable operation in production.
- Support backtesting and simulation of trading strategies to validate performance before live trading.
- Comply with exchange-specific API constraints and rate limits.
- Provide MPC-based features like secure price aggregation, order matching, and portfolio optimization.

The primary goal is to enable collaborative trading with privacy guarantees through MPC, maximizing profitability while minimizing risk through efficient and scalable code.

## Tech Stack
- **Rust**: Core programming language for high performance, memory safety, and concurrency.
- **Tokio**: Asynchronous runtime for handling WebSocket connections, concurrent tasks, and actor-based concurrency.
- **Reqwest**: HTTP client for interacting with exchange REST APIs (authenticated for order placement).
- **Tracing**: Structured logging for debugging and monitoring.
- **Serde**: Serialization/deserialization for handling JSON data from APIs.
- **Sqlx**: Async SQL toolkit for database interactions (SQLite for storing trade history).
- **Clap**: Command-line argument parsing for configuration and backtesting modes.
- **Plotters**: For generating visualizations of trading performance during backtesting.

## Project Conventions

### Code Style
- Follow the **Rustfmt** style guide for consistent code formatting (`cargo fmt`).
- Use **Clippy** for linting to enforce idiomatic Rust practices (`cargo clippy`).
- **Naming Conventions**:
  - Variables and functions: `snake_case`.
  - Structs, Enums, and Traits: `PascalCase`.
  - Constants: `SCREAMING_SNAKE_CASE`.
- Prefer explicit return types for functions and avoid implicit `()` returns.
- Use meaningful variable names that reflect their purpose (e.g., `order_book` instead of `ob`).
- Organize code into modules based on domain (e.g., `exchanges`, `strategies`, `indicators`) aligned with DDD principles.

## Log Configuration
The system uses `tracing` for logging with different verbosity levels. Configuration is done via the `RUST_LOG` environment variable.

**Note:** Environment variables can be configured in a `.env` file at the project root. See `ENV-README.md` for detailed instructions.

### Available Log Levels
- **`trace`**: Most verbose - all internal details (e.g., raw WebSocket messages, internal state transitions, actor messages).
- **`debug`**: Debugging information (e.g., individual price updates, API response details, actor state changes).
- **`info`**: General important information (e.g., aggregated price data, successful connections to exchanges, actor lifecycle events).
- **`warn`**: Warnings (e.g., API rate limit warnings, non-critical connection issues, actor recovery attempts).
- **`error`**: Errors (e.g., failed API calls, critical system failures, actor crashes).

### Architecture Patterns
- **Domain-Driven Design (DDD)**: The project is structured around domain concepts, with clear boundaries between domains (e.g., `Exchanges`, `Strategies`, `MarketData`, `OrderExecution`). Each domain has its own models, services, and repositories, ensuring encapsulation and alignment with business logic.
  - **Bounded Contexts**: Each exchange (Binance, Kraken, Coinbase, dYdX, Hyperliquid) is treated as a separate bounded context with its own domain models and logic to handle API differences.
  - **Aggregates**: Define aggregates like `OrderBook`, `Trade`, and `Position` to manage state and enforce invariants.
  - **Repositories**: Use repositories (backed by SQLite) to persist domain entities like trade history and performance metrics.
  - **Domain Services**: Encapsulate complex trading logic (e.g., MPC-based arbitrage opportunity detection) in domain services.
- **Actor Model (Akka-inspired)**: The system uses an actor-based architecture for concurrency and scalability, leveraging Tokio's async runtime.
  - **Actors**: Each major component (e.g., exchange connector, trading strategy, market data processor) is implemented as an actor, communicating via message passing (using Tokio channels).
  - **Supervision**: Implement supervision hierarchies to handle actor failures, with strategies for restarting or stopping actors on errors.
  - **Message Passing**: Actors communicate asynchronously via typed messages (e.g., `PriceUpdate`, `OrderRequest`, `StrategySignal`) to ensure loose coupling.
  - **State Management**: Each actor maintains its own state (e.g., order book state for an exchange actor) to avoid shared mutable state.
- **Event-Driven Architecture**: Combine DDD and the actor model with event-driven processing, where market data from WebSocket streams triggers events that actors process to make trading decisions.
- **Multi-Party Computation (MPC)**: Integrate MPC protocols for secure collaborative computations, such as price aggregation across participants without revealing individual data.
- **Separation of Concerns**:
  - Exchange actors handle API communication (WebSocket for price data and analysis, authenticated REST for order placement).
  - Strategy actors encapsulate trading logic and emit trading signals.
  - Data pipeline actors process and aggregate market data using MPC.
- **Error Handling**: Use custom error types with the `thiserror` crate, propagated through actor messages, to provide detailed error reporting.
- **Configuration**: Use a `config` module with `serde` to load settings from a TOML file or environment variables.

### Testing Strategy
- **Test-Driven Development (TDD)**: All developments must follow the TDD cycle: Red (failing test) -> Green (minimal implementation) -> Refactor.
- **Unit Tests**: Test individual domain models, services, and actor behaviors (e.g., price aggregation logic, technical indicators) using Rust's built-in testing framework (`cargo test`).
- **Integration Tests**: Test interactions between actors and bounded contexts (e.g., exchange actors with mock API responses) in the `tests/` directory.
- **Property-Based Testing**: Use `proptest` to validate trading strategies and actor message handling against a range of market conditions.
- **Backtesting**: Implement a backtesting framework to simulate trading strategies against historical data stored in a SQLite database, ensuring actor interactions are tested.
- **Code Coverage**: Aim for at least 80% test coverage, measured with `cargo-tarpaulin`.
- **CI Testing**: Run all tests in CI (GitHub Actions) on every push and pull request.

### Git Workflow

## Git Workflow: Trunk-Based Development

### Core Principle
This project uses **Trunk-Based Development**, a version control approach centered on a single, always-deployable `main` branch.

### Trunk-Based Development Rules

#### 1. Main Branch (`main`)
- **Always Stable**: The `main` branch must always compile and pass all tests.
- **Single Source of Truth**: All features originate from `main` and are merged back quickly.
- **Deployable at Any Time**: Every commit on `main` should be potentially deployable to production.

#### 2. Feature Branches
- **Short-Lived**: Maximum 1-2 days before merging.
- **Small and Focused**: One branch = one specific feature or fix.
- **Naming Convention**:
  - `feat/description`: New features
  - `fix/description`: Bug fixes
  - `refactor/description`: Refactoring without behavior changes
  - `docs/description`: Documentation only
  - `test/description`: Adding or modifying tests

#### 3. Frequent Commits
- **Commit Often**: At least several times a day.
- **Conventional Commits**: Use the standard format for commit messages.
- **Atomicity**: Each commit should represent a logical unit of change.

#### 4. Continuous Integration
- **Regular Pull/Push**: Synchronize with `main` multiple times a day.
- **Rebase**: Prefer `git rebase` over `git merge` to maintain a linear history.
- **CI/CD**: All tests must pass before merging.

### Commit Format: Conventional Commits
All commits must follow the **Conventional Commits** format:
```
<type>(<scope>): <description>
[optional body]
[optional footer]
```

#### Commit Types
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Refactoring without behavior changes
- `test`: Adding or modifying tests
- `docs`: Documentation only
- `style`: Formatting, missing semicolons, etc.
- `perf`: Performance improvements
- `chore`: Maintenance tasks (dependencies, config, etc.)

#### Examples
```bash
feat(mpc): add price aggregation across exchanges
fix(websocket): handle reconnection on connection loss
refactor(indicators): simplify RSI calculation logic
test(strategies): add tests for momentum scalping
docs(readme): update installation instructions
```

## Domain Context
- **MPC Features**: The server supports MPC-based computations for collaborative trading:
  - **Price Aggregation**: Securely aggregate prices across participants without revealing individual data.
  - **Order Matching**: Match orders collaboratively using MPC protocols.
  - **Portfolio Optimization**: Optimize portfolios through MPC to maintain privacy.
- **Trading Strategies**: The server supports multiple strategies, including:
  - **Scalping**: Fast Scalping, Momentum Scalping, Conservative Scalping based on technical indicators.
  - **Arbitrage**: Exploit price differences across exchanges (Binance, Kraken, Coinbase, dYdX, Hyperliquid).
  - **Momentum**: Trade based on short-term price trends using technical indicators (e.g., RSI, MACD).
  - **Mean Reversion**: Trade based on the assumption that prices revert to their mean.
- **Technical Indicators**: Available indicators include EMA, RSI, Bollinger Bands, MACD, Stochastic Oscillator, Volume (VWAP).
- **Signal Combination**: Weighted combination of signals with confidence scoring.
- **Market Data**: Real-time data (price, volume, order book depth) is sourced via WebSocket streams from exchanges for low-latency analysis, handled by dedicated actors.
- **Order Execution**: Orders (market and limit) are placed using authenticated REST APIs, managed by exchange actors, adhering to exchange-specific rules (e.g., minimum order size, rate limits).
- **Risk Management**: Implement position sizing, stop-loss, and take-profit mechanisms to limit losses, enforced within strategy actors.
- **Performance Metrics**: Track metrics like Sharpe ratio, max drawdown, and ROI during backtesting and live trading, stored in SQLite.

## Important Constraints
- **Performance**: The server must process market data and execute trades with sub-100ms latency, leveraging the actor model for concurrent processing.
- **Reliability**: Handle intermittent API failures and network issues gracefully with retries and exponential backoff, managed by actor supervision.
- **Rate Limits**: Respect exchange API rate limits to avoid bans (e.g., Binance allows 1200 requests per minute for REST APIs).
- **Regulatory Compliance**: Ensure compliance with local regulations (e.g., KYC/AML requirements for exchange accounts).
- **Security**: Store API keys securely using environment variables or a secrets management system (e.g., `secrecy` crate). Ensure MPC protocols maintain privacy.
- **Scalability**: Support multiple exchanges (Binance, Kraken, Coinbase, dYdX, Hyperliquid) and trading pairs without significant code changes, enabled by modular DDD and actor-based concurrency.

## External Dependencies
- **Exchange APIs**:
  - Binance API (WebSocket for market data, authenticated REST for order placement).
  - Kraken API (WebSocket for market data, authenticated REST for order placement).
  - Coinbase API (WebSocket for market data, authenticated REST for order placement).
  - dYdX API (WebSocket for market data, authenticated REST for order placement).
  - Hyperliquid API (WebSocket for market data, authenticated REST for order placement).
- **Database**:
  - SQLite for storing trade history, backtesting data, and performance metrics, accessed via DDD repositories.
- **Monitoring**:
  - Prometheus for collecting runtime metrics (e.g., latency, trade volume, actor message throughput).
  - Grafana for visualizing metrics and performance dashboards.
- **Third-Party Crates**:
  - `tokio` for async runtime and actor-based concurrency.
  - `reqwest` for HTTP requests (authenticated REST APIs).
  - `serde` for JSON parsing.
  - `sqlx` for database interactions (SQLite).
  - `tracing` for logging, integrated with actor events.
  - `thiserror` for error handling in actors and domain services.
  - `plotters` for backtesting visualizations.