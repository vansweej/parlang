# Pattern Matching Exhaustiveness Checking

ParLang now includes **complete exhaustiveness checking** for pattern matching expressions. This feature helps catch bugs early by warning you when a `match` expression doesn't cover all possible cases.

## Table of Contents

- [What is Exhaustiveness Checking?](#what-is-exhaustiveness-checking)
- [How It Works](#how-it-works)
- [Pattern Types](#pattern-types)
- [Examples](#examples)
- [Warnings](#warnings)
- [Best Practices](#best-practices)
- [Technical Details](#technical-details)

## What is Exhaustiveness Checking?

Exhaustiveness checking analyzes your pattern matching expressions to determine if they handle all possible values. When a `match` expression is **non-exhaustive** (doesn't cover all cases), the checker warns you before the code runs, helping prevent runtime errors.

### Benefits

- **Catch bugs early**: Find missing cases before runtime
- **Better code quality**: Ensures all cases are handled
- **Documentation**: Patterns serve as documentation of possible values
- **Refactoring safety**: Adding new constructors will trigger warnings in existing matches

## How It Works

The exhaustiveness checker runs automatically before evaluating any `match` expression. It analyzes the patterns to ensure they cover all possible values of the scrutinee (the value being matched).

```parlang
type Option a = Some a | None in
match myOption with
| Some n -> n
| None -> 0
# ✓ Exhaustive: Both Some and None are covered
```

```parlang
type Option a = Some a | None in
match myOption with
| Some n -> n
# ⚠ Warning: pattern match is non-exhaustive
# Missing cases: None
```

## Pattern Types

The exhaustiveness checker handles all pattern types in ParLang:

### 1. Literal Patterns

**Boolean literals** must cover both `true` and `false`:

```parlang
# Exhaustive
match flag with
| true -> 1
| false -> 0
```

```parlang
# Non-exhaustive
match flag with
| true -> 1
# Warning: Missing cases: false
```

**Integer literals** are infinite, so they require a catch-all pattern:

```parlang
# Non-exhaustive
match n with
| 0 -> 1
| 1 -> 2
# Warning: Missing cases: <other integers>
```

```parlang
# Exhaustive with wildcard
match n with
| 0 -> 1
| 1 -> 2
| _ -> 999
```

### 2. Variable and Wildcard Patterns

These patterns match everything and make the match exhaustive:

```parlang
# Variable pattern (exhaustive)
match x with
| n -> n + 1
```

```parlang
# Wildcard pattern (exhaustive)
match x with
| 0 -> 1
| _ -> 999
```

### 3. Constructor Patterns (Sum Types)

For sum types, **all constructors** must be covered:

```parlang
type Option a = Some a | None in

# Exhaustive
match x with
| Some n -> n
| None -> 0
```

```parlang
type Either a b = Left a | Right b in

# Non-exhaustive
match x with
| Left n -> n
# Warning: Missing cases: Right
```

```parlang
type List a = Nil | Cons a (List a) in

# Exhaustive
match list with
| Nil -> 0
| Cons head tail -> head
```

### 4. Tuple Patterns

Tuple patterns alone (without a catch-all) are considered non-exhaustive:

```parlang
# Non-exhaustive
match (x, y) with
| (0, 0) -> 1
| (1, 1) -> 2
# Warning: Missing cases: _
```

```parlang
# Exhaustive with wildcard
match (x, y) with
| (0, 0) -> 1
| _ -> 999
```

### 5. Record Patterns

Record patterns alone (without a catch-all) are considered non-exhaustive:

```parlang
# Non-exhaustive
match person with
| { name: 0 } -> 1
# Warning: Missing cases: _
```

```parlang
# Exhaustive with wildcard
match person with
| { name: 0 } -> 1
| _ -> 999
```

### 6. Nested Patterns

The checker analyzes nested constructors properly:

```parlang
type Option a = Some a | None in

# Exhaustive
match x with
| Some (Some n) -> n
| Some None -> 0
| None -> 0
```

```parlang
# Non-exhaustive - missing None at top level
match x with
| Some (Some n) -> n
| Some None -> 0
# Warning: Missing cases: None
```

## Examples

### Option Type

```parlang
type Option a = Some a | None in

# Good: Exhaustive match
let safeDivide = fun x -> fun y ->
  if y == 0 then None else Some (x / y)
in
match safeDivide 10 2 with
| Some result -> result
| None -> 0
# Result: 5
```

### Either Type (Error Handling)

```parlang
type Either a b = Left a | Right b in

# Good: Exhaustive match
let parseNumber = fun x ->
  if x > 0 then Right x else Left 0
in
match parseNumber 5 with
| Right n -> n * 2
| Left err -> err
# Result: 10
```

### List Type (Recursive Data)

```parlang
type List a = Nil | Cons a (List a) in

# Good: Exhaustive match
let rec sum -> fun list ->
  match list with
  | Nil -> 0
  | Cons head tail -> head + sum tail
in
sum (Cons 1 (Cons 2 (Cons 3 Nil)))
# Result: 6
```

### Result Type

```parlang
type Result a b = Ok a | Err b in

# Good: Exhaustive match
let divide = fun x -> fun y ->
  if y == 0 then Err false else Ok (x / y)
in
match divide 10 5 with
| Ok value -> value
| Err _ -> 0
# Result: 2
```

### Tree Type

```parlang
type Tree a = Leaf | Node a (Tree a) (Tree a) in

# Good: Exhaustive match
let tree = Node 5 (Node 3 Leaf Leaf) (Node 7 Leaf Leaf) in
match tree with
| Leaf -> 0
| Node value _ _ -> value
# Result: 5
```

## Warnings

When a match is non-exhaustive, you'll see a warning like this:

```
Warning: pattern match is non-exhaustive
  Missing cases: None
```

or

```
Warning: pattern match is non-exhaustive
  Missing cases: Right
```

These warnings appear before evaluation but don't stop the program from running. However, if the runtime encounters a value that doesn't match any pattern, you'll get a runtime error:

```
Type error: No pattern matched in match expression
```

## Best Practices

### 1. Always Handle All Cases

For sum types, explicitly handle all constructors:

```parlang
# Good
type Option a = Some a | None in
match x with
| Some n -> n
| None -> 0
```

### 2. Use Wildcards for Catch-All Cases

When you don't care about specific values, use wildcards:

```parlang
# Good: Clear intent that we only care about Some
type Option a = Some a | None in
match x with
| Some n -> n
| _ -> 0
```

### 3. Keep Pattern Order in Mind

Patterns are matched top to bottom. Put specific patterns first:

```parlang
# Good order
match n with
| 0 -> 1
| 1 -> 2
| _ -> 999
```

### 4. Use Variable Patterns for Binding

When you need the matched value, use a variable pattern:

```parlang
match n with
| 0 -> 1
| m -> m * 2  # 'm' binds the value for use
```

### 5. Document Missing Cases

If you intentionally omit cases (and use a wildcard), add a comment:

```parlang
type Status = Active | Inactive | Pending | Archived in
match status with
| Active -> handleActive()
| Inactive -> handleInactive()
| _ -> 0  # Pending and Archived treated the same
```

## Technical Details

### Algorithm

The exhaustiveness checker uses a matrix-based algorithm inspired by "Warnings for pattern matching" by Luc Maranget (2007). It:

1. Collects all patterns from the match arms
2. Analyzes each pattern type (literal, constructor, tuple, etc.)
3. Checks if all possible values are covered
4. Reports missing cases if any

### Constructor Coverage

For sum types, the checker:
- Identifies the type from the first constructor used
- Retrieves all constructors for that type from the environment
- Checks if all constructors appear in the patterns
- Reports any missing constructors

### Nested Pattern Analysis

The checker recursively analyzes nested patterns:
- Tracks constructors at all nesting levels
- Ensures inner patterns are exhaustive (for their type)
- Reports the outermost missing cases

### Performance

The exhaustiveness checker is designed to be fast:
- Runs in O(n × m) where n is number of patterns and m is number of constructors
- Uses hash sets for efficient constructor lookup
- Minimal overhead before evaluation

### Limitations

Current limitations (potential future enhancements):

1. **Integer patterns**: Considered non-exhaustive without a catch-all (integers are infinite)
2. **Tuple exhaustiveness**: Basic check only; doesn't analyze all possible tuple combinations
3. **Record exhaustiveness**: Basic check only; doesn't analyze all field combinations
4. **Guard patterns**: Not yet supported (planned for future versions)

## See Also

- [Sum Types Documentation](SUM_TYPES.md) - Learn about algebraic data types
- [Pattern Matching Examples](EXAMPLES.md) - More pattern matching examples
- [Language Specification](LANGUAGE_SPEC.md) - Formal pattern matching grammar
- [Type System](TYPE_SYSTEM.md) - Type inference and checking
