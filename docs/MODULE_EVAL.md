# Evaluator Module Documentation

## Overview

The `eval.rs` module implements the runtime evaluator/interpreter for the ParLang language. It executes ParLang Abstract Syntax Trees (ASTs) by recursively evaluating expressions and managing variable bindings through environments.

**Location**: `crates/parlang/src/eval.rs`  
**Key Exports**: `eval()` function, `Value` enum, `Environment` struct, `EvalError` enum

## Purpose

The evaluator module is responsible for:

1. **Expression Evaluation**: Executing AST nodes to produce runtime values
2. **Environment Management**: Tracking variable bindings and scopes
3. **Closure Semantics**: Implementing lexical scoping and variable capture
4. **Type Checking**: Ensuring operations are performed on compatible types
5. **Error Handling**: Detecting and reporting runtime errors
6. **Control Flow**: Implementing conditionals and function application

## Architecture

### Evaluation Pipeline

```mermaid
flowchart TD
    AST[AST Expression] --> EVAL[eval function]
    ENV[Environment] --> EVAL
    EVAL --> MATCH{Match Expr type}
    
    MATCH -->|Int/Bool| LITERAL[Return Value]
    MATCH -->|Var| LOOKUP[Environment lookup]
    MATCH -->|BinOp| BINOP[Evaluate operands<br/>Apply operator]
    MATCH -->|If| COND[Evaluate condition<br/>Choose branch]
    MATCH -->|Let| EXTEND[Evaluate value<br/>Extend environment<br/>Evaluate body]
    MATCH -->|Fun| CLOSURE[Create closure<br/>Capture environment]
    MATCH -->|App| APPLY[Evaluate function<br/>Evaluate argument<br/>Apply function]
    
    LOOKUP --> RESULT[Result<Value, EvalError>]
    BINOP --> RESULT
    COND --> RESULT
    EXTEND --> RESULT
    CLOSURE --> RESULT
    APPLY --> RESULT
    LITERAL --> RESULT
```

## Data Structures

### Runtime Values (`Value`)

The `Value` enum represents all possible runtime values in ParLang:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Char(char),
    Float(f64),
    Closure(String, Expr, Environment),
    /// Recursive closure: function name, parameter name, body, environment
    RecClosure(String, String, Expr, Environment),
    /// Tuple of values
    Tuple(Vec<Value>),
    /// Record value: field name -> value
    /// Uses `HashMap` for O(1) field access at runtime
    Record(HashMap<String, Value>),
    /// Variant value (sum type instance)
    /// Variant: (`constructor_name`, `payload_values`)
    /// e.g., Some(42) -> Variant("Some", vec![Int(42)])
    ///       None -> Variant("None", vec![])
    ///       Cons(1, rest) -> Variant("Cons", vec![Int(1), <list>])
    Variant(String, Vec<Value>),
}
```

#### Value Types Hierarchy

```mermaid
classDiagram
    class Value {
        <<enumeration>>
        Int(i64)
        Bool(bool)
        Closure(String, Expr, Environment)
    }
    
    class Int {
        i64 value
    }
    
    class Bool {
        bool value
    }
    
    class Closure {
        String parameter
        Expr body
        Environment captured_env
    }
    
    Value --> Int
    Value --> Bool
    Value --> Closure
    Closure --> Environment : captures
```

#### 1. `Int(i64)` - Integer Value

Represents integer values at runtime.

**Range**: -9,223,372,036,854,775,808 to 9,223,372,036,854,775,807 (64-bit signed)

**Example**:
```rust
Value::Int(42)
Value::Int(-10)
Value::Int(0)
```

**Display**: `42`, `-10`, `0`

#### 2. `Bool(bool)` - Boolean Value

Represents boolean values at runtime.

**Values**: `true`, `false`

**Example**:
```rust
Value::Bool(true)
Value::Bool(false)
```

**Display**: `true`, `false`

#### 3. `Closure(String, Expr, Environment)` - Function Closure

Represents a function value with its captured environment.

**Components**:
- **Parameter name** (`String`): The function's parameter
- **Body** (`Expr`): The function's body expression
- **Captured environment** (`Environment`): Variable bindings from the definition site

**Example**:
```rust
// fun x -> x + 1
Value::Closure(
    "x".to_string(),
    Expr::BinOp(
        BinOp::Add,
        Box::new(Expr::Var("x".to_string())),
        Box::new(Expr::Int(1))
    ),
    env
)
```

**Display**: `<function x>` (shows parameter name, hides body)

**Key Feature**: Closures capture their defining environment, enabling lexical scoping.

#### 4. `RecClosure(String, String, Expr, Environment)` - Recursive Function Closure

Represents a recursive function value that can reference itself by name.

**Components**:
- **Function name** (`String`): The recursive function's name (for self-reference)
- **Parameter name** (`String`): The function's parameter
- **Body** (`Expr`): The function's body expression
- **Captured environment** (`Environment`): Variable bindings from the definition site

**Example**:
```rust
// rec factorial -> fun n -> if n == 0 then 1 else n * factorial (n - 1)
Value::RecClosure(
    "factorial".to_string(),
    "n".to_string(),
    Expr::If(
        Box::new(Expr::BinOp(BinOp::Eq, ...)),
        Box::new(Expr::Int(1)),
        Box::new(Expr::BinOp(BinOp::Mul, ...))
    ),
    env
)
```

**Display**: `<recursive function factorial>` (shows function name, hides body)

**Key Features**:
- Self-reference: The function name is bound within its own body
- Tail call optimization: Direct tail calls use iteration instead of recursion
- Lexical scoping: Captures the environment at definition time

### Environment Management

#### Environment Structure

The current frame stores these fields:

```rust
bindings: HashMap<String, Value>,
constructors: HashMap<String, ConstructorInfo>,
parent: Option<Rc<Environment>>,
```

There is also a `fallback: Option<Rc<Environment>>` field. The Environment is a chain of immutable frames. Each frame owns its own `bindings` and `constructors` maps; ancestor frames are shared via `Rc` and never mutated. A `parent` link points to the enclosing frame; a merged environment additionally carries a `fallback` parent. Because frames only point to ancestors, no `Rc` cycle can form. Two environments with identical reachable bindings but different frame shapes compare unequal. See `Environment` in `crates/parlang/src/eval.rs`.

```mermaid
graph TD
    F2[frame: y → Bool true] -->|parent| F1[frame: x → Int 42]
    F1 -->|parent| F0[frame: empty]
```

#### Environment Operations

##### `new()` - Create Empty Environment

Creates a new environment with no bindings.

```rust
bindings: HashMap::new(),
```

`new()` also leaves `constructors` empty with `parent`/`fallback` as `None`. See `new` in `crates/parlang/src/eval.rs`.

**Usage**:
```rust
let env = Environment::new();
```

##### `bind()` - Add Binding (Mutable)

Adds or updates a variable binding in the environment.

```rust
pub fn bind(&mut self, name: String, value: Value) {
    self.bindings.insert(name, value);
}
```

**Usage**:
```rust
let mut env = Environment::new();
env.bind("x".to_string(), Value::Int(42));
```

**Behavior**: Inserts into *this* frame's bindings only. If the same name already exists in this frame it is replaced; bindings in ancestor frames are untouched.

##### `lookup()` - Find Variable

Looks up a variable's value in the environment.

```rust
self.bindings.get(name).or_else(|| {
```

`lookup` probes this frame's `bindings`, and on a miss recurses into `parent`, then into `fallback`. Each frame visited costs one `HashMap` probe. For a plain parent chain this is linear in the chain length; a merged environment has two links, and a miss may visit shared ancestor frames more than once, so the bound is the number of frames visited, not the chain length. See `lookup` in `crates/parlang/src/eval.rs`.

**Returns**:
- `Some(&Value)`: Variable found
- `None`: Variable not found

**Usage**:
```rust
match env.lookup("x") {
    Some(val) => println!("x = {}", val),
    None => println!("x not found"),
}
```

##### `extend()` - Create Extended Environment (Immutable)

Creates a new environment with an additional binding, leaving the original unchanged.

```rust
parent: Some(Rc::new(self.clone())),
```

`extend` builds a fresh innermost frame holding only the one new binding, with `constructors` empty and `fallback: None`, linking the current environment as `parent`. The `self.clone()` copies only the current frame's own two maps; ancestors are shared through the cloned `Rc` pointer, so the cost is O(|frame|) in the size of the innermost frame, not O(whole environment). See `extend` in `crates/parlang/src/eval.rs`.

**Usage**:
```rust
let env1 = Environment::new();
let env2 = env1.extend("x".to_string(), Value::Int(42));
// env1 unchanged, env2 has x binding
```

**Key Feature**: Enables immutable environment handling for closures while sharing ancestors via `Rc` rather than copying them.

##### `merge()` - Combine Two Environments (Immutable)

```rust
parent: Some(Rc::new(other.clone())),
fallback: Some(Rc::new(self.clone())),
```

`merge` builds a fresh empty frame whose `parent` is `other` and whose `fallback` is `self`; because `parent` is searched before `fallback`, `other`'s bindings are found first and shadow `self`. It backs `load` evaluation and binding extraction, giving loaded library bindings last-writer-wins precedence. See `merge` in `crates/parlang/src/eval.rs`.

```mermaid
graph TD
    MERGED[frame: empty] -->|parent| OTHER[other]
    MERGED -->|fallback| SELF[self]
```

#### Environment Evolution Example

For `let x = 5 in let y = 10 in let x = 20 in ...`, each `let` pushes a new frame. The outer `x → 5` is **not** overwritten — it survives in an ancestor frame and is simply shadowed, so `lookup("x")` finds `x → 20` first.

```mermaid
graph TD
    FX2[frame: x → 20] -->|parent| FY[frame: y → 10]
    FY -->|parent| FX1[frame: x → 5]
```

### Evaluation Errors (`EvalError`)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    UnboundVariable(String),
    TypeError(String),
    DivisionByZero,
    LoadError(String),
    IndexOutOfBounds(String),
    /// Field not found in record: field name, available fields
    FieldNotFound(String, Vec<String>),
    /// Expected record but got a different type
    RecordExpected(String),
    /// Unknown constructor
    UnknownConstructor(String),
    /// Constructor arity mismatch: name, expected, got
    ConstructorArityMismatch(String, usize, usize),
    /// Pattern match is non-exhaustive
    PatternMatchNonExhaustive,
    /// Evaluation exceeded the non-tail recursion policy limit.
    RecursionLimit,
}
```

#### Error Types

##### 1. `UnboundVariable(String)`

Occurs when a variable is referenced but not defined in the environment.

**Example**:
```rust
// Source: x + 1  (x not defined)
EvalError::UnboundVariable("x".to_string())
```

**Display**: `"Unbound variable: x"`

##### 2. `TypeError(String)`

Occurs when an operation is performed on incompatible types.

**Examples**:
```rust
// Adding integer and boolean
// Source: 1 + true
EvalError::TypeError("Type error in binary operation: Add".to_string())

// Applying non-function
// Source: 42 10
EvalError::TypeError("Application requires a function".to_string())

// Non-boolean condition
// Source: if 42 then 1 else 2
EvalError::TypeError("If condition must be a boolean".to_string())
```

**Display**: `"Type error: <message>"`

##### 3. `DivisionByZero`

Occurs when dividing by zero.

**Example**:
```rust
// Source: 10 / 0
EvalError::DivisionByZero
```

**Display**: `"Division by zero"`

## Core Evaluation Function

### `eval()` - Main Evaluator

```rust
pub fn eval(expr: &Expr, env: &Environment) -> Result<Value, EvalError>
```

The main evaluation function recursively evaluates expressions in an environment.

**Parameters**:
- `expr`: AST expression to evaluate
- `env`: Current environment with variable bindings

**Returns**:
- `Ok(Value)`: Evaluation successful
- `Err(EvalError)`: Runtime error occurred

### Evaluation Algorithm by Expression Type

#### 1. Integer Literal (`Expr::Int`)

```rust
Expr::Int(n) => Ok(Value::Int(*n))
```

**Algorithm**:
1. Return the integer value directly

**Example**:
```rust
eval(&Expr::Int(42), &env)  // => Ok(Value::Int(42))
```

**Complexity**: O(1)

#### 2. Boolean Literal (`Expr::Bool`)

```rust
Expr::Bool(b) => Ok(Value::Bool(*b))
```

**Algorithm**:
1. Return the boolean value directly

**Example**:
```rust
eval(&Expr::Bool(true), &env)  // => Ok(Value::Bool(true))
```

**Complexity**: O(1)

#### 3. Variable Reference (`Expr::Var`)

```rust
Expr::Var(name) => env
    .lookup(name)
    .cloned()
    .ok_or_else(|| EvalError::UnboundVariable(name.clone()))
```

**Algorithm**:
1. Look up variable name in environment
2. If found, return cloned value
3. If not found, return `UnboundVariable` error

**Example**:
```rust
// With x bound to 42
eval(&Expr::Var("x".to_string()), &env)  // => Ok(Value::Int(42))

// With x unbound
eval(&Expr::Var("x".to_string()), &env)  // => Err(UnboundVariable("x"))
```

**Complexity**: O(f), where f is the number of frames visited: one `HashMap` probe per frame along the parent chain and, for merged environments, the fallback chain (see `lookup`).

#### 4. Binary Operation (`Expr::BinOp`)

```rust
Expr::BinOp(op, left, right) => {
    let left_val = eval(left, env)?;
    let right_val = eval(right, env)?;
    eval_binop(*op, left_val, right_val)
}
```

**Algorithm**:
1. Evaluate left operand
2. Evaluate right operand
3. Apply binary operator to values
4. Return result or type error

**Evaluation Order**: Left-to-right (strict evaluation)

**Example**:
```rust
// 1 + 2
let expr = Expr::BinOp(
    BinOp::Add,
    Box::new(Expr::Int(1)),
    Box::new(Expr::Int(2))
);
eval(&expr, &env)  // => Ok(Value::Int(3))
```

**Complexity**: O(eval(left) + eval(right) + O(1))

#### 5. Conditional (`Expr::If`)

```rust
Expr::If(cond, then_branch, else_branch) => {
    let cond_val = eval(cond, env)?;
    match cond_val {
        Value::Bool(true) => eval(then_branch, env),
        Value::Bool(false) => eval(else_branch, env),
        _ => Err(EvalError::TypeError(
            "If condition must be a boolean".to_string(),
        )),
    }
}
```

**Algorithm**:
1. Evaluate condition expression
2. Check if condition is a boolean
3. If `true`, evaluate then-branch
4. If `false`, evaluate else-branch
5. If not boolean, return type error

**Example**:
```rust
// if true then 1 else 2
let expr = Expr::If(
    Box::new(Expr::Bool(true)),
    Box::new(Expr::Int(1)),
    Box::new(Expr::Int(2))
);
eval(&expr, &env)  // => Ok(Value::Int(1))
```

**Key Feature**: Only evaluates the chosen branch (lazy branches)

**Complexity**: O(eval(cond) + eval(chosen_branch))

#### 6. Let Binding (`Expr::Let`)

```rust
let val = eval_inner(value, env, depth + 1)?;
let new_env = env.extend(name.clone(), val);
eval_inner(body, &new_env, depth + 1)
```

This is the `Expr::Let` arm of `eval_inner`. Each nested evaluation runs one level deeper, so the `depth` guard turns runaway recursion into `EvalError::RecursionLimit`. The type annotation is ignored at run time. When a recursive (`rec`) function is applied to all of its parameters, its body runs under the tail-call loop `eval_with_tco`, and a `let` reached by that loop is handled by the loop's own `Let` arm: it extends the loop's current environment and continues with the body instead of recursing, and falls back to `eval_inner` if the binding shadows the recursive function's name. See `eval_inner` and `eval_with_tco` in `crates/parlang/src/eval.rs`.

**Algorithm**:
1. Evaluate the value expression in current environment
2. Create new environment extending current one with binding
3. Evaluate body in extended environment
4. Return body's result

**Scoping**: Variable is only visible in the body

**Example**:
```rust
// let x = 42 in x + 1
let expr = Expr::Let(
    "x".to_string(),
    None,
    Box::new(Expr::Int(42)),
    Box::new(Expr::BinOp(
        BinOp::Add,
        Box::new(Expr::Var("x".to_string())),
        Box::new(Expr::Int(1))
    ))
);
eval(&expr, &env)  // => Ok(Value::Int(43))
```

**Complexity**: O(eval(value) + |frame| + eval(body))
- `|frame|` is the size of the current (innermost) frame: `extend` creates a new single-binding frame whose parent is a copy of the current frame behind a new `Rc`, so ancestor frames are shared rather than copied

#### 7. Function Definition (`Expr::Fun`)

```rust
Ok(Value::Closure(param.clone(), (**body).clone(), env.clone()))
```

This is the `Expr::Fun` arm of `eval_inner` (the recursive worker behind `eval`). It builds a closure from the parameter name, a copy of the body and a clone of the current environment. Any parameter type annotation is ignored at run time; the type checker has already used it. See `eval_inner` in `crates/parlang/src/eval.rs`.

**Algorithm**:
1. Create closure capturing parameter name
2. Capture function body
3. **Capture current environment** (lexical scoping)
4. Return closure value

**Key Feature**: Environment is captured at definition time, not call time.

**Example**:
```rust
// fun x -> x + 1
let expr = Expr::Fun(
    "x".to_string(),
    None,
    Box::new(Expr::BinOp(
        BinOp::Add,
        Box::new(Expr::Var("x".to_string())),
        Box::new(Expr::Int(1))
    ))
);
eval(&expr, &env)  // => Ok(Value::Closure("x", body, env))
```

**Complexity**: O(|frame| + |body|) - `env.clone()` copies only the current (innermost) frame's own maps and shares ancestor frames via `Rc`, and the body expression is cloned into the closure

#### 8. Function Application (`Expr::App`)

```rust
Expr::App(_, _) => eval_app(expr, env, depth),
```

Application is handled by `eval_app`. See `eval_app` in `crates/parlang/src/eval.rs`.

**Algorithm**:
1. Collect the application spine: the head expression and all argument expressions (`collect_app_spine`)
2. Evaluate the head (must yield a `Closure` or `RecClosure`)
3. Evaluate the argument expressions left to right (`eval_spine_args`)
4. For a `Closure`: extend its **captured environment** with the parameter binding, evaluate the body one level deeper, and use the result as the function for the next argument
5. For a `RecClosure`: hand all remaining arguments to `apply_rec_closure`, which runs the body under the tail-call loop `eval_with_tco`
6. Return the final result

**Key Feature**: Uses closure's captured environment, not current environment.

**Example**:
```rust
// (fun x -> x + 1) 41
let expr = Expr::App(
    Box::new(Expr::Fun(
        "x".to_string(),
        None,
        Box::new(Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Var("x".to_string())),
            Box::new(Expr::Int(1))
        ))
    )),
    Box::new(Expr::Int(41))
);
eval(&expr, &env)  // => Ok(Value::Int(42))
```

**Complexity**: O(eval(head) + Σ eval(argᵢ) + Σ (|closure frame| + eval(body))) over the spine; recursive application additionally runs the tail-call loop (see `apply_rec_closure`).

### Evaluation Flow Diagram

```mermaid
graph TD
    START[eval expr, env] --> TYPE{Expression Type}
    
    TYPE -->|Int n| INT[Return Int n]
    TYPE -->|Bool b| BOOL[Return Bool b]
    TYPE -->|Var name| VAR[Lookup in env]
    
    VAR -->|Found| RETURN1[Return value]
    VAR -->|Not found| ERROR1[UnboundVariable error]
    
    TYPE -->|BinOp op left right| BINOP_START[Eval left]
    BINOP_START --> BINOP_MID[Eval right]
    BINOP_MID --> BINOP_END[Apply operator]
    BINOP_END -->|Success| RETURN2[Return result]
    BINOP_END -->|Type error| ERROR2[TypeError]
    
    TYPE -->|If cond then else| IF_COND[Eval condition]
    IF_COND -->|Bool true| IF_THEN[Eval then-branch]
    IF_COND -->|Bool false| IF_ELSE[Eval else-branch]
    IF_COND -->|Not bool| ERROR3[TypeError]
    IF_THEN --> RETURN3[Return result]
    IF_ELSE --> RETURN3
    
    TYPE -->|Let name val body| LET_VAL[Eval value]
    LET_VAL --> LET_EXTEND[Extend env with binding]
    LET_EXTEND --> LET_BODY[Eval body in new env]
    LET_BODY --> RETURN4[Return result]
    
    TYPE -->|Fun param body| FUN_CLOSE[Capture environment]
    FUN_CLOSE --> FUN_RET[Return Closure]
    
    TYPE -->|App func arg| APP_HEAD[Eval head]
    APP_HEAD --> APP_ARGS[Eval all args L→R]
    APP_ARGS --> APP_DISPATCH{Closure / RecClosure / other?}
    APP_DISPATCH -->|Closure| APP_EXTEND[extend closure frame]
    APP_EXTEND --> APP_BODY[Eval body]
    APP_BODY --> RETURN5[Return result]
    APP_DISPATCH -->|RecClosure| APP_REC[apply_rec_closure + TCO loop]
    APP_REC --> RETURN5
    APP_DISPATCH -->|other| ERROR4[TypeError]
```

## Binary Operation Evaluation

### `eval_binop()` - Binary Operator Application

```rust
fn eval_binop(op: BinOp, left: Value, right: Value) -> Result<Value, EvalError>
```

Applies a binary operator to two runtime values.

### Arithmetic Operations

```mermaid
graph TD
    ARITH[Arithmetic Operation]
    ARITH --> CHECK{Both Int?}
    CHECK -->|Yes| APPLY{Which op?}
    CHECK -->|No| ERROR[TypeError]
    
    APPLY -->|Add| ADD[a + b]
    APPLY -->|Sub| SUB[a - b]
    APPLY -->|Mul| MUL[a * b]
    APPLY -->|Div| DIV{b == 0?}
    
    DIV -->|Yes| DIV_ERROR[DivisionByZero]
    DIV -->|No| DIV_OK[a / b]
    
    ADD --> RESULT[Int result]
    SUB --> RESULT
    MUL --> RESULT
    DIV_OK --> RESULT
```

#### Addition (`BinOp::Add`)

```rust
(BinOp::Add, Value::Int(a), Value::Int(b)) => a
    .checked_add(b)
    .map(Value::Int)
    .ok_or_else(|| EvalError::TypeError("Integer overflow in addition".to_string()))
```

**Type**: `Int → Int → Int`

**Example**: `5 + 3` → `Value::Int(8)`

#### Subtraction (`BinOp::Sub`)

```rust
(BinOp::Sub, Value::Int(a), Value::Int(b)) => a
    .checked_sub(b)
    .map(Value::Int)
    .ok_or_else(|| EvalError::TypeError("Integer overflow in subtraction".to_string()))
```

**Type**: `Int → Int → Int`

**Example**: `10 - 3` → `Value::Int(7)`

#### Multiplication (`BinOp::Mul`)

```rust
(BinOp::Mul, Value::Int(a), Value::Int(b)) => a
    .checked_mul(b)
    .map(Value::Int)
    .ok_or_else(|| EvalError::TypeError("Integer overflow in multiplication".to_string()))
```

**Type**: `Int → Int → Int`

**Example**: `6 * 7` → `Value::Int(42)`

#### Division (`BinOp::Div`)

```rust
(BinOp::Div, Value::Int(a), Value::Int(b)) => {
    if b == 0 {
        Err(EvalError::DivisionByZero)
    } else {
        Ok(Value::Int(a / b))
    }
}
```

**Type**: `Int → Int → Int`

**Special Case**: Division by zero returns error

**Example**: 
- `10 / 2` → `Value::Int(5)`
- `10 / 0` → `Err(DivisionByZero)`

**Note**: Integer division truncates towards zero

### Comparison Operations

```mermaid
graph TD
    CMP[Comparison Operation]
    CMP --> TYPE{Operand types?}
    
    TYPE -->|Int, Int| INT_CMP{Which op?}
    TYPE -->|Bool, Bool| BOOL_CMP{Which op?}
    TYPE -->|Mixed| ERROR[TypeError]
    
    INT_CMP -->|Eq| INT_EQ[a == b]
    INT_CMP -->|Neq| INT_NEQ[a != b]
    INT_CMP -->|Lt| INT_LT[a < b]
    INT_CMP -->|Le| INT_LE[a <= b]
    INT_CMP -->|Gt| INT_GT[a > b]
    INT_CMP -->|Ge| INT_GE[a >= b]
    
    BOOL_CMP -->|Eq| BOOL_EQ[a == b]
    BOOL_CMP -->|Neq| BOOL_NEQ[a != b]
    BOOL_CMP -->|Other| ERROR
    
    INT_EQ --> RESULT[Bool result]
    INT_NEQ --> RESULT
    INT_LT --> RESULT
    INT_LE --> RESULT
    INT_GT --> RESULT
    INT_GE --> RESULT
    BOOL_EQ --> RESULT
    BOOL_NEQ --> RESULT
```

#### Integer Comparisons

All six comparison operators work on integers:

| Operator | Type | Example | Result |
|----------|------|---------|--------|
| `Eq` | `Int → Int → Bool` | `5 == 5` | `Bool(true)` |
| `Neq` | `Int → Int → Bool` | `5 != 3` | `Bool(true)` |
| `Lt` | `Int → Int → Bool` | `3 < 5` | `Bool(true)` |
| `Le` | `Int → Int → Bool` | `5 <= 5` | `Bool(true)` |
| `Gt` | `Int → Int → Bool` | `5 > 3` | `Bool(true)` |
| `Ge` | `Int → Int → Bool` | `5 >= 5` | `Bool(true)` |

#### Boolean Comparisons

Only equality operators work on booleans:

| Operator | Type | Example | Result |
|----------|------|---------|--------|
| `Eq` | `Bool → Bool → Bool` | `true == true` | `Bool(true)` |
| `Neq` | `Bool → Bool → Bool` | `true != false` | `Bool(true)` |

**Other operators** on booleans result in `TypeError`.

## Closure Semantics

### Lexical Scoping

ParLang implements **lexical scoping** (static scoping) where functions capture their defining environment.

In the diagrams and examples below, bracket notation such as `[x → 10, y → 5]` lists the bindings *visible* from the innermost frame; each binding actually lives in its own frame of the chain described in [Environment Structure](#environment-structure).

```mermaid
sequenceDiagram
    participant Code as Source Code
    participant Eval as Evaluator
    participant Env1 as Environment (outer)
    participant Closure as Closure Value
    participant Env2 as Environment (inner)
    
    Code->>Eval: let x = 10 in fun y -> x + y
    Eval->>Env1: Create env with x=10
    Eval->>Eval: Evaluate (fun y -> x + y)
    Eval->>Closure: Create closure, capture Env1
    Note over Closure: Closure stores:<br/>param=y<br/>body=x+y<br/>env=[x→10]
    
    Code->>Eval: Apply closure to 5
    Eval->>Env2: Extend captured env with y=5
    Note over Env2: [x→10, y→5]
    Eval->>Eval: Evaluate x + y in Env2
    Eval-->>Code: Result: 15
```

### Example: Variable Capture

```rust
// let x = 10 in (fun y -> x + y) 32

// Step 1: Evaluate let binding
let env0 = Environment::new();
// Evaluate value: 10
let val = Value::Int(10);
// Extend environment
let env1 = env0.extend("x".to_string(), val);
// Now env1 = [x → 10]

// Step 2: Evaluate function definition
// Create closure capturing env1
let closure = Value::Closure(
    "y".to_string(),
    body,  // x + y
    env1.clone()  // Captures [x → 10]
);

// Step 3: Apply function to 32
// Extend closure's captured environment
let env2 = env1.extend("y".to_string(), Value::Int(32));
// Now env2 = [x → 10, y → 32]

// Step 4: Evaluate body (x + y) in env2
// x = 10, y = 32
// Result: 42
```

### Example: Closure Capturing Closure

```rust
// let add = fun x -> fun y -> x + y in 
// let add5 = add 5 in 
// add5 10

// Creates outer closure capturing x
let outer_closure = Value::Closure(
    "x",
    fun y -> x + y,
    []
);

// Apply outer_closure to 5
// Creates inner closure capturing [x → 5]
let inner_closure = Value::Closure(
    "y",
    x + y,
    [x → 5]
);

// Apply inner_closure to 10
// Evaluates in [x → 5, y → 10]
// Result: 15
```

### Shadowing Example

```rust
// let x = 1 in let x = 2 in x

// Step 1: First let
let env1 = env0.extend("x", Value::Int(1));
// env1 has an inner frame holding x → 1

// Step 2: Second let (shadows x)
let env2 = env1.extend("x", Value::Int(2));
// The inner frame holds x → 2 while the parent frame still holds x → 1

// Step 3: Lookup x
// lookup("x") walks inner-to-outer and returns 2; the outer x → 1 remains live
// (not discarded) in the ancestor frame.
```

## Detailed Evaluation Examples

### Example 1: Simple Arithmetic

```rust
// Source: (1 + 2) * 3

let expr = Expr::BinOp(
    BinOp::Mul,
    Box::new(Expr::BinOp(
        BinOp::Add,
        Box::new(Expr::Int(1)),
        Box::new(Expr::Int(2))
    )),
    Box::new(Expr::Int(3))
);

// Evaluation trace:
eval(&expr, &env)
├─ eval(BinOp(Mul, left, right), env)
│  ├─ eval(BinOp(Add, 1, 2), env)
│  │  ├─ eval(Int(1), env) → Value::Int(1)
│  │  ├─ eval(Int(2), env) → Value::Int(2)
│  │  └─ eval_binop(Add, Int(1), Int(2)) → Value::Int(3)
│  ├─ eval(Int(3), env) → Value::Int(3)
│  └─ eval_binop(Mul, Int(3), Int(3)) → Value::Int(9)
└─ Result: Ok(Value::Int(9))
```

### Example 2: Conditional

```rust
// Source: if 5 > 3 then 100 else 0

let expr = Expr::If(
    Box::new(Expr::BinOp(
        BinOp::Gt,
        Box::new(Expr::Int(5)),
        Box::new(Expr::Int(3))
    )),
    Box::new(Expr::Int(100)),
    Box::new(Expr::Int(0))
);

// Evaluation trace:
eval(&expr, &env)
├─ eval(If(cond, then, else), env)
│  ├─ eval(BinOp(Gt, 5, 3), env)
│  │  ├─ eval(Int(5), env) → Value::Int(5)
│  │  ├─ eval(Int(3), env) → Value::Int(3)
│  │  └─ eval_binop(Gt, Int(5), Int(3)) → Value::Bool(true)
│  ├─ Condition is Bool(true), evaluate then-branch
│  └─ eval(Int(100), env) → Value::Int(100)
└─ Result: Ok(Value::Int(100))

// Note: else-branch (Int(0)) is NOT evaluated
```

### Example 3: Let Binding with Shadowing

```rust
// Source: let x = 1 in let x = x + 1 in x

let expr = Expr::Let(
    "x".to_string(),
    None,
    Box::new(Expr::Int(1)),
    Box::new(Expr::Let(
        "x".to_string(),
        None,
        Box::new(Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Var("x".to_string())),
            Box::new(Expr::Int(1))
        )),
        Box::new(Expr::Var("x".to_string()))
    ))
);

// Evaluation trace:
eval(&expr, env0=[])
├─ eval(Let(x, 1, body), env0=[])
│  ├─ eval(Int(1), env0=[]) → Value::Int(1)
│  ├─ Extend env0 with x → 1: env1=[x→1]
│  └─ eval(Let(x, x+1, x), env1=[x→1])
│     ├─ eval(BinOp(Add, x, 1), env1=[x→1])
│     │  ├─ eval(Var(x), env1=[x→1]) → Value::Int(1)
│     │  ├─ eval(Int(1), env1=[x→1]) → Value::Int(1)
│     │  └─ eval_binop(Add, Int(1), Int(1)) → Value::Int(2)
│     ├─ Extend env1 with x → 2: env2=[x→2]
│     └─ eval(Var(x), env2=[x→2]) → Value::Int(2)
└─ Result: Ok(Value::Int(2))
```

### Example 4: Closure with Captured Variable

```rust
// Source: let x = 10 in (fun y -> x + y) 32

let expr = Expr::Let(
    "x".to_string(),
    None,
    Box::new(Expr::Int(10)),
    Box::new(Expr::App(
        Box::new(Expr::Fun(
            "y".to_string(),
            None,
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Var("x".to_string())),
                Box::new(Expr::Var("y".to_string()))
            ))
        )),
        Box::new(Expr::Int(32))
    ))
);

// Evaluation trace:
eval(&expr, env0=[])
├─ eval(Let(x, 10, body), env0=[])
│  ├─ eval(Int(10), env0=[]) → Value::Int(10)
│  ├─ Extend env0 with x → 10: env1=[x→10]
│  └─ eval(App(fun, 32), env1=[x→10])
│     ├─ eval(Fun(y, x+y), env1=[x→10])
│     │  └─ Create Closure(y, x+y, env1=[x→10])
│     ├─ eval(Int(32), env1=[x→10]) → Value::Int(32)
│     ├─ Extract closure: param=y, body=x+y, closure_env=[x→10]
│     ├─ Extend closure_env with y → 32: env2=[x→10, y→32]
│     └─ eval(BinOp(Add, x, y), env2=[x→10, y→32])
│        ├─ eval(Var(x), env2) → Value::Int(10)
│        ├─ eval(Var(y), env2) → Value::Int(32)
│        └─ eval_binop(Add, Int(10), Int(32)) → Value::Int(42)
└─ Result: Ok(Value::Int(42))
```

### Example 5: Curried Function

```rust
// Source: (fun x -> fun y -> x + y) 40 2

let expr = Expr::App(
    Box::new(Expr::App(
        Box::new(Expr::Fun(
            "x".to_string(),
            None,
            Box::new(Expr::Fun(
                "y".to_string(),
                None,
                Box::new(Expr::BinOp(
                    BinOp::Add,
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Var("y".to_string()))
                ))
            ))
        )),
        Box::new(Expr::Int(40))
    )),
    Box::new(Expr::Int(2))
);

// Evaluation trace:
eval(&expr, env0=[])
├─ eval(App(App(outer_fun, 40), 2), env0=[])
│  ├─ eval(App(outer_fun, 40), env0=[])
│  │  ├─ eval(Fun(x, inner_fun), env0=[])
│  │  │  └─ Closure(x, Fun(y, x+y), env0=[])
│  │  ├─ eval(Int(40), env0=[]) → Value::Int(40)
│  │  ├─ Extend [] with x → 40: env1=[x→40]
│  │  └─ eval(Fun(y, x+y), env1=[x→40])
│  │     └─ Closure(y, x+y, env1=[x→40])
│  ├─ eval(Int(2), env0=[]) → Value::Int(2)
│  ├─ Extend [x→40] with y → 2: env2=[x→40, y→2]
│  └─ eval(BinOp(Add, x, y), env2=[x→40, y→2])
│     ├─ eval(Var(x), env2) → Value::Int(40)
│     ├─ eval(Var(y), env2) → Value::Int(2)
│     └─ eval_binop(Add, Int(40), Int(2)) → Value::Int(42)
└─ Result: Ok(Value::Int(42))
```

## Error Handling Examples

### Example 1: Unbound Variable

```rust
// Source: x + 1  (x not defined)

eval(&Expr::BinOp(
    BinOp::Add,
    Box::new(Expr::Var("x".to_string())),
    Box::new(Expr::Int(1))
), &Environment::new())

// Evaluation trace:
├─ eval(BinOp(Add, x, 1), [])
│  ├─ eval(Var(x), [])
│  │  └─ lookup("x") → None
│  │  └─ Error: UnboundVariable("x")
│  └─ Propagate error
└─ Result: Err(UnboundVariable("x"))
```

### Example 2: Type Error - Wrong Operand Type

```rust
// Source: 1 + true

eval(&Expr::BinOp(
    BinOp::Add,
    Box::new(Expr::Int(1)),
    Box::new(Expr::Bool(true))
), &Environment::new())

// Evaluation trace:
├─ eval(BinOp(Add, 1, true), [])
│  ├─ eval(Int(1), []) → Value::Int(1)
│  ├─ eval(Bool(true), []) → Value::Bool(true)
│  └─ eval_binop(Add, Int(1), Bool(true))
│     └─ Pattern doesn't match any valid operation
│     └─ Error: TypeError
└─ Result: Err(TypeError("Type error in binary operation: Add"))
```

### Example 3: Type Error - Non-Boolean Condition

```rust
// Source: if 42 then 1 else 2

eval(&Expr::If(
    Box::new(Expr::Int(42)),
    Box::new(Expr::Int(1)),
    Box::new(Expr::Int(2))
), &Environment::new())

// Evaluation trace:
├─ eval(If(42, 1, 2), [])
│  ├─ eval(Int(42), []) → Value::Int(42)
│  ├─ Match on condition value
│  │  └─ Not Bool(true) or Bool(false)
│  └─ Error: TypeError
└─ Result: Err(TypeError("If condition must be a boolean"))
```

### Example 4: Type Error - Applying Non-Function

```rust
// Source: 42 10

eval(&Expr::App(
    Box::new(Expr::Int(42)),
    Box::new(Expr::Int(10))
), &Environment::new())

// Evaluation trace:
├─ eval(App(42, 10), [])
│  ├─ eval(Int(42), []) → Value::Int(42)
│  ├─ eval(Int(10), []) → Value::Int(10)
│  ├─ Match func_val
│  │  └─ Int(42), not Closure
│  └─ Error: TypeError
└─ Result: Err(TypeError("Application requires a function"))
```

### Example 5: Division by Zero

```rust
// Source: 10 / 0

eval(&Expr::BinOp(
    BinOp::Div,
    Box::new(Expr::Int(10)),
    Box::new(Expr::Int(0))
), &Environment::new())

// Evaluation trace:
├─ eval(BinOp(Div, 10, 0), [])
│  ├─ eval(Int(10), []) → Value::Int(10)
│  ├─ eval(Int(0), []) → Value::Int(0)
│  └─ eval_binop(Div, Int(10), Int(0))
│     └─ Check: b == 0? Yes
│     └─ Error: DivisionByZero
└─ Result: Err(DivisionByZero)
```

## Testing

The evaluator module includes comprehensive unit tests:

### Test Categories

1. **Basic Evaluation**: Literals, simple expressions
2. **Arithmetic Operations**: All arithmetic operators
3. **Comparison Operations**: Integer and boolean comparisons
4. **Conditional Expressions**: If-then-else, condition checking
5. **Let Bindings**: Simple, nested, shadowing
6. **Functions and Closures**: Creation, application, currying, capture
7. **Type Errors**: Invalid operations
8. **Environment Tests**: Operations on environments
9. **Display Tests**: Value and error formatting
10. **Value Tests**: Cloning and equality
11. **Complex Scenarios**: Real-world programs
12. **Error Display**: Error message formatting

### Example Tests

#### Test: Simple Function Application

```rust
#[test]
fn test_eval_fun_and_app() {
    let env = Environment::new();
    // (fun x -> x + 1) 41
    let expr = Expr::App(
        Box::new(Expr::Fun(
            "x".to_string(),
            None,
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Var("x".to_string())),
                Box::new(Expr::Int(1)),
            )),
        )),
        Box::new(Expr::Int(41)),
    );
    assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
}
```

#### Test: Closure Captures Environment

```rust
#[test]
fn test_eval_closure_captures_environment() {
    let env = Environment::new();
    // let x = 10 in (fun y -> x + y) 32
    let expr = Expr::Let(
        "x".to_string(),
        None,
        Box::new(Expr::Int(10)),
        Box::new(Expr::App(
            Box::new(Expr::Fun(
                "y".to_string(),
                None,
                Box::new(Expr::BinOp(
                    BinOp::Add,
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Var("y".to_string())),
                )),
            )),
            Box::new(Expr::Int(32)),
        )),
    );
    assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
}
```

#### Test: Division by Zero

```rust
#[test]
fn test_eval_div_by_zero() {
    let env = Environment::new();
    let expr = Expr::BinOp(
        BinOp::Div,
        Box::new(Expr::Int(42)),
        Box::new(Expr::Int(0)),
    );
    assert_eq!(eval(&expr, &env), Err(EvalError::DivisionByZero));
}
```

#### Test: Complex Nested Expression

```rust
#[test]
fn test_eval_complex_nested() {
    let env = Environment::new();
    // let double = fun x -> x + x in double 21
    let expr = Expr::Let(
        "double".to_string(),
        None,
        Box::new(Expr::Fun(
            "x".to_string(),
            None,
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Var("x".to_string())),
                Box::new(Expr::Var("x".to_string())),
            )),
        )),
        Box::new(Expr::App(
            Box::new(Expr::Var("double".to_string())),
            Box::new(Expr::Int(21)),
        )),
    );
    assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
}
```

## Performance Characteristics

| Operation | Time Complexity | Space Complexity | Notes |
|-----------|----------------|------------------|-------|
| Literal evaluation | O(1) | O(1) | Direct value return |
| Variable lookup | O(f) | O(f) | One HashMap probe per frame visited; recursive walk of the frame chain |
| Binary operation | O(eval(left) + eval(right)) | O(call stack depth) | Recursive evaluation |
| Conditional | O(eval(cond) + eval(branch)) | O(call stack depth) | Only one branch evaluated |
| Let binding | O(eval(value) + \|frame\| + eval(body)) | O(\|frame\|) | Pushes one new frame; ancestors shared via Rc |
| Function creation | O(\|frame\|) | O(\|frame\|) | Captures a clone of the current frame; ancestors shared |
| Function application | O(eval(head) + Σ eval(arg) + \|frame\| + eval(body)) | O(\|frame\| + call stack) | Extends closure frame per argument |

### Memory Considerations

**Environment Cloning**: Each `let` binding and function creation copies only the current innermost frame's own maps; ancestor frames are **shared** through reference-counted (`Rc`) links rather than deep-copied.
- **Cost**: O(|frame|) in the innermost frame
- **Trade-off**: Simplicity and structural sharing rather than deep copying

**Closure Size**: Each closure stores:
- Parameter name
- Body expression (varies)
- A clone of the current frame's own maps (ancestors shared via Rc, not copied)

**Call Stack Depth**:
- **Deep non-tail recursion**: bounded by a `RecursionLimit` policy — exceeding it yields `EvalError::RecursionLimit` (see `eval_inner` in `crates/parlang/src/eval.rs`) rather than a native stack overflow.
- Evaluation runs on a dedicated worker thread with an `EVALUATOR_STACK_SIZE` (256 MiB) stack via `run_on_evaluator_stack`, used by both the `parlang` CLI/REPL and the `parlang-driver` runner.
- **Direct tail self-calls**: trampolined by the TCO loop and run in constant evaluator depth.

## Design Decisions

### 1. Strict Evaluation

ParLang uses **strict (eager) evaluation** - arguments are evaluated before function application.

**Example**:
```rust
// (fun x -> 42) (1 / 0)
// Evaluates 1 / 0 first → DivisionByZero error
// Never reaches function body
```

**Alternative**: Lazy evaluation (evaluate arguments only when needed)
- **Not chosen** due to complexity and typical use case

### 2. Lexical Scoping

Functions capture their **defining environment**, not their calling environment.

**Rationale**:
- Predictable behavior
- Standard in modern languages
- Enables proper closures

**Alternative**: Dynamic scoping (use calling environment)
- **Not chosen** due to unpredictability

### 3. Immutable Environment Extension

`extend()` creates a new environment rather than mutating existing one.

**Rationale**:
- Safe closure capture
- No aliasing issues
- Easier to reason about

**Trade-off**: A small per-frame clone on each extension, offset by `Rc` sharing of all ancestor frames.

### 4. Tail Call Optimization (TCO)

Curried, `match`-tail and `let`-tail self-calls are optimized by the TCO loop
in `eval_with_tco`: the flat loop is entered with all parameters already bound,
and depth remains constant across iterations (measured flat at `[1,1,1]` across
a `[1_000, 10_000, 100_000]` ramp).

**Accepted trade-off**: Mutual/indirect tail recursion is not optimized (only
direct self-calls are); infinite tail loops spin forever rather than hitting the
depth guard, consistent with the pre-existing `if`-tail behaviour.

### 5. Multiple Value Types

The runtime value domain is `Int`, `Bool`, `Char`, `Float`, `Closure`, `RecClosure`,
`Tuple`, `Record`, and `Variant`. Integer arithmetic is checked; overflow yields
`EvalError::TypeError` rather than wrapping.

### 6. Pattern Matching for Operations

Uses exhaustive pattern matching for type checking and operation dispatch.

**Advantages**:
- Type-safe
- Compiler ensures completeness
- Clear error messages

**Alternative**: Runtime type checking with casting
- **Not chosen** due to Rust's type system

### 7. Error Propagation with `?` Operator

Uses `Result` type and `?` for error handling.

**Advantages**:
- Idiomatic Rust
- Clear error flow
- Type-safe

**Alternative**: Exceptions or panic
- **Not chosen** - Rust doesn't have exceptions

## Extension Points

The evaluator can be extended to support:

### 1. Additional Value Types

```rust
pub enum Value {
    Int(i64),
    Bool(bool),
    Float(f64),        // Floating-point numbers
    String(String),    // String values
    List(Vec<Value>),  // Lists
    Closure(String, Expr, Environment),
}
```

### 2. More Binary Operators

```rust
// Logical operators
BinOp::And | BinOp::Or => // Boolean logic

// String concatenation
(BinOp::Concat, Value::String(a), Value::String(b)) => 
    Ok(Value::String(format!("{}{}", a, b)))
```

### 3. Built-in Functions

```rust
impl Environment {
    pub fn with_builtins() -> Self {
        let mut env = Environment::new();
        // Add print function
        env.bind("print".to_string(), Value::BuiltinFn("print"));
        env
    }
}
```

### 4. Pattern Matching

Pattern matching is implemented by `Expr::Match` and evaluated by `eval_match`.
An exhaustiveness-analysis extension could broaden coverage analysis: it runs at
evaluation time only, prints a warning to stderr, never becomes a hard error,
and is never run by the typechecker. It analyses only booleans and sum-type
constructors; integer, tuple, and record patterns fall back to a catch-all rule,
and char literals are not analysed.

## Integration with Other Modules

```mermaid
graph TB
    SOURCE[Source Code]
    PARSER[Parser Module]
    AST[AST Module]
    EVAL[Evaluator Module]
    RESULT[Runtime Value]
    
    SOURCE -->|parse| PARSER
    PARSER -->|produces| AST
    AST -->|provides types| EVAL
    EVAL -->|interprets| AST
    EVAL -->|produces| RESULT
    
    style EVAL fill:#ffcccc
```

**Dependencies**:
- **Requires**: `ast.rs` (for `Expr`, `BinOp` types)
- **Uses**: `std::collections::HashMap` (for environment storage)
- **Provides**: Runtime execution capability

**Data Flow**:
1. Parser produces `Expr` (AST)
2. Evaluator consumes `Expr`
3. Evaluator produces `Value` or `EvalError`
4. REPL displays result to user

## Summary

The evaluator module provides:

✅ **Language execution** across ParLang's core constructs  
✅ **Lexical scoping** with proper closure semantics  
✅ **Type-checked operations** with runtime error reporting  
✅ **Environment management** supporting nested scopes and shadowing  
✅ **Strict evaluation** with left-to-right operand evaluation  
✅ **Unit and integration test coverage**  
✅ **Clear error messages** for debugging  
✅ **Extensible design** for future language features  

The evaluator successfully executes ParLang programs by recursively traversing the AST and maintaining runtime state through environments, implementing a clean and correct interpreter for the functional programming language.
