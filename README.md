# ParLang

A small ML-alike functional language written in Rust, with a parser built using the [combine](https://github.com/Marwes/combine) parser combinator library.

## Features

ParLang is a simple functional programming language with:

- **Basic Types**: Integers and booleans
- **Variables**: Let bindings for creating local variables
- **Functions**: First-class functions with closure support
- **Conditionals**: If-then-else expressions
- **Binary Operations**: Arithmetic (`+`, `-`, `*`, `/`) and comparison (`==`, `!=`, `<`, `<=`, `>`, `>=`)
- **Function Application**: Call functions with arguments
- **Currying**: Functions naturally support partial application

## Syntax

### Literals
```
42          # Integer
-10         # Negative integer
true        # Boolean
false       # Boolean
```

### Variables and Let Bindings
```
let x = 42 in x + 1                    # Result: 43
let double = fun x -> x + x in double 5 # Result: 10
```

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

The REPL supports both single-line and multiline input. For multiline expressions, press Enter after each line and submit with a blank line:

```
ParLang v0.1.0 - A small ML-alike functional language
Type expressions to evaluate them. Press Ctrl+C to exit.

> 42
42
> let x = 10 in x * 2
20
> (fun x -> x + 1) 41
42
> let double = fun x -> x + x
... in double 5

10
>
```

### File Execution Mode

Run a `.par` file:

```bash
cargo run -- examples/simple.par
```

## Examples

See the `examples/` directory for sample programs:

- `simple.par` - Basic let bindings and function application
- `conditional.par` - Conditional expressions
- `currying.par` - Currying and partial application

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

### 📖 API Reference
- **[API Reference](docs/API_REFERENCE.md)** - Complete API documentation for using ParLang as a library

## Architecture

The language implementation consists of:

1. **AST** (`src/ast.rs`): Abstract syntax tree definitions for expressions
2. **Parser** (`src/parser.rs`): Parser built with the `combine` library
3. **Evaluator** (`src/eval.rs`): Interpreter that evaluates expressions
4. **REPL/CLI** (`src/main.rs`): Command-line interface

For detailed architecture information, see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Running Tests

```bash
cargo test
```

## License

MIT
