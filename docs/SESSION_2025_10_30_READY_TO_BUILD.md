# 🚀 READY TO IMPLEMENT: Portfolio Reconciliation Service

## Quick Start Guide

You've successfully completed the **Planning Phase** for the **Portfolio Reconciliation Service**.

### Current Status
✅ Project analyzed  
✅ Feature selected and justified  
✅ Complete OpenSpec proposal created  
✅ Technical design documented  
✅ Implementation roadmap ready  
✅ 3 commits pushed  

### What's Next?

Choose your next action:

---

## Option 1: Launch Automated Implementation (RECOMMENDED) ✅

**Time Required:** 3-4 hours (fully automated)

### Step 1: Write Failing Tests (RED Phase)
Use the test-writer agent to create comprehensive test suite:

```bash
task(
  description="Write failing tests for portfolio reconciliation",
  prompt="""Create comprehensive test suite for portfolio reconciliation service using the design document in openspec/changes/add-portfolio-reconciliation/design.md.

Tests should include:
1. Balance Fetching Tests (5 tests):
   - Test fetching from single exchange
   - Test fetching with multiple currencies  
   - Test timeout handling
   - Test API error handling
   - Test retry mechanism

2. Discrepancy Detection Tests (5 tests):
   - Test detection of missing currencies
   - Test balance mismatch above threshold
   - Test balance mismatch below threshold
   - Test precision/rounding tolerance
   - Test zero-value balances

3. Reconciliation Logic Tests (5 tests):
   - Test report generation
   - Test multi-exchange reconciliation
   - Test no-discrepancy scenarios
   - Test multiple discrepancies
   - Test status classification

4. Error Handling Tests (5 tests):
   - Test network timeouts
   - Test rate limit handling
   - Test malformed data
   - Test graceful degradation
   - Test exponential backoff

5. Actor & Integration Tests (3 tests):
   - Test actor message handling
   - Test repository persistence
   - Test concurrent reconciliations

Target: 23 tests, all failing because implementation doesn't exist yet.
Place tests in: tests/portfolio_reconciliation_e2e.rs
Follow TDD principles: Write tests BEFORE implementation.""",
  subagent_type="test-writer"
)
```

### Step 2: Implement Feature (GREEN Phase)
Use the implementer agent to write code that makes tests pass:

```bash
task(
  description="Implement portfolio reconciliation service",
  prompt="""Implement portfolio reconciliation service to make all 23 failing tests pass.

Reference the design document: openspec/changes/add-portfolio-reconciliation/design.md

Required components:
1. Create src/domain/services/portfolio_reconciliation.rs
   - Trait: PortfolioReconciliationService
   - Struct: ReconciliationReport
   - Enum: BalanceDiscrepancy
   - Enum: ReconciliationStatus

2. Create src/domain/services/reconciliation/ module
   - mod.rs - Exports
   - models.rs - Data structures
   
3. Create exchange-specific reconcilers
   - CoinbaseReconciler
   - DydxReconciler
   
4. Create src/application/actors/reconciliation_actor.rs
   - ReconciliationMessage enum
   - Actor implementation
   
5. Create src/persistence/reconciliation_audit.rs
   - ReconciliationRepository
   - Database schema
   - Audit trail persistence

6. Update src/config.rs
   - Add 5 new configuration parameters
   - Add environment variable loading

Ensure all 23 tests pass with minimal, focused implementation.
Use existing patterns from portfolio_manager.rs and position_manager.rs.""",
  subagent_type="implementer"
)
```

### Step 3: Optimize & Commit (REFACTOR Phase)
Use the reviewer agent to polish and commit:

```bash
task(
  description="Review and commit portfolio reconciliation feature",
  prompt="""Finalize portfolio reconciliation service implementation.

Steps:
1. Run cargo fmt --check and fix any formatting issues
2. Run cargo clippy and address any relevant warnings
3. Run cargo test and verify all 23 tests pass
4. Run cargo build and ensure clean compilation
5. Review code for quality and documentation
6. Create inline code documentation with examples
7. Generate conventional commit with format:

   feat(reconciliation): implement portfolio reconciliation service
   
   Implement comprehensive portfolio reconciliation service for:
   - Detecting balance discrepancies with exchanges
   - Automatic recovery and state synchronization
   - Complete audit trail persistence
   - Multi-exchange support
   
   - Add PortfolioReconciliationService trait
   - Add exchange-specific reconcilers (Coinbase, dYdX)
   - Add ReconciliationActor for async orchestration
   - Add ReconciliationRepository for audit persistence
   - Add configuration parameters (5 new env vars)
   - Add comprehensive test suite (23 tests)
   
   Tests: 23 passing, 100% success rate
   Files: 7 new, 2 modified
   Lines: ~1,500 insertions

8. Ensure working tree is clean after commit""",
  subagent_type="reviewer"
)
```

### Step 4: Merge & Archive
After all 3 agents complete:

```bash
# Create feature branch
git checkout -b feat/portfolio-reconciliation

# After agents complete, all commits should be on main
# Verify tests pass
cargo test

# Archive the OpenSpec change
openspec archive add-portfolio-reconciliation --yes

# Push to remote
git push origin main
```

**Expected Result:** ✅ Feature complete, tested, committed, and deployed

---

## Option 2: Review Before Proceeding

If you want to review the proposal first:

1. **Read the full proposal:**
   ```bash
   cat openspec/changes/add-portfolio-reconciliation/proposal.md
   ```

2. **Review the design document:**
   ```bash
   cat openspec/changes/add-portfolio-reconciliation/design.md
   ```

3. **Check implementation tasks:**
   ```bash
   cat openspec/changes/add-portfolio-reconciliation/tasks.md
   ```

4. **Then decide** if you want to proceed or choose alternative feature

---

## Option 3: Choose Different Feature

If you prefer a different feature instead:

**Alternative Features Available:**

1. **Signal Aggregation Service** (HIGH PRIORITY)
   - Combines multiple trading signals
   - Phase 5.2 prerequisite
   - 2-3 days, 15-20 tests
   - Status: Proposal ready on demand

2. **Error Recovery & Backoff Strategy** (MEDIUM PRIORITY)
   - Exponential backoff for API calls
   - Circuit breaker patterns
   - 1-2 days, 10-12 tests
   - Status: Proposal ready on demand

3. **Performance Metrics Dashboard** (MEDIUM PRIORITY)
   - Prometheus metrics collection
   - Grafana dashboard
   - 2 days, 8-10 tests
   - Status: Proposal ready on demand

---

## Portfolio Reconciliation Service Summary

### What It Solves
```
❌ Problem: System doesn't know if exchange balances match local state
❌ Risk: Over-leveraging or missing funds undetected
❌ Result: Operational uncertainty

✅ Solution: Automatic reconciliation with audit trail
✅ Benefit: Real-time balance confidence + compliance
✅ Result: Safe, auditable trading operations
```

### Components to Build
```
PortfolioReconciliationService (trait)
├── CoinbaseReconciler (exchange-specific)
├── DydxReconciler (exchange-specific)
├── ReconciliationActor (orchestration)
├── ReconciliationRepository (persistence)
└── Configuration (5 new env vars)
```

### Testing Coverage
```
23 tests planned:
- 5 balance fetching scenarios
- 5 discrepancy detection scenarios
- 5 reconciliation logic scenarios
- 5 error handling scenarios
- 3 actor integration scenarios
```

### Expected Outcomes
```
✅ 23 tests passing (100% success rate)
✅ ~1,500 lines of new code
✅ 7 new files created
✅ 2 files modified
✅ Full documentation
✅ Conventional commit
✅ Merged to main
✅ Archived in openspec
```

---

## Files to Reference

### Documentation
- `openspec/changes/add-portfolio-reconciliation/proposal.md` - Business case
- `openspec/changes/add-portfolio-reconciliation/design.md` - Technical design
- `openspec/changes/add-portfolio-reconciliation/tasks.md` - Implementation checklist
- `docs/SESSION_2025_10_30_STRATEGIC_PLAN.md` - Strategic analysis
- `docs/SESSION_2025_10_30_COMPLETION.md` - Session summary

### Previous Examples
- `src/domain/services/portfolio_manager.rs` - Similar service
- `src/domain/services/position_manager.rs` - Similar service
- `tests/portfolio_consistency_tests.rs` - Similar test patterns

---

## Success Criteria

After implementation completes:

- [ ] All 23 tests passing
- [ ] `cargo fmt --check` passes
- [ ] No new clippy warnings
- [ ] Documentation complete
- [ ] Conventional commit created
- [ ] Feature merged to main
- [ ] Change archived in openspec
- [ ] Main branch stable
- [ ] All files committed

---

## Estimated Timeline

| Task | Duration | Tool |
|------|----------|------|
| RED Phase | 45 mins | test-writer agent |
| GREEN Phase | 90 mins | implementer agent |
| REFACTOR Phase | 30 mins | reviewer agent |
| Archive & Deploy | 15 mins | Manual |
| **TOTAL** | **3-4 hours** | **Fully Automated** |

---

## Questions?

Review these documents for clarification:

1. **Why this feature?** → `docs/SESSION_2025_10_30_STRATEGIC_PLAN.md`
2. **How is it designed?** → `openspec/changes/add-portfolio-reconciliation/design.md`
3. **What needs testing?** → `openspec/changes/add-portfolio-reconciliation/tasks.md`
4. **How does project work?** → `openspec/project.md`
5. **What's the workflow?** → `AGENTS.md`

---

## 🎯 DECISION TIME

**Choose one:**

✅ **Option A:** Launch automated implementation NOW (3-4 hours)  
📚 **Option B:** Review more first, then decide  
🔄 **Option C:** Choose different feature  

**Recommendation:** **Option A** - The proposal is complete, design is solid, implementation is straightforward. Launch the agents and have a complete feature in 3-4 hours.

---

**Next Step:** Reply with your choice and I'll proceed accordingly.

**Status:** ✅ READY TO BUILD  
**Awaiting:** Your confirmation to launch agents  

🚀
