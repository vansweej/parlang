# Type Checker Module (`src/typechecker.rs`)

The Type Checker module implements Hindley-Milner type inference for ParLang.

## Overview

This module provides a complete implementation of Algorithm W, the classic type inference algorithm for functional languages. It automatically infers types for all expressions without requiring type annotations, supports polymorphic types through let-polymorphism, and catches type errors before runtime.

## Core Components

### `TypeEnv` - Type Environment

The type environment maintains variable bindings and generates fresh type variables:

```rust
pub struct TypeEnv {
    bindings: HashMap<String, TypeScheme>,
    next_var: usize,
}
```

**Key Methods:**

- `new() -> Self`: Creates a new empty environment
- `fresh_var(&mut self) -> Type`: Generates a fresh type variable
- `lookup(&mut self, name: &str) -> Option<Type>`: Looks up a variable and instantiates its type scheme
- `bind(&mut self, name: String, scheme: TypeScheme)`: Binds a variable to a type scheme
- `extend(&self, name: String, ty: Type) -> Self`: Creates a new environment with an additional monomorphic binding
- `generalize(&self, ty: &Type) -> TypeScheme`: Generalizes a type by quantifying free type variables

### `TypeError` - Type Errors

Represents different kinds of type errors:

```rust
pub enum TypeError {
    UnboundVariable(String),
    UnificationError(Type, Type),
    OccursCheckFailed(TypeVar, Type),
    RecursionRequiresAnnotation,
}
```

**Error Types:**
- `UnboundVariable(name)`: Variable used before definition
- `UnificationError(t1, t2)`: Cannot make types t1 and t2 equal
- `OccursCheckFailed(var, ty)`: Type variable occurs in the type it's being unified with (would create infinite type)
- `RecursionRequiresAnnotation`: Recursive functions not yet supported

### `Substitution` Type

Maps type variables to types:

```rust
type Substitution = HashMap<TypeVar, Type>;
```

Substitutions are used throughout the algorithm to record what we've learned about type variables.

## Main Functions

### `typecheck(expr: &Expr) -> Result<Type, TypeError>`

Public API for type checking an expression:

```rust
pub fn typecheck(expr: &Expr) -> Result<Type, TypeError>
```

**Parameters:**
- `expr`: The expression to type check

**Returns:**
- `Ok(Type)`: The inferred type
- `Err(TypeError)`: A type error with details

**Example:**
```rust
use parlang::{parse, typecheck};

let expr = parse("fun x -> x + 1").unwrap();
let ty = typecheck(&expr).unwrap();
println!("{}", ty); // "Int -> Int"
```

### `infer(expr: &Expr, env: &mut TypeEnv) -> Result<(Type, Substitution), TypeError>`

Core type inference function implementing Algorithm W:

```rust
pub fn infer(expr: &Expr, env: &mut TypeEnv) 
    -> Result<(Type, Substitution), TypeError>
```

**Parameters:**
- `expr`: The expression to infer types for
- `env`: The type environment containing variable bindings

**Returns:**
- `Ok((Type, Substitution))`: The inferred type and accumulated substitutions
- `Err(TypeError)`: A type error

This function recursively infers types for subexpressions and unifies constraints.

## Algorithm Details

### Unification

Unification makes two types equal by finding a substitution:

```rust
fn unify(t1: &Type, t2: &Type) -> Result<Substitution, TypeError>
```

**Rules:**
- `Int` unifies with `Int`, `Bool` with `Bool`
- Type variables unify with any type (with occurs check)
- Function types `T1 -> T2` unify with `T3 -> T4` if T1 unifies with T3 and T2 unifies with T4

**Occurs Check:**
The occurs check prevents creating infinite types like `t0 = t0 -> Int` by ensuring a type variable doesn't occur in the type it's being bound to.

### Substitution

Substitutions are applied to types and composed:

```rust
fn apply_subst(subst: &Substitution, ty: &Type) -> Type
fn compose_subst(s1: &Substitution, s2: &Substitution) -> Substitution
```

**Cycle Detection:**
The implementation includes cycle detection to prevent stack overflow when applying substitutions with circular dependencies.

### Generalization and Instantiation

**Generalization** converts monomorphic types to polymorphic type schemes:

```rust
env.generalize(&ty) -> TypeScheme
```

This quantifies over type variables that are free in the type but not free in the environment, implementing let-polymorphism.

**Instantiation** converts type schemes back to types:

```rust
env.instantiate(&scheme) -> Type
```

This replaces quantified variables with fresh type variables.

## Type Inference Rules

### Literals

```rust
Expr::Int(_)  => Type::Int
Expr::Bool(_) => Type::Bool
```

### Variables

```rust
Expr::Var(name) => lookup name in environment and instantiate
```

### Binary Operations

```rust
Expr::BinOp(op, left, right) =>
    infer left => t1
    infer right => t2
    unify t1 with expected_arg_type
    unify t2 with expected_arg_type
    return expected_result_type
```

Arithmetic operators (`+`, `-`, `*`, `/`) require `Int` arguments and return `Int`.
Comparison operators (`<`, `<=`, `>`, `>=`) require `Int` arguments and return `Bool`.
Equality operators (`==`, `!=`) work on any type but both sides must match.

### If Expressions

```rust
Expr::If(cond, then_br, else_br) =>
    infer cond => t_cond
    unify t_cond with Bool
    infer then_br => t_then
    infer else_br => t_else
    unify t_then with t_else
    return t_then
```

### Let Expressions

```rust
Expr::Let(name, value, body) =>
    infer value => t_value
    generalize t_value => scheme
    bind name to scheme in extended environment
    infer body in extended environment => t_body
    return t_body
```

Let expressions support polymorphism through generalization.

### Functions

```rust
Expr::Fun(param, body) =>
    create fresh type variable t_param
    infer body with param:t_param in environment => t_body
    return Fun(t_param, t_body)
```

### Function Application

```rust
Expr::App(func, arg) =>
    infer func => t_func
    infer arg => t_arg
    create fresh type variable t_result
    unify t_func with Fun(t_arg, t_result)
    return t_result
```

## Usage Examples

### Basic Type Inference

```rust
use parlang::{parse, typecheck, Type};

// Integer literal
let expr = parse("42").unwrap();
assert_eq!(typecheck(&expr).unwrap(), Type::Int);

// Function
let expr = parse("fun x -> x + 1").unwrap();
let ty = typecheck(&expr).unwrap();
// ty is Int -> Int
```

### Polymorphic Functions

```rust
// Identity function
let expr = parse("fun x -> x").unwrap();
let ty = typecheck(&expr).unwrap();
// ty is t0 -> t0 (polymorphic)

// Using polymorphic function at different types
let expr = parse("let id = fun x -> x in id 42").unwrap();
assert_eq!(typecheck(&expr).unwrap(), Type::Int);

let expr = parse("let id = fun x -> x in id true").unwrap();
assert_eq!(typecheck(&expr).unwrap(), Type::Bool);
```

### Type Errors

```rust
// Type mismatch
let expr = parse("1 + true").unwrap();
assert!(typecheck(&expr).is_err());

// Unbound variable
let expr = parse("x + 1").unwrap();
assert!(typecheck(&expr).is_err());
```

### Higher-Order Functions

```rust
// Function composition
let expr = parse("fun f -> fun g -> fun x -> f (g x)").unwrap();
let ty = typecheck(&expr).unwrap();
// ty is (t2 -> t3) -> (t1 -> t2) -> t1 -> t3
```

## Limitations

### Recursive Functions

Recursive functions using `rec` are not yet supported:

```rust
let expr = parse("rec f -> fun n -> if n == 0 then 1 else n * f (n - 1)").unwrap();
assert!(typecheck(&expr).is_err());
```

This would require fixpoint typing or explicit type annotations.

### Tuples and Pattern Matching

Tuples and pattern matching currently receive type variables but are not fully type-checked. Full support would require:
- Tuple types (e.g., `(Int, Bool)`)
- Pattern type checking
- Exhaustiveness checking

## Testing

The module includes comprehensive tests:

### Unit Tests

Run with:
```bash
cargo test typechecker::tests
```

Tests cover:
- Literal inference
- Arithmetic and comparison operators
- If expressions
- Functions and application
- Let-polymorphism
- Currying and higher-order functions
- Type errors
- Edge cases

### Integration Tests

Run with:
```bash
cargo test --test type_inference_tests
```

Integration tests verify the complete type checking pipeline with parsed expressions.

## Performance Considerations

- **Cycle Detection**: Substitution application includes cycle detection to prevent infinite loops
- **Environment Cloning**: The implementation clones environments for each scope. For production use, persistent data structures could improve performance
- **Substitution Composition**: Substitutions are composed frequently. Optimization opportunities exist here

## Error Messages

The type checker provides clear, actionable error messages:

```
Unbound variable: x
Cannot unify types: Int and Bool
Occurs check failed: t0 occurs in t0 -> Int
Recursive functions require type annotations
```

## Related Modules

- **[Types Module](MODULE_TYPES.md)**: Type representations
- **[AST Module](MODULE_AST.md)**: Expression types being checked
- **[Parser Module](MODULE_PARSER.md)**: Parses expressions before type checking

## References

- Damas, Luis; Milner, Robin (1982). "Principal type-schemes for functional programs"
- Pierce, Benjamin C. "Types and Programming Languages" (2002), Chapter 22
- Algorithm W: A classic type inference algorithm

## Future Enhancements

Potential improvements:
1. **Recursive Functions**: Support for `rec` with fixpoint types
2. **Tuple Types**: Full type checking for tuples
3. **Pattern Exhaustiveness**: Check that pattern matching covers all cases
4. **Type Annotations**: Allow optional type annotations
5. **Better Error Messages**: More context in error messages
6. **Type Classes**: Support for ad-hoc polymorphism
