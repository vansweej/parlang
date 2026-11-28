# Type Checker Module (`crates/parlang/src/typechecker.rs`)

The Type Checker module implements Hindley-Milner type inference for ParLang.

## Overview

This module implements Algorithm W, the classic type inference algorithm for functional languages. It infers types for most expressions without requiring type annotations and supports polymorphic types through let-polymorphism. Tuples (`Type::Tuple`), strict tuple projection, and match expressions are now type-checked: patterns are typed against the scrutinee and arm bodies are unified. Module loading (`load`) still receives a fresh type variable and is not type-checked; a constructor used without a `data` declaration (including the `Cons`/`Nil` that string literals desugar to) is an `UnknownConstructor` type error; a non-exhaustive `match` is a `NonExhaustiveMatch` type error. Inferred type errors are caught before runtime.

## Core Components

### `TypeEnv` - Type Environment

The type environment maintains variable bindings, generates fresh type variables, and records the declared type aliases, constructors and data types:

```rust
pub struct TypeEnv {
    bindings: HashMap<String, TypeScheme>,
    next_var: usize,
    type_aliases: HashMap<String, Type>,
    constructors: HashMap<String, ConstructorInfo>,
    data_kinds: HashMap<String, Vec<parlang_core::Kind>>,
}
```

`type_aliases` maps alias names to their types, `constructors` maps each constructor name to its type information, and `data_kinds` maps each declared data type to the kinds of its type parameters.

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
    UnificationError(Box<Type>, Box<Type>),
    OccursCheckFailed(TypeVar, Type),
    RecursionRequiresAnnotation,
    FieldNotFound(String, Vec<String>),
    RecordExpected(String),
    RecordFieldMismatch,
    ConstructorArityMismatch(String, usize, usize),
    TypeConstructorArityMismatch(String, usize, usize),
    TypeAliasInDataPayload(String),
    KindMismatch(String, String, String),
    TypeAliasCycle(Vec<String>),
    DuplicateTypeName(String),
    DuplicateConstructor(String),
    TupleIndexOutOfBounds(usize, usize),
    TupleExpected(String),
    UnknownConstructor(String),
    NonExhaustiveMatch { missing: Vec<String> },
}
```

**Error Types:**
- `UnboundVariable(name)`: Variable used before definition
- `UnificationError(t1, t2)`: Cannot make types t1 and t2 equal
- `OccursCheckFailed(var, ty)`: Type variable occurs in the type it's being unified with (would create infinite type)
- `RecursionRequiresAnnotation`: Never produced by the type checker; the variant and its message remain, but recursive functions type-check without annotations
- `FieldNotFound(field, available)`: Field not found in record type; carries the field name and the available fields
- `RecordExpected(found)`: Expected a record type but got something else
- `RecordFieldMismatch`: Record type field mismatch during unification
- `ConstructorArityMismatch(name, expected, actual)`: Constructor applied with the wrong number of arguments
- `TypeConstructorArityMismatch(name, expected, actual)`: Type constructor over-applied, or left unapplied where a plain type is expected, in a data payload or annotation (expected is 0 for base types)
- `KindMismatch(type, expected, found)`: A type whose kind does not fit its position, e.g. `Int` as the `f` of `Wrap f a`; data parameter kinds are inferred per declaration group by `kind_check.rs`
- `TypeAliasInDataPayload(name)`: A type alias was used in a data constructor payload; payload names must be base types, declared data types, or type variables (unknown names fail with `UnboundVariable("Unknown type constructor in data declaration: <name>")`)
- `TypeAliasCycle(names)`: A cycle was found while resolving top-level type aliases
- `DuplicateTypeName(name)`: A top-level data type or type alias was declared more than once
- `DuplicateConstructor(name)`: A top-level constructor name was declared more than once
- `TupleIndexOutOfBounds(index, arity)`: Tuple projection index was out of range for the tuple's arity
- `TupleExpected(found)`: Expected a tuple type but got something else (e.g. an unresolved type variable)
- `UnknownConstructor(name)`: A constructor (in an expression or pattern) has no `data` declaration in scope; string literals need `data List a = Nil | Cons a (List a)`
- `NonExhaustiveMatch { missing }`: a `match` does not cover every value; `missing` lists witness patterns

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
use parlang::{parse_expr, typecheck};

let expr = parse_expr("fun x -> x + 1").unwrap();
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

## Typed AST pipeline

Alongside `typecheck`, three entrypoints produce a typed tree:

- `typecheck_typed`: type checks a single expression and returns the typed expression
- `typecheck_program_typed`: type checks a whole program (declarations plus body)
- `typecheck_program_typed_with_env`: the same, starting from a caller-supplied `TypeEnv`

**Threading design.** The typed tree is built *through* the inference helpers as they run; it is not produced by re-walking the untyped AST afterwards.

**Per-root resolution.** Each root is resolved with `resolve_typed` under the final substitution for that root:
- the expression root;
- each top-level let value, resolved with its own substitution (including any annotation unification);
- the trailing body.

**Always-free cases.** Some nodes keep free type variables:
- the `Load` node carries its body untyped and gets a fresh type variable;
- (constructors without a `data` declaration are rejected with `UnknownConstructor`, so they never reach the typed tree);
- a generalized let-value's internal type variables stay `Var` after root resolution.

**`next_var` sync invariant.** Every inference run on a cloned-env child must `max`-sync `next_var` back into the parent, so variables allocated inside the subtree are never reissued. This is why the syncs in `infer_binop` and `infer_type_alias` were added, and why `infer_app` syncs again after allocating its result variable.

**Unregistered constructors.** A constructor with no `data` declaration is rejected with `UnknownConstructor` before its arguments are inferred, so the constructor name is the reported error even when an argument is itself ill-typed (for example, `Foo undefinedVar` reports `UnknownConstructor("Foo")`, not an unbound variable).

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

**Instantiation** converts type schemes back to types. It is a private helper of `TypeEnv`, used by the public `lookup`; `lookup_instantiated` uses the sibling `instantiate_with_args`, which creates the same fresh variables and also returns them:

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
Equality operators (`==`, `!=`) require both sides to have the same type `t` and emit the wanted constraint `Eq t`.
The built-in `Eq` instances are `Int`, `Bool`, `Char` and `Float`; `Eq` on any other type constructor (tuples, lists and strings, records, functions, unit, data types) is a `NoInstance` error.
A constraint on a type variable is generalized into the binding's scheme (for example `forall t0. Eq t0 => t0 -> t0 -> Bool`). A program body or REPL expression is implicitly generalized, so a residual constraint is accepted when all of its variables occur in the body's type. Any other constraint whose variable is neither generalized nor fixed is an `AmbiguousConstraint` error (Haskell 98: no defaulting).

### Type Classes and Methods

`Eq` is a real built-in class, `class Eq a where { eq : a -> a -> Bool }`, seeded into every `TypeEnv` (`class_info`, `method_info`). The method `eq` has the scheme `forall a. Eq a => a -> a -> Bool`, so `eq 1 2 : Bool` and `let f = fun x -> eq x x` gets `forall t. Eq t => t -> Bool`. A variable resolves to a binding first and falls back to a class method, so any user binding named `eq` (top-level or local `let`, a lambda or a `rec` binder) shadows the method; `==` and `!=` always mean `Eq`.

A user `class C p where { m : ty; ... };` is checked in declaration order, like a `let`: a method is invisible to earlier declarations. The kind of `p` is inferred from all signatures (`class Container f where { empty : f Int }` gives `f : * -> *`). Each method gets the scheme `forall p ... . C p => ty`. Errors: `DuplicateClass` (including redeclaring `Eq`), `DuplicateMethod` (across all classes, including `eq`), `MethodSignatureMissingClassParam`, and `KindMismatch` inside signatures. A constraint `C T` on a concrete `T` needs an instance (see Instances below); without one it is `NoInstance`.

`TypedProgram.method_uses` lists the methods a program uses by name. `typed_ast::is_class_program` is true for a program with a class or instance declaration or a method use. Such programs run through dictionary passing (`docs/adr/0014-dictionary-passing.md`). Combined with `load`, `typed_ast::class_load_conflict` rejects them with the same text on both evaluators, both REPLs and the driver, for example `type classes cannot be combined with load yet: method eq`.

### Instances

`instance [ctx =>] C head where { m = expr; ... };` is checked in declaration order (Haskell 98 strict). An instance is visible only to later declarations and the body.

- Class: `C` must exist (`UnknownClass`).
- Head: a base type or a data type, bare or applied to distinct type variables (`InvalidInstanceHead`); never an alias (`TypeSynonymInstance`). Its kind must equal the class parameter's kind (`KindMismatch`; for example `instance Functor Option` for a `* -> *` class).
- Context: names known classes on head variables only (`InvalidInstanceContext`).
- Overlap: an instance whose head overlaps an earlier one of the same class, built-ins included, is `DuplicateInstance` (`instance Eq Int`).
- Methods: every class method is defined exactly once (`MissingInstanceMethod`, `UnknownInstanceMethod`, `DuplicateInstanceMethod`). There are no default methods.

Each body is checked against the method scheme with the class parameter replaced by the head. Head variables and the scheme's other variables must stay distinct and unconstrained (`InstanceMethodTooSpecific`). Constraints wanted by the body must follow from the context; a missing context constraint is `NoInstance`. The instance is registered before its bodies are checked, so recursive uses resolve.

Context reduction matches instance heads one way and reduces the matched instance's context through a worklist.

Known limitations:
- Superclass entailment comes in M2.5.
- A program body whose constructor-class variable stays residual runs only
  on the surface evaluator (see ADR 0014).
- Execution limitations of instances are listed in ADR 0014.

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

### Recursive Functions

```rust
Expr::Rec(name, body) =>
    create fresh type variable t_rec for the recursive function
    extend environment with name:t_rec
    infer body in extended environment => t_body
    apply accumulated substitutions to t_rec
    unify t_rec with t_body
    return t_body
```

Recursive functions are supported through **fixpoint typing**:
1. A fresh type variable is created for the recursive function
2. The function name is bound to this type in the environment before checking the body
3. The body is type-checked with the recursive name available
4. The inferred type is unified with the assumed type

This allows the type checker to correctly handle self-references in recursive functions like factorial, fibonacci, etc.

**Example:**
```rust
// rec f -> fun n -> if n == 0 then 1 else n * f (n - 1)
// Type: Int -> Int
```

**Limitations:**
- Recursive functions that would create infinite types (e.g., certain curried recursive patterns) may fail the occurs check
- Recursive functions are monomorphic (not generalized like let-bound functions)


## Usage Examples

### Basic Type Inference

```rust
use parlang::{parse_expr, typecheck, Type};

// Integer literal
let expr = parse_expr("42").unwrap();
assert_eq!(typecheck(&expr).unwrap(), Type::Int);

// Function
let expr = parse_expr("fun x -> x + 1").unwrap();
let ty = typecheck(&expr).unwrap();
// ty is Int -> Int
```

### Polymorphic Functions

```rust
// Identity function
let expr = parse_expr("fun x -> x").unwrap();
let ty = typecheck(&expr).unwrap();
// ty is t0 -> t0 (polymorphic)

// Using polymorphic function at different types
let expr = parse_expr("let id = fun x -> x in id 42").unwrap();
assert_eq!(typecheck(&expr).unwrap(), Type::Int);

let expr = parse_expr("let id = fun x -> x in id true").unwrap();
assert_eq!(typecheck(&expr).unwrap(), Type::Bool);
```

### Type Errors

```rust
// Type mismatch
let expr = parse_expr("1 + true").unwrap();
assert!(typecheck(&expr).is_err());

// Unbound variable
let expr = parse_expr("x + 1").unwrap();
assert!(typecheck(&expr).is_err());
```

### Higher-Order Functions

```rust
// Function composition
let expr = parse_expr("fun f -> fun g -> fun x -> f (g x)").unwrap();
let ty = typecheck(&expr).unwrap();
// ty is (t2 -> t3) -> (t1 -> t2) -> t1 -> t3
```

### Recursive Functions

```rust
// Factorial function
let expr = parse_expr("rec f -> fun n -> if n == 0 then 1 else n * f (n - 1)").unwrap();
let ty = typecheck(&expr).unwrap();
assert_eq!(ty, Type::Fun(Box::new(Type::Int), Box::new(Type::Int)));

// Fibonacci function
let expr = parse_expr("rec fib -> fun n -> if n == 0 then 0 else if n == 1 then 1 else fib (n - 1) + fib (n - 2)").unwrap();
let ty = typecheck(&expr).unwrap();
// ty is Int -> Int

// Using recursive function in let binding
let expr = parse_expr("let fact = rec f -> fun n -> if n == 0 then 1 else n * f (n - 1) in fact 5").unwrap();
assert_eq!(typecheck(&expr).unwrap(), Type::Int);

// Type error: wrong argument type in recursive call
let expr = parse_expr("rec f -> fun n -> if n == 0 then 1 else f true").unwrap();
assert!(typecheck(&expr).is_err());  // f expects Int but gets Bool
```

## Limitations

### Tuples and Pattern Matching

Tuples, tuple projection, and match expressions are type-checked, and a non-exhaustive match is a `NonExhaustiveMatch` type error. Patterns are typed against the scrutinee and arm bodies are unified into one result type. A constructor without a `data` declaration is an `UnknownConstructor` type error. Remaining leniencies: a record pattern requires an already-known record type; tuple projection requires an already-known tuple type (use a tuple pattern otherwise); and `load` is not type-checked.

Exhaustiveness is checked during inference: `infer_match` calls the sound
checker `check_match` (`match_coverage.rs`), and a non-exhaustive match is
rejected with `TypeError::NonExhaustiveMatch { missing }`.

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
1. **Module Typing**: Type-check `load`ed files instead of assigning a fresh type variable
2. **Better Error Messages**: More context in error messages
3. **Type Classes**: Support for ad-hoc polymorphism
