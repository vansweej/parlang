# Error Handling in ParLang

This document describes error handling patterns, error types, and best practices for working with errors in ParLang.

## Table of Contents
- [Error Types](#error-types)
- [Error Handling Patterns](#error-handling-patterns)
- [Parser Errors](#parser-errors)
- [Type Errors](#type-errors)
- [Runtime Errors](#runtime-errors)
- [Error Messages](#error-messages)
- [Best Practices](#best-practices)

## Error Types

ParLang has three main categories of errors:

### 1. Parse Errors

Errors that occur during parsing of source code:

```rust
pub enum ParseError {
    UnexpectedToken,
    UnexpectedEOF,
    IntegerOverflow,
    InvalidSyntax(String),
}
```

**Common causes:**
- Syntax errors: `{ x: 1` (unclosed brace)
- Integer overflow during parsing: `99999999999999999999`
- Invalid operators: `1 ++ 2`
- Malformed expressions: `if then else`

**Example:**
```parlang
// Parse error: unclosed brace
{ x: 1, y: 2
```

### 2. Type Errors

Errors that occur during type checking (when `PARLANG_TYPECHECK=1`):

```rust
pub enum TypeError {
    UnificationError(Type, Type),
    UnboundVariable(String),
    InfiniteType(TypeVar, Type),
    OccursCheck(TypeVar, Type),
    RecursionRequiresAnnotation,
    ConstructorArityMismatch(String, usize, usize),
}
```

**Common causes:**
- Type mismatch: `1 + true`
- Unbound variables: `x` when x is not defined
- Recursive functions without annotations
- Constructor arity mismatch: `Some 1 2` (too many args)
- Occurs check failure (infinite type)

**Example:**
```parlang
> 1 + true
Type error: Cannot unify types: Bool and Int
```

### 3. Runtime Errors

Errors that occur during evaluation:

```rust
pub enum EvalError {
    UnboundVariable(String),
    TypeError(String),
    DivisionByZero,
    LoadError(String),
    IndexOutOfBounds(String),
    FieldNotFound(String, Vec<String>),
    RecordExpected(String),
    UnknownConstructor(String),
    ConstructorArityMismatch(String, usize, usize),
    PatternMatchNonExhaustive,
}
```

**Common causes:**
- Division by zero: `42 / 0`
- Integer overflow: `9223372036854775807 + 1`
- Unbound variables: `x` when x is not in environment
- Field access on non-record: `42.field`
- Field not found: `{x: 1}.y`
- Tuple projection out of bounds: `(1, 2).5`
- Pattern match failure: no matching pattern
- Unknown constructor: using undefined sum type constructor

## Error Handling Patterns

### Parsing Errors

**Pattern:** Return `Result<Expr, String>`

```rust
let code = "1 + 2";
match parse(code) {
    Ok(expr) => println!("Parsed successfully: {:?}", expr),
    Err(e) => eprintln!("Parse error: {}", e),
}
```

**Handling specific errors:**
```rust
match parse(code) {
    Ok(expr) => { /* success */ },
    Err(e) if e.contains("overflow") => {
        eprintln!("Integer too large: {}", e);
    },
    Err(e) if e.contains("Unexpected") => {
        eprintln!("Syntax error: {}", e);
    },
    Err(e) => eprintln!("Parse error: {}", e),
}
```

### Type Checking Errors

**Pattern:** Return `Result<(Type, Substitution), TypeError>`

```rust
use parlang::typechecker::infer;

match infer(&expr, &env) {
    Ok((ty, subst)) => println!("Type: {}", ty),
    Err(TypeError::UnificationError(t1, t2)) => {
        eprintln!("Cannot unify {} and {}", t1, t2);
    },
    Err(TypeError::UnboundVariable(name)) => {
        eprintln!("Variable '{}' not found", name);
    },
    Err(e) => eprintln!("Type error: {:?}", e),
}
```

### Runtime Evaluation Errors

**Pattern:** Return `Result<Value, EvalError>`

```rust
match eval(&expr, &env) {
    Ok(value) => println!("Result: {}", value),
    Err(EvalError::DivisionByZero) => {
        eprintln!("Error: Division by zero");
    },
    Err(EvalError::UnboundVariable(name)) => {
        eprintln!("Error: Variable '{}' is not defined", name);
    },
    Err(EvalError::PatternMatchNonExhaustive) => {
        eprintln!("Error: No pattern matched");
    },
    Err(e) => eprintln!("Runtime error: {}", e),
}
```

## Parser Errors

### Integer Overflow Detection

The parser detects integer overflow during parsing:

```parlang
// This will fail at parse time:
99999999999999999999999999999

// Error: "integer overflow" or "Unexpected input"
```

**Implementation:**
```rust
let number = many1(digit()).and_then(|s: String| {
    s.parse::<i64>()
        .map_err(|_| StreamErrorFor::<Input>::unexpected_static_message("integer overflow"))
});
```

### Handling Parse Errors

**Best practice:** Provide context to users

```rust
fn parse_with_context(code: &str, filename: &str) -> Result<Expr, String> {
    parse(code).map_err(|e| {
        format!("Error in {}: {}", filename, e)
    })
}
```

### Common Parse Errors

| Input | Error | Explanation |
|-------|-------|-------------|
| `{ x: 1` | Unexpected EOF | Unclosed brace |
| `999...999` | Integer overflow | Number too large |
| `-9223372036854775808` | Unexpected input | i64::MIN as literal |
| `if 1` | Parse error | Incomplete if expression |
| `` (empty) | Default to 0 | Empty input parses as 0 |

## Type Errors

### Unification Errors

When types cannot be unified:

```parlang
> 1 + true
Type error: Cannot unify types: Bool and Int

> if 1 then 2 else 3
Type error: Cannot unify types: Int and Bool
```

**Debugging tip:** Check the types of sub-expressions:
```parlang
> 1      # Type: Int
> true   # Type: Bool
> 1 + true  # Error: Can't add Int and Bool
```

### Occurs Check

Prevents infinite types:

```parlang
> rec f -> f f
Type error: Occurs check failed: t0 occurs in t0 -> t1
```

This prevents creating types like `t0 = t0 -> t1` which are infinite.

### Recursive Functions

Recursive functions are not yet supported in the type checker:

```parlang
> rec factorial -> fun n -> if n == 0 then 1 else n * factorial (n - 1)
Type error: RecursionRequiresAnnotation
```

**Workaround:** Disable type checking:
```bash
# Without PARLANG_TYPECHECK, recursion works at runtime
cargo run examples/factorial.par
```

### Constructor Arity Mismatch

Using constructors with wrong number of arguments:

```parlang
type Option a = Some a | None in
Some 1 2  # Error: Some expects 1 argument, got 2
```

## Runtime Errors

### Integer Overflow

Arithmetic operations detect overflow:

```parlang
> 9223372036854775807 + 1
Type error: Integer overflow in addition

> 9223372036854775807 * 2
Type error: Integer overflow in multiplication

> (-9223372036854775807 - 1) - 1
Type error: Integer overflow in subtraction

> (-9223372036854775807 - 1) / -1
Type error: Integer overflow in division
```

**Note:** Overflow is detected at runtime, not compile time (except for literals).

### Division by Zero

```parlang
> 42 / 0
Runtime error: Division by zero
```

Always caught and reported.

### Unbound Variables

```parlang
> x + 1
Runtime error: Unbound variable: x

> let y = 10 in x + y
Runtime error: Unbound variable: x
```

**Debugging tip:** Check variable names for typos.

### Pattern Match Non-Exhaustive

When no pattern matches:

```parlang
type Option a = Some a | None in
let x = None in
match x with
| Some n -> n
# Runtime error: Pattern match is non-exhaustive
```

**Solution:** Add a catch-all pattern:
```parlang
match x with
| Some n -> n
| None -> 0
| _ -> 0  # Or use wildcard
```

### Field Access Errors

Accessing non-existent fields:

```parlang
> { x: 1, y: 2 }.z
Runtime error: Field 'z' not found. Available fields: ["x", "y"]

> 42.field
Runtime error: Expected record, got Int
```

### Tuple Projection Errors

Invalid tuple indices:

```parlang
> (1, 2, 3).5
Runtime error: Index out of bounds: tuple has 3 elements, tried to access index 5

> ().0
Runtime error: Index out of bounds: tuple has 0 elements, tried to access index 0
```

## Error Messages

### Format Guidelines

Good error messages should:
1. **Be specific:** "Field 'x' not found" not just "Error"
2. **Provide context:** Show available options when relevant
3. **Suggest fixes:** When possible
4. **Use consistent wording:** Similar errors use similar messages

### Error Message Examples

**Good:**
```
Field 'age' not found. Available fields: ["name", "address", "phone"]
```

**Bad:**
```
Error
```

### Improving Error Messages

When adding new error types:

```rust
// Good: Specific error with context
Err(EvalError::FieldNotFound(field.clone(), available_fields))

// Bad: Generic error
Err(EvalError::TypeError("field error".to_string()))
```

## Best Practices

### 1. Use Specific Error Types

**Do:**
```rust
Err(EvalError::DivisionByZero)
Err(EvalError::PatternMatchNonExhaustive)
```

**Don't:**
```rust
Err(EvalError::TypeError("division by zero".to_string()))
Err(EvalError::TypeError("pattern match failed".to_string()))
```

### 2. Propagate Errors with `?`

```rust
fn evaluate_program(code: &str) -> Result<Value, String> {
    let expr = parse(code)?;
    eval(&expr, &Environment::new()).map_err(|e| e.to_string())
}
```

### 3. Add Context When Re-throwing

```rust
fn load_and_eval(path: &str) -> Result<Value, String> {
    let code = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to load {}: {}", path, e))?;
    
    parse_and_eval(&code)
        .map_err(|e| format!("Error in {}: {}", path, e))
}
```

### 4. Test Error Conditions

Always test that errors are properly raised:

```rust
#[test]
fn test_division_by_zero() {
    let code = "42 / 0";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Division by zero"));
}
```

### 5. Document Known Limitations

When errors indicate limitations, document them:

```rust
// Known limitation: i64::MIN cannot be parsed as a literal
// Use expression instead: -9223372036854775807 - 1
#[test]
fn test_i64_min_literal() {
    let code = "-9223372036854775808";
    assert!(parse(code).is_err());
}
```

## Error Recovery

### Parser Recovery

The parser does not implement error recovery. A single syntax error causes parsing to fail entirely.

**Future improvement:** Implement error recovery to:
- Report multiple errors at once
- Continue parsing after errors
- Provide better error locations

### Type Checker Recovery

The type checker stops at the first type error.

**Workaround:** Fix errors one at a time, starting from the first reported error.

### Runtime Recovery

Runtime errors stop execution immediately. There is no exception handling or error recovery mechanism.

**Workaround:** Use pattern matching and conditionals to handle edge cases:

```parlang
let safe_divide = fun x -> fun y ->
    if y == 0
    then None
    else Some (x / y)
```

## Debugging Error Cases

### Enable Type Checking

Type checking can catch many errors before runtime:

```bash
PARLANG_TYPECHECK=1 cargo run
```

### Use Smaller Test Cases

Break down complex expressions:

```parlang
# Instead of:
let x = (fun y -> y + z) (a + b)

# Try:
let temp1 = a + b in
let temp2 = fun y -> y + z in
temp2 temp1
```

### Check Sub-expressions

Evaluate parts of the expression separately:

```parlang
> let f = fun x -> x + 1
> f 42  # Works
> f true  # Error: type mismatch
```

### Add Logging

When embedding ParLang, add logging:

```rust
match eval(&expr, &env) {
    Ok(value) => {
        println!("DEBUG: Evaluated to {:?}", value);
        Ok(value)
    },
    Err(e) => {
        eprintln!("DEBUG: Error during evaluation: {:?}", e);
        Err(e)
    },
}
```

## Common Error Patterns

### Pattern 1: Overflow in Recursive Functions

```parlang
# Problem: Stack overflow or arithmetic overflow
rec factorial -> fun n ->
    if n == 0 then 1 else n * factorial (n - 1)
    
factorial 100  # Stack overflow or arithmetic overflow
```

**Solution:** Use accumulator with TCO:
```parlang
let factorial = rec helper -> fun acc -> fun n ->
    if n == 0 then acc else helper (acc * n) (n - 1)
in factorial 1
```

### Pattern 2: Non-Exhaustive Pattern Match

```parlang
# Problem: Missing patterns
type Option a = Some a | None in
match opt with
| Some x -> x
# Missing None case
```

**Solution:** Add all patterns:
```parlang
match opt with
| Some x -> x
| None -> default_value
```

### Pattern 3: Field Access on Wrong Type

```parlang
# Problem: Accessing field on non-record
let x = 42 in x.field
```

**Solution:** Ensure x is a record:
```parlang
let x = { field: 42 } in x.field
```

## Error Handling Checklist

When implementing new features:

- [ ] Define specific error types
- [ ] Add error messages with context
- [ ] Test error conditions
- [ ] Document known limitations
- [ ] Handle overflow/boundary cases
- [ ] Provide helpful error messages
- [ ] Test error recovery behavior

## Future Improvements

Planned error handling improvements:

1. **Better error locations**: Line and column numbers in error messages
2. **Error recovery**: Multiple errors per run
3. **Suggestions**: "Did you mean X?" style messages
4. **Stack traces**: Show call stack for runtime errors
5. **Type error explanations**: More detailed type mismatch explanations
6. **Warnings**: Non-fatal warnings for suspicious code

## See Also

- [TEST_GUIDELINES.md](TEST_GUIDELINES.md) - Testing error conditions
- [TYPE_SYSTEM.md](TYPE_SYSTEM.md) - Type error details
- [SECURITY.md](SECURITY.md) - Security-related errors
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Error handling standards
