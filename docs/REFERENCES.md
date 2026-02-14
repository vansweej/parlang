# Reference/Pointer Types in ParLang

ParLang supports mutable references (also known as pointers) that allow controlled mutation of values. References provide a way to have multiple references to the same value and mutate it through any of those references.

## Table of Contents

- [Overview](#overview)
- [Syntax](#syntax)
- [Type System](#type-system)
- [Examples](#examples)
- [Use Cases](#use-cases)
- [Implementation Details](#implementation-details)
- [Best Practices](#best-practices)

## Overview

References in ParLang are mutable containers that hold a value. They enable:

- **Mutation**: Change values in-place without creating new bindings
- **Aliasing**: Multiple variables can reference the same mutable location
- **State Management**: Build stateful abstractions like counters, accumulators, and caches
- **Closure Capture**: Closures can capture and mutate shared state

### Key Features

- **Type Safety**: References are typed as `Ref T` where `T` is the type of the contained value
- **Interior Mutability**: References can be mutated even when captured in closures
- **Deterministic**: Mutations are sequenced and predictable
- **Memory Safe**: References are managed by Rust's `Rc<RefCell<_>>` ensuring memory safety

## Syntax

### Creating References

Use the `ref` keyword to create a new reference:

```parlang
ref 42          # Create a reference to 42
ref true        # Create a reference to a boolean
ref (1, 2)      # Create a reference to a tuple
```

### Dereferencing

Use the `!` operator to read the value from a reference:

```parlang
let r = ref 10 in
!r              # Returns 10
```

### Assignment

Use the `:=` operator to update the value in a reference:

```parlang
let r = ref 5 in
r := 10         # Update r to hold 10
```

**Note**: Assignment returns the unit value `()`.

### Complete Example

```parlang
let r = ref 0 in        # Create a reference to 0
let dummy = r := 5 in   # Update to 5 (returns unit)
!r                      # Read the value: 5
```

## Type System

### Reference Types

A reference to a value of type `T` has type `Ref T`:

```parlang
ref 42          # Type: Ref Int
ref true        # Type: Ref Bool
ref (1, 2)      # Type: Ref (Int, Int)
```

### Type Checking Rules

1. **Reference Creation**: `ref expr` has type `Ref T` if `expr` has type `T`
2. **Dereferencing**: `!ref_expr` has type `T` if `ref_expr` has type `Ref T`
3. **Assignment**: `ref_expr := value_expr` has type `()` if:
   - `ref_expr` has type `Ref T`
   - `value_expr` has type `T`

### Polymorphic References

References work with polymorphic types:

```parlang
fun x -> ref x          # Type: t0 -> Ref t0
```

### Type Errors

The type checker catches reference-related errors:

```parlang
let r = ref 10 in
r := true               # Type error: Cannot unify Bool and Int

!42                     # Type error: Dereference requires a reference

42 := 10                # Type error: Assignment requires a reference
```

## Examples

### Basic Counter

A simple counter using a reference:

```parlang
let counter = ref 0 in
let increment = fun x -> counter := !counter + 1 in
let dummy1 = increment 0 in
let dummy2 = increment 0 in
let dummy3 = increment 0 in
!counter                # Returns 3
```

### Swap Function

Swap the values of two references:

```parlang
let swap = fun r1 -> fun r2 ->
    let temp = !r1 in
    let dummy = r1 := !r2 in
    r2 := temp
in
let x = ref 10 in
let y = ref 20 in
let dummy = swap x y in
(!x, !y)                # Returns (20, 10)
```

### Accumulator

Build an accumulator using closure-captured references:

```parlang
let accumulator = fun initial ->
    let sum = ref initial in
    fun x -> (
        let dummy = sum := !sum + x in
        !sum
    )
in
let acc = accumulator 0 in
let a = acc 5 in        # Returns 5
let b = acc 10 in       # Returns 15
let c = acc 15 in       # Returns 30
c
```

### Counter Factory

Create independent counters:

```parlang
let make_counter = fun initial ->
    let count = ref initial in
    fun x -> (
        let old = !count in
        let dummy = count := old + 1 in
        old
    )
in
let counter1 = make_counter 0 in
let counter2 = make_counter 100 in
let a = counter1 0 in   # Returns 0
let b = counter1 0 in   # Returns 1
let c = counter2 0 in   # Returns 100
let d = counter2 0 in   # Returns 101
(b, d)                  # Returns (1, 101)
```

### Conditional Mutation

Mutate references based on conditions:

```parlang
let r = ref 5 in
let dummy = if !r > 0 
    then r := !r * 2 
    else r := 0 
in
!r                      # Returns 10
```

### Recursive Countdown

Use references with recursive functions:

```parlang
let count_down = rec f -> fun r ->
    if !r == 0
    then 0
    else (
        let dummy = r := !r - 1 in
        1 + f r
    )
in
let counter = ref 5 in
count_down counter      # Returns 5
```

### Reference Aliasing

Multiple variables can reference the same location:

```parlang
let r = ref 5 in
let alias = r in
let dummy = r := 10 in
!alias                  # Returns 10 (both refer to same location)
```

### References in Records

Store references in data structures:

```parlang
let state = { count: ref 0, active: ref true } in
let dummy = state.count := 5 in
let dummy = state.active := false in
(!state.count, !state.active)   # Returns (5, false)
```

### Higher-Order Functions with References

Pass references to functions:

```parlang
let increment = fun r -> r := !r + 1 in
let decrement = fun r -> r := !r - 1 in
let apply_twice = fun f -> fun r -> (
    let dummy = f r in
    f r
) in
let counter = ref 10 in
let dummy = apply_twice increment counter in
!counter                # Returns 12
```

## Use Cases

### 1. State Management

References enable building stateful computations:

```parlang
let cache = ref [] in       # Hypothetical list
# Build memoization, caching, or state machines
```

### 2. Mutable Data Structures

Implement mutable data structures:

```parlang
# Mutable stack
let stack = ref [] in
let push = fun item -> stack := Cons item !stack in
let pop = fun x -> (
    match !stack with
    | Nil -> None
    | Cons head tail -> (
        let dummy = stack := tail in
        Some head
    )
) in
# ...
```

### 3. Counters and Accumulators

Track state across function calls:

```parlang
let stats = { calls: ref 0, errors: ref 0 } in
let record_call = fun x -> stats.calls := !stats.calls + 1 in
let record_error = fun x -> stats.errors := !stats.errors + 1 in
# ...
```

### 4. Simulation and Game State

Model mutable game state:

```parlang
let player = { 
    health: ref 100, 
    score: ref 0,
    position: ref (0, 0)
} in
let damage = fun amount -> player.health := !player.health - amount in
let move_to = fun pos -> player.position := pos in
# ...
```

## Implementation Details

### Internal Representation

References are implemented using Rust's `Rc<RefCell<Value>>`:

```rust
pub enum Value {
    // ... other variants
    Reference(usize, Rc<RefCell<Value>>),
}
```

- **`Rc`**: Reference counting for memory management
- **`RefCell`**: Interior mutability at runtime
- **`usize`**: Unique reference ID for display purposes

### Memory Management

- References are automatically managed by Rust's ownership system
- Multiple ParLang variables can reference the same `Rc<RefCell<_>>`
- Memory is freed when the last reference goes out of scope
- No manual memory management required

### Type Checking

References are type-checked at compile time:

```rust
pub enum Type {
    // ... other variants
    Ref(Box<Type>),
}
```

The type checker ensures:
- Type safety for all reference operations
- Proper type unification with references
- Polymorphic reference support

### Evaluation

Reference operations are evaluated as follows:

1. **Creation** (`ref expr`):
   - Evaluate `expr` to get a value
   - Wrap in `Rc::new(RefCell::new(value))`
   - Generate unique ID

2. **Dereferencing** (`!ref_expr`):
   - Evaluate `ref_expr` to get a `Reference` value
   - Call `cell.borrow().clone()` to read the value

3. **Assignment** (`ref_expr := value_expr`):
   - Evaluate `ref_expr` to get a `Reference` value
   - Evaluate `value_expr` to get the new value
   - Call `*cell.borrow_mut() = new_value`
   - Return unit value `()`

## Best Practices

### 1. Use References Sparingly

Prefer immutable values and pure functions when possible. Use references only when mutation is necessary:

```parlang
# Good: Pure functional style
let sum = fun list -> rec go -> fun acc -> fun xs ->
    match xs with
    | Nil -> acc
    | Cons x rest -> go (acc + x) rest
in go 0 list

# Use references when needed for state
let counter = ref 0 in
# ...
```

### 2. Avoid Complex Aliasing

Keep reference aliasing simple and local:

```parlang
# Good: Clear ownership
let r = ref 10 in
let dummy = r := 20 in
!r

# Avoid: Complex sharing across many functions
# (unless that's the intent)
```

### 3. Document Mutation

Make it clear when functions mutate their arguments:

```parlang
# increment: Ref Int -> ()
# Increments the counter by 1
let increment = fun counter ->
    counter := !counter + 1
```

### 4. Use Descriptive Names

Name references to indicate they're mutable:

```parlang
let counter_ref = ref 0 in      # Good
let count = ref 0 in            # Also fine
let x = ref 0 in                # Less clear
```

### 5. Handle Assignment Results

Remember that assignment returns unit:

```parlang
# Good: Bind assignment result
let dummy = r := 10 in
!r

# Also works: Use in sequence
let dummy1 = r := 10 in
let dummy2 = r := 20 in
!r
```

### 6. Combine with Pattern Matching

Use references with pattern matching for complex state:

```parlang
type State = Ready | Running | Done in
let state = ref Ready in
let start = fun x -> state := Running in
let stop = fun x -> state := Done in
let is_running = fun x ->
    match !state with
    | Running -> true
    | dummy -> false
in
# ...
```

## Limitations

### No Weak References

ParLang does not support weak references or cycles. All references use `Rc` which can create memory leaks if circular references are created:

```parlang
# Potential issue: Circular references
# (Not easily expressible in current ParLang, but be aware)
```

### No Reference Equality

You cannot test if two references point to the same location:

```parlang
let r1 = ref 10 in
let r2 = ref 10 in
# No way to test if r1 and r2 are the same reference
```

### No Null or Optional References

References always point to a valid value. Use sum types for optional values:

```parlang
type RefOption a = RefSome (Ref a) | RefNone in
# ...
```

## Comparison with Other Languages

### ML/OCaml

ParLang references are similar to OCaml's `ref` type:

```ocaml
(* OCaml *)
let r = ref 0 in
r := 10;
!r

(* ParLang *)
let r = ref 0 in
let dummy = r := 10 in
!r
```

### Rust

ParLang references map to Rust's `Rc<RefCell<T>>`:

```rust
// Rust
let r = Rc::new(RefCell::new(0));
*r.borrow_mut() = 10;
*r.borrow()

// ParLang
let r = ref 0 in
let dummy = r := 10 in
!r
```

### JavaScript

ParLang references provide controlled mutation similar to JavaScript objects:

```javascript
// JavaScript
let obj = { value: 0 };
obj.value = 10;
obj.value

// ParLang (conceptually similar)
let r = ref 0 in
let dummy = r := 10 in
!r
```

## See Also

- [Type System Documentation](TYPE_SYSTEM.md) - Understand the type system
- [Examples Guide](EXAMPLES.md) - More comprehensive examples
- [API Reference](API_REFERENCE.md) - API documentation
