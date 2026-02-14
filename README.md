# ParLang

A small ML-alike functional language written in Rust, with a parser built using the [combine](https://github.com/Marwes/combine) parser combinator library.

## Features

ParLang is a simple functional programming language with:

- **Basic Types**: Integers, booleans, characters, and floating point numbers
- **Arrays**: Fixed-size, homogeneous arrays with zero-based indexing (designed for FFI)
- **Type Inference**: Optional Hindley-Milner type system with automatic type inference
- **Type Aliases**: Define alternative names for types for better code documentation
- **Generic Types**: Full support for parameterized types (Option Int, List Bool, Either A B)
- **Sum Types**: Algebraic data types with constructors and pattern matching (Option, Either, List, etc.)
- **Records**: Product types with named fields for structured data
- **Tuples**: Heterogeneous tuples with projection and pattern matching
- **Variables**: Let bindings for creating local variables
- **Functions**: First-class functions with closure support
- **Polymorphism**: Let-polymorphism for generic functions
- **Recursion**: Named recursive functions with tail call optimization
- **Conditionals**: If-then-else expressions
- **Pattern Matching**: Match expressions for cleaner multi-branch logic (supports records, tuples, literals, and sum types)
- **Exhaustiveness Checking**: Automatic warnings for non-exhaustive pattern matches, helping catch bugs early
- **Binary Operations**: Arithmetic (`+`, `-`, `*`, `/`) and comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`)
- **Function Application**: Call functions with arguments
- **Currying**: Functions naturally support partial application
- **Library/Module System**: Load and reuse functions from library files

## Syntax

### Literals
```
42          # Integer
-10         # Negative integer
3.14        # Float
-2.5        # Negative float
true        # Boolean
false       # Boolean
'a'         # Character
```

### Variables and Let Bindings

**Traditional let-in syntax (for expressions):**
```
let x = 42 in x + 1                    # Result: 43
let double = fun x -> x + x in double 5 # Result: 10
```

**Sequential let bindings (for programs and REPL):**
```
let x = 42;
let y = 10;
x + y                                   # Result: 52
```

You can define multiple bindings without nesting `in` keywords by using semicolons.

### Functions
```
fun x -> x + 1                         # Anonymous function
(fun x -> x * 2) 21                    # Result: 42
```

### Type Annotations

ParLang supports **explicit type annotations** for variables to improve code documentation and enable early error detection:

```
let x : Int = 42 in x + 1              # Type annotation on variable
let active : Bool = true;              # Sequential binding with annotation
let count : Int = 100;                 # Multiple annotated bindings
```

Type annotations are optional and work alongside automatic type inference. See [Type Annotations Documentation](docs/TYPE_ANNOTATIONS.md) for details.

### Conditionals
```
if true then 1 else 2                  # Result: 1
if 5 > 3 then 100 else 0               # Result: 100
```

### Pattern Matching

Pattern matching provides a cleaner way to handle multiple conditions, avoiding deeply nested if-then-else statements.

**Basic pattern matching:**
```
match n with
| 0 -> 1
| 1 -> 1
| n -> n * 2     # Variable pattern binds n and can use it in the result
```

**Wildcard pattern:**
```
match value with
| 0 -> 10
| 1 -> 20
| _ -> 999   # Underscore matches anything
```

**Pattern matching with booleans:**
```
match flag with
| true -> 1
| false -> 0
```

**Pattern matching in recursive functions:**
```
let factorial = rec fact -> fun n ->
    match n with
    | 0 -> 1
    | n -> n * fact (n - 1)
```

Pattern matching evaluates patterns from top to bottom and executes the first matching arm. This is especially useful for replacing deeply nested if-then-else chains with more readable code.

### Exhaustiveness Checking

**New Feature:** ParLang now includes automatic exhaustiveness checking for pattern matching!

The exhaustiveness checker warns you when a `match` expression doesn't cover all possible cases:

```
type Option a = Some a | None in
match x with
| Some n -> n
# Warning: pattern match is non-exhaustive
# Missing cases: None
```

**Exhaustive match (no warning):**
```
type Option a = Some a | None in
match x with
| Some n -> n
| None -> 0
# ✓ All cases covered
```

The checker helps catch bugs early by ensuring you handle all constructors in sum types. For more details, see [Exhaustiveness Checking Documentation](docs/EXHAUSTIVENESS_CHECKING.md).

### Tuples

Tuples group multiple values together and support projection and pattern matching.

**Tuple creation:**
```
(1, 2, 3)                # Result: (1, 2, 3)
(42, true)               # Mixed types: (42, true)
()                       # Empty tuple (unit type)
```

**Tuple projection (zero-based indexing):**
```
(10, 20).0               # First element: 10
(10, 20).1               # Second element: 20
((1, 2), (3, 4)).0.1     # Chained projection: 2
```

**Tuple pattern matching:**
```
match (10, 20) with
| (0, 0) -> 0
| (x, y) -> x + y        # Result: 30
```

**Functions with tuples:**
```
let swap = fun p -> (p.1, p.0)
in swap (5, 10)          # Result: (10, 5)
```

### Arrays

Arrays are fixed-size, homogeneous collections with zero-based indexing, primarily designed for FFI (Foreign Function Interface) use cases.

**Array creation:**
```
[|1, 2, 3|]              # Array of 3 integers
[|true, false|]          # Array of 2 booleans
[|'a', 'b', 'c'|]        # Array of 3 characters
[||]                     # Empty array
```

**Array indexing:**
```
let arr = [|10, 20, 30|]
in arr[0]                # First element: 10

[|100, 200, 300|][2]     # Third element: 300
```

**Array indexing with expressions:**
```
let arr = [|100, 200, 300|]
in let index = 1 + 1
in arr[index]            # Result: 300
```

**Nested arrays:**
```
let matrix = [|[|1, 2|], [|3, 4|]|]
in matrix[0]             # Result: [|1, 2|]

# Deep indexing
matrix[1][0]             # Result: 3
```

**Arrays in functions:**
```
let getFirst = fun arr -> arr[0]
in getFirst [|42, 43, 44|]  # Result: 42

let makeArray = fun x -> fun y -> [|x, y|]
in makeArray 10 20          # Result: [|10, 20|]
```

**Array type:**
When type checking is enabled, arrays have type `Array[T, n]` where `T` is the element type and `n` is the size:
```
> [|1, 2, 3|]
Type: Array[Int, 3]
[|1, 2, 3|] (size: 3)
```

**Key features:**
- Fixed size determined at creation
- All elements must have the same type
- Zero-based indexing with bounds checking
- Efficient for FFI and fixed-size data structures

For more details, see [Arrays Documentation](docs/ARRAYS.md).

### Records

Records are product types with named fields, providing structured data with type-safe field access.

**Record creation:**
```
{ name: 42, age: 30 }                 # Simple record
{ active: true, verbose: false }      # Boolean fields
{}                                    # Empty record
```

**Field access:**
```
let person = { name: 42, age: 30 }
in person.age                          # Result: 30

# Nested field access
let person = { address: { city: 100 } }
in person.address.city                 # Result: 100
```

**Record pattern matching (full):**
```
let person = { name: 42, age: 30 }
in match person with
| { name: n, age: a } -> n + a         # Result: 72
```

**Partial pattern matching:**
```
let person = { name: 42, age: 30, city: 100 }
in match person with
| { name: n } -> n                     # Result: 42 (ignores age, city)
```

**Functions with records:**
```
let getAge = fun p -> p.age
in let person = { name: 42, age: 25 }
in getAge person                       # Result: 25

# Function returning a record
let makePerson = fun n -> fun a -> { name: n, age: a }
in makePerson 42 30                    # Result: { name: 42, age: 30 }
```

**Record updates (functional style):**
```
# Records are immutable - create new record to "update"
let person = { name: 42, age: 30 }
in let olderPerson = { name: person.name, age: person.age + 1 }
in olderPerson                         # Result: { name: 42, age: 31 }
```

**Row Polymorphism:**

Records support **row polymorphism**, allowing functions to work with any record that has at least certain fields:

```
# Function that works with any record having an 'age' field
let getAge = fun r -> r.age

# Works with different record types
getAge { name: 42, age: 30 }           # Result: 30
getAge { age: 25, city: 100 }          # Result: 25
```

When type checking is enabled, row polymorphic functions show their flexible type:
```
> fun p -> p.age
Type: { age: t0 | r0 } -> t0
```

The type `{ age: t0 | r0 }` means "a record with at least an `age` field (type `t0`), plus any other fields (`r0`)". This provides flexibility while maintaining type safety. See [docs/RECORDS.md](docs/RECORDS.md) for comprehensive documentation and advanced examples.

### Type Aliases

Type aliases let you define alternative names for types, making code more self-documenting without runtime overhead.

**Simple type alias:**
```
type MyInt = Int in 42
# Type: Int
# Result: 42
```

**Function type alias:**
```
type IntFunc = Int -> Int in
let increment = fun x -> x + 1 in
increment 41
# Result: 42
```

**Higher-order function type alias:**
```
type Transform = (Int -> Int) -> Int in
let apply = fun f -> f 21 in
let double = fun x -> x + x in
apply double
# Result: 42
```

**Nested type aliases:**
```
type IntFunc = Int -> Int in
type BoolFunc = Bool -> Bool in
let f = fun x -> x + 1 in
f 41
# Result: 42
```

Type aliases are transparent - they don't affect runtime behavior, just provide documentation for types.

### Sum Types (Algebraic Data Types)

Sum types allow you to define types with multiple variants, enabling type-safe representation of data that can be one of several forms.

**Defining sum types:**
```
# Option type for nullable values
type Option a = Some a | None in
let x = Some 42 in
match x with
| Some n -> n + 1
| None -> 0
# Result: 43

# Either type for values that can be one of two types
type Either a b = Left a | Right b in
let result = Left 10 in
match result with
| Left n -> n
| Right m -> m
# Result: 10
```

**Recursive sum types:**
```
# List type
type List a = Nil | Cons a (List a) in
let list = Cons 1 (Cons 2 (Cons 3 Nil)) in
let rec sum -> fun l ->
  match l with
  | Nil -> 0
  | Cons head tail -> head + sum tail
in sum list
# Result: 6
```

**Nested pattern matching:**
```
type Option a = Some a | None in
let x = Some (Some 42) in
match x with
| Some (Some n) -> n
| Some None -> 0
| None -> 0
# Result: 42
```

Sum types support:
- Multiple constructors per type (separated by `|`)
- Constructor arguments (payload data)
- Type parameters for polymorphism
- Recursive type definitions
- Pattern matching to extract values

See [docs/SUM_TYPES.md](docs/SUM_TYPES.md) for comprehensive documentation and examples.

### Generic Types

ParLang now has **full support for generic types** with proper type inference and type checking. Generic types allow you to write type-safe polymorphic data structures.

**Type inference for generic types:**
```
# With PARLANG_TYPECHECK=1 enabled

> type Option a = Some a | None in Some 42
Type: Option Int
Some(42)

> type List a = Nil | Cons a (List a) in Cons 1 (Cons 2 Nil)
Type: List Int
Cons(1, Cons(2, Nil))

> type Either a b = Left a | Right b in Left true
Type: Either Bool t0
Left(true)
```

**Key features:**
- Type parameters are properly tracked and instantiated
- Full type unification for generic types
- Support for nested generics (e.g., `Option (List Int)`)
- Type inference works seamlessly with constructors

**Example with type checking:**
```parlang
type Result a b = Ok a | Err b in

let divide = fun x -> fun y ->
  if y == 0
  then Err false
  else Ok (x / y)
in

let result = divide 10 5 in
match result with
| Ok value -> value
| Err _ -> 0
# Type: Int
# Result: 2
```

See [docs/GENERIC_TYPES.md](docs/GENERIC_TYPES.md) for comprehensive documentation and examples.

### Binary Operations
```
1 + 2        # Addition: 3
10 - 3       # Subtraction: 7
4 * 5        # Multiplication: 20
10 / 2       # Division: 5
5 == 5       # Equality: true
5 != 3       # Inequality: true
5 > 3        # Greater than: true
```

### Currying and Partial Application
```
let add = fun x -> fun y -> x + y
in let add5 = add 5
in add5 10   # Result: 15
```

### Recursion

ParLang supports named recursion using the `rec` keyword. Recursive functions can reference themselves by name, enabling powerful iterative patterns.

**Basic recursion:**
```
rec factorial -> fun n ->
    if n == 0
    then 1
    else n * factorial (n - 1)
```

**Tail recursion with accumulator (optimized):**
```
let sum_to_n = rec helper -> fun acc -> fun n ->
    if n == 0
    then acc
    else helper (acc + n) (n - 1)
in sum_to_n 0 100   # Result: 5050
```

**Using recursive functions:**
```
let factorial = rec f -> fun n ->
    if n == 0 then 1 else n * f (n - 1)
in factorial 10     # Result: 3628800
```

The language implements tail call optimization (TCO) for recursive functions, allowing deep recursion without stack overflow for tail-recursive patterns.

### Loading Libraries
```
load "examples/stdlib.par" in double 21    # Result: 42

load "examples/math.par"
in let result = square 5
in result                                   # Result: 25
```

Library files can define multiple functions using semicolon-separated let bindings:
```parlang
let double = fun x -> x * 2;
let triple = fun x -> x * 3;
0
```

## Type System

ParLang includes an optional Hindley-Milner type inference system that can catch type errors before runtime.

### Enabling Type Checking

Type checking is optional and can be enabled in the REPL by setting the `PARLANG_TYPECHECK` environment variable:

```bash
# Enable type checking for a single session
PARLANG_TYPECHECK=1 cargo run

# Or export it for all sessions
export PARLANG_TYPECHECK=1
cargo run
```

### Type Inference Examples

When type checking is enabled, the REPL displays inferred types before evaluation:

```
> 42
Type: Int
42

> true
Type: Bool
true

> fun x -> x + 1
Type: Int -> Int
<function x>

> let id = fun x -> x in id
Type: t0 -> t0
<function x>

> let id = fun x -> x in id 42
Type: Int
42
```

### Polymorphic Functions

The type system supports let-polymorphism, allowing functions to work with multiple types:

```
> let id = fun x -> x in let a = id 42 in let b = id true in b
Type: Bool
true
```

Here, `id` is used at both `Int` (for `id 42`) and `Bool` (for `id true`) types.

### Type Errors

Type errors are caught before evaluation:

```
> 1 + true
Type error: Cannot unify types: Bool and Int

> if 1 then 2 else 3
Type error: Cannot unify types: Int and Bool

> if true then 1 else false
Type error: Cannot unify types: Int and Bool
```

### Type System Features

- **Automatic Type Inference**: No type annotations required (but supported!)
- **Explicit Type Annotations**: Optional type annotations for better documentation and early error detection
- **Polymorphic Types**: Functions can work with multiple types
- **Let-Polymorphism**: Let-bound functions are generalized
- **Sound Type System**: Well-typed programs won't have type errors at runtime
- **Clear Error Messages**: Helpful messages when types don't match

For detailed information about the type system, see **[Type System Documentation](docs/TYPE_SYSTEM.md)** and **[Type Annotations Documentation](docs/TYPE_ANNOTATIONS.md)**.

## Installation

### Using Cargo

```bash
cargo build --release
```

### Using Nix Flakes

```bash
nix build
```

## Usage

### REPL Mode

Run the interpreter without arguments to start an interactive REPL:

```bash
cargo run
```

The REPL automatically submits expressions when they are complete and parseable. Simply press Enter after typing your expression - no need for a blank line:

```
ParLang v0.1.0 - A small ML-alike functional language
Type expressions to evaluate them. Press Ctrl+C to exit.

> 42
42
> let x = 10 in x * 2
20
> (fun x -> x + 1) 41
42
> let double = fun x -> x + x;
0
> let triple = fun x -> x + x + x;
0
> triple 5
15
>
```

**Note:** The REPL intelligently detects when your expression is complete and parseable, automatically submitting it after you press Enter. For incomplete multiline expressions (like `let...in` syntax split across lines), simply continue typing on new lines - the REPL waits until your expression is complete.

In the REPL and file mode, you can use semicolon-separated let bindings to define multiple functions without nesting `in` keywords. The trailing expression is optional - if omitted, it defaults to `0`.

#### Persistent Environment

**New in this version:** The REPL now maintains a persistent environment across evaluations. When you define functions or load libraries using semicolon syntax, they remain available for subsequent evaluations:

```
> let double = fun x -> x + x;
0
> double 21
42
> let triple = fun x -> x + x + x;
0
> triple 14
42
> load "examples/stdlib.par"
0
> max 10 20
20
>
```

**Note:** You no longer need to type a trailing `0` or `in 0` - the parser makes these optional for convenience! The REPL automatically submits complete expressions when you press Enter.

This makes the REPL much more convenient for interactive development, as you don't need to redefine functions after each evaluation.

### File Execution Mode

Run a `.par` file:

```bash
cargo run -- examples/simple.par
```

### AST Visualization

Dump the Abstract Syntax Tree (AST) to a DOT file for visualization:

```bash
# Dump AST to a DOT file (Graphviz format)
cargo run -- examples/simple.par --dump-ast ast.dot

# Or using short form
cargo run -- examples/simple.par -d ast.dot
```

Then render the DOT file to an image using Graphviz:

```bash
# Install Graphviz (if not already installed)
# Ubuntu/Debian: sudo apt install graphviz
# macOS: brew install graphviz
# Windows: choco install graphviz

# Render to PNG
dot -Tpng ast.dot -o ast.png

# Or render to SVG (scalable)
dot -Tsvg ast.dot -o ast.svg
```

The AST visualization is useful for:
- Understanding program structure
- Debugging parser behavior
- Learning how expressions are represented internally
- Documentation and teaching

### Command-Line Options

```bash
# Show help
cargo run -- --help

# Show version
cargo run -- --version

# Start REPL explicitly
cargo run -- repl
```

## Examples

See the `examples/` directory for sample programs:

- `simple.par` - Basic let bindings and function application
- `conditional.par` - Conditional expressions
- `pattern_matching.par` - Pattern matching examples (factorial, fibonacci, etc.)
- `currying.par` - Currying and partial application
- `factorial.par` - Recursive factorial function
- `recursion.par` - Library of recursive functions (factorial, fibonacci, gcd, etc.)
- `use_recursion.par` - Example of loading and using recursive functions
- `stdlib.par` - Standard library with common functions
- `math.par` - Mathematical utility functions
- `use_stdlib.par` - Example of loading and using library functions
- `type_aliases.par` - Basic type alias example
- `type_alias_simple.par` - Simple function type alias
- `type_alias_binary.par` - Binary operation type alias
- `type_alias_nested.par` - Nested type aliases
- `type_annotations.par` - Explicit type annotations for variables

## Documentation

Comprehensive documentation is available in the `docs/` directory:

### 📚 Core Documentation
- **[Architecture Guide](docs/ARCHITECTURE.md)** - System architecture, component interaction, and design patterns
- **[Language Specification](docs/LANGUAGE_SPEC.md)** - Formal language specification with grammar and semantics
- **[Type System](docs/TYPE_SYSTEM.md)** - Hindley-Milner type inference system documentation
- **[Type Inference](docs/TYPE_INFERENCE.md)** - Deep dive into the type inference algorithm (Algorithm W)
- **[Type Annotations](docs/TYPE_ANNOTATIONS.md)** - Explicit type annotations for better documentation and error detection
- **[Generic Types](docs/GENERIC_TYPES.md)** - Parameterized types and type inference for generic data structures
- **[Sum Types](docs/SUM_TYPES.md)** - Algebraic data types with pattern matching
- **[Arrays](docs/ARRAYS.md)** - Fixed-size arrays with indexing for FFI and structured data
- **[Exhaustiveness Checking](docs/EXHAUSTIVENESS_CHECKING.md)** - Automatic checking for complete pattern matches
- **[Examples Guide](docs/EXAMPLES.md)** - Tutorial-style examples from basic to advanced
- **[Security & Performance](docs/SECURITY.md)** - Security considerations and performance best practices
- **[Error Handling](docs/ERROR_HANDLING.md)** - Error types, patterns, and debugging strategies
- **[Test Guidelines](docs/TEST_GUIDELINES.md)** - Testing standards, patterns, and best practices
- **[Contributing Guide](CONTRIBUTING.md)** - Guidelines for contributing to ParLang

### 🔧 Module Documentation
- **[AST Module](docs/MODULE_AST.md)** - Abstract syntax tree data structures
- **[Parser Module](docs/MODULE_PARSER.md)** - Parser implementation using combinators
- **[Types Module](docs/MODULE_TYPES.md)** - Type representations (Int, Bool, Fun, Var)
- **[Type Checker Module](docs/MODULE_TYPECHECKER.md)** - Type inference algorithm implementation
- **[Evaluator Module](docs/MODULE_EVAL.md)** - Expression evaluation and runtime
- **[Main Module](docs/MODULE_MAIN.md)** - CLI and REPL interface
- **[DOT Module](docs/MODULE_DOT.md)** - AST visualization in Graphviz DOT format

### 📖 API Reference
- **[API Reference](docs/API_REFERENCE.md)** - Complete API documentation for using ParLang as a library

## Architecture

The language implementation consists of:

1. **AST** (`src/ast.rs`): Abstract syntax tree definitions for expressions
2. **Parser** (`src/parser.rs`): Parser built with the `combine` library
3. **Types** (`src/types.rs`): Type representations for the type system
4. **Type Checker** (`src/typechecker.rs`): Hindley-Milner type inference implementation
5. **Evaluator** (`src/eval.rs`): Interpreter that evaluates expressions
6. **DOT** (`src/dot.rs`): AST visualization in Graphviz DOT format
7. **REPL/CLI** (`src/main.rs`): Command-line interface using clap

For detailed architecture information, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Running Tests

```bash
cargo test
```

### Code Coverage

To generate code coverage reports, use [cargo-tarpaulin](https://github.com/xd009642/tarpaulin):

```bash
# Install tarpaulin (if not already installed)
cargo install cargo-tarpaulin

# Run tests with coverage
cargo tarpaulin

# Generate HTML report
cargo tarpaulin --out Html

# Generate detailed coverage with line-by-line details
cargo tarpaulin --out Html --output-dir coverage
```

The project currently achieves over 84% code coverage across all modules.

## License

MIT
