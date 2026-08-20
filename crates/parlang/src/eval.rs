/// Evaluator/Interpreter for the `ParLang` language
/// This module implements the runtime evaluation of `ParLang` expressions
use crate::ast::{BinOp, Decl, Expr, Literal, Pattern, Program};
use crate::exhaustiveness::{check_exhaustiveness, ExhaustivenessResult};
#[cfg(test)]
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// Stack reserved for evaluation workers.
///
/// At the policy limit, 4,000 × 24,912 B is about 95 MB, leaving roughly 2.7× headroom in a
/// 256 MiB worker stack.
pub const EVALUATOR_STACK_SIZE: usize = 256 * 1024 * 1024;

/// Failure while starting or joining an evaluator worker.
#[derive(Debug)]
pub enum EvaluatorStackError {
    /// The operating system could not create the evaluator worker.
    Spawn(std::io::Error),
    /// The evaluator worker panicked, optionally with its recovered message.
    WorkerPanicked(Option<String>),
}

impl fmt::Display for EvaluatorStackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluatorStackError::Spawn(error) => {
                write!(f, "failed to spawn evaluator worker: {error}")
            }
            EvaluatorStackError::WorkerPanicked(Some(message)) => {
                write!(f, "evaluator worker panicked: {message}")
            }
            EvaluatorStackError::WorkerPanicked(None) => write!(f, "evaluator worker panicked"),
        }
    }
}

impl std::error::Error for EvaluatorStackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EvaluatorStackError::Spawn(error) => Some(error),
            EvaluatorStackError::WorkerPanicked(_) => None,
        }
    }
}

/// Runs a `Send` result-producing operation on a dedicated evaluator stack.
///
/// `Value` and `Environment` are not `Send`, so callers must turn results into a `Send` payload
/// such as formatted text before the worker returns.
///
/// # Errors
///
/// Returns an error when the evaluator worker cannot be spawned or panics.
pub fn run_on_evaluator_stack<F, R>(f: F) -> Result<R, EvaluatorStackError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let worker = std::thread::Builder::new()
        .stack_size(EVALUATOR_STACK_SIZE)
        .spawn(f)
        .map_err(EvaluatorStackError::Spawn)?;

    worker.join().map_err(|payload| {
        let message = payload
            .downcast_ref::<&'static str>()
            .map(ToString::to_string)
            .or_else(|| payload.downcast_ref::<String>().cloned());
        EvaluatorStackError::WorkerPanicked(message)
    })
}

/// Policy limit for non-tail evaluator recursion.
///
/// A depth of 4,000 commits about 95 MB at the measured 24,912 B per increment; the 256 MiB
/// evaluator worker stack therefore provides roughly 2.7× headroom. Curried multi-argument tail
/// loops cost about 2–3.3 increments per iteration, so this guarantees at least 1,200 iterations
/// (likely about 2,000); single-argument tail loops remain unbounded.
///
/// Historical measurement (2026-08): aggregate cost is 24,912 B per evaluator-depth increment,
/// independently reconfirmed at 24,896 B. A one-increment trace was inferred as
/// `eval_with_tco@d → eval_inner(BinOp@d) → eval_inner(App@d+1) → eval_app@d+1 →
/// eval_with_tco@d+1`, with an 83.0% (20,656 B) `eval_with_tco→eval_app` split and 17.0%
/// (4,240 B) return path. This gave an arm-extraction ceiling of about 83%, below the 91.6%
/// required reduction. The split and ceiling are no longer re-derivable because the known-wrong
/// per-frame diagnostic was removed; re-derivation requires new instrumentation. The aggregate
/// cost remains reproducible with `measures_stack_bytes_per_eval_depth_increment_in_test_profile`.
const DEFAULT_MAX_EVAL_DEPTH: usize = 4_000;

#[cfg(test)]
thread_local! {
    static MAX_EVAL_DEPTH: Cell<usize> = const { Cell::new(0) };
    static EVAL_DEPTH_LIMIT_OVERRIDE: Cell<Option<usize>> = const { Cell::new(None) };
    static STACK_POINTER_MEASUREMENT_ACTIVE: Cell<bool> = const { Cell::new(false) };
    static ENTRY_STACK_POINTER: Cell<Option<usize>> = const { Cell::new(None) };
    static DEEPEST_STACK_POINTER: Cell<Option<usize>> = const { Cell::new(None) };
    static COUNT_EVAL_INNER: Cell<usize> = const { Cell::new(0) };
    static COUNT_EVAL_APP: Cell<usize> = const { Cell::new(0) };
    static COUNT_EVAL_WITH_TCO: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn take_max_eval_depth() -> usize {
    MAX_EVAL_DEPTH.with(|max_depth| max_depth.replace(0))
}

/// Overrides the evaluation-depth limit only so tests can trip the guard without exhausting the
/// native stack.
#[cfg(test)]
fn set_eval_depth_limit_override(limit: Option<usize>) {
    EVAL_DEPTH_LIMIT_OVERRIDE.with(|override_limit| override_limit.set(limit));
}

#[cfg(test)]
fn clear_eval_depth_limit_override() {
    set_eval_depth_limit_override(None);
}

#[cfg(test)]
fn start_stack_pointer_measurement() {
    STACK_POINTER_MEASUREMENT_ACTIVE.with(|active| active.set(true));
    ENTRY_STACK_POINTER.with(|entry| entry.set(None));
    DEEPEST_STACK_POINTER.with(|deepest| deepest.set(None));
}

#[cfg(test)]
fn take_stack_pointer_measurement() -> Option<(usize, usize)> {
    STACK_POINTER_MEASUREMENT_ACTIVE.with(|active| active.set(false));
    let entry = ENTRY_STACK_POINTER.with(|entry| entry.replace(None));
    let deepest = DEEPEST_STACK_POINTER.with(|deepest| deepest.replace(None));
    entry.zip(deepest)
}

#[cfg(test)]
fn reset_frame_diagnostics() {
    COUNT_EVAL_INNER.with(|count| count.set(0));
    COUNT_EVAL_APP.with(|count| count.set(0));
    COUNT_EVAL_WITH_TCO.with(|count| count.set(0));
}

#[cfg(test)]
fn frame_diagnostic_entry_counts() -> (usize, usize, usize) {
    COUNT_EVAL_INNER.with(|eval_inner| {
        COUNT_EVAL_APP.with(|eval_app| {
            COUNT_EVAL_WITH_TCO
                .with(|eval_with_tco| (eval_inner.get(), eval_app.get(), eval_with_tco.get()))
        })
    })
}

/// Runtime values in the language
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

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Float(fl) => write!(f, "{fl}"),
            Value::Char(c) => {
                write!(f, "'")?;
                match c {
                    '\n' => write!(f, "\\n")?,
                    '\t' => write!(f, "\\t")?,
                    '\r' => write!(f, "\\r")?,
                    '\\' => write!(f, "\\\\")?,
                    '\'' => write!(f, "\\'")?,
                    _ => write!(f, "{c}")?,
                }
                write!(f, "'")
            }
            Value::Closure(param, _, _) => write!(f, "<function {param}>"),
            Value::RecClosure(name, _, _, _) => write!(f, "<recursive function {name}>"),
            Value::Tuple(values) => {
                write!(f, "(")?;
                for (i, val) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{val}")?;
                }
                write!(f, ")")
            }
            Value::Record(fields) => {
                write!(f, "{{")?;
                // Sort fields by name for consistent display
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(name, _)| *name);

                for (i, (name, value)) in sorted_fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name}: {value}")?;
                }
                write!(f, "}}")
            }
            Value::Variant(ctor, args) => {
                write!(f, "{ctor}")?;
                if !args.is_empty() {
                    write!(f, "(")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{arg}")?;
                    }
                    write!(f, ")")?;
                }
                Ok(())
            }
        }
    }
}

/// Constructor information for sum types
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorInfo {
    /// Type name this constructor belongs to
    pub type_name: String,
    /// Number of arguments this constructor takes
    pub arity: usize,
}

/// Environment for variable bindings.
///
/// Parent environments are immutable and shared via `Rc`; every mutation
/// creates a new nearest frame and leaves ancestors frozen. A merged
/// environment additionally retains a fallback parent so both source chains
/// remain shared. The recursion knot rebinds itself at call time, so frames
/// only point to ancestors and cannot form an `Rc` cycle. Consequently,
/// environments with identical reachable bindings but different frame shapes
/// compare unequal. This also affects closure equality through `Value`; it has
/// no current observable effect because value equality and literal pattern
/// matching only compare scalar values. A future equality-on-values builtin
/// must account for it explicitly.
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    bindings: HashMap<String, Value>,
    constructors: HashMap<String, ConstructorInfo>,
    parent: Option<Rc<Environment>>,
    fallback: Option<Rc<Environment>>,
}

impl Environment {
    #[must_use]
    pub fn new() -> Self {
        Environment {
            bindings: HashMap::new(),
            constructors: HashMap::new(),
            parent: None,
            fallback: None,
        }
    }

    pub fn bind(&mut self, name: String, value: Value) {
        self.bindings.insert(name, value);
    }

    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.bindings.get(name).or_else(|| {
            self.parent
                .as_deref()
                .and_then(|parent| parent.lookup(name))
                .or_else(|| {
                    self.fallback
                        .as_deref()
                        .and_then(|fallback| fallback.lookup(name))
                })
        })
    }

    #[must_use]
    pub fn extend(&self, name: String, value: Value) -> Self {
        let mut bindings = HashMap::new();
        bindings.insert(name, value);
        Self {
            bindings,
            constructors: HashMap::new(),
            parent: Some(Rc::new(self.clone())),
            fallback: None,
        }
    }

    #[must_use]
    pub fn merge(&self, other: &Environment) -> Self {
        Self {
            bindings: HashMap::new(),
            constructors: HashMap::new(),
            parent: Some(Rc::new(other.clone())),
            fallback: Some(Rc::new(self.clone())),
        }
    }

    pub fn register_constructor(&mut self, name: String, info: ConstructorInfo) {
        self.constructors.insert(name, info);
    }

    pub fn lookup_constructor(&self, name: &str) -> Option<&ConstructorInfo> {
        self.constructors.get(name).or_else(|| {
            self.parent
                .as_deref()
                .and_then(|parent| parent.lookup_constructor(name))
                .or_else(|| {
                    self.fallback
                        .as_deref()
                        .and_then(|fallback| fallback.lookup_constructor(name))
                })
        })
    }

    /// Get constructor information by name (used by exhaustiveness checker)
    pub fn get_constructor(&self, name: &str) -> Option<&ConstructorInfo> {
        self.lookup_constructor(name)
    }

    /// Get all constructors for a given type name (used by exhaustiveness checker)
    pub fn get_constructors_for_type(&self, type_name: &str) -> Vec<String> {
        let mut constructors = Vec::new();
        let mut seen = HashSet::new();
        self.collect_constructors_for_type(type_name, &mut seen, &mut constructors);
        constructors
    }

    fn collect_constructors_for_type(
        &self,
        type_name: &str,
        seen: &mut HashSet<String>,
        constructors: &mut Vec<String>,
    ) {
        for (name, info) in &self.constructors {
            if !seen.insert(name.clone()) {
                continue;
            }
            if info.type_name == type_name {
                constructors.push(name.clone());
            }
        }
        if let Some(parent) = &self.parent {
            parent.collect_constructors_for_type(type_name, seen, constructors);
        }
        if let Some(fallback) = &self.fallback {
            fallback.collect_constructors_for_type(type_name, seen, constructors);
        }
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// Evaluation errors
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

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EvalError::UnboundVariable(name) => write!(f, "Unbound variable: {name}"),
            EvalError::TypeError(msg) => write!(f, "Type error: {msg}"),
            EvalError::DivisionByZero => write!(f, "Division by zero"),
            EvalError::LoadError(msg) => write!(f, "Load error: {msg}"),
            EvalError::IndexOutOfBounds(msg) => write!(f, "Index out of bounds: {msg}"),
            EvalError::FieldNotFound(field, available) => {
                write!(
                    f,
                    "Field '{field}' not found. Available fields: {available:?}"
                )
            }
            EvalError::RecordExpected(got) => {
                write!(f, "Expected record, got {got}")
            }
            EvalError::UnknownConstructor(name) => {
                write!(f, "Unknown constructor: {name}")
            }
            EvalError::ConstructorArityMismatch(name, expected, got) => {
                write!(
                    f,
                    "Constructor {name} expects {expected} arguments, got {got}"
                )
            }
            EvalError::PatternMatchNonExhaustive => {
                write!(f, "Pattern match is non-exhaustive")
            }
            EvalError::RecursionLimit => write!(f, "recursion limit exceeded"),
        }
    }
}

impl std::error::Error for EvalError {}

/// Evaluate a recursive function body with tail call optimization (TCO)
///
/// This function implements tail call optimization for recursive functions. Instead of
/// creating a new stack frame for each recursive call, it iteratively updates the
/// environment and re-evaluates the body expression. This allows deep recursion without
/// stack overflow for tail-recursive functions.
///
/// # Arguments
/// * `body` - The body expression of the recursive function
/// * `initial_env` - The initial environment with the argument bindings
/// * `rec_name` - The name of the recursive function
/// * `param_names` - The names of the saturated function parameters
/// * `closure_env` - The environment captured in the closure
///
/// # Returns
/// The result value of evaluating the function, or an error
///
/// # Example
/// For a tail-recursive factorial with accumulator:
/// ```text
/// rec fact -> fun acc -> fun n ->
///     if n == 0 then acc else fact (acc * n) (n - 1)
/// ```
/// Instead of recursing, this function updates `acc` and `n` and re-evaluates the body.
/// `if`, `let`, and `match` preserve tail position while this loop runs. Infinite
/// `let`- or `match`-tail recursion therefore spins rather than reaching the depth guard,
/// consistent with the existing behaviour for an infinite `if`-tail loop.
///
/// The `App` arm rebuilds `current_env` from `closure_env` on every iteration. This reset
/// bounds bindings introduced by the `Let` and `Match` arms.
fn eval_with_tco(
    body: &Expr,
    initial_env: &Environment,
    rec_name: &str,
    param_names: &[String],
    closure_env: &Environment,
    rec_value: &Value,
    depth: usize,
) -> Result<Value, EvalError> {
    #[cfg(test)]
    {
        COUNT_EVAL_WITH_TCO.with(|count| count.set(count.get() + 1));
    }

    let mut current_expr = body.clone();
    let mut current_env = initial_env.clone();

    'tco: loop {
        // Check if the expression is a tail call to the recursive function
        match &current_expr {
            Expr::App(_, _) => {
                let (func, args) = collect_app_spine(&current_expr);
                if is_tail_call_to(func, rec_name) {
                    let values = eval_spine_args(&args, &current_env, depth)?;
                    if values.len() != param_names.len() {
                        break apply_value_spine(rec_value.clone(), values, depth);
                    }
                    current_env =
                        bind_rec_params(closure_env, rec_name, rec_value, param_names, &values);
                    current_expr = body.clone();
                    continue;
                }
                // Not a tail call to self - evaluate normally and return
                break eval_inner(&current_expr, &current_env, depth);
            }
            // Handle if expressions - evaluate condition and continue with the appropriate branch
            Expr::If(cond, then_branch, else_branch) => {
                let cond_val = eval_inner(cond, &current_env, depth)?;
                match cond_val {
                    Value::Bool(true) => {
                        current_expr = (**then_branch).clone();
                    }
                    Value::Bool(false) => {
                        current_expr = (**else_branch).clone();
                    }
                    _ => {
                        return Err(EvalError::TypeError(
                            "if condition must evaluate to a boolean".to_string(),
                        ))
                    }
                }
            }
            Expr::Let(name, _, value, body) => {
                if name == rec_name {
                    break eval_inner(&current_expr, &current_env, depth);
                }
                let value = eval_inner(value, &current_env, depth)?;
                current_env = current_env.extend(name.clone(), value);
                current_expr = (**body).clone();
            }
            Expr::Match(scrutinee, arms) => {
                let value = eval_inner(scrutinee, &current_env, depth)?;
                for (pattern, result) in arms {
                    if let Some(matched_env) = match_pattern(pattern, &value, &current_env) {
                        if pattern_binds(pattern, rec_name) {
                            break 'tco eval_inner(&current_expr, &current_env, depth);
                        }
                        current_env = matched_env;
                        current_expr = result.clone();
                        continue 'tco;
                    }
                }
                break Err(EvalError::PatternMatchNonExhaustive);
            }
            // For other expressions, evaluate normally and return
            _ => break eval_inner(&current_expr, &current_env, depth),
        }
    }
}

/// Check if an expression is ultimately a call to the recursive function
///
/// This helper function determines whether an expression is a direct or indirect call
/// to the named recursive function. It handles nested applications like `(rec_name arg1) arg2`
/// by recursively checking the function part of applications.
///
/// # Arguments
/// * `expr` - The expression to check
/// * `rec_name` - The name of the recursive function
///
/// # Returns
/// `true` if the expression calls the recursive function, `false` otherwise
///
/// # Example
/// - `is_tail_call_to(Var("fact"), "fact")` returns `true`
/// - `is_tail_call_to(App(Var("fact"), Lit(5)), "fact")` returns `true`
/// - `is_tail_call_to(Var("other"), "fact")` returns `false`
fn is_tail_call_to(expr: &Expr, rec_name: &str) -> bool {
    match expr {
        Expr::Var(name) => name == rec_name,
        Expr::App(func, _) => is_tail_call_to(func, rec_name),
        _ => false,
    }
}

/// Collect an applicative spine as its head and source-ordered arguments.
fn collect_app_spine(expr: &Expr) -> (&Expr, Vec<&Expr>) {
    let mut args = Vec::new();
    let mut head = expr;
    while let Expr::App(func, arg) = head {
        args.push(arg.as_ref());
        head = func;
    }
    args.reverse();
    (head, args)
}

/// Peel requested parameters from a recursive closure body.
///
/// `eval_rec` has already peeled the first `fun`, so the returned list begins with
/// `rec_param`; subsequent leading `Fun` layers provide any further parameters.
fn peel_rec_params<'a>(
    rec_param: &str,
    body: &'a Expr,
    requested: usize,
) -> (Vec<String>, &'a Expr) {
    let mut params = Vec::new();
    let mut remaining = body;
    if requested == 0 {
        return (params, remaining);
    }
    params.push(rec_param.to_string());
    while params.len() < requested {
        let Expr::Fun(param, _, next_body) = remaining else {
            break;
        };
        params.push(param.clone());
        remaining = next_body;
    }
    (params, remaining)
}

fn rec_arity(body: &Expr) -> usize {
    let mut arity = 1;
    let mut remaining = body;
    while let Expr::Fun(_, _, next_body) = remaining {
        arity += 1;
        remaining = next_body;
    }
    arity
}

fn eval_spine_args(
    args: &[&Expr],
    env: &Environment,
    depth: usize,
) -> Result<Vec<Value>, EvalError> {
    args.iter().map(|arg| eval_inner(arg, env, depth)).collect()
}

fn bind_rec_params(
    closure_env: &Environment,
    rec_name: &str,
    rec_value: &Value,
    params: &[String],
    values: &[Value],
) -> Environment {
    let mut bound = closure_env.extend(rec_name.to_string(), rec_value.clone());
    for (param, value) in params.iter().zip(values) {
        bound = bound.extend(param.clone(), value.clone());
    }
    bound
}

fn pattern_binds(pattern: &Pattern, name: &str) -> bool {
    match pattern {
        Pattern::Wildcard | Pattern::Literal(_) => false,
        Pattern::Var(bound) => bound == name,
        Pattern::Tuple(patterns) => patterns.iter().any(|pattern| pattern_binds(pattern, name)),
        Pattern::Record(fields) => fields
            .iter()
            .any(|(_, pattern)| pattern_binds(pattern, name)),
        Pattern::Constructor(_, patterns) => {
            patterns.iter().any(|pattern| pattern_binds(pattern, name))
        }
    }
}

/// Extract bindings from nested let expressions
/// This walks through the AST and extracts all top-level let bindings.
/// Used by the REPL to persist function definitions and library loads across evaluations.
///
/// # Errors
///
/// Returns an error if:
/// - Evaluation of a let binding value fails
/// - Loading a library file fails (file not found or parse error)
/// - A binding value causes a type error or other evaluation error
pub fn extract_bindings(expr: &Expr, env: &Environment) -> Result<Environment, EvalError> {
    match expr {
        Expr::Let(name, _ty_ann, value, body) => {
            // Evaluate the value in the current environment
            let val = eval(value, env)?;
            // Extend the environment with this binding
            let new_env = env.extend(name.clone(), val);
            // Continue extracting from the body
            extract_bindings(body, &new_env)
        }
        Expr::Load(filepath, body) => {
            // Handle nested load expressions
            // Read and parse the file
            let content = fs::read_to_string(Path::new(filepath)).map_err(|e| {
                EvalError::LoadError(format!("Failed to read file '{filepath}': {e}"))
            })?;
            let lib_program = crate::parser::parse_program(&content).map_err(|e| {
                EvalError::LoadError(format!("Failed to parse file '{filepath}': {e}"))
            })?;

            // Extract bindings from the loaded library
            // Pass current environment so type constructors are available
            let lib_env = extend_env_with_program(&lib_program, env)?;
            // Preserve the existing last-writer-wins behavior: loaded bindings
            // are nearest and shadow the current environment.
            let new_env = env.merge(&lib_env);
            // Continue extracting from the body
            extract_bindings(body, &new_env)
        }
        Expr::TypeAlias(_name, _ty_expr, body) => {
            // Type aliases don't create runtime bindings, just pass through to the body
            extract_bindings(body, env)
        }
        // If we reach anything other than a Let, Load, or TypeAlias, we're done extracting
        // Return the accumulated environment
        _ => Ok(env.clone()),
    }
}

/// Match a pattern against a value, returning an extended environment if successful
///
/// This function implements pattern matching by recursively checking if a pattern
/// matches a given value. If successful, it returns an environment extended with
/// any variable bindings from the pattern. If the match fails, it returns `None`.
///
/// # Arguments
/// * `pattern` - The pattern to match against
/// * `value` - The value to match
/// * `env` - The current environment to extend with bindings
///
/// # Returns
/// `Some(Environment)` with new bindings if the pattern matches, `None` otherwise
///
/// # Supported Patterns
/// - `Wildcard`: Matches anything without binding
/// - `Var(name)`: Matches anything and binds it to `name`
/// - `Literal(lit)`: Matches only the specific literal value
/// - `Tuple(patterns)`: Matches tuples with matching sub-patterns
/// - `Record(fields)`: Matches records with specified fields (supports partial matching)
/// - `Constructor(name, args)`: Matches sum type constructors
///
/// # Example
/// ```text
/// match_pattern(Var("x"), Int(42), env) → Some(env + {x: 42})
/// match_pattern(Literal(Int(0)), Int(0), env) → Some(env)
/// match_pattern(Literal(Int(0)), Int(1), env) → None
/// ```
fn match_pattern(pattern: &Pattern, value: &Value, env: &Environment) -> Option<Environment> {
    match pattern {
        Pattern::Wildcard => {
            // Wildcard matches anything without binding
            Some(env.clone())
        }
        Pattern::Literal(lit) => {
            // Literal pattern must match exactly
            match (lit, value) {
                (Literal::Int(n1), Value::Int(n2)) if n1 == n2 => Some(env.clone()),
                (Literal::Bool(b1), Value::Bool(b2)) if b1 == b2 => Some(env.clone()),
                (Literal::Char(c1), Value::Char(c2)) if c1 == c2 => Some(env.clone()),
                _ => None,
            }
        }
        Pattern::Var(name) => {
            // Variable pattern binds the value to the name
            Some(env.extend(name.clone(), value.clone()))
        }
        Pattern::Tuple(patterns) => {
            // Tuple pattern must match a tuple value with the same number of elements
            match value {
                Value::Tuple(values) => {
                    // Check if the number of patterns matches the number of values
                    if patterns.len() != values.len() {
                        return None;
                    }
                    // Match each pattern against corresponding value
                    let mut current_env = env.clone();
                    for (pat, val) in patterns.iter().zip(values.iter()) {
                        match match_pattern(pat, val, &current_env) {
                            Some(new_env) => current_env = new_env,
                            None => return None,
                        }
                    }
                    Some(current_env)
                }
                _ => None,
            }
        }
        Pattern::Record(pattern_fields) => {
            // Record pattern can be partial - only matches specified fields
            match value {
                Value::Record(value_fields) => {
                    let mut current_env = env.clone();

                    // Match each field in the pattern
                    for (field_name, field_pattern) in pattern_fields {
                        match value_fields.get(field_name) {
                            Some(field_value) => {
                                // Recursively match the field
                                match match_pattern(field_pattern, field_value, &current_env) {
                                    Some(new_env) => current_env = new_env,
                                    None => return None,
                                }
                            }
                            None => {
                                // Field not found in value record - pattern doesn't match
                                return None;
                            }
                        }
                    }

                    Some(current_env)
                }
                _ => None,
            }
        }
        Pattern::Constructor(pattern_ctor, pattern_args) => {
            // Constructor pattern matching
            match value {
                Value::Variant(value_ctor, value_args) => {
                    // Constructor names must match
                    if pattern_ctor != value_ctor {
                        return None;
                    }

                    // Argument counts must match
                    if pattern_args.len() != value_args.len() {
                        return None;
                    }

                    // Match each argument recursively
                    let mut current_env = env.clone();
                    for (pat, val) in pattern_args.iter().zip(value_args.iter()) {
                        match match_pattern(pat, val, &current_env) {
                            Some(new_env) => current_env = new_env,
                            None => return None,
                        }
                    }

                    Some(current_env)
                }
                _ => None,
            }
        }
    }
}

/// Evaluate an application spine (extracted from `eval` to keep its line count down).
fn eval_app(expr: &Expr, env: &Environment, depth: usize) -> Result<Value, EvalError> {
    #[cfg(test)]
    {
        COUNT_EVAL_APP.with(|count| count.set(count.get() + 1));
    }

    let (head, args) = collect_app_spine(expr);
    let function = eval_inner(head, env, depth + 1)?;
    let args = eval_spine_args(&args, env, depth + 1)?;
    apply_value_spine(function, args, depth)
}

/// Apply an evaluated function to already-evaluated arguments.
fn apply_value_spine(
    mut function: Value,
    args: Vec<Value>,
    depth: usize,
) -> Result<Value, EvalError> {
    let mut remaining = args.into_iter().peekable();
    while let Some(arg) = remaining.next() {
        function = match function {
            Value::Closure(param, body, closure_env) => {
                let bound_env = closure_env.extend(param, arg);
                eval_inner(
                    &body,
                    &bound_env,
                    depth.checked_add(1).ok_or(EvalError::RecursionLimit)?,
                )?
            }
            Value::RecClosure(rec_name, rec_param, body, closure_env) => {
                let mut rec_args = vec![arg];
                rec_args.extend(remaining);
                return apply_rec_closure(
                    &rec_name,
                    rec_param,
                    &body,
                    &closure_env,
                    &rec_args,
                    depth,
                );
            }
            _ => {
                return Err(EvalError::TypeError(
                    "Application requires a function".to_string(),
                ))
            }
        };
    }
    Ok(function)
}

fn apply_rec_closure(
    rec_name: &str,
    rec_param: String,
    body: &Expr,
    closure_env: &Environment,
    args: &[Value],
    depth: usize,
) -> Result<Value, EvalError> {
    let available = rec_arity(body);
    let consumed = args.len().min(available);
    let (params, remaining_body) = peel_rec_params(&rec_param, body, consumed);
    if params.len() != consumed {
        return Err(EvalError::TypeError(
            "Recursive function parameter arity is inconsistent".to_string(),
        ));
    }
    let rec_value = Value::RecClosure(
        rec_name.to_string(),
        rec_param,
        body.clone(),
        closure_env.clone(),
    );
    let bound_env = bind_rec_params(
        closure_env,
        rec_name,
        &rec_value,
        &params,
        &args[..consumed],
    );

    if args.len() < available {
        let Expr::Fun(param, _, residual_body) = remaining_body else {
            return Err(EvalError::TypeError(
                "Recursive function parameter arity is inconsistent".to_string(),
            ));
        };
        return Ok(Value::Closure(
            param.clone(),
            (**residual_body).clone(),
            bound_env,
        ));
    }

    let result = eval_with_tco(
        remaining_body,
        &bound_env,
        rec_name,
        &params,
        closure_env,
        &rec_value,
        depth,
    )?;
    apply_value_spine(result, args[consumed..].to_vec(), depth)
}

/// Evaluate `load "path" in body` (extracted from `eval` to keep its line
/// count down).
fn eval_load(
    filepath: &str,
    body: &Expr,
    env: &Environment,
    depth: usize,
) -> Result<Value, EvalError> {
    // Read the file contents
    let content = fs::read_to_string(Path::new(filepath))
        .map_err(|e| EvalError::LoadError(format!("Failed to read file '{filepath}': {e}")))?;

    // Parse the file contents
    let lib_program = crate::parser::parse_program(&content)
        .map_err(|e| EvalError::LoadError(format!("Failed to parse file '{filepath}': {e}")))?;

    // Extract bindings from the library file
    // Pass current environment so type constructors are available
    let lib_env = extend_env_with_program(&lib_program, env)?;

    // Preserve the existing last-writer-wins behavior: loaded bindings are
    // nearest and shadow the current environment.
    let extended_env = env.merge(&lib_env);

    // Evaluate the body in the extended environment
    eval_inner(body, &extended_env, depth + 1)
}

/// Evaluate a `match scrutinee with arms` expression (extracted from `eval`
/// to keep its line count down).
fn eval_match(
    scrutinee: &Expr,
    arms: &[(Pattern, Expr)],
    env: &Environment,
    depth: usize,
) -> Result<Value, EvalError> {
    // Check exhaustiveness of patterns
    let patterns: Vec<Pattern> = arms.iter().map(|(p, _)| p.clone()).collect();
    let exhaustiveness = check_exhaustiveness(&patterns, env);

    if !exhaustiveness.is_exhaustive() {
        // Print warning to stderr for non-exhaustive patterns
        if let ExhaustivenessResult::NonExhaustive(missing) = exhaustiveness {
            eprintln!("Warning: pattern match is non-exhaustive");
            eprintln!("  Missing cases: {}", missing.join(", "));
        }
    }

    // Evaluate the scrutinee expression
    let val = eval_inner(scrutinee, env, depth + 1)?;

    // Try to match against each pattern arm in order
    for (pattern, result_expr) in arms {
        if let Some(new_env) = match_pattern(pattern, &val, env) {
            // Pattern matched, evaluate the result expression with the extended environment
            return eval_inner(result_expr, &new_env, depth + 1);
        }
    }

    // No pattern matched - use the dedicated error variant
    Err(EvalError::PatternMatchNonExhaustive)
}

/// Evaluate `tuple_expr.index` (extracted from `eval` to keep its line count
/// down).
fn eval_tuple_proj(
    tuple_expr: &Expr,
    index: usize,
    env: &Environment,
    depth: usize,
) -> Result<Value, EvalError> {
    // Evaluate the tuple expression
    let tuple_val = eval_inner(tuple_expr, env, depth + 1)?;

    // Check that the value is a tuple
    match tuple_val {
        Value::Tuple(values) => {
            // Check bounds
            if index >= values.len() {
                Err(EvalError::IndexOutOfBounds(format!(
                    "Tuple index {} out of bounds for tuple of size {}",
                    index,
                    values.len()
                )))
            } else {
                Ok(values[index].clone())
            }
        }
        _ => Err(EvalError::TypeError(
            "Tuple projection requires a tuple".to_string(),
        )),
    }
}

/// Evaluate a `data Name = ... in body` declaration, registering its
/// constructors (extracted from `eval` to keep its line count down).
fn eval_type_def(
    name: &str,
    constructors: &[(String, Vec<crate::ast::TypeAnnotation>)],
    body: &Expr,
    env: &Environment,
    depth: usize,
) -> Result<Value, EvalError> {
    // Register all constructors in the environment
    let mut new_env = env.clone();

    for (ctor_name, ctor_types) in constructors {
        let ctor_info = ConstructorInfo {
            type_name: name.to_string(),
            arity: ctor_types.len(),
        };
        new_env.register_constructor(ctor_name.clone(), ctor_info);
    }

    // Evaluate body in extended environment
    eval_inner(body, &new_env, depth + 1)
}

/// Evaluate a sum-type constructor application (extracted from `eval` to
/// keep its line count down).
fn eval_constructor(
    ctor_name: &str,
    args: &[Expr],
    env: &Environment,
    depth: usize,
) -> Result<Value, EvalError> {
    // Look up constructor info
    let ctor_info = env
        .lookup_constructor(ctor_name)
        .ok_or_else(|| EvalError::UnknownConstructor(ctor_name.to_string()))?;

    // Check arity
    if args.len() != ctor_info.arity {
        return Err(EvalError::ConstructorArityMismatch(
            ctor_name.to_string(),
            ctor_info.arity,
            args.len(),
        ));
    }

    // Evaluate all arguments
    let mut values = Vec::new();
    for arg in args {
        values.push(eval_inner(arg, env, depth + 1)?);
    }

    Ok(Value::Variant(ctor_name.to_string(), values))
}

/// Evaluate `rec name -> body`, producing a recursive closure (extracted
/// from `eval` to keep its line count down).
fn eval_rec(name: &str, body: &Expr, env: &Environment) -> Result<Value, EvalError> {
    // Parse the body which should be a function (fun param -> expr)
    // The recursive function can reference itself by name within its body
    match body {
        Expr::Fun(param, _ty_ann, fun_body) => {
            // Create a recursive closure that captures the function name
            Ok(Value::RecClosure(
                name.to_string(),
                param.clone(),
                (**fun_body).clone(),
                env.clone(),
            ))
        }
        _ => Err(EvalError::TypeError(
            "rec expression body must be a function".to_string(),
        )),
    }
}

/// Evaluate a record literal `{ field: expr, ... }` (extracted from `eval`
/// to keep its line count down).
fn eval_record(
    fields: &[(String, Expr)],
    env: &Environment,
    depth: usize,
) -> Result<Value, EvalError> {
    // Evaluate all field expressions and build the record
    let mut record = HashMap::new();

    for (name, expr) in fields {
        let value = eval_inner(expr, env, depth + 1)?;
        record.insert(name.clone(), value);
    }

    Ok(Value::Record(record))
}

/// Evaluate `record_expr.field_name` (extracted from `eval` to keep its
/// line count down).
fn eval_field_access(
    record_expr: &Expr,
    field_name: &str,
    env: &Environment,
    depth: usize,
) -> Result<Value, EvalError> {
    // Evaluate the record expression
    let record_value = eval_inner(record_expr, env, depth + 1)?;

    // Check that the value is a record and access the field
    match record_value {
        Value::Record(fields) => fields.get(field_name).cloned().ok_or_else(|| {
            let mut available: Vec<String> = fields.keys().cloned().collect();
            available.sort();
            EvalError::FieldNotFound(field_name.to_string(), available)
        }),
        other => Err(EvalError::RecordExpected(format!("{other:?}"))),
    }
}

/// Evaluate `if cond then then_branch else else_branch` (extracted from
/// `eval` to keep its line count down).
fn eval_if(
    cond: &Expr,
    then_branch: &Expr,
    else_branch: &Expr,
    env: &Environment,
    depth: usize,
) -> Result<Value, EvalError> {
    let cond_val = eval_inner(cond, env, depth + 1)?;
    match cond_val {
        Value::Bool(true) => eval_inner(then_branch, env, depth + 1),
        Value::Bool(false) => eval_inner(else_branch, env, depth + 1),
        _ => Err(EvalError::TypeError(
            "If condition must be a boolean".to_string(),
        )),
    }
}

/// Evaluate an expression in an environment
///
/// # Errors
///
/// Returns an error if:
/// - A variable is unbound (not found in the environment)
/// - A type error occurs (e.g., applying a non-function, or arithmetic on non-integers)
/// - Division by zero is attempted
/// - A pattern match fails (no pattern matches the scrutinee)
/// - Loading a library file fails
/// - A tuple projection index is out of bounds
pub fn eval(expr: &Expr, env: &Environment) -> Result<Value, EvalError> {
    eval_inner(expr, env, 0)
}

// `extract_bindings`, `extend_env_with_program`, and load evaluation re-enter through
// public `eval` at depth zero; deeply nested top-level lets or loads remain out of scope.
fn eval_inner(expr: &Expr, env: &Environment, depth: usize) -> Result<Value, EvalError> {
    #[cfg(test)]
    let max_depth = EVAL_DEPTH_LIMIT_OVERRIDE
        .with(Cell::get)
        .unwrap_or(DEFAULT_MAX_EVAL_DEPTH);
    #[cfg(not(test))]
    let max_depth = DEFAULT_MAX_EVAL_DEPTH;

    if depth >= max_depth {
        return Err(EvalError::RecursionLimit);
    }
    #[cfg(test)]
    {
        MAX_EVAL_DEPTH.with(|max_depth| max_depth.set(max_depth.get().max(depth)));
        COUNT_EVAL_INNER.with(|count| count.set(count.get() + 1));

        // This is the sole stack-pointer probe site. Debug/test measurements use it to avoid
        // comparing distinct evaluator-frame layouts.
        let stack_probe = depth;
        let stack_pointer = std::hint::black_box(&raw const stack_probe as usize);
        STACK_POINTER_MEASUREMENT_ACTIVE.with(|active| {
            if active.get() {
                ENTRY_STACK_POINTER.with(|entry| {
                    if entry.get().is_none() {
                        entry.set(Some(stack_pointer));
                    }
                });
                DEEPEST_STACK_POINTER.with(|deepest| {
                    deepest.set(Some(
                        deepest
                            .get()
                            .map_or(stack_pointer, |current| current.min(stack_pointer)),
                    ));
                });
            }
        });
    }

    match expr {
        Expr::Int(n) => Ok(Value::Int(*n)),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Char(c) => Ok(Value::Char(*c)),
        Expr::Float(f) => Ok(Value::Float(*f)),

        Expr::Var(name) => env
            .lookup(name)
            .cloned()
            .ok_or_else(|| EvalError::UnboundVariable(name.clone())),

        Expr::BinOp(op, left, right) => {
            let left_val = eval_inner(left, env, depth + 1)?;
            let right_val = eval_inner(right, env, depth + 1)?;
            eval_binop(*op, left_val, right_val)
        }

        Expr::If(cond, then_branch, else_branch) => {
            eval_if(cond, then_branch, else_branch, env, depth)
        }

        Expr::Let(name, _ty_ann, value, body) => {
            let val = eval_inner(value, env, depth + 1)?;
            let new_env = env.extend(name.clone(), val);
            eval_inner(body, &new_env, depth + 1)
        }

        Expr::Fun(param, _ty_ann, body) => {
            Ok(Value::Closure(param.clone(), (**body).clone(), env.clone()))
        }

        Expr::App(_, _) => eval_app(expr, env, depth),

        Expr::Load(filepath, body) => eval_load(filepath, body, env, depth),

        Expr::Rec(name, body) => eval_rec(name, body, env),

        Expr::Match(scrutinee, arms) => eval_match(scrutinee, arms, env, depth),

        Expr::Tuple(elements) => {
            // Evaluate all elements of the tuple
            let mut values = Vec::new();
            for elem in elements {
                values.push(eval_inner(elem, env, depth + 1)?);
            }
            Ok(Value::Tuple(values))
        }

        Expr::TupleProj(tuple_expr, index) => eval_tuple_proj(tuple_expr, *index, env, depth),

        Expr::TypeAlias(_name, _ty_expr, body) => {
            // Type aliases are transparent at runtime - they're only used during type checking
            // We simply evaluate the body in the current environment
            eval_inner(body, env, depth + 1)
        }

        Expr::Record(fields) => eval_record(fields, env, depth),

        Expr::FieldAccess(record_expr, field_name) => {
            eval_field_access(record_expr, field_name, env, depth)
        }

        Expr::TypeDef {
            name,
            type_params: _,
            constructors,
            body,
        } => eval_type_def(name, constructors, body, env, depth),

        Expr::Constructor(ctor_name, args) => eval_constructor(ctor_name, args, env, depth),
    }
}

/// Evaluate a top-level program by threading declaration bindings into the environment.
///
/// # Errors
///
/// Returns an `EvalError` if a declaration or the trailing body cannot be evaluated.
pub fn eval_program(program: &Program, env: &Environment) -> Result<Value, EvalError> {
    eval_program_with_env(program, env).map(|(value, _)| value)
}

/// Evaluate a top-level program and return its persistent environment.
///
/// # Errors
///
/// Returns an `EvalError` if a declaration or the trailing body cannot be evaluated.
pub fn eval_program_with_env(
    program: &Program,
    env: &Environment,
) -> Result<(Value, Environment), EvalError> {
    let current_env = extend_env_with_program(program, env)?;
    let value = match &program.body {
        Some(body) => eval(body, &current_env),
        None => Ok(Value::Int(0)),
    }?;
    Ok((value, current_env))
}

/// Extend an environment with the evaluated declarations of a top-level program.
///
/// # Errors
///
/// Returns an `EvalError` if evaluating a declaration value fails.
pub fn extend_env_with_program(
    program: &Program,
    env: &Environment,
) -> Result<Environment, EvalError> {
    let mut current_env = env.clone();

    // Constructors must all be registered before evaluating any declarations:
    // every subsequent `extend` creates a child frame that shares this frame.
    // Main programs and REPL entries are typechecked first, which rejects duplicate
    // declarations. Loaded libraries bypass typechecking, so duplicates there retain
    // the existing last-writer-wins runtime behaviour.
    for decl in &program.decls {
        if let Decl::Data {
            name, constructors, ..
        } = decl
        {
            for (constructor_name, payload_types) in constructors {
                current_env.register_constructor(
                    constructor_name.clone(),
                    ConstructorInfo {
                        type_name: name.clone(),
                        arity: payload_types.len(),
                    },
                );
            }
        }
    }

    for decl in &program.decls {
        match decl {
            Decl::Let { name, value, .. } => {
                let value = eval(value, &current_env)?;
                current_env = current_env.extend(name.clone(), value);
            }
            Decl::Data { .. } | Decl::TypeAlias { .. } => {}
        }
    }
    match &program.body {
        Some(body) => extract_bindings(body, &current_env),
        None => Ok(current_env),
    }
}

/// Evaluate a binary operation
fn eval_binop(op: BinOp, left: Value, right: Value) -> Result<Value, EvalError> {
    match (op, left, right) {
        // Arithmetic operations with overflow checking for Int
        (BinOp::Add, Value::Int(a), Value::Int(b)) => a
            .checked_add(b)
            .map(Value::Int)
            .ok_or_else(|| EvalError::TypeError("Integer overflow in addition".to_string())),
        (BinOp::Sub, Value::Int(a), Value::Int(b)) => a
            .checked_sub(b)
            .map(Value::Int)
            .ok_or_else(|| EvalError::TypeError("Integer overflow in subtraction".to_string())),
        (BinOp::Mul, Value::Int(a), Value::Int(b)) => a
            .checked_mul(b)
            .map(Value::Int)
            .ok_or_else(|| EvalError::TypeError("Integer overflow in multiplication".to_string())),
        (BinOp::Div, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                Err(EvalError::DivisionByZero)
            } else {
                a.checked_div(b)
                    .map(Value::Int)
                    .ok_or_else(|| EvalError::TypeError("Integer overflow in division".to_string()))
            }
        }

        // Arithmetic operations for Float
        (BinOp::Add, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (BinOp::Sub, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
        (BinOp::Mul, Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
        (BinOp::Div, Value::Float(a), Value::Float(b)) => {
            if b == 0.0 {
                Err(EvalError::DivisionByZero)
            } else {
                Ok(Value::Float(a / b))
            }
        }

        // Comparison operations for Int
        (BinOp::Eq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a == b)),
        (BinOp::Neq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a != b)),
        (BinOp::Lt, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
        (BinOp::Le, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
        (BinOp::Gt, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
        (BinOp::Ge, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),

        // Comparison operations for Float
        // Bit-exact comparison (not epsilon-based): ParLang's == on floats is
        // defined as exact equality, mirroring parlang-core's eval_eq approach.
        (BinOp::Eq, Value::Float(a), Value::Float(b)) => {
            Ok(Value::Bool(a.to_bits() == b.to_bits()))
        }
        (BinOp::Neq, Value::Float(a), Value::Float(b)) => {
            Ok(Value::Bool(a.to_bits() != b.to_bits()))
        }
        (BinOp::Lt, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a < b)),
        (BinOp::Le, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a <= b)),
        (BinOp::Gt, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a > b)),
        (BinOp::Ge, Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a >= b)),

        // Comparison operations for Bool
        (BinOp::Eq, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
        (BinOp::Neq, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a != b)),

        // Comparison operations for Char
        (BinOp::Eq, Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a == b)),
        (BinOp::Neq, Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a != b)),
        (BinOp::Lt, Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a < b)),
        (BinOp::Le, Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a <= b)),
        (BinOp::Gt, Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a > b)),
        (BinOp::Ge, Value::Char(a), Value::Char(b)) => Ok(Value::Bool(a >= b)),

        (op, left, right) => Err(EvalError::TypeError(format!(
            "Type error in binary operation {op:?}: cannot apply to {left:?} and {right:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_int() {
        let env = Environment::new();
        assert_eq!(eval(&Expr::Int(42), &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_bool() {
        let env = Environment::new();
        assert_eq!(eval(&Expr::Bool(true), &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_binop() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Add, Box::new(Expr::Int(1)), Box::new(Expr::Int(2)));
        assert_eq!(eval(&expr, &env), Ok(Value::Int(3)));
    }

    #[test]
    fn test_eval_let() {
        let env = Environment::new();
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_if() {
        let env = Environment::new();
        let expr = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(1)));
    }

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

    #[test]
    fn test_eval_unbound_var() {
        let env = Environment::new();
        let expr = Expr::Var("x".to_string());
        assert!(matches!(
            eval(&expr, &env),
            Err(EvalError::UnboundVariable(_))
        ));
    }

    // Test all arithmetic operations
    #[test]
    fn test_eval_add() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Add, Box::new(Expr::Int(10)), Box::new(Expr::Int(32)));
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_sub() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Sub, Box::new(Expr::Int(50)), Box::new(Expr::Int(8)));
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_mul() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Mul, Box::new(Expr::Int(6)), Box::new(Expr::Int(7)));
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_div() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Div, Box::new(Expr::Int(84)), Box::new(Expr::Int(2)));
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_div_by_zero() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Div, Box::new(Expr::Int(42)), Box::new(Expr::Int(0)));
        assert_eq!(eval(&expr, &env), Err(EvalError::DivisionByZero));
    }

    // Test all comparison operations
    #[test]
    fn test_eval_eq_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Eq, Box::new(Expr::Int(42)), Box::new(Expr::Int(42)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_eq_false() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Eq, Box::new(Expr::Int(42)), Box::new(Expr::Int(43)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn test_eval_neq_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Neq, Box::new(Expr::Int(42)), Box::new(Expr::Int(43)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_neq_false() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Neq, Box::new(Expr::Int(42)), Box::new(Expr::Int(42)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn test_eval_lt_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Lt, Box::new(Expr::Int(3)), Box::new(Expr::Int(5)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_lt_false() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Lt, Box::new(Expr::Int(5)), Box::new(Expr::Int(3)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn test_eval_le_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Le, Box::new(Expr::Int(3)), Box::new(Expr::Int(5)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_le_equal() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Le, Box::new(Expr::Int(5)), Box::new(Expr::Int(5)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_gt_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Gt, Box::new(Expr::Int(5)), Box::new(Expr::Int(3)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_gt_false() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Gt, Box::new(Expr::Int(3)), Box::new(Expr::Int(5)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn test_eval_ge_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Ge, Box::new(Expr::Int(5)), Box::new(Expr::Int(3)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_ge_equal() {
        let env = Environment::new();
        let expr = Expr::BinOp(BinOp::Ge, Box::new(Expr::Int(5)), Box::new(Expr::Int(5)));
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    // Test boolean comparisons
    #[test]
    fn test_eval_bool_eq() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Eq,
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Bool(true)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_bool_neq() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Neq,
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Bool(false)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    // Test if-then-else
    #[test]
    fn test_eval_if_true_branch() {
        let env = Environment::new();
        let expr = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Int(42)),
            Box::new(Expr::Int(0)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_if_false_branch() {
        let env = Environment::new();
        let expr = Expr::If(
            Box::new(Expr::Bool(false)),
            Box::new(Expr::Int(0)),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_if_with_comparison() {
        let env = Environment::new();
        // if 5 > 3 then 100 else 0
        let expr = Expr::If(
            Box::new(Expr::BinOp(
                BinOp::Gt,
                Box::new(Expr::Int(5)),
                Box::new(Expr::Int(3)),
            )),
            Box::new(Expr::Int(100)),
            Box::new(Expr::Int(0)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(100)));
    }

    #[test]
    fn test_eval_if_non_bool_condition() {
        let env = Environment::new();
        let expr = Expr::If(
            Box::new(Expr::Int(42)),
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert!(matches!(eval(&expr, &env), Err(EvalError::TypeError(_))));
    }

    // Test let bindings
    #[test]
    fn test_eval_let_simple() {
        let env = Environment::new();
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_let_with_expression() {
        let env = Environment::new();
        // let x = 10 in x + 32
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(10)),
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Var("x".to_string())),
                Box::new(Expr::Int(32)),
            )),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_nested_let() {
        let env = Environment::new();
        // let x = 1 in let y = 2 in x + y
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Let(
                "y".to_string(),
                None,
                Box::new(Expr::Int(2)),
                Box::new(Expr::BinOp(
                    BinOp::Add,
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Var("y".to_string())),
                )),
            )),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(3)));
    }

    #[test]
    fn test_eval_let_shadowing() {
        let env = Environment::new();
        // let x = 1 in let x = 2 in x
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Let(
                "x".to_string(),
                None,
                Box::new(Expr::Int(2)),
                Box::new(Expr::Var("x".to_string())),
            )),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(2)));
    }

    // Test functions and closures
    #[test]
    fn test_eval_fun_creates_closure() {
        let env = Environment::new();
        let expr = Expr::Fun("x".to_string(), None, Box::new(Expr::Var("x".to_string())));
        let result = eval(&expr, &env);
        assert!(matches!(result, Ok(Value::Closure(_, _, _))));
    }

    #[test]
    fn test_eval_simple_app() {
        let env = Environment::new();
        // (fun x -> x) 42
        let expr = Expr::App(
            Box::new(Expr::Fun(
                "x".to_string(),
                None,
                Box::new(Expr::Var("x".to_string())),
            )),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_app_with_operation() {
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

    #[test]
    fn test_eval_curried_function() {
        let env = Environment::new();
        // (fun x -> fun y -> x + y) 40 2
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
                            Box::new(Expr::Var("y".to_string())),
                        )),
                    )),
                )),
                Box::new(Expr::Int(40)),
            )),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

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

    #[test]
    fn test_eval_app_non_function() {
        let env = Environment::new();
        let expr = Expr::App(Box::new(Expr::Int(42)), Box::new(Expr::Int(1)));
        assert!(matches!(eval(&expr, &env), Err(EvalError::TypeError(_))));
    }

    // Test type errors
    #[test]
    fn test_eval_type_error_add_bool() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Bool(true)),
        );
        assert!(matches!(eval(&expr, &env), Err(EvalError::TypeError(_))));
    }

    #[test]
    fn test_eval_type_error_compare_different_types() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Lt,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Bool(true)),
        );
        assert!(matches!(eval(&expr, &env), Err(EvalError::TypeError(_))));
    }

    // Test Environment methods
    #[test]
    fn test_environment_new() {
        let env = Environment::new();
        assert_eq!(env.lookup("x"), None);
    }

    #[test]
    fn test_environment_bind() {
        let mut env = Environment::new();
        env.bind("x".to_string(), Value::Int(42));
        assert_eq!(env.lookup("x"), Some(&Value::Int(42)));
    }

    #[test]
    fn test_environment_lookup_none() {
        let env = Environment::new();
        assert_eq!(env.lookup("nonexistent"), None);
    }

    #[test]
    fn test_environment_extend() {
        let env = Environment::new();
        let new_env = env.extend("x".to_string(), Value::Int(42));
        assert_eq!(new_env.lookup("x"), Some(&Value::Int(42)));
        assert_eq!(env.lookup("x"), None); // Original unchanged
    }

    #[test]
    fn test_environment_extend_shadowing() {
        let mut env = Environment::new();
        env.bind("x".to_string(), Value::Int(1));
        let new_env = env.extend("x".to_string(), Value::Int(2));
        assert_eq!(new_env.lookup("x"), Some(&Value::Int(2)));
        assert_eq!(env.lookup("x"), Some(&Value::Int(1)));
    }

    #[test]
    fn test_environment_default() {
        let env = Environment::default();
        assert_eq!(env.lookup("x"), None);
    }

    // Test Value Display implementation
    #[test]
    fn test_value_display_int() {
        assert_eq!(format!("{}", Value::Int(42)), "42");
        assert_eq!(format!("{}", Value::Int(-10)), "-10");
    }

    #[test]
    fn test_value_display_bool() {
        assert_eq!(format!("{}", Value::Bool(true)), "true");
        assert_eq!(format!("{}", Value::Bool(false)), "false");
    }

    #[test]
    fn test_value_display_closure() {
        let env = Environment::new();
        let closure = Value::Closure("x".to_string(), Expr::Var("x".to_string()), env);
        assert_eq!(format!("{closure}"), "<function x>");
    }

    // Test EvalError Display implementation
    #[test]
    fn test_eval_error_display_unbound_var() {
        let err = EvalError::UnboundVariable("x".to_string());
        assert_eq!(format!("{err}"), "Unbound variable: x");
    }

    #[test]
    fn test_eval_error_display_type_error() {
        let err = EvalError::TypeError("test error".to_string());
        assert_eq!(format!("{err}"), "Type error: test error");
    }

    #[test]
    fn test_eval_error_display_division_by_zero() {
        let err = EvalError::DivisionByZero;
        assert_eq!(format!("{err}"), "Division by zero");
    }

    #[test]
    fn measures_stack_bytes_per_eval_depth_increment_in_test_profile() {
        const STACK_SIZE_BYTES: usize = 4 * 1024 * 1024;
        const DEPTH_LIMIT: usize = 128;

        let thread = std::thread::Builder::new()
            // The test-only override keeps this policy-measurement harness below its 4 MiB stack.
            .stack_size(STACK_SIZE_BYTES)
            .spawn(|| {
                reset_frame_diagnostics();
                set_eval_depth_limit_override(Some(DEPTH_LIMIT));
                start_stack_pointer_measurement();

                let expr = crate::parser::parse_expr("(rec f -> fun n -> 1 + f n) 0")
                    .expect("the non-tail recursion trip expression must parse");
                crate::typechecker::typecheck(&expr)
                    .expect("the non-tail recursion trip expression must typecheck");
                let evaluation = eval(&expr, &Environment::new());
                clear_eval_depth_limit_override();

                assert!(matches!(evaluation, Err(EvalError::RecursionLimit)));
                let (entry_stack_pointer, deepest_stack_pointer) = take_stack_pointer_measurement()
                    .expect("the guard probe must record entry and deepest stack pointers");
                let stack_delta = entry_stack_pointer.abs_diff(deepest_stack_pointer);
                let depth_increments = DEPTH_LIMIT - 1;
                let (eval_inner_entries, eval_app_entries, eval_with_tco_entries) =
                    frame_diagnostic_entry_counts();

                assert!(
                    stack_delta > 0,
                    "the evaluator must consume stack while recursing"
                );
                assert!(eval_inner_entries > 0);
                assert!(eval_app_entries > 0);
                assert!(eval_with_tco_entries > 0);
                eprintln!(
                    "debug/test stack measurement: {stack_delta} bytes across {depth_increments} \
                     depth increments ({} bytes/increment)",
                    stack_delta / depth_increments
                );
            })
            .expect("the measurement thread must spawn");

        thread
            .join()
            .expect("the measurement thread must not panic");
    }

    #[test]
    fn measures_eval_depth_increments_per_non_tail_recursion_level() {
        const LOWER_LOGICAL_DEPTH: i64 = 16;
        const UPPER_LOGICAL_DEPTH: i64 = 32;

        let lower_expr = crate::parser::parse_expr(&format!(
            "(rec f -> fun n -> if n == 0 then 0 else 1 + f (n - 1)) {LOWER_LOGICAL_DEPTH}"
        ))
        .expect("the lower-depth non-tail recursion expression must parse");
        let upper_expr = crate::parser::parse_expr(&format!(
            "(rec f -> fun n -> if n == 0 then 0 else 1 + f (n - 1)) {UPPER_LOGICAL_DEPTH}"
        ))
        .expect("the upper-depth non-tail recursion expression must parse");

        crate::typechecker::typecheck(&lower_expr)
            .expect("the lower-depth non-tail recursion expression must typecheck");
        crate::typechecker::typecheck(&upper_expr)
            .expect("the upper-depth non-tail recursion expression must typecheck");

        let _ = take_max_eval_depth();
        assert_eq!(
            eval(&lower_expr, &Environment::new()),
            Ok(Value::Int(LOWER_LOGICAL_DEPTH))
        );
        let lower_depth = take_max_eval_depth();

        assert_eq!(
            eval(&upper_expr, &Environment::new()),
            Ok(Value::Int(UPPER_LOGICAL_DEPTH))
        );
        let upper_depth = take_max_eval_depth();
        let additional_depth_increments = upper_depth
            .checked_sub(lower_depth)
            .expect("non-tail recursion must increase the recorded evaluator depth");
        let additional_logical_levels = UPPER_LOGICAL_DEPTH - LOWER_LOGICAL_DEPTH;

        assert!(additional_depth_increments > 0);
        eprintln!(
            "debug/test depth multiplier: {additional_depth_increments}/{additional_logical_levels} \
             depth increments per logical non-tail recursion level"
        );
    }

    #[test]
    fn tail_calls_do_not_accumulate_evaluator_depth() {
        const RAMP: [i64; 3] = [1_000, 10_000, 100_000];

        let thread = std::thread::Builder::new()
            .stack_size(EVALUATOR_STACK_SIZE)
            .spawn(|| {
                let mut observed_depths = Vec::with_capacity(RAMP.len());

                for depth in RAMP {
                    let expr = crate::parser::parse_expr(&format!(
                        "(rec f -> fun n -> if n == 0 then 0 else f (n - 1)) {depth}"
                    ))
                    .expect("the tail-recursive countdown must parse");
                    crate::typechecker::typecheck(&expr)
                        .expect("the tail-recursive countdown must typecheck");

                    let _ = take_max_eval_depth();
                    assert_eq!(eval(&expr, &Environment::new()), Ok(Value::Int(0)));
                    observed_depths.push(take_max_eval_depth());
                }

                eprintln!("tail forwarding depth ramp: {observed_depths:?}");
                observed_depths
            })
            .expect("the tail-forwarding test thread must spawn");

        let observed_depths = thread
            .join()
            .expect("the tail-forwarding test thread must not panic");
        // If this grows with the ramp, the expression is not tail-position under this evaluator
        // or the forwarding invariant is broken; stop and report rather than weakening the guard.
        assert!(
            observed_depths
                .windows(2)
                .all(|window| window[0] == window[1]),
            "tail calls must preserve a flat evaluator depth across the ramp"
        );
    }

    #[test]
    fn curried_rec_tail_call_stays_flat_for_small_ramp() {
        const LOWER: i64 = 16;
        const UPPER: i64 = 32;

        let thread = std::thread::Builder::new()
            .stack_size(EVALUATOR_STACK_SIZE)
            .spawn(|| {
                let mut depths = Vec::new();
                for loops in [LOWER, UPPER] {
                    let expr = crate::parser::parse_expr(&format!(
                        "(rec f -> fun acc -> fun n -> if n == 0 then acc else f (acc + n) (n - 1)) 0 {loops}"
                    ))
                    .expect("the curried recursive accumulator must parse");
                    crate::typechecker::typecheck(&expr)
                        .expect("the curried recursive accumulator must typecheck");
                    let _ = take_max_eval_depth();
                    assert_eq!(eval(&expr, &Environment::new()), Ok(Value::Int(loops * (loops + 1) / 2)));
                    depths.push(take_max_eval_depth());
                }
                eprintln!("curried tail-call depth ramp: {depths:?}");
                depths
            })
            .expect("the depth measurement thread must spawn");
        let depths = thread
            .join()
            .expect("the depth measurement thread must not panic");
        assert_eq!(depths[0], depths[1], "curried tail calls must remain flat");
    }

    #[test]
    fn peel_rec_params_binds_acc_first_for_two_arg_call() {
        let body =
            crate::parser::parse_expr("fun n -> if n == 0 then acc else f (acc + n) (n - 1)")
                .expect("the recursive body must parse");
        let (params, inner) = peel_rec_params("acc", &body, 2);
        assert_eq!(params, ["acc", "n"]);
        assert!(matches!(inner, Expr::If(_, _, _)));
    }

    #[test]
    fn curried_if_tail_rec_stays_flat_across_ramp() {
        const RAMP: [i64; 3] = [1_000, 10_000, 100_000];
        let thread = std::thread::Builder::new()
            .stack_size(EVALUATOR_STACK_SIZE)
            .spawn(|| {
                let mut depths = Vec::new();
                for loops in RAMP {
                    let expr = crate::parser::parse_expr(&format!(
                        "(rec f -> fun acc -> fun n -> if n == 0 then acc else f (acc + n) (n - 1)) 0 {loops}"
                    ))
                    .expect("the curried recursive accumulator must parse");
                    crate::typechecker::typecheck(&expr)
                        .expect("the curried recursive accumulator must typecheck");
                    let _ = take_max_eval_depth();
                    assert_eq!(
                        eval(&expr, &Environment::new()),
                        Ok(Value::Int(loops * (loops + 1) / 2))
                    );
                    depths.push(take_max_eval_depth());
                }
                eprintln!("curried if-tail depth ramp: {depths:?}");
                depths
            })
            .expect("the curried ramp thread must spawn");
        let depths = thread.join().expect("the curried ramp must not panic");
        assert!(depths.windows(2).all(|window| window[0] == window[1]));
    }

    #[test]
    fn corpus_match_tail_workload_stays_flat_across_ramp() {
        // Value environments clone their recursive list bindings; keep the corpus smoke ramp
        // small while the scalar curried-rec ramp above establishes the 100,000-call proof.
        const RAMP: [usize; 3] = [1, 5, 10];
        let thread = std::thread::Builder::new()
            .stack_size(EVALUATOR_STACK_SIZE)
            .spawn(|| {
                let definitions = crate::parser::parse_expr(
                    "data List a = Nil | Cons a (List a) in load \"examples/string.par\" in strrev",
                )
                .expect("the string corpus wrapper must parse");
                let strrev = eval(&definitions, &Environment::new())
                    .expect("the string corpus strrev closure must evaluate");
                let mut depths = Vec::new();
                for length in RAMP {
                    let mut input = Value::Variant("Nil".to_string(), Vec::new());
                    for _ in 0..length {
                        input = Value::Variant("Cons".to_string(), vec![Value::Char('x'), input]);
                    }
                    let _ = take_max_eval_depth();
                    let result = apply_value_spine(
                        strrev.clone(),
                        vec![Value::Variant("Nil".to_string(), Vec::new()), input],
                        0,
                    );
                    assert!(matches!(
                        result,
                        Ok(Value::Variant(name, _)) if name == "Cons"
                    ));
                    depths.push(take_max_eval_depth());
                }
                eprintln!("corpus match-tail depth ramp: {depths:?}");
                depths
            })
            .expect("the corpus match-tail measurement thread must spawn");
        let depths = thread
            .join()
            .expect("the corpus match-tail measurement thread must not panic");
        assert!(depths.windows(2).all(|window| window[0] == window[1]));
    }

    // Test Value Clone and PartialEq
    #[test]
    fn test_value_clone() {
        let val = Value::Int(42);
        let cloned = val.clone();
        assert_eq!(val, cloned);
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
    }

    // Test complex scenarios
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

    #[test]
    fn test_eval_partial_application() {
        let env = Environment::new();
        // let add = fun x -> fun y -> x + y in let add5 = add 5 in add5 10
        let expr = Expr::Let(
            "add".to_string(),
            None,
            Box::new(Expr::Fun(
                "x".to_string(),
                None,
                Box::new(Expr::Fun(
                    "y".to_string(),
                    None,
                    Box::new(Expr::BinOp(
                        BinOp::Add,
                        Box::new(Expr::Var("x".to_string())),
                        Box::new(Expr::Var("y".to_string())),
                    )),
                )),
            )),
            Box::new(Expr::Let(
                "add5".to_string(),
                None,
                Box::new(Expr::App(
                    Box::new(Expr::Var("add".to_string())),
                    Box::new(Expr::Int(5)),
                )),
                Box::new(Expr::App(
                    Box::new(Expr::Var("add5".to_string())),
                    Box::new(Expr::Int(10)),
                )),
            )),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(15)));
    }

    #[test]
    fn test_eval_nested_if() {
        let env = Environment::new();
        // if true then (if false then 1 else 2) else 3
        let expr = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::If(
                Box::new(Expr::Bool(false)),
                Box::new(Expr::Int(1)),
                Box::new(Expr::Int(2)),
            )),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(2)));
    }

    // Test load expression
    #[test]
    fn test_load_simple_library() {
        use std::fs;

        // Create a temporary library file
        let lib_content = "let double = fun x -> x * 2 in 0";
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_load_simple.par");
        fs::write(&temp_file, lib_content).unwrap();

        let env = Environment::new();
        let expr = Expr::Load(
            temp_file.to_str().unwrap().to_string(),
            Box::new(Expr::App(
                Box::new(Expr::Var("double".to_string())),
                Box::new(Expr::Int(21)),
            )),
        );

        let result = eval(&expr, &env);
        assert_eq!(result, Ok(Value::Int(42)));

        // Cleanup
        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_load_multiple_functions() {
        use std::fs;

        // Create a library with multiple functions
        let lib_content = "let double = fun x -> x * 2 in let triple = fun x -> x * 3 in 0";
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_load_multiple.par");
        fs::write(&temp_file, lib_content).unwrap();

        let env = Environment::new();
        // Use both double and triple
        let expr = Expr::Load(
            temp_file.to_str().unwrap().to_string(),
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::App(
                    Box::new(Expr::Var("double".to_string())),
                    Box::new(Expr::Int(10)),
                )),
                Box::new(Expr::App(
                    Box::new(Expr::Var("triple".to_string())),
                    Box::new(Expr::Int(7)),
                )),
            )),
        );

        let result = eval(&expr, &env);
        assert_eq!(result, Ok(Value::Int(41))); // 10*2 + 7*3 = 20 + 21 = 41

        // Cleanup
        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_load_with_nested_lets() {
        use std::fs;

        // Library with nested lets creating multiple bindings
        let lib_content = "let square = fun x -> x * x in let cube = fun x -> x * x * x in 0";
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_load_nested_lets.par");
        fs::write(&temp_file, lib_content).unwrap();

        let env = Environment::new();
        let expr = Expr::Load(
            temp_file.to_str().unwrap().to_string(),
            Box::new(Expr::App(
                Box::new(Expr::Var("cube".to_string())),
                Box::new(Expr::Int(3)),
            )),
        );

        let result = eval(&expr, &env);
        assert_eq!(result, Ok(Value::Int(27))); // 3^3 = 27

        // Cleanup
        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_load_file_not_found() {
        let env = Environment::new();
        let expr = Expr::Load("/nonexistent/file.par".to_string(), Box::new(Expr::Int(42)));

        let result = eval(&expr, &env);
        assert!(matches!(result, Err(EvalError::LoadError(_))));
        if let Err(EvalError::LoadError(msg)) = result {
            assert!(msg.contains("Failed to read file"));
        }
    }

    #[test]
    fn test_load_parse_error() {
        use std::fs;

        // Create a file with invalid syntax
        let lib_content = "let x = ";
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_load_parse_error.par");
        fs::write(&temp_file, lib_content).unwrap();

        let env = Environment::new();
        let expr = Expr::Load(
            temp_file.to_str().unwrap().to_string(),
            Box::new(Expr::Int(42)),
        );

        let result = eval(&expr, &env);
        assert!(matches!(result, Err(EvalError::LoadError(_))));
        if let Err(EvalError::LoadError(msg)) = result {
            assert!(msg.contains("Failed to parse file"));
        }

        // Cleanup
        fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_load_nested_load() {
        use std::fs;

        // Create first library
        let lib1_content = "let helper = fun x -> x + 1 in 0";
        let temp_dir = std::env::temp_dir();
        let temp_file1 = temp_dir.join("test_load_lib1.par");
        fs::write(&temp_file1, lib1_content).unwrap();

        // Create second library that loads the first
        let lib2_content = format!(
            "load \"{}\" in let double_helper = fun x -> helper (helper x) in 0",
            temp_file1.to_str().unwrap()
        );
        let temp_file2 = temp_dir.join("test_load_lib2.par");
        fs::write(&temp_file2, &lib2_content).unwrap();

        let env = Environment::new();
        let expr = Expr::Load(
            temp_file2.to_str().unwrap().to_string(),
            Box::new(Expr::App(
                Box::new(Expr::Var("double_helper".to_string())),
                Box::new(Expr::Int(10)),
            )),
        );

        let result = eval(&expr, &env);
        assert_eq!(result, Ok(Value::Int(12))); // 10 + 1 + 1 = 12

        // Cleanup
        fs::remove_file(&temp_file1).ok();
        fs::remove_file(&temp_file2).ok();
    }

    #[test]
    fn test_load_preserves_outer_bindings() {
        use std::fs;

        // Create a library
        let lib_content = "let double = fun x -> x * 2 in 0";
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_load_preserve.par");
        fs::write(&temp_file, lib_content).unwrap();

        // Create an environment with existing bindings
        let mut env = Environment::new();
        env.bind("y".to_string(), Value::Int(10));

        // Load library and use both outer and library bindings
        let expr = Expr::Load(
            temp_file.to_str().unwrap().to_string(),
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Var("y".to_string())),
                Box::new(Expr::App(
                    Box::new(Expr::Var("double".to_string())),
                    Box::new(Expr::Int(5)),
                )),
            )),
        );

        let result = eval(&expr, &env);
        assert_eq!(result, Ok(Value::Int(20))); // 10 + (5*2) = 20

        // Cleanup
        fs::remove_file(&temp_file).ok();
    }

    // Test environment merge
    #[test]
    fn test_environment_merge() {
        let mut env1 = Environment::new();
        env1.bind("x".to_string(), Value::Int(1));

        let mut env2 = Environment::new();
        env2.bind("y".to_string(), Value::Int(2));

        let merged = env1.merge(&env2);
        assert_eq!(merged.lookup("x"), Some(&Value::Int(1)));
        assert_eq!(merged.lookup("y"), Some(&Value::Int(2)));
    }

    #[test]
    fn test_environment_merge_shadowing() {
        let mut env1 = Environment::new();
        env1.bind("x".to_string(), Value::Int(1));

        let mut env2 = Environment::new();
        env2.bind("x".to_string(), Value::Int(2));

        let merged = env1.merge(&env2);
        // Later binding should shadow
        assert_eq!(merged.lookup("x"), Some(&Value::Int(2)));
    }

    // Test extract_bindings helper
    #[test]
    fn test_extract_bindings_single() {
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Int(0)),
        );
        let env = Environment::new();
        let result_env = extract_bindings(&expr, &env).unwrap();
        assert_eq!(result_env.lookup("x"), Some(&Value::Int(42)));
    }

    #[test]
    fn test_extract_bindings_nested() {
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Let(
                "y".to_string(),
                None,
                Box::new(Expr::Int(2)),
                Box::new(Expr::Int(0)),
            )),
        );
        let env = Environment::new();
        let result_env = extract_bindings(&expr, &env).unwrap();
        assert_eq!(result_env.lookup("x"), Some(&Value::Int(1)));
        assert_eq!(result_env.lookup("y"), Some(&Value::Int(2)));
    }

    #[test]
    fn test_extract_bindings_with_functions() {
        let expr = Expr::Let(
            "double".to_string(),
            None,
            Box::new(Expr::Fun(
                "x".to_string(),
                None,
                Box::new(Expr::BinOp(
                    BinOp::Mul,
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Int(2)),
                )),
            )),
            Box::new(Expr::Int(0)),
        );
        let env = Environment::new();
        let result_env = extract_bindings(&expr, &env).unwrap();
        assert!(matches!(
            result_env.lookup("double"),
            Some(Value::Closure(_, _, _))
        ));
    }

    // Test EvalError Display for LoadError
    #[test]
    fn test_eval_error_display_load_error() {
        let err = EvalError::LoadError("test load error".to_string());
        assert_eq!(format!("{err}"), "Load error: test load error");
    }

    // Test top-level program evaluation
    #[test]
    fn test_eval_program_single() {
        let env = Environment::new();
        let program = Program {
            decls: vec![Decl::Let {
                name: "x".to_string(),
                ty_ann: None,
                value: Expr::Int(42),
                doc: None,
            }],
            body: Some(Expr::Var("x".to_string())),
        };
        assert_eq!(eval_program(&program, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_program_multiple() {
        let env = Environment::new();
        let program = Program {
            decls: vec![
                Decl::Let {
                    name: "x".to_string(),
                    ty_ann: None,
                    value: Expr::Int(10),
                    doc: None,
                },
                Decl::Let {
                    name: "y".to_string(),
                    ty_ann: None,
                    value: Expr::Int(32),
                    doc: None,
                },
            ],
            body: Some(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Var("x".to_string())),
                Box::new(Expr::Var("y".to_string())),
            )),
        };
        assert_eq!(eval_program(&program, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_program_with_functions() {
        let env = Environment::new();
        let program = Program {
            decls: vec![Decl::Let {
                name: "double".to_string(),
                ty_ann: None,
                value: Expr::Fun(
                    "x".to_string(),
                    None,
                    Box::new(Expr::BinOp(
                        BinOp::Mul,
                        Box::new(Expr::Var("x".to_string())),
                        Box::new(Expr::Int(2)),
                    )),
                ),
                doc: None,
            }],
            body: Some(Expr::App(
                Box::new(Expr::Var("double".to_string())),
                Box::new(Expr::Int(21)),
            )),
        };
        assert_eq!(eval_program(&program, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_program_scoping() {
        let env = Environment::new();
        // let x = 10; let y = x + 5; y
        let program = Program {
            decls: vec![
                Decl::Let {
                    name: "x".to_string(),
                    ty_ann: None,
                    value: Expr::Int(10),
                    doc: None,
                },
                Decl::Let {
                    name: "y".to_string(),
                    ty_ann: None,
                    value: Expr::BinOp(
                        BinOp::Add,
                        Box::new(Expr::Var("x".to_string())),
                        Box::new(Expr::Int(5)),
                    ),
                    doc: None,
                },
            ],
            body: Some(Expr::Var("y".to_string())),
        };
        assert_eq!(eval_program(&program, &env), Ok(Value::Int(15)));
    }

    #[test]
    fn test_extend_env_with_program() {
        let program = Program {
            decls: vec![
                Decl::Let {
                    name: "x".to_string(),
                    ty_ann: None,
                    value: Expr::Int(1),
                    doc: None,
                },
                Decl::Let {
                    name: "y".to_string(),
                    ty_ann: None,
                    value: Expr::Int(2),
                    doc: None,
                },
            ],
            body: Some(Expr::Int(0)),
        };
        let env = Environment::new();
        let result_env = extend_env_with_program(&program, &env).unwrap();
        assert_eq!(result_env.lookup("x"), Some(&Value::Int(1)));
        assert_eq!(result_env.lookup("y"), Some(&Value::Int(2)));
    }

    #[test]
    fn test_load_many_function_bindings() {
        use std::fs;

        let bindings = (0..8)
            .map(|index| format!("let f{index} = fun x -> x in"))
            .collect::<Vec<_>>()
            .join(" ");
        let lib_content = format!("{bindings} 0");
        let temp_file = std::env::temp_dir().join(format!(
            "test_load_many_function_bindings_{}.par",
            std::process::id()
        ));
        fs::write(&temp_file, lib_content).unwrap();

        let expr = Expr::Load(
            temp_file.to_string_lossy().into_owned(),
            Box::new(Expr::App(
                Box::new(Expr::Var("f7".to_string())),
                Box::new(Expr::Int(42)),
            )),
        );

        assert_eq!(eval(&expr, &Environment::new()), Ok(Value::Int(42)));
        fs::remove_file(&temp_file).ok();
    }

    // Test Tuple evaluation
    #[test]
    fn test_eval_tuple_simple() {
        let env = Environment::new();
        let expr = Expr::Tuple(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)]);
        assert_eq!(
            eval(&expr, &env),
            Ok(Value::Tuple(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3)
            ]))
        );
    }

    #[test]
    fn test_eval_tuple_empty() {
        let env = Environment::new();
        let expr = Expr::Tuple(vec![]);
        assert_eq!(eval(&expr, &env), Ok(Value::Tuple(vec![])));
    }

    #[test]
    fn test_eval_tuple_mixed() {
        let env = Environment::new();
        let expr = Expr::Tuple(vec![Expr::Int(42), Expr::Bool(true)]);
        assert_eq!(
            eval(&expr, &env),
            Ok(Value::Tuple(vec![Value::Int(42), Value::Bool(true)]))
        );
    }

    #[test]
    fn test_eval_tuple_nested() {
        let env = Environment::new();
        let expr = Expr::Tuple(vec![
            Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]),
            Expr::Int(3),
        ]);
        assert_eq!(
            eval(&expr, &env),
            Ok(Value::Tuple(vec![
                Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
                Value::Int(3)
            ]))
        );
    }

    #[test]
    fn test_eval_tuple_with_var() {
        let mut env = Environment::new();
        env.bind("x".to_string(), Value::Int(10));
        let expr = Expr::Tuple(vec![Expr::Var("x".to_string()), Expr::Int(20)]);
        assert_eq!(
            eval(&expr, &env),
            Ok(Value::Tuple(vec![Value::Int(10), Value::Int(20)]))
        );
    }

    // Test TupleProj evaluation
    #[test]
    fn test_eval_tuple_proj_first() {
        let env = Environment::new();
        let expr = Expr::TupleProj(Box::new(Expr::Tuple(vec![Expr::Int(10), Expr::Int(20)])), 0);
        assert_eq!(eval(&expr, &env), Ok(Value::Int(10)));
    }

    #[test]
    fn test_eval_tuple_proj_second() {
        let env = Environment::new();
        let expr = Expr::TupleProj(Box::new(Expr::Tuple(vec![Expr::Int(10), Expr::Int(20)])), 1);
        assert_eq!(eval(&expr, &env), Ok(Value::Int(20)));
    }

    #[test]
    fn test_eval_tuple_proj_nested() {
        let env = Environment::new();
        // ((1, 2), (3, 4)).0.1 => 2
        let expr = Expr::TupleProj(
            Box::new(Expr::TupleProj(
                Box::new(Expr::Tuple(vec![
                    Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]),
                    Expr::Tuple(vec![Expr::Int(3), Expr::Int(4)]),
                ])),
                0,
            )),
            1,
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(2)));
    }

    #[test]
    fn test_eval_tuple_proj_out_of_bounds() {
        let env = Environment::new();
        let expr = Expr::TupleProj(Box::new(Expr::Tuple(vec![Expr::Int(10), Expr::Int(20)])), 2);
        assert!(matches!(
            eval(&expr, &env),
            Err(EvalError::IndexOutOfBounds(_))
        ));
    }

    #[test]
    fn test_eval_tuple_proj_non_tuple() {
        let env = Environment::new();
        let expr = Expr::TupleProj(Box::new(Expr::Int(42)), 0);
        assert!(matches!(eval(&expr, &env), Err(EvalError::TypeError(_))));
    }

    // Test pattern matching with tuples
    #[test]
    fn test_match_pattern_tuple_simple() {
        let env = Environment::new();
        let pattern = Pattern::Tuple(vec![
            Pattern::Var("x".to_string()),
            Pattern::Var("y".to_string()),
        ]);
        let value = Value::Tuple(vec![Value::Int(1), Value::Int(2)]);
        let result = match_pattern(&pattern, &value, &env);
        assert!(result.is_some());
        let new_env = result.unwrap();
        assert_eq!(new_env.lookup("x"), Some(&Value::Int(1)));
        assert_eq!(new_env.lookup("y"), Some(&Value::Int(2)));
    }

    #[test]
    fn test_match_pattern_tuple_with_literal() {
        let env = Environment::new();
        let pattern = Pattern::Tuple(vec![
            Pattern::Literal(Literal::Int(0)),
            Pattern::Var("y".to_string()),
        ]);
        let value = Value::Tuple(vec![Value::Int(0), Value::Int(5)]);
        let result = match_pattern(&pattern, &value, &env);
        assert!(result.is_some());
        let new_env = result.unwrap();
        assert_eq!(new_env.lookup("y"), Some(&Value::Int(5)));
    }

    #[test]
    fn test_match_pattern_tuple_mismatch() {
        let env = Environment::new();
        let pattern = Pattern::Tuple(vec![
            Pattern::Literal(Literal::Int(0)),
            Pattern::Var("y".to_string()),
        ]);
        let value = Value::Tuple(vec![Value::Int(1), Value::Int(5)]);
        let result = match_pattern(&pattern, &value, &env);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_pattern_tuple_wrong_length() {
        let env = Environment::new();
        let pattern = Pattern::Tuple(vec![Pattern::Var("x".to_string())]);
        let value = Value::Tuple(vec![Value::Int(1), Value::Int(2)]);
        let result = match_pattern(&pattern, &value, &env);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_pattern_tuple_nested() {
        let env = Environment::new();
        let pattern = Pattern::Tuple(vec![
            Pattern::Tuple(vec![
                Pattern::Var("a".to_string()),
                Pattern::Var("b".to_string()),
            ]),
            Pattern::Var("c".to_string()),
        ]);
        let value = Value::Tuple(vec![
            Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
            Value::Int(3),
        ]);
        let result = match_pattern(&pattern, &value, &env);
        assert!(result.is_some());
        let new_env = result.unwrap();
        assert_eq!(new_env.lookup("a"), Some(&Value::Int(1)));
        assert_eq!(new_env.lookup("b"), Some(&Value::Int(2)));
        assert_eq!(new_env.lookup("c"), Some(&Value::Int(3)));
    }

    #[test]
    fn test_eval_match_with_tuple() {
        let env = Environment::new();
        // match (10, 20) with | (0, 0) -> 0 | (x, y) -> x + y
        let expr = Expr::Match(
            Box::new(Expr::Tuple(vec![Expr::Int(10), Expr::Int(20)])),
            vec![
                (
                    Pattern::Tuple(vec![
                        Pattern::Literal(Literal::Int(0)),
                        Pattern::Literal(Literal::Int(0)),
                    ]),
                    Expr::Int(0),
                ),
                (
                    Pattern::Tuple(vec![
                        Pattern::Var("x".to_string()),
                        Pattern::Var("y".to_string()),
                    ]),
                    Expr::BinOp(
                        BinOp::Add,
                        Box::new(Expr::Var("x".to_string())),
                        Box::new(Expr::Var("y".to_string())),
                    ),
                ),
            ],
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(30)));
    }

    // Test Value Display
    #[test]
    fn test_value_display_tuple() {
        let val = Value::Tuple(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(format!("{val}"), "(1, 2, 3)");
    }

    #[test]
    fn test_value_display_tuple_empty() {
        let val = Value::Tuple(vec![]);
        assert_eq!(format!("{val}"), "()");
    }

    #[test]
    fn test_value_display_tuple_nested() {
        let val = Value::Tuple(vec![
            Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
            Value::Int(3),
        ]);
        assert_eq!(format!("{val}"), "((1, 2), 3)");
    }
}
