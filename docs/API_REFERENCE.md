# ParLang API Reference

Complete API documentation for using ParLang as a library in Rust applications.

---

## Table of Contents

1. [Overview](#overview)
2. [Getting Started](#getting-started)
3. [Module Structure](#module-structure)
4. [Public API](#public-api)
5. [AST Module](#ast-module)
6. [Parser Module](#parser-module)
7. [Evaluator Module](#evaluator-module)
8. [Error Handling](#error-handling)
9. [Usage Examples](#usage-examples)
10. [Integration Patterns](#integration-patterns)
11. [API Design Patterns](#api-design-patterns)

---

## Overview

ParLang is a library crate that provides a complete implementation of a small ML-alike functional language. The library exposes:

- **Abstract Syntax Tree (AST)** definitions for representing programs
- **Parser** using the `combine` parser combinator library
- **Evaluator** for executing ParLang programs
- **Type-safe error handling** with custom error types

### Design Philosophy

The API is designed to be:
- **Simple**: Minimal surface area with clear abstractions
- **Type-safe**: Leverage Rust's type system for correctness
- **Composable**: Modules work independently or together
- **Well-documented**: Comprehensive documentation and examples

---

## Getting Started

### Adding ParLang to Your Project

Add to your `Cargo.toml`:

```toml
[dependencies]
parlang = { path = "../parlang" }  # or from crates.io when published
```

### Basic Usage

```rust
use parlang::{parse, eval, Environment};

fn main() {
    let program = "let x = 42 in x + 1";
    
    // Parse the program
    let expr = parse(program).expect("Parse error");
    
    // Create an environment
    let env = Environment::new();
    
    // Evaluate the expression
    let result = eval(&expr, &env).expect("Evaluation error");
    
    println!("Result: {}", result);  // Output: Result: 43
}
```

---

## Module Structure

ParLang is organized into three main modules:

```
parlang/
├── ast      - Abstract Syntax Tree definitions
├── parser   - Parser implementation
└── eval     - Evaluator/interpreter
```

### Module Visibility

The library re-exports commonly used types and functions at the crate root:

```rust
// Re-exported from ast module
pub use ast::{Expr, BinOp};

// Re-exported from parser module
pub use parser::parse;

// Re-exported from eval module
pub use eval::{eval, Value, Environment, EvalError};
```

---

## Public API

### Crate Root Exports

```rust
pub mod ast;      // AST definitions
pub mod parser;   // Parser implementation
pub mod eval;     // Evaluator implementation

// Commonly used types
pub use ast::{Expr, BinOp};
pub use parser::parse;
pub use eval::{eval, Value, Environment, EvalError};
```

### Quick Reference

| Type | Module | Purpose |
|------|--------|---------|
| `Expr` | `ast` | AST node representing an expression |
| `BinOp` | `ast` | Binary operators (Add, Sub, etc.) |
| `parse()` | `parser` | Parse string to `Expr` |
| `eval()` | `eval` | Evaluate `Expr` to `Value` |
| `Value` | `eval` | Runtime values (Int, Bool, Closure) |
| `Environment` | `eval` | Variable bindings |
| `EvalError` | `eval` | Evaluation errors |

---

## AST Module

The `ast` module defines the Abstract Syntax Tree for ParLang programs.

### Expr Enum

The `Expr` enum represents all possible expressions in ParLang:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Bool(bool),
    Var(String),
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Let(String, Box<Expr>, Box<Expr>),
    Fun(String, Box<Expr>),
    App(Box<Expr>, Box<Expr>),
    Load(String, Box<Expr>),
}
```

### Expr Variants

#### `Expr::Int(i64)`

Integer literal.

**Example:**
```rust
use parlang::Expr;

let expr = Expr::Int(42);
assert_eq!(format!("{}", expr), "42");
```

#### `Expr::Bool(bool)`

Boolean literal.

**Example:**
```rust
let expr = Expr::Bool(true);
assert_eq!(format!("{}", expr), "true");
```

#### `Expr::Var(String)`

Variable reference.

**Example:**
```rust
let expr = Expr::Var("x".to_string());
assert_eq!(format!("{}", expr), "x");
```

#### `Expr::BinOp(BinOp, Box<Expr>, Box<Expr>)`

Binary operation (e.g., addition, comparison).

**Parameters:**
- `BinOp`: The operator (Add, Sub, Mul, Div, Eq, Neq, Lt, Le, Gt, Ge)
- `Box<Expr>`: Left operand
- `Box<Expr>`: Right operand

**Example:**
```rust
use parlang::{Expr, BinOp};

let expr = Expr::BinOp(
    BinOp::Add,
    Box::new(Expr::Int(1)),
    Box::new(Expr::Int(2)),
);
assert_eq!(format!("{}", expr), "(1 + 2)");
```

#### `Expr::If(Box<Expr>, Box<Expr>, Box<Expr>)`

Conditional expression.

**Parameters:**
- First `Box<Expr>`: Condition
- Second `Box<Expr>`: Then branch
- Third `Box<Expr>`: Else branch

**Example:**
```rust
let expr = Expr::If(
    Box::new(Expr::Bool(true)),
    Box::new(Expr::Int(1)),
    Box::new(Expr::Int(2)),
);
assert_eq!(format!("{}", expr), "(if true then 1 else 2)");
```

#### `Expr::Let(String, Box<Expr>, Box<Expr>)`

Let binding.

**Parameters:**
- `String`: Variable name
- First `Box<Expr>`: Value to bind
- Second `Box<Expr>`: Body expression

**Example:**
```rust
let expr = Expr::Let(
    "x".to_string(),
    Box::new(Expr::Int(42)),
    Box::new(Expr::Var("x".to_string())),
);
assert_eq!(format!("{}", expr), "(let x = 42 in x)");
```

#### `Expr::Fun(String, Box<Expr>)`

Function definition.

**Parameters:**
- `String`: Parameter name
- `Box<Expr>`: Function body

**Example:**
```rust
let expr = Expr::Fun(
    "x".to_string(),
    Box::new(Expr::Var("x".to_string())),
);
assert_eq!(format!("{}", expr), "(fun x -> x)");
```

#### `Expr::App(Box<Expr>, Box<Expr>)`

Function application.

**Parameters:**
- First `Box<Expr>`: Function expression
- Second `Box<Expr>`: Argument expression

**Example:**
```rust
let expr = Expr::App(
    Box::new(Expr::Var("f".to_string())),
    Box::new(Expr::Int(42)),
);
assert_eq!(format!("{}", expr), "(f 42)");
```

#### `Expr::Load(String, Box<Expr>)`

Load expression for importing library files.

**Parameters:**
- `String`: File path to the library file (relative to current working directory)
- `Box<Expr>`: Body expression to evaluate with library bindings

**Semantics:**
1. Reads and parses the library file
2. Extracts bindings from nested `let` expressions in the library
3. Extends current environment with library bindings
4. Evaluates body expression in extended environment

**Library File Structure:**
Library files should use nested `let` expressions:
```parlang
let func1 = fun x -> x * 2
in let func2 = fun y -> y + 1
in 0
```

**Example:**
```rust
let expr = Expr::Load(
    "examples/stdlib.par".to_string(),
    Box::new(Expr::App(
        Box::new(Expr::Var("double".to_string())),
        Box::new(Expr::Int(21)),
    )),
);
assert_eq!(format!("{}", expr), "(load \"examples/stdlib.par\" in (double 21))");
```

**Error Handling:**
- File not found: `EvalError::LoadError`
- Parse error: `EvalError::LoadError`

### BinOp Enum

Binary operators:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,  // +
    Sub,  // -
    Mul,  // *
    Div,  // /
    Eq,   // ==
    Neq,  // !=
    Lt,   // <
    Le,   // <=
    Gt,   // >
    Ge,   // >=
}
```

**Example:**
```rust
use parlang::BinOp;

let op = BinOp::Add;
assert_eq!(format!("{}", op), "+");
```

### Display Trait

Both `Expr` and `BinOp` implement `Display` for human-readable output:

```rust
use parlang::{Expr, BinOp};

let expr = Expr::BinOp(
    BinOp::Mul,
    Box::new(Expr::Int(6)),
    Box::new(Expr::Int(7)),
);
println!("{}", expr);  // Output: (6 * 7)
```

### Trait Implementations

- `Debug`: For debugging output
- `Clone`: Deep cloning of expressions
- `PartialEq`: Structural equality
- `Display`: Human-readable formatting (for `Expr` and `BinOp`)

---

## Parser Module

The `parser` module provides parsing functionality using the `combine` parser combinator library.

### parse Function

```rust
pub fn parse(input: &str) -> Result<Expr, String>
```

Parse a string into an `Expr` AST.

**Parameters:**
- `input: &str`: The ParLang source code to parse

**Returns:**
- `Result<Expr, String>`: 
  - `Ok(Expr)`: Successfully parsed expression
  - `Err(String)`: Parse error message

**Example:**
```rust
use parlang::parse;

let result = parse("42");
assert!(result.is_ok());
assert_eq!(result.unwrap(), parlang::Expr::Int(42));
```

### Error Messages

Parse errors include descriptive messages:

```rust
let result = parse("let x = 42");  // Missing "in"
assert!(result.is_err());
assert!(result.unwrap_err().contains("Parse error"));
```

### Parsing Examples

#### Literals

```rust
use parlang::{parse, Expr};

assert_eq!(parse("42"), Ok(Expr::Int(42)));
assert_eq!(parse("true"), Ok(Expr::Bool(true)));
assert_eq!(parse("false"), Ok(Expr::Bool(false)));
```

#### Variables

```rust
assert_eq!(parse("x"), Ok(Expr::Var("x".to_string())));
assert_eq!(parse("foo_bar"), Ok(Expr::Var("foo_bar".to_string())));
```

#### Binary Operations

```rust
use parlang::BinOp;

let expr = parse("1 + 2").unwrap();
assert_eq!(
    expr,
    Expr::BinOp(
        BinOp::Add,
        Box::new(Expr::Int(1)),
        Box::new(Expr::Int(2))
    )
);
```

#### Let Bindings

```rust
let expr = parse("let x = 42 in x").unwrap();
assert_eq!(
    expr,
    Expr::Let(
        "x".to_string(),
        Box::new(Expr::Int(42)),
        Box::new(Expr::Var("x".to_string()))
    )
);
```

#### Functions

```rust
let expr = parse("fun x -> x + 1").unwrap();
assert_eq!(
    expr,
    Expr::Fun(
        "x".to_string(),
        Box::new(Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Var("x".to_string())),
            Box::new(Expr::Int(1))
        ))
    )
);
```

#### Complex Expressions

```rust
let program = "let double = fun x -> x + x in double 21";
let expr = parse(program);
assert!(expr.is_ok());
```

### Parser Features

- **Operator precedence**: `*`, `/` higher than `+`, `-`
- **Comparison operators**: Lower precedence than arithmetic
- **Whitespace handling**: Flexible whitespace between tokens
- **Parentheses**: For grouping and overriding precedence
- **Keyword rejection**: Keywords cannot be used as identifiers
- **Negative numbers**: Support for negative integer literals

---

## Evaluator Module

The `eval` module provides the interpreter for executing ParLang programs.

### Value Enum

Runtime values in ParLang:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Closure(String, Expr, Environment),
}
```

#### Value Variants

**`Value::Int(i64)`**

Integer value.

```rust
use parlang::Value;

let val = Value::Int(42);
assert_eq!(format!("{}", val), "42");
```

**`Value::Bool(bool)`**

Boolean value.

```rust
let val = Value::Bool(true);
assert_eq!(format!("{}", val), "true");
```

**`Value::Closure(String, Expr, Environment)`**

Function closure capturing its environment.

**Parameters:**
- `String`: Parameter name
- `Expr`: Function body
- `Environment`: Captured environment

```rust
use parlang::{Value, Expr, Environment};

let env = Environment::new();
let closure = Value::Closure(
    "x".to_string(),
    Expr::Var("x".to_string()),
    env,
);
assert_eq!(format!("{}", closure), "<function x>");
```

### Environment Struct

Variable bindings:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    bindings: HashMap<String, Value>,
}
```

#### Environment Methods

**`new() -> Self`**

Create an empty environment.

```rust
use parlang::Environment;

let env = Environment::new();
```

**`bind(&mut self, name: String, value: Value)`**

Add or update a binding (mutable).

```rust
use parlang::{Environment, Value};

let mut env = Environment::new();
env.bind("x".to_string(), Value::Int(42));
```

**`lookup(&self, name: &str) -> Option<&Value>`**

Look up a variable.

```rust
let mut env = Environment::new();
env.bind("x".to_string(), Value::Int(42));

assert_eq!(env.lookup("x"), Some(&Value::Int(42)));
assert_eq!(env.lookup("y"), None);
```

**`extend(&self, name: String, value: Value) -> Self`**

Create a new environment with an additional binding (immutable).

```rust
let env = Environment::new();
let new_env = env.extend("x".to_string(), Value::Int(42));

assert_eq!(new_env.lookup("x"), Some(&Value::Int(42)));
assert_eq!(env.lookup("x"), None);  // Original unchanged
```

#### Default Trait

```rust
let env: Environment = Default::default();
```

### eval Function

```rust
pub fn eval(expr: &Expr, env: &Environment) -> Result<Value, EvalError>
```

Evaluate an expression in an environment.

**Parameters:**
- `expr: &Expr`: Expression to evaluate
- `env: &Environment`: Variable bindings

**Returns:**
- `Result<Value, EvalError>`:
  - `Ok(Value)`: Successfully evaluated value
  - `Err(EvalError)`: Evaluation error

**Example:**
```rust
use parlang::{parse, eval, Environment};

let expr = parse("let x = 42 in x + 1").unwrap();
let env = Environment::new();
let result = eval(&expr, &env).unwrap();

assert_eq!(result, parlang::Value::Int(43));
```

### Evaluation Rules

#### Literals

Literals evaluate to themselves:

```rust
use parlang::{Expr, eval, Environment, Value};

let env = Environment::new();
assert_eq!(eval(&Expr::Int(42), &env), Ok(Value::Int(42)));
assert_eq!(eval(&Expr::Bool(true), &env), Ok(Value::Bool(true)));
```

#### Variables

Variables are looked up in the environment:

```rust
let mut env = Environment::new();
env.bind("x".to_string(), Value::Int(42));

let expr = Expr::Var("x".to_string());
assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
```

#### Binary Operations

Operators are evaluated on operands:

```rust
use parlang::BinOp;

let expr = Expr::BinOp(
    BinOp::Add,
    Box::new(Expr::Int(1)),
    Box::new(Expr::Int(2)),
);
assert_eq!(eval(&expr, &env), Ok(Value::Int(3)));
```

#### Conditionals

If-then-else evaluates condition first:

```rust
let expr = Expr::If(
    Box::new(Expr::Bool(true)),
    Box::new(Expr::Int(1)),
    Box::new(Expr::Int(2)),
);
assert_eq!(eval(&expr, &env), Ok(Value::Int(1)));
```

#### Let Bindings

Let expressions create new bindings:

```rust
let expr = Expr::Let(
    "x".to_string(),
    Box::new(Expr::Int(42)),
    Box::new(Expr::Var("x".to_string())),
);
assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
```

#### Functions

Functions evaluate to closures:

```rust
let expr = Expr::Fun(
    "x".to_string(),
    Box::new(Expr::Var("x".to_string())),
);
let result = eval(&expr, &env);
assert!(matches!(result, Ok(Value::Closure(_, _, _))));
```

#### Function Application

Application evaluates function and argument, then applies:

```rust
let expr = Expr::App(
    Box::new(Expr::Fun(
        "x".to_string(),
        Box::new(Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Var("x".to_string())),
            Box::new(Expr::Int(1)),
        )),
    )),
    Box::new(Expr::Int(41)),
);
assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
```

---

## Error Handling

### EvalError Enum

Evaluation errors:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    UnboundVariable(String),
    TypeError(String),
    DivisionByZero,
    LoadError(String),
}
```

#### Error Variants

**`EvalError::UnboundVariable(String)`**

Variable not found in environment.

```rust
use parlang::{parse, eval, Environment, EvalError};

let expr = parse("x").unwrap();
let env = Environment::new();
let result = eval(&expr, &env);

assert_eq!(result, Err(EvalError::UnboundVariable("x".to_string())));
```

**`EvalError::TypeError(String)`**

Type mismatch or invalid operation.

```rust
let expr = parse("if 42 then 1 else 2").unwrap();
let result = eval(&expr, &env);

assert!(matches!(result, Err(EvalError::TypeError(_))));
```

**`EvalError::DivisionByZero`**

Attempted division by zero.

```rust
let expr = parse("10 / 0").unwrap();
let result = eval(&expr, &env);

assert_eq!(result, Err(EvalError::DivisionByZero));
```

**`EvalError::LoadError(String)`**

Error loading or parsing library file.

Occurs when:
- File does not exist or cannot be read
- File contains invalid ParLang syntax

```rust
use parlang::{parse, eval, Environment, EvalError};

// File not found
let expr = parse("load \"nonexistent.par\" in 42").unwrap();
let env = Environment::new();
let result = eval(&expr, &env);

assert!(matches!(result, Err(EvalError::LoadError(_))));
```

### Display Trait

Errors implement `Display` for user-friendly messages:

```rust
let err = EvalError::UnboundVariable("x".to_string());
assert_eq!(format!("{}", err), "Unbound variable: x");

let err = EvalError::DivisionByZero;
assert_eq!(format!("{}", err), "Division by zero");
```

### std::error::Error Trait

`EvalError` implements `std::error::Error` for compatibility:

```rust
use std::error::Error;

fn process() -> Result<(), Box<dyn Error>> {
    let expr = parse("x")?;
    let env = Environment::new();
    let result = eval(&expr, &env)?;
    Ok(())
}
```

---

## Usage Examples

### Example 1: Simple REPL

```rust
use parlang::{parse, eval, Environment};
use std::io::{self, Write};

fn main() {
    let mut env = Environment::new();
    
    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if input.trim().is_empty() {
            break;
        }
        
        match parse(&input) {
            Ok(expr) => {
                match eval(&expr, &env) {
                    Ok(value) => println!("{}", value),
                    Err(err) => eprintln!("Error: {}", err),
                }
            }
            Err(err) => eprintln!("Parse error: {}", err),
        }
    }
}
```

### Example 2: Evaluating a Program

```rust
use parlang::{parse, eval, Environment};

fn evaluate_program(source: &str) -> Result<String, String> {
    let expr = parse(source)
        .map_err(|e| format!("Parse error: {}", e))?;
    
    let env = Environment::new();
    let value = eval(&expr, &env)
        .map_err(|e| format!("Eval error: {}", e))?;
    
    Ok(format!("{}", value))
}

fn main() {
    let program = "let x = 42 in x + 1";
    match evaluate_program(program) {
        Ok(result) => println!("Result: {}", result),
        Err(err) => eprintln!("Error: {}", err),
    }
}
```

### Example 3: Building AST Manually

```rust
use parlang::{Expr, BinOp, eval, Environment, Value};

fn main() {
    // Build AST for: let x = 10 in x * 2
    let expr = Expr::Let(
        "x".to_string(),
        Box::new(Expr::Int(10)),
        Box::new(Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::Var("x".to_string())),
            Box::new(Expr::Int(2)),
        )),
    );
    
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(20));
    println!("Result: {}", result);
}
```

### Example 4: Persistent Environment

```rust
use parlang::{parse, eval, Environment, Value};

fn main() {
    let mut env = Environment::new();
    
    // Pre-populate environment
    env.bind("pi".to_string(), Value::Int(3));
    env.bind("tau".to_string(), Value::Int(6));
    
    let expr = parse("pi + tau").unwrap();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(9));
}
```

### Example 5: Error Recovery

```rust
use parlang::{parse, eval, Environment, EvalError};

fn safe_eval(source: &str) -> String {
    let expr = match parse(source) {
        Ok(e) => e,
        Err(err) => return format!("Parse error: {}", err),
    };
    
    let env = Environment::new();
    match eval(&expr, &env) {
        Ok(value) => format!("{}", value),
        Err(EvalError::UnboundVariable(name)) => {
            format!("Variable '{}' is not defined", name)
        }
        Err(EvalError::TypeError(msg)) => {
            format!("Type error: {}", msg)
        }
        Err(EvalError::DivisionByZero) => {
            format!("Cannot divide by zero")
        }
    }
}

fn main() {
    println!("{}", safe_eval("42"));           // "42"
    println!("{}", safe_eval("x"));            // "Variable 'x' is not defined"
    println!("{}", safe_eval("10 / 0"));       // "Cannot divide by zero"
}
```

### Example 6: File Execution

```rust
use parlang::{parse, eval, Environment};
use std::fs;

fn run_file(path: &str) -> Result<String, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let expr = parse(&source)?;
    let env = Environment::new();
    let value = eval(&expr, &env)
        .map_err(|e| format!("{}", e))?;
    
    Ok(format!("{}", value))
}

fn main() {
    match run_file("examples/simple.par") {
        Ok(result) => println!("Result: {}", result),
        Err(err) => eprintln!("Error: {}", err),
    }
}
```

---

## Integration Patterns

### Pattern 1: Embedding in Configuration

Use ParLang for dynamic configuration:

```rust
use parlang::{parse, eval, Environment, Value};
use std::collections::HashMap;

struct Config {
    values: HashMap<String, i64>,
}

impl Config {
    fn new() -> Self {
        Config {
            values: HashMap::new(),
        }
    }
    
    fn set(&mut self, key: &str, value: i64) {
        self.values.insert(key.to_string(), value);
    }
    
    fn eval_expression(&self, expr: &str) -> Result<i64, String> {
        let ast = parse(expr)?;
        
        let mut env = Environment::new();
        for (key, value) in &self.values {
            env.bind(key.clone(), Value::Int(*value));
        }
        
        match eval(&ast, &env) {
            Ok(Value::Int(n)) => Ok(n),
            Ok(_) => Err("Expression must evaluate to an integer".to_string()),
            Err(e) => Err(format!("{}", e)),
        }
    }
}

fn main() {
    let mut config = Config::new();
    config.set("width", 1920);
    config.set("height", 1080);
    
    let area = config.eval_expression("width * height").unwrap();
    println!("Area: {}", area);  // 2073600
}
```

### Pattern 2: Sandboxed Scripting

Safe execution of user-provided scripts:

```rust
use parlang::{parse, eval, Environment, Value};
use std::time::{Duration, Instant};

struct Sandbox {
    max_depth: usize,
    timeout: Duration,
}

impl Sandbox {
    fn new() -> Self {
        Sandbox {
            max_depth: 100,
            timeout: Duration::from_secs(5),
        }
    }
    
    fn eval_safe(&self, source: &str) -> Result<Value, String> {
        let expr = parse(source)?;
        let env = Environment::new();
        
        let start = Instant::now();
        let result = eval(&expr, &env)
            .map_err(|e| format!("{}", e))?;
        
        if start.elapsed() > self.timeout {
            return Err("Evaluation timeout".to_string());
        }
        
        Ok(result)
    }
}

fn main() {
    let sandbox = Sandbox::new();
    
    match sandbox.eval_safe("let x = 42 in x * 2") {
        Ok(value) => println!("Result: {}", value),
        Err(err) => eprintln!("Error: {}", err),
    }
}
```

### Pattern 3: Expression Builder

Fluent API for building expressions:

```rust
use parlang::{Expr, BinOp};

struct ExprBuilder;

impl ExprBuilder {
    fn int(n: i64) -> Expr {
        Expr::Int(n)
    }
    
    fn bool(b: bool) -> Expr {
        Expr::Bool(b)
    }
    
    fn var(name: &str) -> Expr {
        Expr::Var(name.to_string())
    }
    
    fn add(left: Expr, right: Expr) -> Expr {
        Expr::BinOp(BinOp::Add, Box::new(left), Box::new(right))
    }
    
    fn mul(left: Expr, right: Expr) -> Expr {
        Expr::BinOp(BinOp::Mul, Box::new(left), Box::new(right))
    }
    
    fn let_in(name: &str, value: Expr, body: Expr) -> Expr {
        Expr::Let(name.to_string(), Box::new(value), Box::new(body))
    }
}

fn main() {
    use ExprBuilder::*;
    
    // Build: let x = 10 in x * 2
    let expr = let_in(
        "x",
        int(10),
        mul(var("x"), int(2))
    );
    
    println!("{}", expr);  // (let x = 10 in (x * 2))
}
```

### Pattern 4: AST Visitor

Implement a visitor pattern for AST traversal:

```rust
use parlang::Expr;

trait ExprVisitor {
    type Result;
    
    fn visit_expr(&mut self, expr: &Expr) -> Self::Result {
        match expr {
            Expr::Int(n) => self.visit_int(*n),
            Expr::Bool(b) => self.visit_bool(*b),
            Expr::Var(name) => self.visit_var(name),
            Expr::BinOp(op, left, right) => {
                self.visit_expr(left);
                self.visit_expr(right);
                self.visit_binop(op)
            }
            Expr::If(cond, then_br, else_br) => {
                self.visit_expr(cond);
                self.visit_expr(then_br);
                self.visit_expr(else_br);
                self.visit_if()
            }
            Expr::Let(name, value, body) => {
                self.visit_expr(value);
                self.visit_expr(body);
                self.visit_let(name)
            }
            Expr::Fun(param, body) => {
                self.visit_expr(body);
                self.visit_fun(param)
            }
            Expr::App(func, arg) => {
                self.visit_expr(func);
                self.visit_expr(arg);
                self.visit_app()
            }
        }
    }
    
    fn visit_int(&mut self, n: i64) -> Self::Result;
    fn visit_bool(&mut self, b: bool) -> Self::Result;
    fn visit_var(&mut self, name: &str) -> Self::Result;
    fn visit_binop(&mut self, op: &parlang::BinOp) -> Self::Result;
    fn visit_if(&mut self) -> Self::Result;
    fn visit_let(&mut self, name: &str) -> Self::Result;
    fn visit_fun(&mut self, param: &str) -> Self::Result;
    fn visit_app(&mut self) -> Self::Result;
}

// Example: Count nodes
struct NodeCounter {
    count: usize,
}

impl ExprVisitor for NodeCounter {
    type Result = ();
    
    fn visit_int(&mut self, _: i64) { self.count += 1; }
    fn visit_bool(&mut self, _: bool) { self.count += 1; }
    fn visit_var(&mut self, _: &str) { self.count += 1; }
    fn visit_binop(&mut self, _: &parlang::BinOp) { self.count += 1; }
    fn visit_if(&mut self) { self.count += 1; }
    fn visit_let(&mut self, _: &str) { self.count += 1; }
    fn visit_fun(&mut self, _: &str) { self.count += 1; }
    fn visit_app(&mut self) { self.count += 1; }
}
```

---

## API Design Patterns

### Immutability

The API favors immutable operations:

```rust
use parlang::Environment;

// Immutable extend
let env1 = Environment::new();
let env2 = env1.extend("x".to_string(), Value::Int(42));
// env1 is unchanged, env2 has the new binding
```

### Builder Pattern

Use Rust's type system for safe construction:

```rust
use parlang::{Expr, BinOp};

// Type-safe expression building
let expr = Expr::BinOp(
    BinOp::Add,
    Box::new(Expr::Int(1)),
    Box::new(Expr::Int(2)),
);
```

### Result Types

All fallible operations return `Result`:

```rust
// Parse can fail
let expr: Result<Expr, String> = parse("...");

// Eval can fail
let value: Result<Value, EvalError> = eval(&expr, &env);

// Use ? operator for error propagation
fn process() -> Result<Value, Box<dyn std::error::Error>> {
    let expr = parse("let x = 42 in x")?;
    let env = Environment::new();
    let value = eval(&expr, &env)?;
    Ok(value)
}
```

### Ownership and Borrowing

The API respects Rust's ownership rules:

```rust
// eval borrows expr and env (no ownership transfer)
let value = eval(&expr, &env);

// You can still use expr and env after eval
println!("Expr: {}", expr);
```

### Display for User Output

Types implement `Display` for end-user output:

```rust
use parlang::{Value, EvalError};

let val = Value::Int(42);
println!("Result: {}", val);  // "Result: 42"

let err = EvalError::DivisionByZero;
eprintln!("Error: {}", err);  // "Error: Division by zero"
```

### Debug for Development

Types implement `Debug` for debugging:

```rust
let expr = Expr::Int(42);
dbg!(&expr);  // Debug output with full structure
```

---

## Summary

The ParLang API provides:

- **Simple parsing**: Single `parse()` function
- **Straightforward evaluation**: Single `eval()` function
- **Type-safe AST**: Strong typing for all expressions
- **Flexible environments**: Both mutable and immutable operations
- **Comprehensive error handling**: Clear error types and messages
- **Rustic design**: Leverages Rust's type system and ownership model

### Quick Start Checklist

1. Add ParLang to `Cargo.toml`
2. Import types: `use parlang::{parse, eval, Environment};`
3. Parse source: `let expr = parse(source)?;`
4. Create environment: `let env = Environment::new();`
5. Evaluate: `let value = eval(&expr, &env)?;`
6. Display result: `println!("{}", value);`

For more examples and patterns, see:
- [Examples Guide](EXAMPLES.md)
- [Language Specification](LANGUAGE_SPEC.md)
- [Architecture Documentation](ARCHITECTURE.md)

---

## Further Reading

- **AST Module**: [MODULE_AST.md](MODULE_AST.md)
- **Parser Module**: [MODULE_PARSER.md](MODULE_PARSER.md)
- **Evaluator Module**: [MODULE_EVAL.md](MODULE_EVAL.md)
- **Language Specification**: [LANGUAGE_SPEC.md](LANGUAGE_SPEC.md)
- **Source Code**: Check `src/lib.rs`, `src/ast.rs`, `src/parser.rs`, `src/eval.rs`
