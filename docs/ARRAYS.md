# Arrays

ParLang now supports **fixed-size arrays** - homogeneous collections with compile-time known sizes, optimized for FFI (Foreign Function Interface) use cases.

## Overview

Arrays in ParLang are:
- **Fixed-size**: The size is determined at creation and cannot be changed
- **Homogeneous**: All elements must have the same type
- **Zero-indexed**: Elements are accessed using zero-based indices
- **Type-safe**: Type checking ensures arrays and indices are used correctly

## Syntax

### Array Literals

Arrays are created using the `[| |]` syntax:

```parlang
[|1, 2, 3|]         # Array of integers with size 3
[|true, false|]     # Array of booleans with size 2
[|'a', 'b', 'c'|]   # Array of characters with size 3
[||]                # Empty array with size 0
```

### Array Indexing

Arrays are accessed using bracket notation `[index]`:

```parlang
let arr = [|10, 20, 30|]
in arr[0]           # Result: 10 (first element)

[|100, 200, 300|][2]  # Result: 300 (third element)
```

## Type System

Arrays have the type `Array[T, n]` where:
- `T` is the element type
- `n` is the array size

When type checking is enabled:

```parlang
> [|1, 2, 3|]
Type: Array[Int, 3]
[|1, 2, 3|] (size: 3)

> [|true, false|]
Type: Array[Bool, 2]
[|true, false|] (size: 2)
```

## Features

### 1. Basic Array Operations

**Creating arrays:**
```parlang
let numbers = [|1, 2, 3, 4, 5|]
```

**Accessing elements:**
```parlang
let arr = [|10, 20, 30|]
in arr[1]           # Result: 20
```

**Using expressions in arrays:**
```parlang
[|1 + 1, 2 * 2, 3|]  # Result: [|2, 4, 3|]
```

### 2. Arrays with Let Bindings

```parlang
let x = 10;
let y = 20;
let z = 30;
[|x, y, z|]         # Result: [|10, 20, 30|]
```

### 3. Array Indexing with Expressions

```parlang
let arr = [|100, 200, 300|]
in let index = 1 + 1
in arr[index]       # Result: 300
```

### 4. Nested Arrays

Arrays can contain other arrays:

```parlang
let matrix = [|[|1, 2|], [|3, 4|], [|5, 6|]|]
in matrix[1]        # Result: [|3, 4|]

# Deep indexing
let matrix = [|[|1, 2|], [|3, 4|]|]
in matrix[0][1]     # Result: 2
```

### 5. Arrays in Functions

**Function taking an array:**
```parlang
let getFirst = fun arr -> arr[0]
in getFirst [|42, 43, 44|]    # Result: 42
```

**Function returning an array:**
```parlang
let makeArray = fun x -> fun y -> fun z -> [|x, y, z|]
in makeArray 1 2 3              # Result: [|1, 2, 3|]
```

**Higher-order functions with arrays:**
```parlang
let applyToFirst = fun f -> fun arr -> f (arr[0])
in let double = fun x -> x * 2
in applyToFirst double [|10, 20, 30|]  # Result: 20
```

### 6. Arrays with Pattern Matching

While arrays themselves don't support pattern matching directly, you can use them with other constructs:

```parlang
let processArray = fun arr ->
  match arr[0] with
  | 0 -> 100
  | n -> n * 10
  
in processArray [|5, 10, 15|]  # Result: 50
```

## Error Handling

### Out of Bounds Access

Accessing an index outside the array bounds results in a runtime error:

```parlang
> [|1, 2, 3|][5]
Error: Array index 5 out of bounds for array of size 3
```

### Negative Index

Negative indices are not supported:

```parlang
> [|1, 2, 3|][-1]
Error: Array index -1 is negative
```

### Type Errors

**Non-integer index:**
```parlang
> [|1, 2, 3|][true]
Error: Array index must be an integer
```

**Indexing a non-array:**
```parlang
> 42[0]
Error: Array indexing requires an array
```

**Mixed types in array:**
When type checking is enabled, arrays with mixed types will fail type checking:
```parlang
> [|1, true|]
Type error: Cannot unify types: Int and Bool
```

## Use Cases

### 1. FFI (Foreign Function Interface)

Arrays are designed to be compatible with C/FFI interfaces where fixed-size buffers are common:

```parlang
# Preparing data for FFI call
let buffer = [|0, 0, 0, 0|]  # Fixed-size buffer of 4 integers
```

### 2. Mathematical Operations

```parlang
let vector = [|1, 2, 3|]
in let x = vector[0]
in let y = vector[1]
in let z = vector[2]
in x + y + z         # Result: 6
```

### 3. Lookup Tables

```parlang
let days = [|31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31|]
in days[0]           # Days in January: 31
```

### 4. Multi-dimensional Data

```parlang
let grid = [|
  [|1, 2, 3|],
  [|4, 5, 6|],
  [|7, 8, 9|]
|]
in grid[1][1]        # Result: 5 (center of 3x3 grid)
```

## Performance Considerations

1. **Fixed Size**: Arrays have a fixed size known at creation time
2. **Contiguous Memory**: Array elements are stored contiguously (implementation detail)
3. **Zero-cost Indexing**: Array indexing is a constant-time operation
4. **Bounds Checking**: Runtime bounds checking ensures safety

## Comparison with Other Types

### Arrays vs Tuples

| Feature | Arrays | Tuples |
|---------|--------|--------|
| Element types | Must be homogeneous | Can be heterogeneous |
| Indexing | Dynamic: `arr[i]` | Static: `tuple.0` |
| Size | Fixed at creation | Fixed at creation |
| Syntax | `[|1, 2, 3|]` | `(1, 2, 3)` |

**Example:**
```parlang
let arr = [|1, 2, 3|]      # All elements must be Int
let tup = (1, true, 'a')   # Mixed types allowed

arr[0]                     # Dynamic indexing
tup.0                      # Static projection
```

### Arrays vs Lists (Sum Types)

Lists can be built using sum types (e.g., `Cons`/`Nil`), but arrays are more efficient for:
- Fixed-size data
- Random access (O(1) vs O(n))
- FFI interoperability

## Limitations

1. **No array literals in patterns**: Arrays cannot be deconstructed in pattern matching
2. **Fixed size**: Cannot dynamically resize arrays
3. **No array concatenation**: Built-in concatenation is not provided
4. **No slice operations**: Cannot extract subarrays directly

## Examples

### Example 1: Simple Array Operations

```parlang
let arr = [|10, 20, 30, 40, 50|]
in let first = arr[0]
in let last = arr[4]
in first + last      # Result: 60
```

### Example 2: Array Function Composition

```parlang
let getElement = fun arr -> fun i -> arr[i]
in let arr = [|100, 200, 300|]
in getElement arr 1  # Result: 200
```

### Example 3: Multi-dimensional Arrays

```parlang
let matrix = [|
  [|1, 0, 0|],
  [|0, 1, 0|],
  [|0, 0, 1|]
|]
in matrix[0][0]      # Result: 1 (identity matrix)
```

### Example 4: Array Computation

```parlang
let arr = [|1, 2, 3, 4, 5|]
in let a0 = arr[0]
in let a1 = arr[1]
in let a2 = arr[2]
in let a3 = arr[3]
in let a4 = arr[4]
in a0 + a1 + a2 + a3 + a4  # Result: 15
```

## Best Practices

1. **Use arrays for homogeneous, fixed-size data**
2. **Validate indices before accessing when possible**
3. **Prefer arrays over nested tuples for matrix-like data**
4. **Use arrays for FFI data structures**
5. **Document expected array sizes in function signatures**

## Future Enhancements

Potential future improvements to the array system:
- Array comprehensions
- Built-in array manipulation functions (map, fold, etc.)
- Array slicing operations
- Type-level size constraints
- Multi-dimensional array literals

## See Also

- [Type System](TYPE_SYSTEM.md) - Understanding ParLang types
- [Examples Guide](EXAMPLES.md) - More usage examples
- [Language Specification](LANGUAGE_SPEC.md) - Formal language definition
