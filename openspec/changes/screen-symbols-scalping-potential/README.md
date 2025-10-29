# Symbol Screening for Scalping Potential - Change Proposal

**Change ID**: `screen-symbols-scalping-potential`  
**Status**: Ready for Review  
**Phase**: 1 (dYdX Integration)  
**Complexity**: Medium  
**Time Estimate**: 15-18 hours  

## 📋 Quick Summary

Implement an automated **symbol screening system** that identifies the best trading pairs on dYdX for scalping strategies. The bot will:

1. **Discover** all available symbols from dYdX API
2. **Evaluate** each pair against 4 scalping criteria (volatility, volume, spreads, momentum)
3. **Rank** by overall scalping potential (0-1 score)
4. **Persist** results with historical tracking
5. **Expose** via REST API for strategy integration

## 🎯 Problem This Solves

Currently:
- ❌ Static, manually-configured symbol lists
- ❌ Misses new trading opportunities
- ❌ Can't adapt to changing market conditions
- ❌ No data-driven pair selection

With this change:
- ✅ Dynamic symbol discovery from exchange APIs
- ✅ Data-driven candidate selection
- ✅ Real-time ranking updates (every 5 min)
- ✅ Historical trend analysis

## 📁 Files in This Proposal

```
screen-symbols-scalping-potential/
├── proposal.md                    # Executive summary & success criteria
├── design.md                      # Architecture, data models, trade-offs
├── tasks.md                       # 29 ordered development tasks (TDD)
└── specs/
    ├── symbol-discovery/
    │   └── spec.md               # REQ-DISCOVERY: Find all trading pairs
    ├── screening-evaluation/
    │   └── spec.md               # REQ-EVAL: Score by scalping criteria
    ├── result-ranking/
    │   └── spec.md               # REQ-RANK: Order by potential
    ├── result-persistence/
    │   └── spec.md               # REQ-PERSIST: Database storage
    └── api-endpoint/
        └── spec.md               # REQ-API: REST endpoints
```

## 🏗️ Architecture

```
dYdX Exchange Actor (existing)
        ↓
   Streams: prices, volumes, spreads
        ↓
Symbol Screening Service (NEW)
├── Discovery:  Fetch available symbols
├── Evaluation: Calculate volatility, volume, spread, momentum scores
├── Ranking:    Sort by combined scalping potential (0.3*v + 0.3*vol + 0.2*s + 0.2*m)
└── API:        Expose top candidates & trends
```

## 📊 Key Metrics

### Scalping Potential Score Components

| Component | Weight | What it Measures | Good Value |
|-----------|--------|-----------------|-----------|
| **Volatility** | 30% | Price movement intensity | 0.7+ (high swings) |
| **Volume** | 30% | Trading liquidity | 0.7+ ($1M+/day) |
| **Spread** | 20% | Bid-ask tightness | 0.9+ (<0.1%) |
| **Momentum** | 20% | Current trend strength | 0.6+ (strong signal) |

**Overall Score** = 0.3×vol + 0.3×volume + 0.2×spread + 0.2×momentum

**Recommendations**:
- Score ≥ 0.75: **BestCandidate** (execute immediately)
- Score 0.60-0.75: **GoodCandidate** (strong potential)
- Score 0.50-0.60: **FairCandidate** (marginal)
- Score < 0.50: **Avoid** (too risky/illiquid)

## 🗄️ Database Schema

5 new tables in SQLite:
- `symbol_screening_results` - Individual symbol scores
- `discovered_symbols` - Available pairs on each exchange
- `ranking_snapshots` - Complete ranking at each cycle
- `symbol_ranks` - Individual ranks within snapshot
- `ranking_history` - Rank changes over time

All with proper indexes for performance <100ms queries.

## 🔌 API Endpoints

| Endpoint | Purpose |
|----------|---------|
| `GET /api/screening/symbols/dydx` | Top 10-100 candidates |
| `GET /api/screening/symbols/dydx/categories` | Distribution by category |
| `GET /api/screening/symbols/dydx/{symbol}` | Detailed score breakdown |
| `GET /api/screening/symbols/dydx/{symbol}/history` | Rank trends over time |
| `GET /api/screening/symbols/dydx/trends` | Most improved pairs |

All with filtering, sorting, and pagination support.

## ✅ Success Criteria

- ✅ Bot discovers all dYdX trading pairs dynamically
- ✅ Screening produces ranked list with scores 0.0-1.0
- ✅ Bot prioritizes high-potential symbols in strategy evaluation
- ✅ Screening updates every 5 minutes (configurable)
- ✅ >80% test coverage (unit, integration, property-based)
- ✅ Zero regression in existing trading functionality
- ✅ API endpoints fully functional with < 200ms latency

## 📋 Implementation Plan

### Red Phase (4-6 hours)
Write failing tests for all 29 tasks. Tests drive requirements clarity.

### Green Phase (6-8 hours)
Implement minimal code to pass tests. No over-engineering.

### Refactor Phase (3-4 hours)
Clean up, optimize, add docs. Keep tests green.

## 🧪 Testing Strategy

- **Unit Tests**: Each score calculation, validation logic
- **Integration Tests**: Full pipeline with mock dYdX API
- **Property-Based Tests**: Scores always [0,1], ranks unique/sequential
- **Performance Tests**: Evaluate 100 symbols in <10 seconds
- **E2E Tests**: Real dYdX API (read-only, no trades)

All using Rust's `cargo test` + Tokio for async tests.

## 🚨 Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Screening identifies crashes | High loss | Position sizing limits, stop-loss enforcement |
| API rate limits | Outage | Start 5-min intervals, monitor, throttle if needed |
| Compute overhead | Latency | Run on separate actor thread, profile regularly |
| Delisted pairs mid-trade | Lost order | Graceful handling in order executor |
| False signals | Lost trades | Multiple indicators + confluence, backtest |

## 🔄 Integration Points

**Existing Systems to Connect**:
- ✅ Exchange Actor (for market data)
- ✅ Candle Builder (for OHLCV data)
- ✅ Strategy Actors (consume screening results)
- ✅ Database (SQLite repository)
- ✅ HTTP API (main.rs routes)

**No Breaking Changes**: All additions, backward-compatible.

## 📦 Deliverables

This proposal includes:
- ✅ 1 executive summary (proposal.md)
- ✅ 1 detailed design document (design.md)
- ✅ 29 ordered development tasks (tasks.md)
- ✅ 5 capability specifications (24 requirements, 75 scenarios)
- ✅ Complete database schema
- ✅ API contract examples
- ✅ Testing strategy
- ✅ Risk analysis

## 🚀 Next Steps

1. **Review**: Team review of proposal & design decisions
2. **Approve**: Product sign-off on scoring formula & thresholds
3. **Estimate**: Validate 15-18 hour estimate
4. **Begin**: Start Red Phase (write failing tests)
5. **Track**: Use tasks.md as development checklist

## 📚 References

- `proposal.md` - High-level summary
- `design.md` - Architecture & technical decisions
- `tasks.md` - Step-by-step implementation checklist
- `specs/*/spec.md` - Detailed requirements per capability
- `openspec/project.md` - Project conventions & architecture patterns
- `openspec/AGENTS.md` - Development workflow & TDD guidance

---

**Ready to implement?** → Start with `tasks.md` Red Phase
