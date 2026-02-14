# Explicit Type Annotations

ParLang now supports **explicit type annotations** for variables and function parameters. Type annotations allow you to explicitly declare the expected type of a variable, providing better documentation and enabling the type checker to catch mismatches between the declared type and the inferred type.

## Overview

Type annotations are optional and work alongside ParLang's existing Hindley-Milner type inference system. When provided, the type checker verifies that the annotated type matches the inferred type, catching type errors early.

## Syntax

### Let Bindings with Type Annotations

You can annotate variables in let bindings using the `: Type` syntax:

```parlang
let x : Int = 42 in x + 1
```

This declares that `x` should have type `Int`. The type checker will verify that the value `42` is indeed an `Int`.

### Sequential Let Bindings

Type annotations work in sequential bindings (used in files and REPL):

```parlang
let x : Int = 42;
let y : Bool = true;
let z : Int = x + 10;
z
```

### Function Parameters

**Note:** Type annotations for function parameters currently have a parser limitation due to the `->` operator precedence. For now, use type inference for function parameters:

```parlang
# This works (inferred types)
fun x -> x + 1

# Type annotations for function params are supported in the AST
# but may have parsing ambiguities with the -> operator
```

## Type Annotation Forms

### Concrete Types

Annotate with concrete types like `Int` or `Bool`:

```parlang
let count : Int = 100;
let active : Bool = true;
```

### Function Types

Function types use the `->` operator:

```parlang
# In type annotations (future improvement)
let f : Int -> Bool = fun x -> x > 0 in f 42
```

**Current Limitation:** Function type annotations in parameters have precedence issues and should be avoided for now.

### Type Variables

Type variables (lowercase identifiers) can be used in annotations for polymorphic functions:

```parlang
# Type variable 'a' represents any type
# (Implementation detail: type variables create fresh type variables)
```

## Type Checking with Annotations

When type checking is enabled (`PARLANG_TYPECHECK=1`), the type checker:

1. **Infers the type** of the expression using Hindley-Milner type inference
2. **Checks the annotation** against the inferred type
3. **Reports errors** if they don't match

### Correct Annotation Example

```parlang
> PARLANG_TYPECHECK=1
> let x : Int = 42 in x + 1
Type: Int
43
```

The annotation `: Int` matches the inferred type `Int`, so this succeeds.

### Type Mismatch Example

```parlang
> PARLANG_TYPECHECK=1
> let x : Bool = 42 in x
Type error: Cannot unify types: Int and Bool
```

The annotation says `Bool`, but `42` has type `Int`, so the type checker reports an error.

## Benefits of Type Annotations

### 1. Documentation

Type annotations make code more readable by explicitly showing expected types:

```parlang
let userId : Int = 12345;
let userName : Int = 98765;  # Note: Int is used for string IDs in this toy language
let isActive : Bool = true;
```

### 2. Early Error Detection

Catch type errors at compile-time instead of runtime:

```parlang
# This catches the error before evaluation
let age : Int = true;  # Error: Bool is not Int
```

### 3. Better Error Messages

When an annotation doesn't match, you get clearer error messages indicating the mismatch.

## Examples

### Basic Let Binding

```parlang
# Without annotation (inferred)
let x = 42 in x + 1

# With annotation (explicit)
let x : Int = 42 in x + 1
```

Both work identically, but the second makes the type explicit.

### Multiple Variables

```parlang
let x : Int = 10;
let y : Int = 20;
let z : Int = x + y;
z
# Result: 30
```

### Nested Let Bindings

```parlang
let x : Int = 5 in
  let y : Int = 10 in
    x + y
# Result: 15
```

### Type Checking Example

Enable type checking to see inferred types and catch errors:

```bash
PARLANG_TYPECHECK=1 cargo run -- examples/type_annotations.par
```

## Current Limitations

### Function Parameter Annotations

Due to parser precedence with the `->` operator, type annotations on function parameters have ambiguities:

```parlang
# This has parsing issues:
fun x : Int -> x + 1
# Parser sees: fun x : (Int -> x) + 1
```

**Workaround:** Use type inference for function parameters for now:

```parlang
# This works correctly:
fun x -> x + 1
# Type: Int -> Int (inferred)
```

### Applied Types

Applied types (like `Option Int` or `List Bool`) are not yet supported in annotations:

```parlang
# This is not yet supported in annotations:
let opt : Option Int = Some 42
```

## Implementation Details

### AST Changes

The AST now includes optional type annotations:

```rust
pub enum Expr {
    // ...
    Let(String, Option<TypeAnnotation>, Box<Expr>, Box<Expr>),
    Fun(String, Option<TypeAnnotation>, Box<Expr>),
    Seq(Vec<(String, Option<TypeAnnotation>, Expr)>, Box<Expr>),
    // ...
}
```

The `Option<TypeAnnotation>` field is `None` when no annotation is provided, and `Some(ty)` when an annotation is specified.

### Type Checking Algorithm

When a type annotation is present:

1. The type checker infers the type normally
2. It resolves the type annotation to a `Type`
3. It unifies the inferred type with the annotated type
4. If unification fails, it reports a type error

### Parser Implementation

The parser uses the `optional()` combinator to parse type annotations:

```rust
optional(
    token(':').skip(spaces())
        .with(type_annotation().skip(spaces()))
)
```

This makes type annotations optional - if the `:` token is not found, the parser continues without an annotation.

## Future Improvements

Potential enhancements for type annotations:

1. **Fix function parameter annotation precedence** - Allow clear syntax like `fun (x : Int) -> body`
2. **Support applied types** - Enable `Option Int`, `List Bool` in annotations
3. **Add type alias support** - Use type aliases in annotations
4. **Bi-directional type checking** - Use annotations to guide inference
5. **Better error messages** - Show both expected (annotated) and actual (inferred) types

## Related Documentation

- [Type System Documentation](TYPE_SYSTEM.md) - Hindley-Milner type inference
- [Language Specification](LANGUAGE_SPEC.md) - Complete language syntax
- [Examples Guide](EXAMPLES.md) - More code examples

## Testing

Comprehensive tests for type annotations can be found in:
- `tests/type_annotation_tests.rs` - Unit tests for parsing and type checking
- `examples/type_annotations.par` - Example usage

Run the tests with:

```bash
cargo test type_annotation_tests
```

## Conclusion

Explicit type annotations provide a powerful way to document code and catch type errors early. While there are some current limitations (particularly with function parameter annotations), the feature integrates seamlessly with ParLang's existing type inference system.

Type annotations are completely optional - you can continue using ParLang without annotations and rely entirely on type inference. Add annotations where they improve code clarity or help catch bugs.
