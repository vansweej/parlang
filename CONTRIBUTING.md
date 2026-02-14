# Contributing to ParLang

Thank you for your interest in contributing to ParLang! This document provides guidelines and best practices for contributing to the project.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Quality Guidelines](#code-quality-guidelines)
- [Testing Guidelines](#testing-guidelines)
- [Documentation Guidelines](#documentation-guidelines)
- [Pull Request Process](#pull-request-process)
- [Code Review Process](#code-review-process)

## Getting Started

### Prerequisites

- Rust 1.70 or later (edition 2021)
- Cargo (comes with Rust)
- Optional: Nix with flakes (for reproducible builds)

### Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/vansweej/parlang.git
   cd parlang
   ```

2. **Build the project**:
   ```bash
   cargo build
   ```

3. **Run tests**:
   ```bash
   cargo test
   ```

4. **Run the REPL**:
   ```bash
   cargo run
   ```

5. **Run with type checking enabled**:
   ```bash
   PARLANG_TYPECHECK=1 cargo run
   ```

## Code Quality Guidelines

### General Principles

1. **Clarity over Cleverness**: Write code that is easy to understand and maintain
2. **Minimal Changes**: Make the smallest change necessary to achieve the goal
3. **Test Coverage**: All new features should have tests
4. **Documentation**: Document public APIs and complex algorithms
5. **Error Handling**: Use proper error types, avoid panics in library code

### Rust-Specific Guidelines

#### Avoid Panics in Library Code

**❌ Bad**:
```rust
let number = s.parse::<i64>().unwrap();
```

**✅ Good**:
```rust
let number = s.parse::<i64>()
    .map_err(|_| ParseError::IntegerOverflow)?;
```

**Exception**: Using `unwrap()` or `expect()` is acceptable in:
- Test code (with descriptive messages)
- Internal assertions that are guaranteed by program logic (with comments explaining why)
- Binary/CLI code where a panic is an acceptable failure mode

#### Use Descriptive Error Messages

**❌ Bad**:
```rust
Err(EvalError::TypeError(format!("Type error: {op:?}")))
```

**✅ Good**:
```rust
Err(EvalError::TypeError(format!(
    "Type error in binary operation {:?}: cannot apply to {:?} and {:?}",
    op, left, right
)))
```

#### Checked Arithmetic for User Input

Always use checked arithmetic when dealing with user-provided values:

**❌ Bad**:
```rust
Ok(Value::Int(a + b))  // May overflow silently
```

**✅ Good**:
```rust
a.checked_add(b)
    .map(Value::Int)
    .ok_or_else(|| EvalError::ArithmeticOverflow("addition"))
```

#### Prefer Pattern Matching Over if-let Chains

**❌ Bad**:
```rust
if let Some(x) = opt {
    if let Some(y) = get_y(x) {
        if let Some(z) = get_z(y) {
            // ...
        }
    }
}
```

**✅ Good**:
```rust
match (opt, opt.and_then(get_y)) {
    (Some(x), Some(y)) => {
        // ...
    }
    _ => {
        // ...
    }
}
```

Or use early returns:
```rust
let x = opt?;
let y = get_y(x)?;
let z = get_z(y)?;
```

#### Use Dedicated Error Variants

**❌ Bad**:
```rust
Err(EvalError::TypeError("No pattern matched".to_string()))
```

**✅ Good**:
```rust
Err(EvalError::PatternMatchNonExhaustive)
```

Define specific error variants for different failure modes.

### Parser Combinator Guidelines

#### Document Complex Parsers

Add doc comments explaining:
- What the parser matches
- Examples of valid input
- Any special edge cases

```rust
/// Parse a type annotation for sum type constructors.
/// 
/// Examples:
/// - `Option Int` -> TypeAnnotation with "Option" and args ["Int"]
/// - `Either Bool String` -> TypeAnnotation with "Either" and args ["Bool", "String"]
/// 
/// Type parameters (lowercase) are instantiated with fresh type variables.
/// Concrete types (uppercase) are looked up in the type environment.
fn type_annotation<Input>() -> impl Parser<Input, Output = TypeAnnotation>
```

#### Extract Duplicate Parsers

**❌ Bad**:
```rust
// Integer parsing duplicated in multiple places
many1(digit()).and_then(|s: String| s.parse::<i64>()...)
many1(digit()).and_then(|s: String| s.parse::<i64>()...)
```

**✅ Good**:
```rust
fn integer<Input>() -> impl Parser<Input, Output = i64> {
    many1(digit()).and_then(|s: String| {
        s.parse::<i64>()
            .map_err(|_| StreamErrorFor::<Input>::unexpected_static_message("integer overflow"))
    })
}
```

#### Use Attempt Judiciously

`attempt()` enables backtracking but has performance cost. Use it only when necessary:

```rust
choice((
    attempt(constructor_pattern()),  // Try first, backtrack on failure
    variable_pattern(),              // If first fails, try this
))
```

## Testing Guidelines

### Unit Tests

- Place unit tests in the same file as the code they test (in a `#[cfg(test)]` mod tests` block)
- Test both success and failure cases
- Use descriptive test names that explain what is being tested

**Example**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_parsing_positive() {
        assert_eq!(parse("42"), Ok(Expr::Int(42)));
    }

    #[test]
    fn test_integer_parsing_negative() {
        assert_eq!(parse("-42"), Ok(Expr::Int(-42)));
    }

    #[test]
    fn test_integer_overflow_error() {
        let huge = "99999999999999999999";
        assert!(matches!(parse(huge), Err(ParseError::IntegerOverflow)));
    }
}
```

### Integration Tests

Place integration tests in the `tests/` directory. These test the system as a whole.

### Test Coverage Goals

- **Critical paths**: 100% coverage (parser, type checker, evaluator)
- **Error handling**: Test all error variants
- **Edge cases**: Empty inputs, boundary values, deeply nested structures
- **Regression tests**: Add tests for any bugs fixed

## Documentation Guidelines

### Code Documentation

#### Public APIs

All public functions, types, and modules should have doc comments:

```rust
/// Infers the type of an expression using Algorithm W.
///
/// # Arguments
/// * `expr` - The expression to type check
/// * `env` - The type environment containing variable types
///
/// # Returns
/// A `Result` containing the inferred type and substitution, or a type error.
///
/// # Examples
/// ```
/// let expr = Expr::Int(42);
/// let env = TypeEnvironment::new();
/// let (ty, subst) = infer(&expr, &env)?;
/// assert_eq!(ty, Type::Int);
/// ```
pub fn infer(expr: &Expr, env: &TypeEnvironment) -> Result<(Type, Substitution), TypeError>
```

#### Complex Algorithms

Document the approach and cite sources:

```rust
/// Implements Algorithm W for type inference (Damas & Milner, 1982).
///
/// This is a constraint-based type inference algorithm that:
/// 1. Traverses the expression tree
/// 2. Generates type constraints
/// 3. Solves constraints through unification
/// 4. Returns the principal (most general) type
///
/// Key components:
/// - `unify`: Finds substitutions to make types equal
/// - `generalize`: Creates polymorphic type schemes
/// - `instantiate`: Specializes polymorphic types
```

### User Documentation

User-facing documentation goes in the `docs/` directory:

- **README.md**: Project overview and quick start
- **docs/TYPE_SYSTEM.md**: Type system guide for users
- **docs/TYPE_INFERENCE.md**: Deep dive into type inference
- **docs/ARCHITECTURE.md**: System architecture
- **docs/MODULE_*.md**: Module-specific documentation

#### Documentation Structure

Each documentation file should include:
1. Overview section
2. Table of contents (for long documents)
3. Examples with expected output
4. Common patterns and idioms
5. Known limitations
6. References (where applicable)

## Pull Request Process

### Before Submitting

1. **Run tests**: Ensure all tests pass
   ```bash
   cargo test
   ```

2. **Run clippy**: Fix any warnings
   ```bash
   cargo clippy --all-targets --all-features
   ```

3. **Format code**: Use rustfmt
   ```bash
   cargo fmt
   ```

4. **Update documentation**: Add or update docs for any changes

5. **Add tests**: Include tests for new features or bug fixes

### PR Description Template

```markdown
## Description
Brief description of what this PR does.

## Motivation
Why is this change needed? What problem does it solve?

## Changes
- List of specific changes made
- Include before/after behavior if applicable

## Testing
- Describe how you tested the changes
- List any new tests added

## Documentation
- Note any documentation updates
- Link to relevant docs

## Breaking Changes
- List any breaking changes
- Describe migration path if applicable
```

### PR Size Guidelines

- **Small PRs** (< 200 lines): Preferred, easier to review
- **Medium PRs** (200-500 lines): Acceptable if well-organized
- **Large PRs** (> 500 lines): Should be split into smaller PRs when possible

## Code Review Process

### As a Reviewer

1. **Be constructive**: Suggest improvements, don't just point out problems
2. **Ask questions**: If something is unclear, ask for clarification
3. **Check tests**: Verify test coverage and quality
4. **Consider alternatives**: Suggest alternative approaches when appropriate
5. **Be timely**: Try to review within 2-3 days

### As an Author

1. **Respond to feedback**: Address comments or explain why you disagree
2. **Make requested changes**: Or discuss alternatives
3. **Keep discussions focused**: Move complex discussions to issues
4. **Be patient**: Reviews take time
5. **Learn from feedback**: Use it to improve future contributions

## Specific Areas for Contribution

### High Priority

1. **Recursive function type checking**: Add support for typing recursive functions
2. **Pattern matching exhaustiveness**: Check that match expressions cover all cases
3. **Type annotations**: Add syntax for explicit type annotations
4. **Performance optimization**: Optimize environment cloning and substitution

### Medium Priority

1. **Row polymorphism**: Better record type inference
2. **Better error messages**: More context in error messages
3. **Standard library**: Expand the standard library (`examples/stdlib.par`)
4. **Editor support**: Language server protocol implementation

### Documentation Needs

1. **Tutorial**: Step-by-step tutorial for beginners
2. **Examples**: More complex examples showcasing features
3. **Internals guide**: Deep dive into implementation details
4. **Performance guide**: Tips for writing efficient ParLang code

## Questions?

If you have questions about contributing:
- Open an issue with the "question" label
- Check existing documentation in the `docs/` directory
- Look at recent PRs for examples

## License

By contributing to ParLang, you agree that your contributions will be licensed under the MIT License.
