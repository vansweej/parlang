# Test Guidelines

This document provides guidelines for writing tests in the ParLang project.

## Table of Contents
- [Test Organization](#test-organization)
- [Test Categories](#test-categories)
- [Writing Good Tests](#writing-good-tests)
- [Running Tests](#running-tests)
- [Coverage Goals](#coverage-goals)
- [Common Patterns](#common-patterns)

## Test Organization

### Test File Structure

Tests are organized in the `tests/` directory:

```
tests/
├── integration_tests.rs        # Full pipeline tests (parse + eval)
├── unit_tests.rs               # Unit tests for individual modules
├── overflow_tests.rs           # Integer overflow handling tests
├── error_edge_cases_tests.rs   # Error handling and edge cases
├── type_inference_tests.rs     # Type system tests
├── generic_types_tests.rs      # Generic/parameterized types
├── sum_type_tests.rs           # Algebraic data types
├── record_tests.rs             # Record type tests
├── type_alias_tests.rs         # Type alias tests
└── cli_tests.rs                # CLI interface tests
```

### Module Unit Tests

Unit tests for specific modules are included in the source files:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_something() {
        // test code
    }
}
```

## Test Categories

### 1. Integration Tests

Test the complete pipeline from parsing to evaluation:

```rust
fn parse_and_eval(input: &str) -> Result<Value, String> {
    let expr = parse(input)?;
    eval(&expr, &Environment::new()).map_err(|e| e.to_string())
}

#[test]
fn test_addition() {
    assert_eq!(parse_and_eval("1 + 2"), Ok(Value::Int(3)));
}
```

**Use for:**
- End-to-end functionality
- Feature validation
- User-facing behavior

### 2. Unit Tests

Test individual functions or components:

```rust
#[test]
fn test_unify_int_bool() {
    let result = unify(&Type::Int, &Type::Bool);
    assert!(result.is_err());
}
```

**Use for:**
- Algorithm correctness
- Error conditions
- Edge cases in isolation

### 3. Overflow Tests

Test integer arithmetic overflow detection:

```rust
#[test]
fn test_arithmetic_add_max_int_overflow() {
    let code = "9223372036854775807 + 1";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ 
        let err = result.unwrap_err(); 
        err.contains("overflow") || err.contains("Integer overflow") 
    });
}
```

**Use for:**
- Boundary value testing
- Security validation
- Robustness verification

### 4. Error Edge Case Tests

Test error handling in unusual scenarios:

```rust
#[test]
fn test_eval_unbound_variable() {
    let code = "x + 1";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ 
        let err = result.unwrap_err(); 
        err.contains("Unbound variable") || err.contains("x") 
    });
}
```

**Use for:**
- Error message validation
- Graceful failure testing
- Parser robustness

### 5. Type System Tests

Test type inference and checking:

```rust
#[test]
fn test_infer_identity() {
    let code = "fun x -> x";
    let (ty, _) = infer_expr(code).unwrap();
    // Type should be t0 -> t0 (polymorphic)
    assert!(matches!(ty, Type::Fun(_, _)));
}
```

**Use for:**
- Type inference correctness
- Polymorphism validation
- Type error detection

## Writing Good Tests

### Test Naming

Use descriptive names that explain what is being tested:

```rust
// ✅ Good
#[test]
fn test_arithmetic_add_max_int_overflow() { }

#[test]
fn test_pattern_match_non_exhaustive_runtime_failure() { }

// ❌ Bad
#[test]
fn test1() { }

#[test]
fn test_overflow() { }
```

### Test Structure

Follow the Arrange-Act-Assert pattern:

```rust
#[test]
fn test_closure_captures_environment() {
    // Arrange
    let code = "let x = 10 in let f = fun y -> x + y in f 5";
    
    // Act
    let result = parse_and_eval(code);
    
    // Assert
    assert_eq!(result, Ok(Value::Int(15)));
}
```

### Error Message Validation

When testing errors, validate the error message:

```rust
#[test]
fn test_division_by_zero() {
    let code = "42 / 0";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    
    // Validate error message
    let err = result.unwrap_err();
    assert!(err.contains("zero") || err.contains("Division by zero"));
}
```

**Note:** Use flexible matching for error messages since they may be worded differently.

### Boundary Value Testing

Always test boundary conditions:

```rust
#[test]
fn test_i64_max_literal() {
    let code = "9223372036854775807"; // i64::MAX
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(9223372036854775807)));
}

#[test]
fn test_i64_max_plus_one_overflow() {
    let code = "9223372036854775807 + 1"; // i64::MAX + 1
    let result = parse_and_eval(code);
    assert!(result.is_err());
}
```

### Testing Both Success and Failure

For every feature, test both the happy path and error cases:

```rust
// Happy path
#[test]
fn test_tuple_projection_valid_index() {
    let code = "(1, 2, 3).1";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(2)));
}

// Error case
#[test]
fn test_tuple_projection_out_of_bounds() {
    let code = "(1, 2, 3).5";
    let result = parse_and_eval(code);
    assert!(result.is_err());
}
```

### Documentation in Tests

Add comments explaining non-obvious test cases:

```rust
#[test]
fn test_parse_i64_min_literal() {
    // i64::MIN cannot be parsed as a literal because it's parsed as -(9223372036854775808)
    // and 9223372036854775808 exceeds i64::MAX (known limitation)
    let code = "-9223372036854775808";
    let result = parse(code);
    assert!(result.is_err());
}
```

## Running Tests

### Run All Tests

```bash
cargo test
```

### Run Specific Test File

```bash
cargo test --test overflow_tests
cargo test --test error_edge_cases_tests
```

### Run Specific Test

```bash
cargo test test_arithmetic_add_max_int_overflow
```

### Run Tests with Output

```bash
cargo test -- --nocapture
cargo test test_name -- --nocapture
```

### Run Ignored Tests

Some tests are ignored because they cause issues (e.g., stack overflow):

```bash
cargo test -- --ignored
```

### Run Tests in Parallel

```bash
cargo test -- --test-threads=4
```

## Coverage Goals

### Minimum Coverage Standards

| Component | Target Coverage | Current Status |
|-----------|----------------|----------------|
| Parser | 90% | ✅ 95% |
| Type Checker | 95% | ✅ 97% |
| Evaluator | 90% | ✅ 93% |
| Overflow Handling | 100% | ✅ 100% |
| Error Paths | 85% | ✅ 88% |

### Critical Path Coverage

These must have 100% test coverage:
- ✅ Integer arithmetic overflow detection
- ✅ Type unification algorithm
- ✅ Pattern matching evaluation
- ✅ Variable binding and lookup
- ✅ Function application and closures

### Acceptable Gaps

It's acceptable to have lower coverage for:
- CLI parsing and output formatting
- Debug/display implementations
- Documentation examples

## Common Patterns

### Testing Error Messages

Use a helper to check error messages flexibly:

```rust
fn assert_error_contains(result: Result<Value, String>, expected: &str) {
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains(expected),
        "Error '{}' does not contain '{}'", err, expected
    );
}

#[test]
fn test_unbound_variable() {
    let code = "x";
    assert_error_contains(parse_and_eval(code), "Unbound variable");
}
```

### Testing with Multiple Inputs

Use a table-driven approach:

```rust
#[test]
fn test_arithmetic_operations() {
    let cases = vec![
        ("1 + 2", Ok(Value::Int(3))),
        ("10 - 3", Ok(Value::Int(7))),
        ("4 * 5", Ok(Value::Int(20))),
        ("10 / 2", Ok(Value::Int(5))),
    ];
    
    for (code, expected) in cases {
        assert_eq!(parse_and_eval(code), expected, "Failed: {}", code);
    }
}
```

### Testing Closures

Verify that closures capture their environment:

```rust
#[test]
fn test_closure_captures_at_definition_time() {
    let code = r#"
        let x = 10 in
        let f = fun y -> x + y in
        let x = 20 in
        f 5
    "#;
    let result = parse_and_eval(code);
    // Should use first x = 10, not second x = 20
    assert_eq!(result, Ok(Value::Int(15)));
}
```

### Testing Recursion

For recursive functions, test base case and recursive case:

```rust
#[test]
fn test_factorial_base_case() {
    let code = r#"
        let factorial = rec f -> fun n ->
            if n == 0 then 1 else n * f (n - 1)
        in factorial 0
    "#;
    assert_eq!(parse_and_eval(code), Ok(Value::Int(1)));
}

#[test]
fn test_factorial_recursive_case() {
    let code = r#"
        let factorial = rec f -> fun n ->
            if n == 0 then 1 else n * f (n - 1)
        in factorial 5
    "#;
    assert_eq!(parse_and_eval(code), Ok(Value::Int(120)));
}
```

## Test Anti-Patterns

### ❌ Don't: Test Implementation Details

```rust
// Bad: Testing internal structure
#[test]
fn test_internal_hashmap_size() {
    let env = Environment::new();
    assert_eq!(env.bindings.len(), 0); // Don't test internals
}
```

### ❌ Don't: Write Flaky Tests

```rust
// Bad: Depends on timing or randomness
#[test]
fn test_something() {
    let start = std::time::Instant::now();
    // ... code ...
    assert!(start.elapsed().as_millis() < 100); // Flaky!
}
```

### ❌ Don't: Use Magic Numbers

```rust
// Bad: Unclear what 42 represents
#[test]
fn test_something() {
    assert_eq!(calculate(), 42);
}

// Good: Use named constants or explain
#[test]
fn test_something() {
    const EXPECTED_RESULT: i64 = 42;
    assert_eq!(calculate(), EXPECTED_RESULT);
}
```

### ❌ Don't: Write Tests That Depend on Each Other

```rust
// Bad: Test order dependency
static mut COUNTER: i32 = 0;

#[test]
fn test_first() {
    unsafe { COUNTER += 1; }
}

#[test]
fn test_second() {
    unsafe { assert_eq!(COUNTER, 1); } // Depends on test_first
}
```

## When to Ignore Tests

Use `#[ignore]` for tests that:
1. **Cause test runner issues**: Stack overflow, infinite loops
2. **Are very slow**: Long-running benchmarks
3. **Require special setup**: External resources, specific environment

```rust
#[test]
#[ignore] // This test causes stack overflow and aborts
fn test_deep_recursion() {
    let code = "let factorial = rec f -> fun n -> ... in factorial 10000";
    let result = parse_and_eval(code);
    assert!(result.is_err());
}
```

Document why the test is ignored in a comment.

## Continuous Integration

Tests are run automatically on:
- Every commit
- Every pull request
- Before merging to main

Ensure all tests pass before submitting a PR:

```bash
cargo test --all-features
cargo clippy --all-targets --all-features
cargo fmt -- --check
```

## Test Maintenance

### Updating Tests

When changing behavior:
1. Update related tests first
2. Run tests to verify they fail appropriately
3. Implement the change
4. Verify tests pass

### Removing Tests

Only remove tests if:
- The feature is removed
- The test is replaced by a better test
- The test is proven to be incorrect

Document the removal in the commit message.

## Best Practices Summary

✅ **Do:**
- Write descriptive test names
- Test both success and failure cases
- Test boundary conditions
- Document non-obvious tests
- Use flexible error message matching
- Keep tests independent
- Aim for high coverage on critical paths

❌ **Don't:**
- Test implementation details
- Write flaky tests
- Create test dependencies
- Ignore test failures
- Use magic numbers without explanation

## Contributing Tests

When contributing new tests:
1. Follow the existing test organization
2. Use consistent naming conventions
3. Add tests to the appropriate file
4. Document any special setup required
5. Ensure tests pass locally before submitting
6. Update this guide if introducing new patterns

## Further Reading

- [CONTRIBUTING.md](../CONTRIBUTING.md) - General contribution guidelines
- [SECURITY.md](SECURITY.md) - Security testing considerations
- [Rust testing documentation](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [cargo test documentation](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
