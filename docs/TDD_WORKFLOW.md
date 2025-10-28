# TDD Workflow Guide - NZEZA Trading System

## Overview

This project follows **Test-Driven Development (TDD)** with the Red → Green → Refactor cycle. 

**Current Status:** 🔴 RED Phase Complete - Ready for 🟢 GREEN Implementation

---

## RED Phase ✅ DONE

### What Was Done
- ✅ Created 150+ comprehensive failing tests
- ✅ Defined expected behavior and error types
- ✅ Documented all requirements
- ✅ Specified ACID properties and thread safety

### Test Modules

**Location:** `/src/domain/`

```
errors_tests.rs                  (27 tests)  → Error handling specs
position_validation_tests.rs     (21 tests)  → Position operations specs
order_execution_tests.rs         (24 tests)  → Order workflow specs
portfolio_consistency_tests.rs   (35 tests)  → ACID properties specs
concurrency_tests.rs             (28 tests)  → Thread safety specs
```

### How Tests Are Structured

Each test is intentionally **failing** with `panic!("not implemented")`:

```rust
#[test]
fn test_order_validation_error_includes_symbol() {
    let error = DetailedMpcError::OrderValidationFailed {
        symbol: "BTC-USD".to_string(),
        reason: "Symbol not in whitelist".to_string(),
    };
    
    let error_msg = error.to_string();
    assert!(error_msg.contains("BTC-USD"), "Error should include symbol");
}
```

---

## GREEN Phase ⏳ NEXT

### What To Do

For each failing test:

1. **Understand the test requirement**
   - Read the test name
   - Read the comments
   - Understand the expected behavior

2. **Implement minimal code to make it pass**
   - Add the required types/functions
   - Implement the exact behavior tested
   - Don't add extra features

3. **Verify the test passes**
   ```bash
   cargo test test_order_validation_error_includes_symbol
   ```

### Implementation Order (Recommended)

**Task 1: Error Handling** (errors_tests.rs)
- Define `DetailedMpcError` enum with proper variants
- Implement `severity()` method
- Implement `to_string()` with proper context

**Task 2: Position Validation** (position_validation_tests.rs)
- Implement `PositionManager` struct
- Implement `open_position()` with validation
- Implement `close_position()` with PnL calculation
- Implement trigger checking

**Task 3: Order Execution** (order_execution_tests.rs)
- Implement order execution workflow
- Add validation pipeline
- Implement signal processing

**Task 4: Portfolio Consistency** (portfolio_consistency_tests.rs)
- Add portfolio tracking
- Implement ACID transactions
- Add invariant validation

**Task 5: Concurrency Safety** (concurrency_tests.rs)
- Validate lock ordering
- Add deadlock prevention
- Verify concurrent access safety

---

## Running Tests

### Run All Tests
```bash
cd /Users/guy/Developer/guyghost/nzeza
cargo test
```

### Run Specific Test Module
```bash
# Error handling tests
cargo test domain::errors_tests

# Position validation tests
cargo test domain::position_validation_tests

# Order execution tests
cargo test domain::order_execution_tests

# Portfolio consistency tests
cargo test domain::portfolio_consistency_tests

# Concurrency tests
cargo test domain::concurrency_tests
```

### Run Single Test
```bash
cargo test test_order_validation_error_includes_symbol
```

### Run with Output
```bash
cargo test -- --nocapture
```

### Run with Thread Count (for concurrency tests)
```bash
cargo test domain::concurrency_tests -- --test-threads=1
```

---

## TDD Discipline

### The Red → Green → Refactor Cycle

```
┌─────────────────┐
│  1. RED: Write  │
│     failing     │ ← You are here
│     tests       │
└────────┬────────┘
         ↓
┌─────────────────┐
│  2. GREEN:      │
│     Implement   │
│     to pass     │
└────────┬────────┘
         ↓
┌─────────────────┐
│  3. REFACTOR:   │
│     Improve     │
│     code       │
└──────────────────┘
```

### Rules to Follow

**While in GREEN phase:**
- ✅ DO make tests pass (minimum code needed)
- ✅ DO run tests frequently (`cargo test`)
- ✅ DO commit working code regularly
- ❌ DON'T skip failing tests
- ❌ DON'T refactor before tests pass
- ❌ DON'T add features not in tests

**Commit Strategy:**
```bash
# After each test passes
git add src/
git commit -m "test(module): make test_name pass

This implements the expected behavior defined in the test."
```

---

## Example: Making a Test Pass

### Step 1: See the RED test fail
```rust
#[test]
fn test_insufficient_balance_error_includes_amounts() {
    let error = DetailedMpcError::InsufficientBalance {
        required: 1000.50,
        available: 500.25,
        currency: "USD".to_string(),
    };
    
    let error_msg = error.to_string();
    assert!(error_msg.contains("1000.50"), "Should include required");
    assert!(error_msg.contains("500.25"), "Should include available");
}
```

### Step 2: Define the type
```rust
// In src/domain/errors.rs

#[derive(Debug, Clone)]
pub enum DetailedMpcError {
    InsufficientBalance {
        required: f64,
        available: f64,
        currency: String,
    },
    // ... other variants
}
```

### Step 3: Implement the method
```rust
impl DetailedMpcError {
    pub fn to_string(&self) -> String {
        match self {
            DetailedMpcError::InsufficientBalance {
                required,
                available,
                currency,
            } => {
                format!(
                    "Insufficient {} balance: required {:.2}, available {:.2}",
                    currency, required, available
                )
            }
            // ... other patterns
        }
    }
}
```

### Step 4: Run the test
```bash
cargo test test_insufficient_balance_error_includes_amounts
```

✅ **Test passes!** Move to next test.

---

## Test Documentation

Each test file contains:

1. **Module documentation** - Explains the focus area
2. **Test names** - Clearly describe the scenario
3. **Comments** - Explain what's being tested
4. **Assertions** - Clear failure messages

### Reading a Test

```rust
/// Test that order validation errors include the validation failure reason
#[test]
fn test_order_validation_error_includes_reason() {
    let reason = "Symbol not in whitelist";
    let error = DetailedMpcError::OrderValidationFailed {
        symbol: "BTC-USD".to_string(),
        reason: reason.to_string(),
    };

    let error_msg = error.to_string();
    assert!(
        error_msg.contains(reason),
        "Error message should include the validation reason"
    );
}
```

**To understand this test:**
1. **Name:** `test_order_validation_error_includes_reason`
   → Tests that error includes the reason
   
2. **Comment:** "Test that order validation errors include the validation failure reason"
   → Explains the requirement
   
3. **Setup:** Creates error with reason "Symbol not in whitelist"
   → Shows realistic scenario
   
4. **Assertion:** Checks reason is in error message
   → Defines expected behavior

---

## Key Metrics to Track

| Metric | Status |
|--------|--------|
| Total Tests | 150+ |
| Passing Tests | 0 (RED phase) |
| Failing Tests | 150+ (intentional) |
| Test Coverage | 0% (will increase in GREEN) |
| Lines of Test Code | ~1,500 |

---

## Common Questions

### Q: Why are tests failing?
**A:** That's the point of RED phase! Tests define the requirements before implementation.

### Q: How do I implement the error types?
**A:** Look at the test to see what structure is expected, then implement it in `src/domain/errors.rs`.

### Q: Should I run tests before implementing?
**A:** Yes! Run tests to see what fails, then implement to make them pass.

### Q: Can I skip a test?
**A:** No. All tests must pass before moving to refactoring. This ensures all requirements are met.

### Q: What if a test is ambiguous?
**A:** Read the comments, look at similar tests, and check the test module documentation.

---

## Useful Resources

- **Module Docs:** `/src/domain/` - Read each test module's top comments
- **Summary:** `TDD_RED_PHASE_SUMMARY.md` - Comprehensive overview
- **Lock Ordering:** `src/application/services/mpc_service.rs:44-55` - Lock ordering rules
- **Errors:** `src/domain/errors.rs` - Current error types to replace

---

## Next Steps

1. ✅ Understand the tests (you're here!)
2. 🟢 Implement error types (Task 1)
3. 🟢 Implement position manager (Task 2)
4. 🟢 Implement order executor (Task 3)
5. 🟢 Add portfolio tracking (Task 4)
6. 🟢 Validate thread safety (Task 5)
7. 🔄 Refactor code while maintaining green tests
8. 📈 Run full test suite and measure coverage

---

**Status:** Ready for GREEN phase implementation! 🚀
