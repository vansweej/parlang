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

Recursive functions (using `rec`) are not yet supported by the type checker:

```parlang
> rec factorial -> fun n -> if n == 0 then 1 else n * factorial (n - 1)
Type error: Recursive functions require type annotations
```

This is a known limitation. Full support for recursive functions would require fixpoint typing or explicit type annotations.

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

## Known Limitations

While ParLang's type system is robust and well-implemented, there are some known limitations:

### 1. Recursive Functions Not Supported

The type checker cannot currently infer types for recursive functions defined with the `rec` keyword:

```parlang
rec factorial -> fun n ->
    if n == 0 then 1 else n * factorial (n - 1)
# Error: RecursionRequiresAnnotation
```

**Workaround**: You can still evaluate recursive functions by disabling type checking (don't set `PARLANG_TYPECHECK=1`).

**Why**: Typing recursive functions requires either:
- Fixpoint types (fix: (a -> a) -> a)
- Explicit type annotations
- Or special handling of recursive let-bindings

**Future Plans**: Add support through explicit type annotations or automatic fixpoint typing.

### 2. Pattern Matching Type Checking is Incomplete

While pattern matching works at runtime, the type checker has limited support:
- Pattern types are assigned fresh type variables
- No exhaustiveness checking (won't warn about missing cases)
- No redundancy checking (won't warn about unreachable patterns)

```parlang
# This typechecks but might fail at runtime if value is 0
match value with
| 1 -> "one"
| 2 -> "two"
# Missing case for other values - no warning
```

**Future Plans**: Implement full case analysis with exhaustiveness and redundancy checking.

### 3. No Row Polymorphism for Records

Record field access requires exact type matches:

```parlang
fun p -> p.age
# Current type: {age: Int} -> Int (exact match only)
# Desired type: {age: Int, ...rest} -> Int (row polymorphism)
```

This means a function expecting `{age: Int}` won't accept `{age: Int, name: String}`.

**Future Plans**: Implement row polymorphism with row type variables.

### 4. No Type Annotations

Users cannot explicitly specify types:

```parlang
# These are not supported:
(42 : Int)                    # Type annotation on expression
fun (x : Int) -> x + 1        # Type annotation on parameter
let (f : Int -> Int) = ...    # Type annotation on binding
```

**Benefits of Type Annotations**:
- Better error messages (errors at annotation, not at use site)
- Documentation in code
- Early error detection
- Can guide inference in ambiguous cases

**Future Plans**: Add syntax for optional type annotations.

### 5. Performance Considerations

The type checker is efficient for typical programs but has some characteristics to be aware of:

- **Environment Cloning**: Each scope creates a new environment through cloning (O(n) where n = number of bindings)
- **Deep Type Trees**: Deeply nested generic types may slow down inference
- **Large Programs**: Type inference is roughly O(expression_size × average_type_size)

**For typical ParLang programs**, these are not issues. For very large programs, consider:
- Breaking into smaller modules
- Using simpler type structures where possible

## Advanced Topics

### Type Inference Algorithm

ParLang uses Algorithm W, a constraint-based type inference algorithm. For a detailed explanation of how the algorithm works, see [TYPE_INFERENCE.md](TYPE_INFERENCE.md).

Key concepts:
- **Unification**: Finding substitutions to make types equal
- **Generalization**: Creating polymorphic type schemes
- **Instantiation**: Using polymorphic types with fresh variables
- **Let-Polymorphism**: Enabling polymorphic let-bound variables

### Debugging Type Errors

When you encounter a type error, the error message will tell you:
1. What type was expected
2. What type was actually found
3. Where the types couldn't be unified

**Example**:
```parlang
> if 1 then 2 else 3
Type error: Cannot unify types: Int and Bool
```

**Explanation**: The `if` expression expected a `Bool` condition, but received `Int` (the value `1`).

**Tips**:
1. Work backwards from the error location
2. Check types of intermediate expressions
3. Use let-bindings to break complex expressions into parts
4. Remember that type errors may appear far from the actual mistake

### Type System Guarantees

The Hindley-Milner type system provides strong guarantees:

1. **Soundness**: Well-typed programs won't have type errors at runtime
   - If an expression has type `T`, evaluation will produce a value of type `T` or diverge
   - Type errors are caught before execution

2. **Principal Types**: Every typeable expression has a most general type
   - The inferred type is the most general type possible
   - All other valid types are instances of the principal type

3. **Type Safety**: No runtime type confusion
   - Can't add a boolean to an integer
   - Can't apply a non-function
   - Records can't access non-existent fields

4. **Let-Polymorphism**: Flexible reuse of functions
   - Functions can work at multiple types in the same program
   - No need to duplicate code for different types

### Further Reading

For more information about ParLang's type system:

- **[TYPE_INFERENCE.md](TYPE_INFERENCE.md)** - Detailed explanation of the type inference algorithm
- **[MODULE_TYPECHECKER.md](MODULE_TYPECHECKER.md)** - Implementation details of the type checker
- **[GENERIC_TYPES.md](GENERIC_TYPES.md)** - Generic/parameterized types and sum types
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Overall system architecture including type checking

Academic references:
- **Principal Type-Schemes for Functional Programs** (Damas & Milner, 1982)
- **A Theory of Type Polymorphism in Programming** (Milner, 1978)
- **Types and Programming Languages** (Pierce, 2002)
