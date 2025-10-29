# Workflow: Portfolio Reconciliation Service Implementation

**Change ID:** `add-portfolio-reconciliation`  
**Workflow Type:** TDD (Test-Driven Development) with Subagent Orchestration  
**Created:** October 31, 2025  

## Purpose

This document defines the OpenSpec workflow for implementing the portfolio reconciliation feature using specialized subagents. It serves as a template for future feature implementations following the TDD methodology.

## Workflow Overview

```
┌─────────────────┐
│  Proposal Review│ ← Business case, architecture, requirements
└────────┬────────┘
         │
         ▼
┌──────────────────────────────────────┐
│ PHASE 1: RED (Failing Tests)         │
│ Assigned to: test-writer subagent    │
│ Duration: 45 minutes                 │
│ Output: 24 failing tests              │
└──────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────┐
│ PHASE 2: GREEN (Implementation)      │
│ Assigned to: implementer subagent    │
│ Duration: 90 minutes                 │
│ Output: 24 passing tests              │
└──────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────┐
│ PHASE 3: REFACTOR (Optimization)     │
│ Assigned to: reviewer subagent       │
│ Duration: 30 minutes                 │
│ Output: Clean, documented, ready code│
└──────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────┐
│ PHASE 4: ARCHIVE (Post-Deployment)   │
│ Assigned to: reviewer subagent       │
│ Duration: 15 minutes                 │
│ Output: Archived change, specs updated│
└──────────────────────────────────────┘
```

## Phase 1: RED - Write Failing Tests

**Assigned to:** `test-writer` subagent  
**Duration:** ~45 minutes  
**Git Status:** Branch `feat/add-portfolio-reconciliation`

### Subagent Tasks

The test-writer agent SHALL:

1. **Read the proposal and design documents** to understand requirements
2. **Create comprehensive test suite** covering all scenarios
3. **Ensure 100% of tests fail initially** (no implementation exists yet)
4. **Write tests in strict TDD style:**
   - Test names describe behavior, not implementation
   - Each test validates one behavior
   - Use real domain types (not mocks where possible)
   - Include both happy path and error cases

### Expected Test Coverage

- **Balance Fetching:** 5 tests
  - Single exchange, multiple currencies
  - Timeout handling, retry logic
  - API error handling

- **Discrepancy Detection:** 5 tests
  - Missing currency detection
  - Balance mismatch above/below threshold
  - Precision/rounding tolerance
  - Zero-value balance handling

- **Reconciliation Logic:** 5 tests
  - Report generation
  - Multi-exchange reconciliation
  - Status classification
  - Multiple concurrent discrepancies

- **Error Handling:** 5 tests
  - Network timeout, API rate limits
  - Malformed data, graceful degradation
  - Exponential backoff, circuit breaker

- **Actor & Integration:** 4 tests
  - Message handling, persistence
  - Audit trail creation
  - Configuration loading

**Total: 24 tests, all failing**

### Success Criteria
- ✅ All tests compile successfully
- ✅ All tests fail with meaningful error messages
- ✅ Test file has clear structure and documentation
- ✅ Tests are ready for implementation

### Deliverables
- `tests/portfolio_reconciliation_e2e.rs` (comprehensive test suite)
- Git commit: `test(reconciliation): add failing tests for portfolio reconciliation service`

---

## Phase 2: GREEN - Implementation

**Assigned to:** `implementer` subagent  
**Duration:** ~90 minutes  
**Git Status:** Same branch as Phase 1

### Subagent Tasks

The implementer agent SHALL:

1. **Read all failing tests** to understand exact requirements
2. **Implement minimal code to pass tests** (no over-engineering)
3. **Follow DDD patterns** established in the codebase
4. **Write production-quality code** that will not need rewrites

### Implementation Checklist

- [ ] **Core Structures**
  - `Balance` struct
  - `BalanceDiscrepancy` enum
  - `ReconciliationReport` struct
  - `ReconciliationStatus` enum
  - `DiscrepancySeverity` enum

- [ ] **Trait & Service**
  - `PortfolioReconciliationService` trait
  - `ConcretePortfolioReconciliationService` implementation
  - `ReconciliationError` custom error type

- [ ] **Exchange Implementations**
  - `CoinbaseReconciler`
  - `DydxReconciler`
  - Base reconciliation logic reusable across exchanges

- [ ] **Actor & Repository**
  - `ReconciliationActor` async actor
  - `ReconciliationRepository` for persistence
  - Database schema creation
  - Audit trail implementation

- [ ] **Configuration**
  - Add parameters to `TradingConfig`
  - Environment variable loading
  - Default value handling

- [ ] **Integration**
  - Connect to Position Manager for local balances
  - Connect to Exchange Clients for real balances
  - Implement event propagation
  - Add appropriate logging

### Success Criteria
- ✅ All 24 tests pass (100% success rate)
- ✅ Code follows existing patterns (DDD, actor model)
- ✅ No unsafe code introduced
- ✅ Handles all error cases gracefully
- ✅ Includes logging at appropriate levels

### Deliverables
- `src/domain/services/portfolio_reconciliation.rs`
- `src/domain/services/reconciliation/` module
- `src/application/actors/reconciliation_actor.rs`
- `src/persistence/reconciliation_audit.rs`
- Git commit: `feat(reconciliation): implement portfolio reconciliation service`

---

## Phase 3: REFACTOR - Optimization & Quality

**Assigned to:** `reviewer` subagent  
**Duration:** ~30 minutes  
**Git Status:** Same branch as Phase 1-2

### Subagent Tasks

The reviewer agent SHALL:

1. **Verify all tests still pass** after any changes
2. **Optimize performance** without changing behavior
3. **Ensure code quality standards** are met
4. **Document design decisions** and edge cases
5. **Prepare for production deployment**

### Quality Checklist

- [ ] **Code Formatting**
  - ✅ `cargo fmt --check` passes
  - ✅ No formatting issues

- [ ] **Linting**
  - ✅ `cargo clippy` produces no new warnings
  - ✅ All code style issues resolved

- [ ] **Testing**
  - ✅ All 24 tests pass
  - ✅ Test coverage > 80%
  - ✅ Edge cases documented

- [ ] **Documentation**
  - ✅ API documentation complete
  - ✅ Configuration options documented
  - ✅ Usage examples provided
  - ✅ Error handling documented

- [ ] **Performance**
  - ✅ Reconciliation cycle < 2s total
  - ✅ Balance fetch < 1s per exchange
  - ✅ Memory overhead < 10MB
  - ✅ No blocking operations in main thread

- [ ] **Security**
  - ✅ No sensitive data logged
  - ✅ API credentials isolated
  - ✅ Audit trail immutable
  - ✅ No SQL injection vulnerabilities

### Success Criteria
- ✅ Production-ready code
- ✅ All quality gates passed
- ✅ Documentation complete
- ✅ Ready for merge to main

### Deliverables
- Optimized implementation code
- Complete API documentation
- Git commit: `refactor(reconciliation): optimize and document portfolio reconciliation`

---

## Phase 4: ARCHIVE - Post-Deployment

**Assigned to:** `reviewer` subagent  
**Duration:** ~15 minutes  
**Git Status:** main branch (after merge)

### Subagent Tasks

The reviewer agent SHALL:

1. **Archive the change** using OpenSpec tooling
2. **Update specifications** with finalized design
3. **Validate archival** completed successfully
4. **Create completion documentation**

### Archive Checklist

- [ ] **Pre-Archive Validation**
  - ✅ All tests passing on main
  - ✅ Code builds cleanly
  - ✅ Documentation accessible
  - ✅ No open issues

- [ ] **Archive Execution**
  - [ ] Run: `openspec archive add-portfolio-reconciliation --yes`
  - [ ] Verify: Change moved to `openspec/changes/archive/YYYY-MM-DD-add-portfolio-reconciliation/`
  - [ ] Verify: `openspec/specs/portfolio-reconciliation/spec.md` created
  - [ ] Verify: Validation passes: `openspec validate --strict`

- [ ] **Documentation Update**
  - [ ] Create session completion summary
  - [ ] Document any deviations from plan
  - [ ] Record metrics and learnings
  - [ ] Archive this workflow document

### Success Criteria
- ✅ Change archived successfully
- ✅ Specs updated in openspec/specs/
- ✅ Validation passes
- ✅ Session documented
- ✅ Main branch stable

### Deliverables
- Archived change in `openspec/changes/archive/`
- Updated specs in `openspec/specs/`
- `SESSION_2025_10_31_OPENSPEC_COMPLETION.md`

---

## Workflow Coordination Rules

### Between Phases
1. **No overlapping work** - Each phase completes before next starts
2. **Clear handoff** - Previous phase output becomes next phase input
3. **Git hygiene** - Single feature branch used throughout all phases
4. **Communication** - Each agent reads output of previous phases

### Within Phases
1. **Atomic commits** - One logical change per commit
2. **Conventional format** - Commit messages follow standard format
3. **No untested code** - All code must have passing tests
4. **Clean workspace** - No uncommitted changes between phases

### Quality Gates
- ✅ All tests passing: 24/24
- ✅ Code formatting: `cargo fmt --check`
- ✅ Linting: `cargo clippy` (no new warnings)
- ✅ Documentation: Complete and accurate
- ✅ Git history: Linear and descriptive

## Timeline

| Phase | Subagent | Duration | Start | End |
|-------|----------|----------|-------|-----|
| RED | test-writer | 45 min | Oct 30 15:00 | Oct 30 15:45 |
| GREEN | implementer | 90 min | Oct 30 15:45 | Oct 30 17:15 |
| REFACTOR | reviewer | 30 min | Oct 30 17:15 | Oct 30 17:45 |
| ARCHIVE | reviewer | 15 min | Oct 30 17:45 | Oct 30 18:00 |
| **Total** | — | **3 hours** | Oct 30 15:00 | Oct 30 18:00 |

## Integration with OpenSpec

This workflow demonstrates the recommended approach for implementing new features using OpenSpec:

1. **Proposal.md** - Define "Why" and "What"
2. **Design.md** - Define "How" (architecture, decisions)
3. **Tasks.md** - Define checklist for implementation
4. **Workflow.md** - Define agent assignments and process (this file)
5. **Specs/Delta** - Capture requirements changes

After completion and archive:
- Change moves to `openspec/changes/archive/`
- Specifications are updated in `openspec/specs/`
- Workflow becomes template for future changes

## Lessons Learned

### What Worked Well
- ✅ Test-driven approach ensured completeness
- ✅ Subagent coordination prevented conflicts
- ✅ Clear handoffs between phases
- ✅ Comprehensive documentation enabled smooth transitions

### Areas for Improvement
- [ ] Earlier validation of test scenarios
- [ ] More detailed acceptance criteria per test
- [ ] Performance profiling during GREEN phase

### Reusable Patterns
- TDD workflow with subagent assignments
- Phase-based orchestration
- Quality gates between phases
- Documentation-first approach

---

**Status:** Implementation Complete ✅  
**Next Step:** Run archive command and validate all checks pass  
**Contact:** Claude (AI Assistant)
