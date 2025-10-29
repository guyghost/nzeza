# Tasks: Portfolio Reconciliation Service

**Change ID:** `add-portfolio-reconciliation`  
**Status:** PLANNING  
**Priority:** HIGH  

## Implementation Checklist

### Phase 1: Design & Preparation
- [ ] Review proposal.md completely
- [ ] Understand existing Position/Portfolio managers
- [ ] Plan test scenarios with test-writer
- [ ] Design audit trail schema

### Phase 2: RED Phase - Failing Tests

#### 2.1 Test: Balance Fetching
- [ ] Test successful balance fetch from single exchange
- [ ] Test balance fetch with multiple currencies
- [ ] Test balance fetch timeout handling
- [ ] Test balance fetch API error handling
- [ ] Test balance fetch with retry logic

#### 2.2 Test: Discrepancy Detection
- [ ] Test detection of missing currency
- [ ] Test detection of balance mismatch above threshold
- [ ] Test detection of balance mismatch below threshold
- [ ] Test precision/rounding tolerance handling
- [ ] Test zero-value balance handling

#### 2.3 Test: Reconciliation Logic
- [ ] Test reconciliation report generation
- [ ] Test multi-exchange reconciliation
- [ ] Test reconciliation with no discrepancies
- [ ] Test reconciliation with multiple discrepancies
- [ ] Test reconciliation status classification

#### 2.4 Test: Error Handling
- [ ] Test handling of network timeout
- [ ] Test handling of API rate limits
- [ ] Test handling of malformed balance data
- [ ] Test graceful degradation on exchange failure
- [ ] Test retry mechanism with exponential backoff

#### 2.5 Test: Actor & Integration
- [ ] Test ReconciliationActor message handling
- [ ] Test repository persistence
- [ ] Test audit trail creation
- [ ] Test concurrent reconciliation attempts
- [ ] Test configuration parameter loading

**Target: 15-20 RED tests, all failing**

### Phase 3: GREEN Phase - Implementation

#### 3.1 Core Structures
- [ ] Create `ReconciliationReport` struct
- [ ] Create `BalanceDiscrepancy` enum
- [ ] Create `ReconciliationStatus` enum
- [ ] Create `Balance` struct

#### 3.2 Trait Definition
- [ ] Define `PortfolioReconciliationService` trait
- [ ] Define trait methods for balance fetch and reconciliation
- [ ] Create `ReconciliationError` custom error type

#### 3.3 Service Implementation
- [ ] Implement reconciliation logic
- [ ] Implement discrepancy detection
- [ ] Implement balance comparison
- [ ] Implement tolerance handling
- [ ] Implement report generation

#### 3.4 Exchange-Specific Implementations
- [ ] Implement Coinbase reconciler
- [ ] Implement dYdX reconciler
- [ ] Implement fallback for unknown exchanges

#### 3.5 Actor & Repository
- [ ] Create `ReconciliationActor`
- [ ] Create `ReconciliationRepository`
- [ ] Create `reconciliation_audit` table schema
- [ ] Implement audit trail persistence

#### 3.6 Configuration
- [ ] Add configuration parameters to `TradingConfig`
- [ ] Add environment variable loading
- [ ] Implement validation of config values
- [ ] Add default values

#### 3.7 Integration
- [ ] Integrate with Position Manager for local balances
- [ ] Integrate with Exchange Clients for real balances
- [ ] Create reconciliation event propagation
- [ ] Add logging at appropriate levels

**Target: All 15-20 tests passing, GREEN phase complete**

### Phase 4: REFACTOR Phase - Optimize & Commit

#### 4.1 Code Quality
- [ ] Run `cargo fmt --check` and fix issues
- [ ] Run `cargo clippy` and address warnings
- [ ] Run `cargo test` and verify all pass
- [ ] Review test coverage metrics

#### 4.2 Performance Optimization
- [ ] Optimize balance fetch concurrent requests
- [ ] Cache exchange client connections
- [ ] Implement connection pooling if needed
- [ ] Profile memory usage

#### 4.3 Documentation
- [ ] Add inline code documentation
- [ ] Create API documentation examples
- [ ] Document configuration options
- [ ] Add troubleshooting section

#### 4.4 Commit & Archive
- [ ] Create conventional commit with proper format
- [ ] Commit message explains business value
- [ ] Include test count and coverage info
- [ ] Push to feature branch

#### 4.5 Merge to Main
- [ ] Sync with main branch
- [ ] Verify no conflicts
- [ ] Run full test suite
- [ ] Merge feature branch to main

### Phase 5: Archive & Completion

#### 5.1 Archive Change
- [ ] Run `openspec archive add-portfolio-reconciliation --yes`
- [ ] Verify files moved to archive/
- [ ] Create session completion document
- [ ] Update project status

#### 5.2 Validation
- [ ] Verify all tests still passing
- [ ] Verify code builds cleanly
- [ ] Verify documentation is accessible
- [ ] Confirm main branch is stable

## Test Statistics Expected

| Category | Count | Status |
|----------|-------|--------|
| Balance Fetching | 5 | PENDING |
| Discrepancy Detection | 5 | PENDING |
| Reconciliation Logic | 5 | PENDING |
| Error Handling | 5 | PENDING |
| Actor & Integration | 3 | PENDING |
| **Total** | **23** | **PENDING** |

## Files to Create/Modify

### New Files
- `src/domain/services/portfolio_reconciliation.rs` - Main service
- `src/domain/services/reconciliation/` - Module directory
  - `mod.rs` - Exports
  - `repository.rs` - Persistence layer
  - `models.rs` - Data structures
- `src/application/actors/reconciliation_actor.rs` - Async actor
- `src/persistence/reconciliation_audit.rs` - Audit schema
- `tests/portfolio_reconciliation_e2e.rs` - E2E tests
- `docs/PORTFOLIO_RECONCILIATION.md` - Documentation

### Modified Files
- `src/config.rs` - Add config parameters
- `src/domain/services/mod.rs` - Add module export
- `src/application/actors/mod.rs` - Add actor export

## Success Criteria Checklist

- [ ] All 15-20 tests passing
- [ ] `cargo fmt --check` passes
- [ ] No new clippy warnings
- [ ] Documentation complete
- [ ] Conventional commit created
- [ ] Feature merged to main
- [ ] Change archived
- [ ] Main branch stable

## Notes

- Tests should be written BEFORE implementation (strict TDD)
- Use agent-based workflow to parallelize development
- Maintain 100% backward compatibility
- Ensure no breaking changes to existing APIs
- Consider performance impact on trading operations

---

**Created:** Oct 30, 2025  
**Modified:** Oct 30, 2025  
**Status:** READY FOR IMPLEMENTATION
