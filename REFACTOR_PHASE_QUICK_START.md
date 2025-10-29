# REFACTOR Phase Quick Start Guide

## Status
✅ **GREEN Phase Complete** - All 24 tests passing  
🎯 **Next: REFACTOR Phase** - Code optimization and enhancement  

## What Is the REFACTOR Phase?

In TDD, the REFACTOR phase is where you:
1. **Keep all tests GREEN** ✅ (no test changes)
2. **Improve code quality** - Cleanup, optimization, better patterns
3. **Enhance documentation** - Add logging, comments, examples
4. **Refine architecture** - Performance tuning, better error handling

**Golden Rule**: Never change test behavior, only improve implementation!

---

## Before You Start REFACTOR

### 1. Understand Current State
```bash
export PATH="/Users/guy/.cargo/bin:$PATH"
cd /Users/guy/Developer/guyghost/nzeza

# Verify all tests still pass
cargo test --test portfolio_reconciliation_e2e
# Expected: 24 passed; 0 failed
```

### 2. Create a Feature Branch (Optional but Recommended)
```bash
git checkout -b refactor/reconciliation-optimization
```

### 3. Identify Refactor Opportunities
Use these checklist items to guide your work:

---

## REFACTOR Checklist

### Phase 1: Code Quality (Easy, High Impact)

- [ ] **Add Comprehensive Logging**
  - Location: `src/domain/services/portfolio_reconciliation.rs`
  - Action: Add `tracing::debug!()` and `tracing::info!()` calls
  - Why: Helps debug production issues

- [ ] **Improve Error Messages**
  - Location: All error returns in reconciliation code
  - Action: Add context to error messages
  - Why: Better debugging

- [ ] **Add Code Comments**
  - Location: Complex logic sections
  - Action: Explain the "why" not the "what"
  - Why: Helps future maintainers

### Phase 2: Performance (Medium Difficulty, High Impact)

- [ ] **Profile Hot Paths**
  - Tools: `cargo flamegraph` or `cargo bench`
  - Focus: `detect_discrepancies()` and `reconcile()`
  - Goal: Identify bottlenecks

- [ ] **Optimize Balance Comparison**
  - Current: Direct HashMap iteration
  - Potential: Pre-sort or cache strategies
  - Metric: Measure time before/after

- [ ] **Batch API Calls**
  - Current: Sequential exchange balance fetches
  - Potential: Parallel with `tokio::join!`
  - Benefit: Faster reconciliation

### Phase 3: Reliability (Medium Difficulty, Medium Impact)

- [ ] **Enhanced Error Recovery**
  - Location: `ReconciliationError` handling
  - Action: Add retry logic to key operations
  - Why: Production resilience

- [ ] **Circuit Breaker Pattern**
  - Location: Exchange client calls
  - Action: Fail fast if exchange is down
  - Why: Prevent cascading failures

- [ ] **Timeout Configuration**
  - Location: `ReconciliationConfig`
  - Action: Make timeouts configurable
  - Why: Adapt to different network conditions

### Phase 4: Architecture (Hard, Low Immediate Impact)

- [ ] **Extract Common Logic**
  - Location: CoinbaseReconciler vs DydxReconciler
  - Action: Create shared reconciliation logic
  - Why: DRY principle, easier maintenance

- [ ] **Improve Actor Messages**
  - Location: `ReconciliationActor` message enum
  - Action: Add typed responses for request-reply pattern
  - Why: Better async semantics

- [ ] **Database Schema Optimization**
  - Location: `ReconciliationAuditRepository`
  - Action: Add indexes, optimize queries
  - Why: Faster historical queries

---

## How to Execute a REFACTOR Task

### Example: Add Logging

**Step 1: Choose a Small, Contained Task**
```
Scope: Add debug logging to detect_discrepancies()
```

**Step 2: Make Changes**
```rust
fn detect_discrepancies(&self, local: &Portfolio, exchange: &ExchangeBalances) 
    -> Vec<BalanceDiscrepancy> {
    tracing::debug!("Detecting discrepancies for {} currencies", local.balances.len());
    
    let discrepancies = // ... existing logic
    
    tracing::debug!(count = discrepancies.len(), "Discrepancies detected");
    discrepancies
}
```

**Step 3: Run Tests (MUST ALL PASS)**
```bash
export PATH="/Users/guy/.cargo/bin:$PATH"
cargo test --test portfolio_reconciliation_e2e
# Expected: 24 passed; 0 failed
```

**Step 4: Check Formatting**
```bash
export PATH="/Users/guy/.cargo/bin:$PATH"
cargo fmt
cargo clippy
```

**Step 5: Commit**
```bash
git add .
git commit -m "refactor(reconciliation): add debug logging to detect_discrepancies"
```

---

## TDD REFACTOR Rules

### ✅ DO

- ✅ Run tests after **every change**
- ✅ Commit frequently (every 5-10 minutes)
- ✅ Use conventional commits: `refactor(module): description`
- ✅ Keep commits small and focused
- ✅ Verify code formatting with `cargo fmt`
- ✅ Use `cargo clippy` for suggestions

### ❌ DON'T

- ❌ Change test behavior or assertions
- ❌ Make multiple unrelated changes in one commit
- ❌ Skip running tests
- ❌ Commit with failing tests
- ❌ Add new features (that's for a new RED phase)
- ❌ Refactor without understanding the code first

---

## Recommended Refactor Sequence

### Session 1 (30 minutes)
1. Add logging to service methods
2. Improve error messages
3. Add code comments to complex sections
4. Commit: `refactor(reconciliation): add logging and documentation`

### Session 2 (1 hour)
1. Profile performance with flamegraph
2. Optimize hot paths
3. Add benchmarks
4. Commit: `refactor(reconciliation): optimize hot paths`

### Session 3 (1 hour)
1. Add circuit breaker pattern
2. Enhance timeout handling
3. Add retry strategy enhancements
4. Commit: `refactor(reconciliation): improve reliability`

### Session 4+ (Architecture)
1. Extract common reconciliation logic
2. Optimize database schema
3. Improve actor message patterns
4. Commit: `refactor(reconciliation): architectural improvements`

---

## Quick Verification Checklist

After each REFACTOR task:

```bash
# 1. All tests pass
cargo test --test portfolio_reconciliation_e2e
# Expected: ✅ 24 passed

# 2. Code compiles without errors
cargo build
# Expected: ✅ Finished (may have warnings)

# 3. Code is formatted
cargo fmt --check
# Expected: ✅ No output = good

# 4. Linter happy
cargo clippy
# Expected: ✅ No errors (may have suggestions)

# 5. Git status clean
git status
# Expected: ✅ nothing to commit, working tree clean
```

---

## Performance Optimization Tips

### Before Optimizing
- Always measure with `cargo bench` or `cargo flamegraph`
- Don't optimize without data
- Premature optimization = evil 😈

### Common Performance Issues to Look For
1. **Unnecessary allocations** - Use references where possible
2. **Repeated calculations** - Cache results
3. **Sequential operations** - Parallelize with `tokio::join!`
4. **Lock contention** - Use `Arc` + `RwLock` carefully
5. **HashMap inefficiency** - Consider ordering or indexing

### Profiling Commands
```bash
# Flamegraph (requires graphviz)
cargo install flamegraph
cargo flamegraph --bin nzeza -- --filter reconciliation

# Benchmarks
cargo bench --test portfolio_reconciliation_e2e

# Memory usage
cargo valgrind --test portfolio_reconciliation_e2e
```

---

## Logging Best Practices

### Log Levels (Choose the Right One)
- **ERROR** 🔴 - Critical failures requiring attention
- **WARN** 🟡 - Concerning but not critical
- **INFO** 🔵 - Important normal events
- **DEBUG** 🟢 - Detailed diagnostic info
- **TRACE** 🤍 - Very detailed internal state

### Examples
```rust
// Error: Reconciliation failed
tracing::error!("Reconciliation failed: {:?}", error);

// Warn: Discrepancy threshold approaching
tracing::warn!("Large discrepancy detected: {} {}", amount, currency);

// Info: Normal operation milestone
tracing::info!("Reconciliation completed for {}", exchange);

// Debug: Diagnostic details
tracing::debug!("Processing {} balances", count);

// Trace: Internal state
tracing::trace!("Comparing {} vs {}", local, exchange);
```

---

## Common REFACTOR Mistakes

### ❌ Mistake 1: Changing Test Behavior
```rust
// WRONG - Don't do this!
#[test]
fn test_something() {
    // Changed assertion
    assert_eq!(result, new_value); // Was: old_value
}
```
**Fix**: Only refactor implementation, never tests

### ❌ Mistake 2: Forgetting to Run Tests
```bash
# You might do this and think it's fine
git commit -m "refactor: improved code"
# But did you test? 🤔
```
**Fix**: Always run tests before committing

### ❌ Mistake 3: Big Refactors Without Testing
```rust
// WRONG - Changing too much
async fn big_refactor() {
    // Rewrote entire service
    // Changed error handling
    // Modified data structures
}
```
**Fix**: Small, incremental changes with tests after each

### ❌ Mistake 4: Not Committing Frequently
```bash
# WRONG - Too much change in one commit
git commit -m "refactor: everything"
```
**Fix**: One feature per commit, 5-10 minute intervals

---

## When to Stop Refactoring

You've done enough REFACTOR when:

✅ All 24 tests still pass  
✅ Code is readable and maintainable  
✅ Performance meets requirements  
✅ Logging is comprehensive  
✅ Error handling is robust  
✅ Architecture is clean  

**Don't over-engineer!** Remember: simple is often better.

---

## Next Phase After REFACTOR

After REFACTOR is complete:

1. **Code Review**: Get peer feedback on improvements
2. **Merge to Main**: Create PR, get approval, merge
3. **Deploy**: Release to production
4. **Monitor**: Watch metrics in production
5. **Next Feature**: Start new RED phase for new capability

---

## Resources

- **TDD Methodology**: Read AGENTS.md section on TDD workflow
- **Rust Best Practices**: https://doc.rust-lang.org/book/
- **Tokio Async Guide**: https://tokio.rs/tokio/tutorial
- **Project Architecture**: See `openspec/project.md`

---

## Questions?

If you get stuck during REFACTOR:

1. **Check the tests** - They document expected behavior
2. **Read code comments** - Already added during implementation
3. **Review git history** - See how code evolved
4. **Run with logging** - Use `RUST_LOG=nzeza=debug cargo run`
5. **Ask for help** - Review AGENTS.md for escalation

---

**Remember**: In REFACTOR, we keep tests green while making code better! 🎯

Session Duration: ~2-4 hours of focused refactoring  
Target: 5+ incremental improvements committed  

Happy refactoring! 🚀
