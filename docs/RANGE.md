# Range Type

ParLang now supports **Range** - a type representing inclusive integer ranges. Ranges provide a concise way to represent a span of integers from a start value to an end value.

## Overview

Ranges in ParLang are:
- **Inclusive**: Both start and end values are part of the range
- **Integer-based**: Start and end must be integers
- **Immutable**: Once created, a range cannot be modified
- **Type-safe**: Type checking ensures ranges are used correctly

## Syntax

### Range Creation

Ranges are created using the `..` operator:

```parlang
1..10         # Range from 1 to 10 (inclusive)
0..100        # Range from 0 to 100
-5..5         # Range from -5 to 5
10..1         # Descending range (valid but represents 10 to 1)
```

### Range with Expressions

Ranges can be created from any expression that evaluates to an integer:

```parlang
let start = 5 in let end = 15 in start..end
# Result: 5..15

1+1..10*2
# Result: 2..20

let x = 10 in (x - 5)..(x + 5)
# Result: 5..15
```

## Type System

Ranges have the type `Range`:

```parlang
> 1..10
Type: Range
1..10

> let r = 0..100 in r
Type: Range
0..100

> fun start -> fun end -> start..end
Type: Int -> Int -> Range
<function start>
```

### Type Checking

The type checker ensures that:
- Start and end values are integers
- Range operations are type-safe
- Ranges can be compared for equality

```parlang
# Valid: both values are integers
> 1..10
Type: Range
1..10

# Type error: start is boolean
> true..10
Type error: Cannot unify types: Bool and Int

# Type error: end is float
> 1..10.5
Type error: Cannot unify types: Float and Int
```

## Features

### 1. Basic Range Creation

```parlang
# Simple ascending range
let numbers = 1..100

# Descending range
let countdown = 10..1

# Range with zero
let centered = -10..10

# Single value range
let single = 5..5
```

### 2. Ranges with Variables

```parlang
let start = 0
let end = 100
let range = start..end
```

### 3. Ranges in Functions

```parlang
# Function that creates a range
let makeRange = fun start -> fun end -> start..end
let r = makeRange 1 10

# Function returning a range based on parameter
let zeroToN = fun n -> 0..n
let hundred = zeroToN 100

# Function that takes a range
let processRange = fun r -> r
processRange (1..10)
```

### 4. Range Equality

Ranges can be compared for equality:

```parlang
> 1..10 == 1..10
Type: Bool
true

> 1..10 == 2..10
Type: Bool
false

> 1..10 != 5..15
Type: Bool
true
```

Two ranges are equal if both their start and end values are equal.

### 5. Ranges in Conditional Expressions

```parlang
# Range in if-then-else
let r = if condition then 1..10 else 5..15

# Conditional range creation
let range = if x > 0 then 0..x else x..0
```

### 6. Ranges in Let Bindings

```parlang
# Sequential let bindings
let start = 1;
let end = 100;
let range = start..end;
range

# Nested let bindings
let r1 = 1..10 in
  let r2 = 11..20 in
  r2
```

## Operator Precedence

The range operator `..` has specific precedence in ParLang's expression hierarchy:

- **Higher precedence than**: Comparisons (`==`, `!=`, `<`, `>`, etc.), Assignment (`:=`)
- **Lower precedence than**: Addition/Subtraction (`+`, `-`), Multiplication/Division (`*`, `/`)

Examples:

```parlang
# Arithmetic evaluated first
1+2..3+4
# Parses as: (1+2)..(3+4) = 3..7

2*3..4*5
# Parses as: (2*3)..(4*5) = 6..20

# Comparison evaluated after range
1..10 == 1..10
# Parses as: (1..10) == (1..10) = true
```

## Display Format

Ranges are displayed using the `..` notation:

```parlang
> 1..10
1..10

> -5..5
-5..5

> let r = 0..100 in r
0..100
```

## Usage Examples

### Example 1: Creating Different Ranges

```parlang
# Positive range
let positive = 1..100

# Range with negatives
let mixed = -50..50

# Large range
let large = 0..1000000

# Descending
let desc = 100..1
```

### Example 2: Ranges in Functions

```parlang
# Function that creates a centered range around a value
let centered = fun n -> fun radius -> (n - radius)..(n + radius)
let range = centered 50 10  # Creates 40..60

# Function that adjusts range based on condition
let adjustRange = fun r -> fun offset -> 
  if offset > 0 
  then (0..100)
  else (0..50)
```

### Example 3: Range Equality Checks

```parlang
# Check if two computations produce the same range
let r1 = 0..100
let r2 = 0..100
let same = r1 == r2  # true

# Verify range creation
let expected = 5..15
let actual = let x = 10 in (x - 5)..(x + 5)
let correct = expected == actual  # true
```

## Type Inference

ParLang's type system automatically infers range types:

```parlang
# Type inference for range literal
> 1..10
Type: Range

# Type inference through function
> let makeRange = fun a -> fun b -> a..b in makeRange
Type: Int -> Int -> Range

# Type inference in let binding
> let r = 1..10 in r
Type: Range
```

## Implementation Notes

### Internal Representation

- Ranges are stored as a tuple of two integers: `(start, end)`
- Both values are i64 signed integers
- No validation is performed on the ordering (descending ranges are valid)

### Type Checking

- The type checker ensures start and end are both `Int` type
- Range expressions unify with the `Range` type
- Ranges can be compared for equality using `==` and `!=`

## Future Enhancements

Potential future additions to Range functionality:

- **Iteration**: Support for iterating over range values
- **Contains check**: Test if a value is within a range
- **Range arithmetic**: Operations like union, intersection
- **Step parameter**: Ranges with custom step sizes (e.g., `1..10:2` for odd numbers)
- **Conversion to arrays**: Convert a range to an array of values

## See Also

- [Type System](TYPE_SYSTEM.md) - Overview of ParLang's type system
- [Arrays](ARRAYS.md) - Fixed-size array type
- [Type Inference](TYPE_INFERENCE.md) - How type inference works in ParLang
- [Examples](../examples/range.par) - Example code using ranges
