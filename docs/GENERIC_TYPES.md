# Generic Types

ParLang now supports **generic types** (also known as parameterized types) in its type system. This allows you to define and use type-safe generic data structures like `Option<T>`, `List<T>`, and `Either<A, B>`.

## Overview

Generic types enable you to write type-safe code that works with multiple types without sacrificing type checking. The type checker now fully supports:

- **Type parameters** (e.g., `a`, `b` in `Option a`)
- **Type instantiation** (e.g., `Option Int`, `List Bool`)
- **Type inference** for generic constructors
- **Type unification** for generic types
- **Nested generics** (e.g., `Option (List Int)`)

## Defining Generic Types

You can define generic types using the `type` keyword with type parameters:

```parlang
type Option a = Some a | None

type Either a b = Left a | Right b

type List a = Nil | Cons a (List a)
```

### Type Parameters

Type parameters (like `a`, `b`) are placeholders for actual types. They make your type definitions polymorphic, meaning they can work with any type.

- **Single parameter**: `Option a`, `List a`
- **Multiple parameters**: `Either a b`, `Result a b`
- **Recursive**: `List a` can reference itself: `Cons a (List a)`

## Using Generic Types

### Constructor Application

When you use a constructor with a value, the type checker infers the concrete type:

```parlang
type Option a = Some a | None in
Some 42
# Type: Option Int

type Option a = Some a | None in
Some true
# Type: Option Bool
```

### Polymorphic Constructors

Constructors without arguments remain polymorphic:

```parlang
type Option a = Some a | None in
None
# Type: Option t0  (where t0 is a type variable)
```

### Type Inference

The type checker automatically infers the type parameters:

```parlang
type List a = Nil | Cons a (List a) in
Cons 1 (Cons 2 (Cons 3 Nil))
# Type: List Int
```

### Nested Generics

You can nest generic types:

```parlang
type Option a = Some a | None in
type List a = Nil | Cons a (List a) in
Some (Cons 1 (Cons 2 Nil))
# Type: Option (List Int)
```

## Examples

### Option Type

The `Option` type represents a value that may or may not be present:

```parlang
type Option a = Some a | None in

let x = Some 42 in
match x with
| Some n -> n + 1
| None -> 0
# Result: 43
# Type: Int
```

### Either Type

The `Either` type represents a value that can be one of two types:

```parlang
type Either a b = Left a | Right b in

let result = Left 42 in
match result with
| Left n -> n
| Right b -> if b then 1 else 0
# Result: 42
# Type: Int
```

### List Type

The `List` type represents a sequence of values:

```parlang
type List a = Nil | Cons a (List a) in

let rec sum -> fun l ->
  match l with
  | Nil -> 0
  | Cons head tail -> head + sum tail
in

let myList = Cons 1 (Cons 2 (Cons 3 Nil)) in
sum myList
# Result: 6
# Type: Int
```

### Result Type

A common pattern for error handling:

```parlang
type Result a b = Ok a | Err b in

let divide = fun x -> fun y ->
  if y == 0
  then Err false  # Error: division by zero
  else Ok (x / y)
in

let result = divide 10 5 in
match result with
| Ok value -> value
| Err _ -> 0
# Result: 2
# Type: Int
```

### Tree Type

A binary tree with values at leaves:

```parlang
type Tree a = Leaf a | Node (Tree a) (Tree a) in

let myTree = Node (Leaf 1) (Node (Leaf 2) (Leaf 3)) in

let rec sumTree -> fun t ->
  match t with
  | Leaf n -> n
  | Node left right -> sumTree left + sumTree right
in

sumTree myTree
# Result: 6
# Type: Int
```

## Type Checking

### With Type Checking Enabled

When type checking is enabled (`PARLANG_TYPECHECK=1`), the REPL displays the inferred generic type:

```bash
$ PARLANG_TYPECHECK=1 cargo run
```

```parlang
> type Option a = Some a | None in Some 42
Type: Option Int
Some(42)

> type List a = Nil | Cons a (List a) in Cons 1 Nil
Type: List Int
Cons(1, Nil)

> type Either a b = Left a | Right b in Left true
Type: Either Bool t0
Left(true)
```

### Type Display

Generic types are displayed with their type arguments:

- `Option Int` - Option containing integers
- `List Bool` - List of booleans
- `Either Int Bool` - Either an integer or a boolean
- `Option (List Int)` - Option containing a list of integers

### Type Unification

The type checker properly unifies generic types:

```parlang
type List a = Nil | Cons a (List a) in
let x = Cons 1 Nil in
let y = Cons 2 x in
y
# Type: List Int
# The type checker unifies List t0 with List Int
```

## Type System Implementation

### Type Representation

Generic types are represented as `Type::SumType(name, args)`:

- `name`: The type constructor name (e.g., "Option", "List")
- `args`: Vector of type arguments (e.g., `[Type::Int]` for `Option Int`)

### Type Inference Algorithm

1. **Type Definition**: When a `type` definition is encountered, the constructors are registered in the type environment with their type parameters
2. **Constructor Application**: When a constructor is applied, the type checker:
   - Looks up the constructor's type information
   - Creates fresh type variables for each type parameter
   - Type checks the arguments
   - Unifies argument types with expected types
   - Returns the instantiated generic type
3. **Type Unification**: Two generic types unify if:
   - They have the same type constructor name
   - They have the same number of type arguments
   - All corresponding type arguments unify

### Type Environment

The type environment (`TypeEnv`) now tracks:

- **Variable bindings**: Maps variable names to type schemes
- **Type aliases**: Maps alias names to types
- **Constructors**: Maps constructor names to their type information (including type parameters and payload types)

### Constructor Information

Each constructor stores:

- **Type parameters**: The type variables (e.g., `["a", "b"]`)
- **Payload types**: The types of constructor arguments
- **Sum type name**: The name of the generic type it belongs to

## Limitations and Future Work

### Current Limitations

1. **No Higher-Kinded Types**: Type constructors must be fully applied (e.g., can't have `List` without a type argument in expressions)
2. **No Type Constraints**: No way to constrain type parameters (e.g., "must be comparable")
3. **No Type Annotations**: Users can't explicitly annotate types in expressions

### Planned Enhancements

- **Pattern matching exhaustiveness checking**: Warn when match cases don't cover all constructors
- **Type annotations**: Allow explicit type annotations for better error messages
- **Improved error messages**: Better diagnostics for type mismatches with generic types
- **Standard library**: Built-in generic types and functions

## Testing

The implementation includes comprehensive tests in `tests/generic_types_tests.rs`:

- Type inference for Option, Either, List, Result, Tree types
- Nested generic types
- Type unification for recursive types
- Display formatting for generic types
- Multiple uses of generic types with different instantiations

Run tests with:

```bash
cargo test generic_types_tests
```

## Technical Details

### Type Annotation to Type Conversion

When processing sum type definitions, `TypeAnnotation` values (from parsing) are converted to `Type` values:

- `Concrete("Int")` → `Type::Int`
- `Var("a")` → Type variable from parameter map
- `Fun(a, b)` → `Type::Fun(...)`
- `App("Option", [Var("a")])` → `Type::SumType("Option", [...])`

### Substitution and Free Variables

The implementation properly handles generic types in:

- **Substitution application**: Replaces type variables in generic type arguments
- **Free variable collection**: Collects free variables from type arguments
- **Type scheme generalization**: Quantifies free type variables in generic types

### Unification Algorithm

The unification algorithm (`unify`) now handles `SumType`:

```rust
(Type::SumType(name1, args1), Type::SumType(name2, args2)) => {
    // Must have same name and number of arguments
    if name1 != name2 || args1.len() != args2.len() {
        return Err(...);
    }
    
    // Unify all type arguments pairwise
    for (arg1, arg2) in args1.iter().zip(args2.iter()) {
        // Apply substitution and unify
        ...
    }
}
```

## References

- **Hindley-Milner Type System**: The foundation for type inference with polymorphism
- **Let-Polymorphism**: Allows generic types to be instantiated at different types
- **Algorithm W**: The type inference algorithm used by ParLang
- **Algebraic Data Types**: The basis for sum types in functional languages

## See Also

- [Type System Documentation](TYPE_SYSTEM.md)
- [Sum Types Documentation](SUM_TYPES.md)
- [Language Specification](LANGUAGE_SPEC.md)
- [Examples Guide](EXAMPLES.md)
