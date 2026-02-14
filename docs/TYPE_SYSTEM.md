# Type System

ParLang uses a Hindley-Milner type system with automatic type inference.

## Overview

The type system is optional and can be enabled in the REPL using the `PARLANG_TYPECHECK` environment variable. When enabled, all expressions are type-checked before evaluation, catching type errors at compile time rather than runtime.

## Basic Types

ParLang has two basic types:

- **Int**: Integer values (e.g., `42`, `-10`, `0`)
- **Bool**: Boolean values (`true`, `false`)

## Function Types

Functions have types of the form `T1 -> T2`, where `T1` is the argument type and `T2` is the return type.

### Examples

```parlang
fun x -> x + 1
# Type: Int -> Int
# Takes an integer, returns an integer
```

```parlang
fun x -> if x then 1 else 0
# Type: Bool -> Int
# Takes a boolean, returns an integer
```

### Curried Functions

Functions can be curried, creating higher-order functions:

```parlang
fun x -> fun y -> x + y
# Type: Int -> Int -> Int
# Equivalent to: Int -> (Int -> Int)
```

This function takes an integer and returns a function that takes another integer and returns an integer.

## Record Types

Records are product types with named fields, providing structured data with type-safe field access.

### Basic Record Types

```parlang
> { name: 42, age: 30 }
Type: { age: Int, name: Int }
{ name: 42, age: 30 }
```

Record types show all fields and their types. Fields are displayed in alphabetical order for consistency.

### Row Polymorphism

ParLang implements **row polymorphism** for records, allowing functions to work with any record that has **at least** certain fields.

#### Row Variables

A row variable (denoted `r0`, `r1`, etc.) represents "unknown additional fields" in a record:

```parlang
> fun p -> p.age
Type: { age: t0 | r0 } -> t0
```

This type means:
- The function takes a record with at least an `age` field (type `t0`)
- The record may have other fields (represented by `r0`)
- The function returns the value of the `age` field (type `t0`)

#### Benefits of Row Polymorphism

**Flexible, reusable functions:**
```parlang
let getAge = fun r -> r.age
# Works with any record that has an 'age' field

> getAge { name: 42, age: 30 }
Type: Int
30

> getAge { age: 25, city: 100, active: true }
Type: Int
25
```

**Type safety is maintained:**
```parlang
> let getAge = fun r -> r.age
  in let config = { port: 8080 }
  in getAge config
Type error: Field 'age' not found
```

#### Type Variables and Row Variables

Type schemes can quantify both type variables and row variables:

```parlang
> let id = fun x -> x in id
Type: forall t0. t0 -> t0

> let getAge = fun r -> r.age in getAge
Type: forall t0, r0. { age: t0 | r0 } -> t0
```

The `forall t0, r0` means:
- `t0` can be any type (the type of the age field)
- `r0` can be any set of additional fields

#### Closed vs Open Records

- **Closed record**: `{ x: Int, y: Int }` - exactly these fields, no more
- **Open record**: `{ x: Int | r0 }` - at least x field, possibly more

Row polymorphism allows functions to work with open records, providing flexibility while maintaining type safety.

### Record Type Examples

**Simple field access:**
```parlang
> let person = { name: 42, age: 30 } in person.age
Type: Int
30
```

**Polymorphic field access:**
```parlang
> fun r -> r.value
Type: { value: t0 | r0 } -> t0
```

**Multiple field accesses:**
```parlang
> let addXY = fun r -> r.x + r.y
  in addXY { x: 10, y: 20, z: 30 }
Type: Int
30
```

**Nested records:**
```parlang
> let person = { address: { city: 100 } } in person.address
Type: { city: Int }
{ city: 100 }
```

For more details on record types and row polymorphism, see [RECORDS.md](RECORDS.md).

## Type Aliases

Type aliases allow you to define alternative names for existing types, making code more readable and self-documenting.

### Syntax

```parlang
type AliasName = TypeExpression in expression
```

### Examples

**Simple type alias:**
```parlang
> type MyInt = Int in 42
Type: Int
42
```

**Function type alias:**
```parlang
> type IntFunc = Int -> Int in fun x -> x + 1
Type: Int -> Int
<function x>
```

**Complex function type alias:**
```parlang
> type Predicate = Int -> Bool in fun x -> x > 0
Type: Int -> Bool
<function x>
```

**Higher-order function type alias:**
```parlang
> type Transform = (Int -> Int) -> Int in fun f -> f 42
Type: (Int -> Int) -> Int
<function f>
```

### Scoping

Type aliases are scoped to the expression that follows them:

```parlang
type MyInt = Int in
  let x = 10 in
  x + 32
# Result: 42
```

### Nested Type Aliases

You can define multiple type aliases by nesting them:

```parlang
type IntFunc = Int -> Int in
type BoolFunc = Bool -> Bool in
  let f = fun x -> x + 1 in
  f 41
# Result: 42
```

### Transparency

Type aliases are transparent - they don't create new types, just alternative names:

```parlang
> type MyInt = Int in type YourInt = Int in 42
Type: Int
42
```

Both `MyInt` and `YourInt` are just names for `Int` - they're interchangeable.

### Usage Notes

- Type aliases are evaluated at type-checking time, not runtime
- They help document the intent of your code without changing semantics
- Useful for documenting function signatures and complex types
- Type aliases have no runtime overhead

## Type Inference

Types are inferred automatically without annotations. The type checker uses Algorithm W, a variation of the Hindley-Milner algorithm.

### Examples

```parlang
> let double = fun x -> x + x in double 21
Type: Int
42
```

The type checker infers:
1. `x + x` requires `x : Int` (addition works on integers)
2. Therefore `double : Int -> Int`
3. `double 21` applies the function, resulting in `Int`

```parlang
> fun x -> x
Type: t0 -> t0
<function x>
```

The identity function is polymorphic - it can work with any type. The type checker assigns it a type variable `t0` that represents "any type".

## Polymorphic Types

Functions can be polymorphic, meaning they work with multiple types.

### Let-Polymorphism

Let bindings are generalized, allowing polymorphic use:

```parlang
> let id = fun x -> x in id
Type: t0 -> t0
<function x>
```

The `id` function can be used at different types:

```parlang
> let id = fun x -> x in let a = id 42 in let b = id true in b
Type: Bool
true
```

Here, `id` is used both at type `Int` (for `id 42`) and type `Bool` (for `id true`).

### Type Schemes

Internally, the type checker represents polymorphic types as type schemes:

- Monomorphic: `Int`, `Bool`, `Int -> Bool`
- Polymorphic: `forall t0. t0 -> t0` (the identity function)

## Type Checking vs Evaluation

Type checking and evaluation are separate phases:

1. **Type Checking (optional)**: Verifies that the program is well-typed
2. **Evaluation**: Executes the program

When `PARLANG_TYPECHECK` is enabled, type checking happens before evaluation. If type checking fails, evaluation is skipped.

## Type Errors

The type checker provides clear error messages when types don't match:

### Unbound Variable

```parlang
> x + 1
Type error: Unbound variable: x
```

### Type Mismatch

```parlang
> 1 + true
Type error: Cannot unify types: Bool and Int
```

The type checker expected both arguments to `+` to be integers, but found a boolean.

### If Condition Must Be Boolean

```parlang
> if 1 then 2 else 3
Type error: Cannot unify types: Int and Bool
```

The condition in an `if` expression must be a boolean.

### If Branches Must Have Same Type

```parlang
> if true then 1 else false
Type error: Cannot unify types: Int and Bool
```

Both branches of an `if` expression must have the same type.

## Supported Expressions

The type system supports:

- Literals: `42`, `true`, `false`
- Variables: `x`, `foo`
- Binary operations: `+`, `-`, `*`, `/`, `<`, `<=`, `>`, `>=`, `==`, `!=`
- If expressions: `if cond then expr1 else expr2`
- Let bindings: `let x = expr1 in expr2`
- Functions: `fun x -> expr`
- Function application: `f x`
- Type aliases: `type Name = Type in expr`

## Limitations

### Recursive Functions

Recursive functions (using `rec`) are now fully supported by the type checker:

```parlang
> rec factorial -> fun n -> if n == 0 then 1 else n * factorial (n - 1)
Type: Int -> Int
<function factorial>
```

The type checker uses **fixpoint typing** to infer the types of recursive functions. It:
1. Creates a fresh type variable for the recursive function
2. Adds the function name to the environment before checking the body
3. Infers the body type with the recursive name available
4. Unifies the inferred type with the assumed type

**Examples:**

```parlang
# Factorial
> let fact = rec f -> fun n -> if n == 0 then 1 else n * f (n - 1) in fact 5
Type: Int
3628800

# Fibonacci
> rec fib -> fun n -> if n <= 1 then n else fib (n - 1) + fib (n - 2)
Type: Int -> Int
<function fib>

# Type error: wrong argument type
> rec f -> fun n -> if n == 0 then 1 else f true
Type error: Cannot unify types: Bool and Int
```

**Note:** Recursive functions are monomorphic (not generalized like let-bound functions), so a recursive identity function `rec f -> fun x -> x` cannot be used at multiple types within the same scope.

### Tuples and Pattern Matching

The type checker currently assigns type variables to tuples and pattern matching expressions but doesn't fully check them. This is left for future enhancement.

## Usage

### In the REPL

Enable type checking by setting the environment variable:

```bash
export PARLANG_TYPECHECK=1
cargo run
```

Or for a single session:

```bash
PARLANG_TYPECHECK=1 cargo run
```

When enabled, the REPL will display the inferred type before evaluating:

```parlang
> fun x -> x + 1
Type: Int -> Int
<function x>
```

### In Code

Use the `typecheck` function from the library:

```rust
use parlang::{parse, typecheck};

let program = "fun x -> x + 1";
let expr = parse(program).expect("Parse error");
let ty = typecheck(&expr).expect("Type error");
println!("Type: {}", ty); // prints "Type: Int -> Int"
```

## Type System Guarantees

The Hindley-Milner type system provides strong guarantees:

1. **Soundness**: If a program type checks, it won't have type errors at runtime
2. **Completeness**: The algorithm always terminates (no infinite loops in type checking)
3. **Principal Types**: The inferred type is the most general type possible
4. **Decidability**: Type checking always succeeds or fails in finite time

## References

- Damas, Luis; Milner, Robin (1982). "Principal type-schemes for functional programs"
- Pierce, Benjamin C. "Types and Programming Languages" (2002), Chapter 22
- Algorithm W implementation

## Examples

### Simple Arithmetic

```parlang
> 1 + 2 * 3
Type: Int
7
```

### Higher-Order Functions

```parlang
> let apply = fun f -> fun x -> f x in apply (fun n -> n + 1) 41
Type: Int
42
```

### Polymorphic Functions

```parlang
> let compose = fun f -> fun g -> fun x -> f (g x) in compose
Type: t2 -> (t1 -> t2) -> t1 -> t2
<function f>
```

The `compose` function is polymorphic and can compose functions of various types.

### Constant Function

```parlang
> let const = fun x -> fun y -> x in const 42 true
Type: Int
42
```

The `const` function ignores its second argument and returns the first. It's polymorphic: `forall a, b. a -> b -> a`.
