# Quick Start Guide - Phase 4 Implementation

**For:** Next session developer  
**Time to read:** 5 minutes  
**Time to implement:** 2-10 hours (depending on phase chosen)

---

## Current Status (End of Phase 3)

```
✅ 216 tests written (specifications)
✅ 135 tests passing (core features working)
⏳ 81 tests failing (features to implement)
✅ All code compiles (0 errors)
✅ Fast execution (0.16 seconds)
```

---

## The 30-Second Overview

**What we have:** Test-first development process with clear specifications  
**What we need:** Implement the remaining features to pass 81 failing tests  
**Where to start:** Pick ONE of the 4 phases below  
**How to know you're done:** Your chosen domain tests all pass ✅

---

## Quick Commands

Run all tests and see status:
```bash
cd /Users/guy/Developer/guyghost/nzeza
~/.cargo/bin/cargo test --lib 2>&1 | tail -50
```

Run tests for specific domain:
```bash
# Error handling tests (should all pass)
~/.cargo/bin/cargo test --lib errors_tests

# Position manager tests (8 failing)
~/.cargo/bin/cargo test --lib position_validation_tests

# Order execution tests (19 failing)
~/.cargo/bin/cargo test --lib order_execution_tests

# Portfolio tests (10 failing)
~/.cargo/bin/cargo test --lib portfolio_consistency_tests

# Concurrency tests (17 failing)
~/.cargo/bin/cargo test --lib concurrency_tests
```

Check if code compiles:
```bash
~/.cargo/bin/cargo check
```

Format code:
```bash
~/.cargo/bin/cargo fmt --all
```

---

## Choose Your Phase

### 🔴 PHASE 4.1: Position Manager [2-3 hours, HIGHEST impact]

**What to do:**
1. Open `src/domain/position_validation_tests.rs`
2. Pick a failing test (starts with `test_`)
3. Read the test to understand what it expects
4. Implement the feature in `src/domain/services/position_manager.rs`
5. Run `cargo test --lib position_validation` - should pass one more ✅
6. Repeat for 8 tests total

**Failing tests (8 total):**
- `test_open_position_should_validate_symbol_limits`
- `test_open_position_should_validate_total_portfolio_limits`
- `test_open_position_should_validate_available_balance`
- `test_close_position_should_calculate_accurate_pnl_long`
- `test_close_position_should_calculate_accurate_pnl_short`
- `test_stop_loss_trigger_should_auto_close_long_position`
- `test_take_profit_trigger_should_auto_close_long_position`
- (1 more edge case)

**Why this first:** Other phases depend on it!

---

### 🟡 PHASE 4.2: Order Executor [4-5 hours, HIGH impact]

**What to do:**
1. Open `src/domain/order_execution_tests.rs`
2. Find the failing tests
3. Implement in `src/domain/services/order_executor.rs`
4. Run tests - watch them pass ✅

**Key features needed:**
- Signal validation (BUY/SELL/HOLD)
- Confidence threshold checking
- Rate limiting (hourly, daily)
- Trade history recording
- Error handling

**Why skip this first:** Depends on Position Manager working well

---

### 🟡 PHASE 4.3: Portfolio Manager [3-4 hours, MEDIUM impact]

**What to do:**
1. Open `src/domain/portfolio_consistency_tests.rs`
2. Implement durability features
3. Add snapshot/recovery capability

**Key features needed:**
- Snapshot persistence
- Transaction recovery
- ACID property enforcement
- Complex invariant validation

**Why not first:** Also depends on Position Manager

---

### 🟡 PHASE 4.4: Lock Safety [3-4 hours, INTEGRATION task]

**What to do:**
1. Open `src/domain/concurrency_tests.rs`
2. Integrate lock_validator throughout codebase
3. Test under concurrent load

**Key features needed:**
- Deadlock detection
- Lock ordering enforcement
- Fairness guarantees
- Starvation prevention

**Why last:** Depends on all other phases

---

## How to Implement (Step by Step)

### Step 1: Read the Test
```rust
// Example from position_validation_tests.rs
#[test]
fn test_open_position_should_validate_symbol_limits() {
    // Given: manager with max 1 position per symbol
    // When: open 2 positions in same symbol
    // Then: second should fail
    
    panic!("Test not yet implemented");
}
```

### Step 2: Understand What It Needs
The test tells you:
- **Given:** Initial state/setup
- **When:** Action to take
- **Then:** Expected result

### Step 3: Find the Implementation
Open `src/domain/services/position_manager.rs` and find the relevant method:
```rust
pub fn open_position(...) -> Result<String, DetailedMpcError> {
    // Your code here!
}
```

### Step 4: Write the Code
Implement the feature. Example:
```rust
// Check symbol position limit
if self.get_symbol_position_count(symbol) >= self.limits.max_per_symbol {
    return Err(DetailedMpcError::PositionLimitExceeded {
        symbol: symbol.to_string(),
        limit: self.limits.max_per_symbol,
        current: self.get_symbol_position_count(symbol),
        limit_type: PositionLimitType::PerSymbol,
    });
}
```

### Step 5: Test Your Code
```bash
~/.cargo/bin/cargo test --lib test_open_position_should_validate_symbol_limits
```

### Step 6: See It Pass ✅
When test passes, move to next one!

### Step 7: Commit
```bash
git add -A
git commit -m "feat(position_manager): implement symbol position limit validation"
```

---

## Common Patterns You'll See

### Error Handling
```rust
// Use the error types from src/domain/errors.rs
DetailedMpcError::InsufficientBalance { ... }
DetailedMpcError::PositionLimitExceeded { ... }
// etc.
```

### Validation Pattern
```rust
// Check condition
if condition_violated {
    return Err(error);
}
// Do the thing
Ok(result)
```

### Return Types
- `Result<String, DetailedMpcError>` - for operations that can fail
- `Result<(), String>` - for operations with optional error detail
- `bool` - for yes/no operations

---

## Troubleshooting

### Test fails with "not implemented"
You need to implement the feature. Read the test again to understand what.

### Compilation error: "no field `x` on type"
The struct doesn't have that field. Check the actual struct definition.

### Test passes locally but CI fails
Make sure you ran `cargo fmt` before committing:
```bash
~/.cargo/bin/cargo fmt --all
```

### Borrow checker error
Rust is protecting you from data races. Rethink the borrowing pattern.

---

## Success Criteria

✅ **All tests in your domain pass**  
✅ **Code compiles with 0 errors**  
✅ **No new compiler warnings**  
✅ **Tests execute in <0.5 seconds total**  
✅ **Git history clean with meaningful commits**  

---

## Resources

**Read these first (in order):**
1. `docs/TEST_SUITE_ANALYSIS.md` - Strategic overview
2. `docs/SESSION_2025_10_28_SUMMARY.md` - What was done before
3. `src/domain/errors.rs` - Error types to use
4. The failing test file (pick your domain)

**Reference documentation:**
- `docs/AGENTS.md` - TDD approach
- `docs/TDD_WORKFLOW.md` - Red/Green/Refactor cycle
- `docs/ARCHITECTURE_REFACTORING.md` - System design

---

## Your Checklist

Before you start:
- [ ] I read TEST_SUITE_ANALYSIS.md
- [ ] I read SESSION_2025_10_28_SUMMARY.md
- [ ] I picked one phase (4.1, 4.2, 4.3, or 4.4)
- [ ] I can run `cargo test --lib` successfully

After you finish:
- [ ] All tests in my domain pass ✅
- [ ] Code compiles with 0 errors
- [ ] Tests complete in <0.5 seconds
- [ ] I committed my work
- [ ] I updated the session summary with my progress

---

## When You Get Stuck

1. **Read the test** - It tells you exactly what's expected
2. **Check the error message** - Compiler errors are helpful
3. **Look at passing tests** - See how similar features work
4. **Read the struct definition** - Understand what fields you have
5. **Check the error types** - Use the right error variant

---

## Expected Time Breakdown

| Phase | Tests | Time | Priority |
|-------|-------|------|----------|
| 4.1 Position Manager | 8 | 2-3h | 🔴 HIGH |
| 4.2 Order Executor | 19 | 4-5h | 🔴 HIGH |
| 4.3 Portfolio Manager | 10 | 3-4h | 🟡 MEDIUM |
| 4.4 Lock Safety | 17 | 3-4h | 🟡 MEDIUM |

**Recommendation:** Do Phase 4.1 first (highest impact, lowest effort)

---

## Final Thoughts

You have everything you need:
- ✅ 216 tests that specify exactly what to build
- ✅ Previous implementations to learn from
- ✅ Clear error messages to guide you
- ✅ Fast feedback loop (tests in 0.16s)

**Go implement! Build something great! 🚀**

---

**Questions?** Check the analysis documents or the test files themselves - they're self-documenting!
