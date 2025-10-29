# Session 2025-10-30: Strategic Development Plan

**Date:** October 30, 2025  
**Status:** PLANNING  
**Goal:** Resume development with OpenCode TDD workflow, identify next priority feature  

## Context Summary

### Previous Session Accomplishments (2025-10-29)
- ✅ Completed Symbol Screening refactor (Tasks 21-29)
- ✅ 21 new tests with 100% pass rate
- ✅ Comprehensive documentation and configuration system
- ✅ Documented OpenCode TDD workflow for future features

### Current Project Status
- **Domain Layer:** ✅ 100% (129/129 tests passing)
- **Symbol Screening:** ✅ COMPLETE (21 tests, fully functional)
- **WebSocket Integration:** ❌ NOT IMPLEMENTED (20 RED tests, critical blocker)
- **Production Readiness:** 45/100 - NOT READY FOR PRODUCTION

### Critical Blockers for Production (from PRODUCTION_STATUS.md)
1. **Phase 5.1 WebSocket:** Not implemented, 20 RED tests exist
2. **Integration Tests:** No signal generation or end-to-end tests
3. **dYdX Integration:** Uses wrong signing method (Ethereum vs Cosmos)

## Strategic Analysis

### Priority Matrix

**IMPACT × FEASIBILITY:**

1. **HIGH IMPACT + HIGH FEASIBILITY:** ✅ RECOMMENDED
   - Complete WebSocket Circuit Breaker (extends existing Phase 5.1 work)
   - Add portfolio reconciliation service
   - Implement signal aggregation service

2. **HIGH IMPACT + MEDIUM FEASIBILITY:** ⏳ CONSIDER
   - Fix dYdX integration (Cosmos signing)
   - End-to-end integration tests
   - Production monitoring/alerting

3. **MEDIUM IMPACT + HIGH FEASIBILITY:** 📊 QUICK WINS
   - Enhance error recovery mechanisms
   - Add performance metrics tracking
   - Improve rate limit handling

4. **LOW IMPACT + MEDIUM FEASIBILITY:** ❌ DEFER
   - Additional indicators
   - UI/Dashboard components
   - Deployment infrastructure

## Recommended Next Feature: Portfolio Reconciliation Service

### Why This Feature?

**Business Value:**
- Essential for detecting trading anomalies and exchange desynchronization
- Critical safety feature (prevents over-leveraging)
- Reduces operational risk during live trading

**Technical Fit:**
- Builds on existing Position Manager and Portfolio Manager services
- Uses established repository patterns (SQLite)
- Fits TDD workflow well (clear test scenarios)
- 2-3 day implementation (reasonable scope)

**Architecture Alignment:**
- Extends DDD domain layer (domain/services/)
- Integrates with existing actors
- Follows established patterns

### Feature Scope: Portfolio Reconciliation Service

**What It Does:**
1. Periodically fetches balance from exchange APIs
2. Compares with local portfolio state
3. Detects discrepancies and logs them
4. Triggers reconciliation logic when needed
5. Maintains audit trail

**Key Components:**
- `PortfolioReconciliationService` trait with implementations per exchange
- `ReconciliationActor` for async orchestration
- `ReconciliationRepository` for audit trail persistence
- Comprehensive test coverage for edge cases

**Test Coverage Plan:**
- Balance mismatch scenarios (5-7 tests)
- Multi-exchange reconciliation (4-5 tests)
- Error handling and recovery (5-6 tests)
- Performance and concurrency (3-4 tests)
- **Total: ~15-20 tests**

**Implementation Phases (TDD):**

1. **Phase 1: RED** - Write all tests (failing)
   - Test trait interface definition
   - Test balance fetch scenarios
   - Test reconciliation logic
   - Test error cases

2. **Phase 2: GREEN** - Implement to pass tests
   - Create service implementations
   - Add repository layer
   - Create actor
   - Add config parameters

3. **Phase 3: REFACTOR** - Optimize and commit
   - Performance optimization
   - Code cleanup
   - Documentation
   - Conventional commit

**Configuration Needs:**
```rust
pub struct TradingConfig {
    pub reconciliation_enabled: bool,
    pub reconciliation_interval_seconds: u64,
    pub reconciliation_threshold_usd: f64,
    pub reconciliation_retry_count: u32,
}
```

**Environment Variables:**
```
RECONCILIATION_ENABLED=true
RECONCILIATION_INTERVAL_SECONDS=300
RECONCILIATION_THRESHOLD_USD=10.0
RECONCILIATION_RETRY_COUNT=3
```

## Alternative Features (If Reconciliation Chosen Not)

### 2. Signal Aggregation Service
**Scope:** Combine multiple trading signals into actionable recommendations  
**Time:** 2-3 days  
**Priority:** HIGH (needed for Phase 5.2)  
**Tests Estimated:** 12-15  

### 3. Error Recovery & Backoff Strategy
**Scope:** Implement exponential backoff and circuit breaker for all exchange APIs  
**Time:** 1-2 days  
**Priority:** MEDIUM (improves reliability)  
**Tests Estimated:** 10-12  

### 4. Performance Metrics Dashboard
**Scope:** Add Prometheus metrics collection and Grafana dashboard  
**Time:** 2 days  
**Priority:** MEDIUM (helps debugging)  
**Tests Estimated:** 8-10  

## OpenCode Workflow Process

### Step 1: Create Proposal
- [ ] Create `openspec/changes/<feature-id>/` directory
- [ ] Write `proposal.md` with business value and requirements
- [ ] Write `tasks.md` with detailed checklist
- [ ] Write `design.md` with architecture decisions
- [ ] Run `openspec validate <feature-id> --strict`

### Step 2: RED Phase (Test-Writer Agent)
```bash
Task(
  description="Write failing tests for feature",
  prompt="Create comprehensive test suite for [feature] that fails because implementation doesn't exist yet",
  subagent_type="test-writer"
)
```

### Step 3: GREEN Phase (Implementer Agent)
```bash
Task(
  description="Implement feature to pass tests",
  prompt="Write minimal implementation code to make all [feature] tests pass",
  subagent_type="implementer"
)
```

### Step 4: REFACTOR Phase (Reviewer Agent)
```bash
Task(
  description="Review and commit feature",
  prompt="Optimize code, run clippy, format, then create conventional commit for [feature]",
  subagent_type="reviewer"
)
```

### Step 5: Archive & Deploy
```bash
openspec archive <feature-id> --yes
```

## Success Criteria

✅ **All tests passing** (100% success rate)  
✅ **Code properly formatted** (`cargo fmt` passes)  
✅ **No clippy warnings** (relevant to new code)  
✅ **Documentation complete** (API docs + examples)  
✅ **Conventional commit created** (proper format)  
✅ **Feature branch merged to main**  
✅ **Change archived in openspec/changes/archive/**  

## Timeline Estimate

| Phase | Task | Duration |
|-------|------|----------|
| Planning | Create proposal + design | 30 mins |
| RED | Write tests (agent) | 45 mins |
| GREEN | Implement code (agent) | 90 mins |
| REFACTOR | Optimize + commit (agent) | 30 mins |
| Archive | Document and archive | 15 mins |
| **TOTAL** | | **3-4 hours** |

## Risk Mitigation

**Risk:** Feature takes longer than expected  
**Mitigation:** Use agent-based workflow to parallelize work; agents complete tasks autonomously

**Risk:** Tests don't cover all cases  
**Mitigation:** Run manual integration testing after implementation; review coverage metrics

**Risk:** Code quality issues  
**Mitigation:** Reviewer agent automatically runs clippy, fmt, and creates conventional commit

## Next Actions

1. ✅ Create this strategic plan (DONE)
2. ⏭️  **Decide on feature to implement** (waiting for input)
3. Create OpenSpec proposal for selected feature
4. Launch TEST-WRITER agent for RED phase
5. Launch IMPLEMENTER agent for GREEN phase
6. Launch REVIEWER agent for REFACTOR phase
7. Merge and archive

---

**Decision Point:** Which feature should we implement?
- Option A: **Portfolio Reconciliation Service** (recommended - safest, high value)
- Option B: **Signal Aggregation Service** (Phase 5.2 prerequisite)
- Option C: **Error Recovery & Backoff** (improves reliability)
- Option D: **Performance Metrics** (helps debugging)

**Recommendation:** Start with **Portfolio Reconciliation Service** because:
1. Clear scope and well-defined requirements
2. Fits with existing architecture perfectly
3. Highest business value for trading safety
4. Reasonable 2-3 day timeline
5. Good showcase of OpenCode TDD workflow

---

**Session Status:** PLANNING ⏳  
**Ready to begin implementation:** YES ✅  
**Awaiting:** Feature selection and approval
