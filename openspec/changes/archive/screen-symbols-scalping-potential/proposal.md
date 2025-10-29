# Change Proposal: Symbol Screening for Scalping Potential

## Summary
Implement an automated symbol screening system that analyzes available trading pairs on dYdX (and future exchanges) to identify those with the highest scalping potential. The bot will evaluate symbols based on volatility, trading volume, spread dynamics, and technical indicator signals to prioritize which pairs to monitor and trade.

## Problem Statement
Currently, the trading bot monitors a static or manually-configured set of symbols. This approach:
- Misses emerging opportunities on new trading pairs
- Cannot dynamically adapt to changing market conditions
- Requires manual configuration for each new symbol added to exchanges
- Doesn't leverage available market data to identify optimal scalping candidates

## Solution Overview
Create a **Symbol Screening Service** that:
1. **Discovers** all available trading pairs on dYdX via API
2. **Evaluates** each symbol against scalping criteria (volatility, volume, spreads, technical signals)
3. **Ranks** symbols by scalping potential score
4. **Updates** recommendations dynamically as market conditions change
5. **Integrates** screening results into strategy actor decision-making

## Scope - Phase 1: dYdX
- Fetch available markets/symbols from dYdX API
- Implement core screening logic (volatility, volume, spread analysis)
- Create screening actor for periodic evaluation
- Store and expose screening results via API endpoint
- Extend to other exchanges (Binance, Kraken, Coinbase, Hyperliquid) in future phases

## Success Criteria
- ✅ Bot can dynamically discover all dYdX trading pairs
- ✅ Screening produces ranked list of symbols with scalping potential scores
- ✅ Bot prioritizes high-potential symbols for strategy evaluation
- ✅ Screening updates at configurable intervals (e.g., every 5 minutes)
- ✅ All functionality covered by tests (>80% coverage)
- ✅ Zero regression in existing trading functionality
