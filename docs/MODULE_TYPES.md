# Types Module (`src/types.rs`)

The Types module provides type representations for ParLang's Hindley-Milner type system.

## Overview

This module defines the core data structures used to represent types in ParLang, including basic types (Int, Bool), function types, type variables, and type schemes for polymorphic types.

## Data Structures

### `Type` Enum

The main type representation:

```rust
pub enum Type {
    Int,                        // Integer type
    Bool,                       // Boolean type
    Char,                       // Character type
    Float,                      // Floating point type
    Byte,                       // Byte type (unsigned 8-bit integer)
    Unit,                       // Unit type
    Fun(Box<Type>, Box<Type>),  // Function type: T1 -> T2
    Var(TypeVar),               // Type variable for polymorphism
    Record(HashMap<String, Type>), // Record type
    RecordRow(HashMap<String, Type>, RowVar), // Record with row polymorphism
    Row(RowVar),                // Row variable
    SumType(String, Vec<Type>), // Generic sum type
    Array(Box<Type>, usize),    // Fixed-size array
    Ref(Box<Type>),             // Reference type
}
```

**Variants:**
- `Int`: Represents 64-bit signed integer values (e.g., `42`, `-10`)
- `Bool`: Represents boolean values (`true`, `false`)
- `Char`: Represents single Unicode characters (e.g., `'a'`, `'\n'`)
- `Float`: Represents 64-bit floating point values (e.g., `3.14`, `-2.5`)
- `Byte`: Represents unsigned 8-bit integer values (e.g., `0b`, `255b`, `42b`)
- `Unit`: Represents the empty tuple `()`
- `Fun(arg, ret)`: Represents function types where `arg` is the argument type and `ret` is the return type
- `Var(TypeVar)`: Represents a type variable used during type inference
- `Record(fields)`: Represents record types with named fields
- `RecordRow(fields, row_var)`: Represents records with row polymorphism
- `Row(RowVar)`: Represents a row variable
- `SumType(name, args)`: Represents generic algebraic data types
- `Array(elem_type, size)`: Represents fixed-size arrays
- `Ref(inner)`: Represents mutable references

**Examples:**
```rust
Type::Int                                    // Int
Type::Bool                                   // Bool
Type::Byte                                   // Byte
Type::Float                                  // Float
Type::Fun(Box::new(Type::Int), 
          Box::new(Type::Bool))              // Int -> Bool
Type::Var(TypeVar(0))                        // t0 (type variable)
```

### `TypeVar` Struct

Represents a type variable:

```rust
pub struct TypeVar(pub usize);
```

Type variables are used during type inference to represent unknown types. They are identified by a unique integer.

**Properties:**
- Implements `PartialOrd` and `Ord` for ordering
- Implements `Hash` for use in collections
- Implements `Clone`, `PartialEq`, `Eq` for standard operations

### `TypeScheme` Struct

Represents a polymorphic type scheme (∀α.τ):

```rust
pub struct TypeScheme {
    pub vars: Vec<TypeVar>,  // Quantified type variables
    pub ty: Type,            // The type
}
```

Type schemes allow us to express polymorphic types by quantifying over type variables.

**Examples:**
```rust
// Monomorphic: Int
TypeScheme {
    vars: vec![],
    ty: Type::Int,
}

// Polymorphic: forall t0. t0 -> t0 (identity function)
TypeScheme {
    vars: vec![TypeVar(0)],
    ty: Type::Fun(
        Box::new(Type::Var(TypeVar(0))),
        Box::new(Type::Var(TypeVar(0))),
    ),
}
```

## Display Formatting

All types implement `Display` for human-readable output:

### Type Display

```rust
Type::Int                     // "Int"
Type::Bool                    // "Bool"
Type::Byte                    // "Byte"
Type::Float                   // "Float"
Type::Char                    // "Char"
Type::Unit                    // "()"
Type::Var(TypeVar(0))         // "t0"
Type::Fun(Int, Bool)          // "Int -> Bool"
Type::Fun(Fun(Int, Bool),
          Bool)               // "(Int -> Bool) -> Bool"
```

Function arguments are parenthesized when they are themselves function types to maintain clarity.

### TypeScheme Display

```rust
TypeScheme { vars: vec![], ty: Int }                    // "Int"
TypeScheme { vars: vec![TypeVar(0)], 
             ty: Fun(Var(0), Var(0)) }                  // "forall t0. t0 -> t0"
TypeScheme { vars: vec![TypeVar(0), TypeVar(1)],
             ty: Fun(Var(0), Var(1)) }                  // "forall t0, t1. t0 -> t1"
```

## Implementation Details

### Trait Implementations

All types implement these standard traits:
- `Debug`: For debugging output
- `Clone`: For creating copies
- `PartialEq`, `Eq`: For equality comparison
- `Hash`: For use in hash-based collections
- `Display`: For human-readable formatting

`TypeVar` additionally implements:
- `PartialOrd`, `Ord`: For ordering (used in generalization)

## Usage Examples

### Creating Types

```rust
use parlang::types::{Type, TypeVar, TypeScheme};

// Basic types
let int_type = Type::Int;
let bool_type = Type::Bool;
let byte_type = Type::Byte;
let float_type = Type::Float;
let char_type = Type::Char;

// Function type: Int -> Bool
let func_type = Type::Fun(
    Box::new(Type::Int),
    Box::new(Type::Bool),
);

// Type variable
let var_type = Type::Var(TypeVar(0));

// Polymorphic type scheme
let id_scheme = TypeScheme {
    vars: vec![TypeVar(0)],
    row_vars: vec![],
    ty: Type::Fun(
        Box::new(Type::Var(TypeVar(0))),
        Box::new(Type::Var(TypeVar(0))),
    ),
};
```

### Displaying Types

```rust
println!("{}", Type::Int);                              // "Int"
println!("{}", Type::Fun(
    Box::new(Type::Int),
    Box::new(Type::Bool),
));                                                     // "Int -> Bool"

let scheme = TypeScheme {
    vars: vec![TypeVar(0)],
    ty: Type::Fun(
        Box::new(Type::Var(TypeVar(0))),
        Box::new(Type::Var(TypeVar(0))),
    ),
};
println!("{}", scheme);                                 // "forall t0. t0 -> t0"
```

## Type System Properties

The type representations support:

1. **Basic Types**: Int, Bool, Char, Float, Byte, and Unit for primitive values
2. **Function Types**: First-class function types with proper associativity
3. **Type Variables**: For representing unknown or polymorphic types
4. **Type Schemes**: For expressing polymorphic types with universal quantification
5. **Record Types**: Structured data with named fields
6. **Row Polymorphism**: Flexible record types
7. **Sum Types**: Algebraic data types with constructors
8. **Array Types**: Fixed-size homogeneous collections
9. **Reference Types**: Mutable references

## Testing

The module includes comprehensive unit tests covering:
- Type equality and inequality
- Type variable ordering
- Function type construction
- Type scheme construction
- Display formatting for all type variants
- Edge cases (nested functions, multiple type variables, etc.)

Run tests with:
```bash
cargo test types::tests
```

## Related Modules

- **[Type Checker Module](MODULE_TYPECHECKER.md)**: Uses these types for inference
- **[AST Module](MODULE_AST.md)**: Expressions that are type-checked

## References

- Damas, Luis; Milner, Robin (1982). "Principal type-schemes for functional programs"
- Pierce, Benjamin C. "Types and Programming Languages" (2002)
