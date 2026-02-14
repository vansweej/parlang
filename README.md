# ParLang

A small ML-alike functional language written in Rust, with a parser built using the [combine](https://github.com/Marwes/combine) parser combinator library.

## Features

ParLang is a simple functional programming language with:

- **Basic Types**: Integers and booleans
- **Tuples**: Heterogeneous tuples with projection and pattern matching
- **Variables**: Let bindings for creating local variables
- **Functions**: First-class functions with closure support
- **Recursion**: Named recursive functions with tail call optimization
- **Conditionals**: If-then-else expressions
- **Pattern Matching**: Match expressions for cleaner multi-branch logic
- **Binary Operations**: Arithmetic (`+`, `-`, `*`, `/`) and comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`)
- **Function Application**: Call functions with arguments
- **Currying**: Functions naturally support partial application
- **Library/Module System**: Load and reuse functions from library files

## Syntax

### Literals
```
42          # Integer
-10         # Negative integer
true        # Boolean
false       # Boolean
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

## Documentation

Comprehensive documentation is available in the `docs/` directory:

### 📚 Core Documentation
- **[Architecture Guide](docs/ARCHITECTURE.md)** - System architecture, component interaction, and design patterns
- **[Language Specification](docs/LANGUAGE_SPEC.md)** - Formal language specification with grammar and semantics
- **[Examples Guide](docs/EXAMPLES.md)** - Tutorial-style examples from basic to advanced

### 🔧 Module Documentation
- **[AST Module](docs/MODULE_AST.md)** - Abstract syntax tree data structures
- **[Parser Module](docs/MODULE_PARSER.md)** - Parser implementation using combinators
- **[Evaluator Module](docs/MODULE_EVAL.md)** - Expression evaluation and runtime
- **[Main Module](docs/MODULE_MAIN.md)** - CLI and REPL interface
- **[DOT Module](docs/MODULE_DOT.md)** - AST visualization in Graphviz DOT format

### 📖 API Reference
- **[API Reference](docs/API_REFERENCE.md)** - Complete API documentation for using ParLang as a library

## Architecture

The language implementation consists of:

1. **AST** (`src/ast.rs`): Abstract syntax tree definitions for expressions
2. **Parser** (`src/parser.rs`): Parser built with the `combine` library
3. **Evaluator** (`src/eval.rs`): Interpreter that evaluates expressions
4. **DOT** (`src/dot.rs`): AST visualization in Graphviz DOT format
5. **REPL/CLI** (`src/main.rs`): Command-line interface using clap

For detailed architecture information, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Running Tests

```bash
cargo test
```

## License

MIT
