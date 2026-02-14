# Type Inference in ParLang

## Overview

ParLang implements the Hindley-Milner type inference algorithm (also known as Algorithm W), which automatically infers the most general (principal) types for expressions without requiring explicit type annotations. This document provides a detailed explanation of how type inference works in ParLang.

## Core Concepts

### Type Variables

Type variables (written as `t0`, `t1`, `t2`, etc.) represent unknown types that will be determined through inference. They enable polymorphism by allowing a single function to work with multiple types.

```parlang
fun x -> x
# Type: t0 -> t0
# The type variable t0 means "this function works for any type"
```

### Substitutions

A substitution is a mapping from type variables to types. The inference algorithm builds up substitutions as it learns more about the types in an expression.

Example substitution: `{t0 → Int, t1 → Bool}`

### Unification

Unification is the process of finding a substitution that makes two types equal. It's the core operation in type inference.

Examples:
- Unifying `t0` with `Int` gives substitution `{t0 → Int}`
- Unifying `t0 -> t1` with `Int -> Bool` gives `{t0 → Int, t1 → Bool}`
- Unifying `Int` with `Bool` fails with a type error

### Let-Polymorphism

Let-bound variables can have polymorphic types, meaning they can be used at different types in the same scope. This is achieved through **generalization** (introducing type variables) and **instantiation** (replacing type variables with fresh ones).

```parlang
let id = fun x -> x in
let a = id 42 in
let b = id true in
b
# The id function is used at both Int and Bool types
```

## The Algorithm W Implementation

### Step-by-Step Process

The inference algorithm (`infer`) takes an expression and a type environment, and returns:
1. An inferred type for the expression
2. A substitution representing type constraints discovered

Here's how it works for different expression types:

#### 1. Literals

Literals have concrete types:
```rust
Int(n) => (Type::Int, empty_substitution)
Bool(b) => (Type::Bool, empty_substitution)
```

#### 2. Variables

Variables are looked up in the environment. If the variable has a polymorphic type scheme (from a let-binding), it's **instantiated** with fresh type variables:

```parlang
let id = fun x -> x in id 42
# Step 1: id has scheme ∀t0. t0 -> t0
# Step 2: Instantiate to fresh variables: t1 -> t1
# Step 3: Unify (t1 -> t1) with (t2 -> Int) where t2 is the type of 42
# Result: t1 = Int, so id 42 has type Int
```

#### 3. Functions (Lambda Abstractions)

For `fun x -> body`:
1. Create a fresh type variable `tₓ` for parameter `x`
2. Add `x: tₓ` to the environment
3. Infer the type of `body` to get `tbody` and substitution `s`
4. Result type: `s(tₓ) -> tbody`

Example:
```parlang
fun x -> x + 1
# Step 1: Assign x type variable t0
# Step 2: Infer x + 1
#   - x has type t0
#   - 1 has type Int
#   - + requires both operands to be Int
#   - Unify t0 with Int gives {t0 → Int}
# Result: Int -> Int
```

#### 4. Function Application

For `f arg`:
1. Infer type of `f` to get `tf` and substitution `s1`
2. Apply `s1` to environment and infer type of `arg` to get `targ` and `s2`
3. Create fresh type variable `tresult` for the result
4. Unify `s2(s1(tf))` with `s2(targ) -> tresult`
5. Result: `s3(tresult)` where `s3` is the unification result

Example:
```parlang
(fun x -> x + 1) 42
# Step 1: Infer (fun x -> x + 1) gives Int -> Int
# Step 2: Infer 42 gives Int
# Step 3: Create fresh variable t0 for result
# Step 4: Unify (Int -> Int) with (Int -> t0)
# Step 5: Result t0 = Int
```

#### 5. Let-Bindings

For `let x = value in body`:
1. Infer type of `value` to get `tvalue` and substitution `s1`
2. Apply `s1` to environment
3. **Generalize** `tvalue` to create a polymorphic type scheme
   - Find all free type variables in `tvalue` not in the environment
   - Create scheme `∀t1 t2 ... tn. tvalue`
4. Add `x: scheme` to environment
5. Infer type of `body` with extended environment

**Generalization Example:**
```parlang
let id = fun x -> x in body
# Step 1: Infer (fun x -> x) gives t0 -> t0
# Step 2: Generalize: no type variables in environment
#         Create scheme ∀t0. t0 -> t0
# Step 3: In body, each use of id gets fresh type variables
```

#### 6. If-Expressions

For `if cond then e1 else e2`:
1. Infer type of `cond` and unify with `Bool`
2. Infer type of `e1`
3. Infer type of `e2`
4. Unify types of `e1` and `e2` (branches must have same type)
5. Return the unified type

#### 7. Binary Operations

For `e1 + e2`:
1. Infer types of `e1` and `e2`
2. Unify both with the expected operand type (Int for arithmetic)
3. Return result type (Int for arithmetic, Bool for comparisons)

## Type Schemes and Polymorphism

### Type Schemes

A type scheme represents a polymorphic type:
```
∀t0 t1 ... tn. Type
```

The type variables `t0, t1, ..., tn` are universally quantified, meaning they can be replaced with any concrete types.

### Generalization

Generalization converts a type into a type scheme by quantifying over free type variables:

```rust
fn generalize(env: &TypeEnvironment, t: &Type) -> TypeScheme {
    let free_in_env = env.free_type_vars();
    let free_in_type = t.free_type_vars();
    let quantified = free_in_type - free_in_env;
    TypeScheme::new(quantified, t.clone())
}
```

Example:
```parlang
# Environment: {}
# Type: t0 -> t0
# Free in env: {}
# Free in type: {t0}
# Generalize to: ∀t0. t0 -> t0
```

### Instantiation

Instantiation replaces quantified type variables with fresh ones:

```rust
fn instantiate(scheme: &TypeScheme, env: &mut TypeEnvironment) -> Type {
    let mut subst = HashMap::new();
    for var in &scheme.vars {
        subst.insert(var.clone(), env.fresh_var());
    }
    apply_subst(&subst, &scheme.ty)
}
```

Example:
```parlang
# Scheme: ∀t0. t0 -> t0
# Instantiate to: t5 -> t5 (where t5 is fresh)
# Next instantiation: t6 -> t6 (where t6 is fresh)
```

## Unification Algorithm

Unification finds a substitution that makes two types equal:

```rust
fn unify(t1: &Type, t2: &Type) -> Result<Substitution, TypeError>
```

### Cases

1. **Type Variable with Any Type**
   ```
   unify(t0, T) = {t0 → T} if t0 ∉ free_vars(T)  (occurs check)
   unify(t0, T) = error if t0 ∈ free_vars(T)     (infinite type)
   ```

2. **Concrete Types**
   ```
   unify(Int, Int) = {}
   unify(Bool, Bool) = {}
   unify(Int, Bool) = error
   ```

3. **Function Types**
   ```
   unify(T1 -> T2, T3 -> T4) = 
     let s1 = unify(T1, T3)
     let s2 = unify(s1(T2), s1(T4))
     compose(s2, s1)
   ```

4. **Record Types**
   ```
   unify({f1: T1, ...}, {f1: T2, ...}) =
     unify each field type and compose substitutions
   ```

5. **Sum Types**
   ```
   unify(Option T1, Option T2) = unify(T1, T2)
   unify(Option T1, List T2) = error (different constructors)
   ```

### Occurs Check

The occurs check prevents infinite types:
```parlang
# Without occurs check, this would create t0 = t0 -> Int
let f = fun x -> x x in ...  # Error: cannot construct infinite type
```

## Generic Types (Sum Types)

Generic types like `Option a`, `List a`, `Either a b` are handled through:

1. **Constructor Registration**: Each constructor is registered with its type parameters
2. **Type Instantiation**: When a constructor is used, type parameters are instantiated with fresh variables
3. **Type Unification**: Type arguments are unified during type checking

Example:
```parlang
type Option a = Some a | None in
let x = Some 42 in x

# Step 1: Register constructors
#   - Some: ∀a. a -> Option a
#   - None: ∀a. Option a
# Step 2: Infer Some 42
#   - Instantiate Some: t0 -> Option t0 (where t0 is fresh)
#   - Infer 42: Int
#   - Unify t0 with Int: {t0 → Int}
#   - Result: Option Int
```

## Error Handling

The type checker can produce several types of errors:

1. **Unification Error**: Types cannot be unified
   ```parlang
   if 1 then 2 else 3
   # Error: Cannot unify Int and Bool
   ```

2. **Unbound Variable**: Variable not in scope
   ```parlang
   x + 1  # Error: Unbound variable: x
   ```

3. **Infinite Type**: Occurs check failure
   ```parlang
   let f = fun x -> x x in f
   # Error: Cannot construct infinite type
   ```

4. **Constructor Arity Mismatch**: Wrong number of arguments
   ```parlang
   type Option a = Some a | None in
   Some 1 2  # Error: Constructor Some expects 1 argument, got 2
   ```

## Performance Characteristics

### Time Complexity

- **Unification**: O(n) where n is the size of the types
- **Inference**: O(n × m) where n is expression size and m is average type size
- **Generalization**: O(k) where k is the number of free type variables

### Space Complexity

- **Environment**: O(v) where v is the number of variables in scope
- **Substitutions**: O(t) where t is the number of type variables
- **Type Representation**: O(d) where d is type depth

### Optimization Opportunities

1. **Environment Cloning**: Currently clones on each scope extension
   - Could use persistent data structures (im-rs crate)
   - Or use reference counting with Rc<HashMap>

2. **Substitution Composition**: Frequently used operation
   - Could cache or use structure sharing
   - Memoization might help in some cases

3. **Free Variable Calculation**: Recalculated multiple times
   - Could be cached or memoized
   - Use incremental computation

## Current Limitations

### 1. Recursive Functions

Recursive functions (using `rec` keyword) are not yet supported in the type checker:

```parlang
rec factorial -> fun n ->
    if n == 0 then 1 else n * factorial (n - 1)
# Error: Recursion requires explicit type annotation
```

**Workaround**: Use Y-combinator or implement fixpoint typing.

**Future**: Add support through:
- Explicit type annotations
- Equi-recursive types
- Or special handling of recursive let-bindings

### 2. Pattern Matching

Pattern matching is partially supported:
- Type checking assigns fresh type variables to patterns
- No exhaustiveness checking
- No redundancy checking

**Future**: Full pattern matching would require:
- Case analysis to ensure all variants are covered
- Detection of unreachable patterns
- Better type refinement in pattern contexts

### 3. Row Polymorphism

Records are handled structurally but without row polymorphism:
```parlang
fun p -> p.age
# Type: {age: Int, ...rest} -> Int  (not currently supported)
# Currently: {age: Int} -> Int (exact match only)
```

**Future**: Implement row types with row variables.

### 4. Type Annotations

Users cannot explicitly annotate types:
```parlang
(42 : Int)  # Not supported
fun (x : Int) -> x + 1  # Not supported
```

**Future**: Add syntax for type annotations to:
- Improve error messages
- Allow early error detection
- Document code better

## Best Practices

### 1. Write Type-Focused Code

Design expressions to make type inference easier:
```parlang
# Good: Clear intent
let is_positive = fun x -> x > 0 in
let result = is_positive 5 in result

# Avoid: Ambiguous intermediate results
let f = fun x -> if x then 1 else 0 in ...
```

### 2. Use Let-Bindings for Polymorphism

Leverage let-polymorphism for generic functions:
```parlang
# Polymorphic identity
let id = fun x -> x in
let a = id 42 in
let b = id true in
(a, b)
```

### 3. Annotate Complex Types (When Supported)

Once type annotations are supported, use them for:
- Public function interfaces
- Complex recursive functions
- Documentation purposes

### 4. Understand Error Messages

Type errors provide information about:
- What types were expected
- What types were found
- Where the unification failed

```parlang
> 1 + true
Type error: Cannot unify types: Bool and Int
# Explanation: + expects Int, but true is Bool
```

## Examples

### Example 1: Polymorphic Identity

```parlang
let id = fun x -> x in
let a = id 42 in
let b = id true in
b
```

Type inference steps:
1. Infer `fun x -> x`: `t0 -> t0`
2. Generalize: `∀t0. t0 -> t0`
3. Instantiate for `id 42`: `t1 -> t1`, unify with `t2 -> Int`
   - Result: `t1 = Int`, so `id 42` has type `Int`
4. Instantiate for `id true`: `t3 -> t3`, unify with `t4 -> Bool`
   - Result: `t3 = Bool`, so `id true` has type `Bool`
5. Final result: `Bool`

### Example 2: Higher-Order Functions

```parlang
let apply = fun f -> fun x -> f x in
let increment = fun x -> x + 1 in
apply increment 41
```

Type inference steps:
1. Infer `fun f -> fun x -> f x`:
   - `f` has type `t0`
   - `x` has type `t1`
   - `f x` requires unifying `t0` with `t1 -> t2`
   - Result: `(t1 -> t2) -> t1 -> t2`
2. Generalize: `∀t1 t2. (t1 -> t2) -> t1 -> t2`
3. Infer `increment`: `Int -> Int`
4. Instantiate `apply` and unify with `(Int -> Int) -> ...`
   - Result: `(Int -> Int) -> Int -> Int`
5. Apply to `increment` and `41`: `Int`

### Example 3: Generic Data Types

```parlang
type Option a = Some a | None in
let map_option = fun f -> fun opt ->
    match opt with
    | Some x -> Some (f x)
    | None -> None
in
let result = map_option (fun x -> x + 1) (Some 41) in
result
```

Type inference steps:
1. Register constructors:
   - `Some: ∀a. a -> Option a`
   - `None: ∀a. Option a`
2. Infer `map_option` body:
   - `f` has type `t0 -> t1`
   - `opt` has type `Option t0`
   - `f x` has type `t1`
   - `Some (f x)` has type `Option t1`
   - Result: `(t0 -> t1) -> Option t0 -> Option t1`
3. Apply to arguments and unify types
4. Final result: `Option Int`

## References

### Academic Papers

1. **Principal Type-Schemes for Functional Programs**  
   Luis Damas and Robin Milner (1982)  
   The original Algorithm W paper

2. **A Theory of Type Polymorphism in Programming**  
   Robin Milner (1978)  
   Foundational work on polymorphic type systems

### Books

1. **Types and Programming Languages**  
   Benjamin C. Pierce  
   Chapter 22: Type Reconstruction

2. **Practical Foundations for Programming Languages**  
   Robert Harper  
   Chapters on type inference

### Online Resources

1. **Write You a Haskell**: Stephen Diehl's tutorial on implementing type inference
2. **Algorithm W Step by Step**: Martin Grabmüller's detailed walkthrough
3. **Hindley-Milner in Rust**: Various implementations on GitHub

## Implementation Notes

The ParLang type checker is implemented in `src/typechecker.rs` with:
- ~700 lines of well-tested code
- 369+ unit tests covering various scenarios
- Clear separation of concerns (unify, infer, generalize, instantiate)
- Comprehensive error handling

Key design decisions:
- Uses HashMap for substitutions (efficient for sparse mappings)
- Immutable type representations (easier to reason about)
- Separate TypeScheme type for polymorphic types
- Occurs check with cycle detection to prevent infinite recursion

See [MODULE_TYPECHECKER.md](MODULE_TYPECHECKER.md) for implementation details.
