# Tasks: Symbol Screening for Scalping Potential (dYdX Phase 1)

## Red Phase: Write Failing Tests

### Task 1: SymbolScreeningResult Data Model Tests
- [ ] Create `symbol_screening_tests.rs` module
- [ ] Test `SymbolScreeningResult` validation (all scores in [0,1])
- [ ] Test score range enforcement (panic/error on invalid scores)
- [ ] Test ranking order maintenance
- **Validation**: `cargo test symbol_screening_tests`

### Task 2: Volatility Score Calculation Tests
- [ ] Test volatility score = 0 for flat candles
- [ ] Test volatility score increases with price range
- [ ] Test volatility normalization to [0,1]
- [ ] Test edge case: single candle (no history)
- **Validation**: `cargo test volatility_score`

### Task 3: Volume Score Calculation Tests
- [ ] Test volume score based on recent average
- [ ] Test volume normalization with mock volumes
- [ ] Test edge case: zero volume
- [ ] Test edge case: very high volume spikes
- **Validation**: `cargo test volume_score`

### Task 4: Spread Score Calculation Tests
- [ ] Test spread score from bid-ask pairs
- [ ] Test spread normalization to [0,1]
- [ ] Test edge case: zero spread (bid == ask)
- [ ] Test edge case: very wide spreads
- **Validation**: `cargo test spread_score`

### Task 5: Momentum Score Calculation Tests
- [ ] Test momentum score from RSI indicator
- [ ] Test momentum score from MACD indicator
- [ ] Test momentum combining multiple indicators
- [ ] Test confidence decay when indicators conflict
- **Validation**: `cargo test momentum_score`

### Task 6: Overall Scalping Score Aggregation Tests
- [ ] Test weighted score combination
- [ ] Test score aggregation with known inputs
- [ ] Test ranking multiple symbols by score
- [ ] Test recommendation categorization (BestCandidate, GoodCandidate, etc.)
- **Validation**: `cargo test scalping_potential_score`

### Task 7: Symbol Screening Service Tests
- [ ] Test screening service initialization
- [ ] Test screening of single symbol
- [ ] Test screening of multiple symbols
- [ ] Test result caching (no duplicate calculations)
- [ ] Test cache expiration after interval
- **Validation**: `cargo test screening_service`

### Task 8: Screening Repository Tests
- [ ] Test persist screening result to database
- [ ] Test retrieve recent screening results
- [ ] Test retrieve historical screening scores by symbol
- [ ] Test update screening result
- [ ] Test delete old screening records
- **Validation**: `cargo test screening_repository`

### Task 9: Screening Actor Lifecycle Tests
- [ ] Test screening actor spawning
- [ ] Test periodic evaluation trigger
- [ ] Test actor message handling
- [ ] Test actor shutdown and cleanup
- **Validation**: `cargo test screening_actor`

### Task 10: End-to-End API Integration Tests
- [ ] Test `GET /api/screening/symbols/dydx` returns ranked list
- [ ] Test endpoint response format (JSON structure)
- [ ] Test result ordering (highest score first)
- [ ] Test filtering by recommendation level
- **Validation**: `cargo test --test integration_tests`

## Green Phase: Implement Minimal Code

### Task 11: Create SymbolScreeningResult Data Model
- [ ] Define `SymbolScreeningResult` struct with all fields
- [ ] Implement validation in `new()` function
- [ ] Add getters for immutability
- **Files**: `src/domain/entities/symbol_screening.rs`
- **Validation**: All Task 1 tests pass

### Task 12: Implement Volatility Score Calculator
- [ ] Create `VolatilityScoreCalculator` trait/struct
- [ ] Calculate score from candle high/low/close
- [ ] Normalize to [0,1] range
- **Files**: `src/domain/services/screening/volatility.rs`
- **Validation**: All Task 2 tests pass

### Task 13: Implement Volume Score Calculator
- [ ] Create `VolumeScoreCalculator` trait/struct
- [ ] Calculate score from recent volumes
- [ ] Normalize to [0,1] range
- **Files**: `src/domain/services/screening/volume.rs`
- **Validation**: All Task 3 tests pass

### Task 14: Implement Spread Score Calculator
- [ ] Create `SpreadScoreCalculator` trait/struct
- [ ] Calculate score from bid-ask spreads
- [ ] Normalize to [0,1] range
- **Files**: `src/domain/services/screening/spread.rs`
- **Validation**: All Task 4 tests pass

### Task 15: Implement Momentum Score Calculator
- [ ] Create `MomentumScoreCalculator` trait/struct
- [ ] Use existing RSI, MACD indicators
- [ ] Combine multiple indicators with weighting
- **Files**: `src/domain/services/screening/momentum.rs`
- **Validation**: All Task 5 tests pass

### Task 16: Implement Scalping Potential Score Aggregator
- [ ] Create aggregation function combining all scores
- [ ] Weight components: 0.3 volatility, 0.3 volume, 0.2 spread, 0.2 momentum
- [ ] Implement ranking and recommendation logic
- **Files**: `src/domain/services/screening/aggregator.rs`
- **Validation**: All Task 6 tests pass

### Task 17: Create Symbol Screening Service
- [ ] Create `SymbolScreeningService` in domain services
- [ ] Implement `screen_symbol()` method
- [ ] Implement `screen_all_symbols()` method
- [ ] Add result caching mechanism
- **Files**: `src/domain/services/symbol_screening.rs`
- **Validation**: All Task 7 tests pass

### Task 18: Create Screening Repository
- [ ] Create database table: `symbol_screening_results`
- [ ] Implement repository methods (create, read, update, delete)
- [ ] Add migration logic
- **Files**: `src/persistence/screening_repository.rs`
- **Validation**: All Task 8 tests pass, database initialized

### Task 19: Implement Screening Actor
- [ ] Create `ScreeningActor` in application layer
- [ ] Implement periodic evaluation loop (5-minute interval)
- [ ] Handle dYdX market discovery
- [ ] Emit results to channel for downstream consumption
- **Files**: `src/application/actors/screening_actor.rs`
- **Validation**: All Task 9 tests pass

### Task 20: Add Screening API Endpoint
- [ ] Add route handler in `main.rs`
- [ ] Implement `GET /api/screening/symbols/dydx`
- [ ] Return JSON with ranked symbols and scores
- **Files**: Updated `src/main.rs`
- **Validation**: All Task 10 tests pass

## Refactor Phase: Clean Up & Optimize

### Task 21: Extract Common Scoring Logic
- [ ] Create `ScoreCalculator` trait for consistency
- [ ] Remove code duplication in score calculators
- [ ] Add comprehensive documentation
- **Files**: `src/domain/services/screening/mod.rs`
- **Validation**: All tests still pass, code quality improved

### Task 22: Add Error Handling & Logging
- [ ] Add structured logging with `tracing` to screening service
- [ ] Create custom error types for screening failures
- [ ] Handle edge cases (missing data, API errors)
- **Files**: Updated screening files
- **Validation**: Logs visible at `RUST_LOG=nzeza=debug`

### Task 23: Optimize Cache Performance
- [ ] Profile cache hit/miss ratio
- [ ] Consider LRU cache vs. TTL-based approach
- [ ] Document cache behavior in comments
- **Files**: `src/domain/services/symbol_screening.rs`
- **Validation**: No performance regression in actor benchmarks

### Task 24: Add Configuration Options
- [ ] Add screening interval to `config.toml`
- [ ] Add score thresholds to configuration
- [ ] Allow per-strategy screening preferences
- **Files**: `src/config.rs`
- **Validation**: Configuration loads correctly

### Task 25: Comprehensive Integration Test
- [ ] Create end-to-end test with mock exchange data
- [ ] Test full pipeline: discovery → screening → ranking → API
- [ ] Verify no regression in existing trading functionality
- **Files**: `tests/symbol_screening_integration.rs`
- **Validation**: `cargo test --test symbol_screening_integration`

### Task 26: Documentation & Examples
- [ ] Document screening results format
- [ ] Add example API requests/responses
- [ ] Update AGENTS.md with screening guidance
- **Files**: `docs/SYMBOL_SCREENING.md`
- **Validation**: Examples run without errors

## Validation & Sign-Off

### Task 27: Run Full Test Suite
- [ ] `cargo test` → all tests pass
- [ ] `cargo clippy` → no warnings
- [ ] `cargo fmt -- --check` → code properly formatted
- [ ] Coverage > 80% for new code

### Task 28: Manual Acceptance Testing
- [ ] Verify bot discovers dYdX symbols on startup
- [ ] Verify screening scores are reasonable and vary
- [ ] Verify API endpoint returns expected data
- [ ] Verify existing trading strategies still work
- [ ] Verify no performance regression

### Task 29: Create Change Commit
- [ ] Review all changes
- [ ] Ensure no secrets in commits
- [ ] Use Conventional Commit format: `feat(screening): add symbol screening for scalping potential`
- [ ] Include description of what was added and why

---

## Dependencies & Parallelization

**Sequential (must complete in order):**
1. Tasks 1-6 → Task 11-16 (data model and scoring must be tested before implementation)
2. Tasks 7-8 → Task 17-18 (service and repo tests before implementation)
3. Tasks 11-20 must all complete before Task 25

**Parallelizable (can work on simultaneously):**
- Tasks 1-10 can be written in parallel (separate test files)
- Tasks 11-20 can be implemented in parallel once respective tests written
- Tasks 21-24 can begin once Tasks 11-20 complete

## Time Estimate
- **Red Phase**: 4-6 hours (writing all failing tests)
- **Green Phase**: 6-8 hours (implementing minimal code)
- **Refactor Phase**: 3-4 hours (cleanup, optimization, docs)
- **Total**: ~15-18 hours of development work

## Definition of Done
- ✅ All tests pass (`cargo test`)
- ✅ Code formatted (`cargo fmt`)
- ✅ No clippy warnings (`cargo clippy`)
- ✅ Coverage > 80% for new code
- ✅ API endpoint working and documented
- ✅ No regression in existing functionality
- ✅ Changes committed with meaningful messages
