/// Evaluator/Interpreter for the `ParLang` language
/// This module implements the runtime evaluation of `ParLang` expressions
use crate::ast::{BinOp, Expr, Literal, Pattern};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

/// Runtime values in the language
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Closure(String, Expr, Environment),
    /// Recursive closure: function name, parameter name, body, environment
    RecClosure(String, String, Expr, Environment),
    /// Tuple of values
    Tuple(Vec<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
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
        }
    }
}

/// Environment for variable bindings
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    bindings: HashMap<String, Value>,
}

impl Environment {
    #[must_use]
    pub fn new() -> Self {
        Environment {
            bindings: HashMap::new(),
        }
    }

    pub fn bind(&mut self, name: String, value: Value) {
        self.bindings.insert(name, value);
    }

    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.bindings.get(name)
    }

    #[must_use]
    pub fn extend(&self, name: String, value: Value) -> Self {
        let mut new_env = self.clone();
        new_env.bind(name, value);
        new_env
    }

    #[must_use]
    pub fn merge(&self, other: &Environment) -> Self {
        let mut new_env = self.clone();
        for (name, value) in &other.bindings {
            new_env.bind(name.clone(), value.clone());
        }
        new_env
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
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EvalError::UnboundVariable(name) => write!(f, "Unbound variable: {name}"),
            EvalError::TypeError(msg) => write!(f, "Type error: {msg}"),
            EvalError::DivisionByZero => write!(f, "Division by zero"),
            EvalError::LoadError(msg) => write!(f, "Load error: {msg}"),
            EvalError::IndexOutOfBounds(msg) => write!(f, "Index out of bounds: {msg}"),
        }
    }
}

impl std::error::Error for EvalError {}

/// Evaluate an expression with tail call optimization for recursive functions
/// This function uses iteration instead of recursion for tail-recursive calls
/// 
/// Note: This implementation clones the body and environment on each iteration.
/// A future optimization could use Rc/Arc to reduce allocations for deep recursion.
fn eval_with_tco(
    body: &Expr,
    initial_env: &Environment,
    rec_name: &str,
    param_name: &str,
    closure_env: &Environment,
) -> Result<Value, EvalError> {
    let mut current_expr = body.clone();
    let mut current_env = initial_env.clone();
    
    loop {
        // Check if the expression is a tail call to the recursive function
        match &current_expr {
            // Direct tail call: rec_name arg
            Expr::App(func, arg) => {
                // Check if this is a call to the recursive function (possibly nested in applications)
                if is_tail_call_to(func, rec_name) {
                    // This is a tail call - evaluate arg and loop instead of recursing
                    let arg_val = eval(arg, &current_env)?;
                    
                    // Reset environment for next iteration
                    let rec_val = Value::RecClosure(
                        rec_name.to_string(),
                        param_name.to_string(),
                        body.clone(),
                        closure_env.clone(),
                    );
                    current_env = closure_env.extend(rec_name.to_string(), rec_val);
                    current_env = current_env.extend(param_name.to_string(), arg_val);
                    current_expr = body.clone();
                    continue;
                }
                // Not a tail call to self - evaluate normally and return
                break eval(&current_expr, &current_env);
            }
            // Handle if expressions - evaluate condition and continue with the appropriate branch
            Expr::If(cond, then_branch, else_branch) => {
                let cond_val = eval(cond, &current_env)?;
                match cond_val {
                    Value::Bool(true) => {
                        current_expr = (**then_branch).clone();
                    }
                    Value::Bool(false) => {
                        current_expr = (**else_branch).clone();
                    }
                    _ => return Err(EvalError::TypeError(
                        "if condition must evaluate to a boolean".to_string(),
                    )),
                }
            }
            // For other expressions, evaluate normally and return
            _ => break eval(&current_expr, &current_env),
        }
    }
}

/// Check if an expression is ultimately a call to the recursive function
/// Handles nested applications like: (`rec_name` arg1) arg2
fn is_tail_call_to(expr: &Expr, rec_name: &str) -> bool {
    match expr {
        Expr::Var(name) => name == rec_name,
        Expr::App(func, _) => is_tail_call_to(func, rec_name),
        _ => false,
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
        Expr::Let(name, value, body) => {
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
            let content = fs::read_to_string(Path::new(filepath))
                .map_err(|e| EvalError::LoadError(format!("Failed to read file '{filepath}': {e}")))?;
            let lib_expr = crate::parser::parse(&content)
                .map_err(|e| EvalError::LoadError(format!("Failed to parse file '{filepath}': {e}")))?;
            
            // Extract bindings from the loaded library
            let lib_env = extract_bindings(&lib_expr, &Environment::new())?;
            // Merge with current environment
            let new_env = env.merge(&lib_env);
            // Continue extracting from the body
            extract_bindings(body, &new_env)
        }
        Expr::Seq(bindings, body) => {
            // Process each binding in the sequence
            let mut current_env = env.clone();
            for (name, value) in bindings {
                let val = eval(value, &current_env)?;
                current_env = current_env.extend(name.clone(), val);
            }
            // Continue extracting from the body
            extract_bindings(body, &current_env)
        }
        // If we reach anything other than a Let, Load, or Seq, we're done extracting
        // Return the accumulated environment
        _ => Ok(env.clone()),
    }
}

/// Match a pattern against a value, returning an extended environment if successful
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
    match expr {
        Expr::Int(n) => Ok(Value::Int(*n)),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        
        Expr::Var(name) => env
            .lookup(name)
            .cloned()
            .ok_or_else(|| EvalError::UnboundVariable(name.clone())),
        
        Expr::BinOp(op, left, right) => {
            let left_val = eval(left, env)?;
            let right_val = eval(right, env)?;
            eval_binop(*op, left_val, right_val)
        }
        
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
        
        Expr::Let(name, value, body) => {
            let val = eval(value, env)?;
            let new_env = env.extend(name.clone(), val);
            eval(body, &new_env)
        }
        
        Expr::Fun(param, body) => Ok(Value::Closure(
            param.clone(),
            (**body).clone(),
            env.clone(),
        )),
        
        Expr::App(func, arg) => {
            let func_val = eval(func, env)?;
            let arg_val = eval(arg, env)?;
            
            match func_val {
                Value::Closure(param, body, closure_env) => {
                    let new_env = closure_env.extend(param, arg_val);
                    eval(&body, &new_env)
                }
                Value::RecClosure(rec_name, param, body, closure_env) => {
                    // Create an environment with the recursive function bound to itself
                    let rec_val = Value::RecClosure(
                        rec_name.clone(),
                        param.clone(),
                        body.clone(),
                        closure_env.clone(),
                    );
                    let env_with_rec = closure_env.extend(rec_name.clone(), rec_val);
                    let new_env = env_with_rec.extend(param.clone(), arg_val);
                    
                    // Evaluate the body - TCO happens naturally via iteration below
                    // when the body is a tail call
                    eval_with_tco(&body, &new_env, &rec_name, &param, &closure_env)
                }
                _ => Err(EvalError::TypeError(
                    "Application requires a function".to_string(),
                )),
            }
        }
        
        Expr::Load(filepath, body) => {
            // Read the file contents
            let content = fs::read_to_string(Path::new(filepath))
                .map_err(|e| EvalError::LoadError(format!("Failed to read file '{filepath}': {e}")))?;
            
            // Parse the file contents
            let lib_expr = crate::parser::parse(&content)
                .map_err(|e| EvalError::LoadError(format!("Failed to parse file '{filepath}': {e}")))?;
            
            // Extract bindings from the library file
            let lib_env = extract_bindings(&lib_expr, &Environment::new())?;
            
            // Merge library bindings into current environment
            let extended_env = env.merge(&lib_env);
            
            // Evaluate the body in the extended environment
            eval(body, &extended_env)
        }
        
        Expr::Seq(bindings, body) => {
            // Process each binding in sequence, extending the environment
            let mut current_env = env.clone();
            for (name, value) in bindings {
                let val = eval(value, &current_env)?;
                current_env = current_env.extend(name.clone(), val);
            }
            // Evaluate the body in the extended environment
            eval(body, &current_env)
        }
        
        Expr::Rec(name, body) => {
            // Parse the body which should be a function (fun param -> expr)
            // The recursive function can reference itself by name within its body
            match body.as_ref() {
                Expr::Fun(param, fun_body) => {
                    // Create a recursive closure that captures the function name
                    Ok(Value::RecClosure(
                        name.clone(),
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
        
        Expr::Match(scrutinee, arms) => {
            // Evaluate the scrutinee expression
            let val = eval(scrutinee, env)?;
            
            // Try to match against each pattern arm in order
            for (pattern, result_expr) in arms {
                if let Some(new_env) = match_pattern(pattern, &val, env) {
                    // Pattern matched, evaluate the result expression with the extended environment
                    return eval(result_expr, &new_env);
                }
            }
            
            // No pattern matched
            Err(EvalError::TypeError(
                "No pattern matched in match expression".to_string(),
            ))
        }
        
        Expr::Tuple(elements) => {
            // Evaluate all elements of the tuple
            let mut values = Vec::new();
            for elem in elements {
                values.push(eval(elem, env)?);
            }
            Ok(Value::Tuple(values))
        }
        
        Expr::TupleProj(tuple_expr, index) => {
            // Evaluate the tuple expression
            let tuple_val = eval(tuple_expr, env)?;
            
            // Check that the value is a tuple
            match tuple_val {
                Value::Tuple(values) => {
                    // Check bounds
                    if *index >= values.len() {
                        Err(EvalError::IndexOutOfBounds(format!(
                            "Tuple index {} out of bounds for tuple of size {}",
                            index,
                            values.len()
                        )))
                    } else {
                        Ok(values[*index].clone())
                    }
                }
                _ => Err(EvalError::TypeError(
                    "Tuple projection requires a tuple".to_string(),
                )),
            }
        }
    }
}

/// Evaluate a binary operation
fn eval_binop(op: BinOp, left: Value, right: Value) -> Result<Value, EvalError> {
    match (op, left, right) {
        // Arithmetic operations
        (BinOp::Add, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        (BinOp::Sub, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
        (BinOp::Mul, Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
        (BinOp::Div, Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                Err(EvalError::DivisionByZero)
            } else {
                Ok(Value::Int(a / b))
            }
        }
        
        // Comparison operations
        (BinOp::Eq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a == b)),
        (BinOp::Neq, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a != b)),
        (BinOp::Lt, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a < b)),
        (BinOp::Le, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a <= b)),
        (BinOp::Gt, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a > b)),
        (BinOp::Ge, Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a >= b)),
        
        (BinOp::Eq, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
        (BinOp::Neq, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a != b)),
        
        _ => Err(EvalError::TypeError(format!(
            "Type error in binary operation: {op:?}"
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
        let expr = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(3)));
    }

    #[test]
    fn test_eval_let() {
        let env = Environment::new();
        let expr = Expr::Let(
            "x".to_string(),
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
        assert!(matches!(eval(&expr, &env), Err(EvalError::UnboundVariable(_))));
    }

    // Test all arithmetic operations
    #[test]
    fn test_eval_add() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(10)),
            Box::new(Expr::Int(32)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_sub() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Sub,
            Box::new(Expr::Int(50)),
            Box::new(Expr::Int(8)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_mul() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::Int(6)),
            Box::new(Expr::Int(7)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_div() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Div,
            Box::new(Expr::Int(84)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

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

    // Test all comparison operations
    #[test]
    fn test_eval_eq_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Eq,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_eq_false() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Eq,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Int(43)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn test_eval_neq_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Neq,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Int(43)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_neq_false() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Neq,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn test_eval_lt_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Lt,
            Box::new(Expr::Int(3)),
            Box::new(Expr::Int(5)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_lt_false() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Lt,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn test_eval_le_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Le,
            Box::new(Expr::Int(3)),
            Box::new(Expr::Int(5)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_le_equal() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Le,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(5)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_gt_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Gt,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_gt_false() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Gt,
            Box::new(Expr::Int(3)),
            Box::new(Expr::Int(5)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
    }

    #[test]
    fn test_eval_ge_true() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Ge,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
    }

    #[test]
    fn test_eval_ge_equal() {
        let env = Environment::new();
        let expr = Expr::BinOp(
            BinOp::Ge,
            Box::new(Expr::Int(5)),
            Box::new(Expr::Int(5)),
        );
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
            Box::new(Expr::Int(1)),
            Box::new(Expr::Let(
                "y".to_string(),
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
            Box::new(Expr::Int(1)),
            Box::new(Expr::Let(
                "x".to_string(),
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
        let expr = Expr::Fun("x".to_string(), Box::new(Expr::Var("x".to_string())));
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
                    Box::new(Expr::Fun(
                        "y".to_string(),
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
            Box::new(Expr::Int(10)),
            Box::new(Expr::App(
                Box::new(Expr::Fun(
                    "y".to_string(),
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
            Box::new(Expr::Fun(
                "x".to_string(),
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
            Box::new(Expr::Fun(
                "x".to_string(),
                Box::new(Expr::Fun(
                    "y".to_string(),
                    Box::new(Expr::BinOp(
                        BinOp::Add,
                        Box::new(Expr::Var("x".to_string())),
                        Box::new(Expr::Var("y".to_string())),
                    )),
                )),
            )),
            Box::new(Expr::Let(
                "add5".to_string(),
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
        let expr = Expr::Load(
            "/nonexistent/file.par".to_string(),
            Box::new(Expr::Int(42)),
        );
        
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
        let lib2_content = format!("load \"{}\" in let double_helper = fun x -> helper (helper x) in 0", temp_file1.to_str().unwrap());
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
            Box::new(Expr::Int(1)),
            Box::new(Expr::Let(
                "y".to_string(),
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
            Box::new(Expr::Fun(
                "x".to_string(),
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
        assert!(matches!(result_env.lookup("double"), Some(Value::Closure(_, _, _))));
    }

    // Test EvalError Display for LoadError
    #[test]
    fn test_eval_error_display_load_error() {
        let err = EvalError::LoadError("test load error".to_string());
        assert_eq!(format!("{err}"), "Load error: test load error");
    }

    // Test Seq evaluation
    #[test]
    fn test_eval_seq_single() {
        let env = Environment::new();
        let bindings = vec![("x".to_string(), Expr::Int(42))];
        let expr = Expr::Seq(bindings, Box::new(Expr::Var("x".to_string())));
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_seq_multiple() {
        let env = Environment::new();
        let bindings = vec![
            ("x".to_string(), Expr::Int(10)),
            ("y".to_string(), Expr::Int(32)),
        ];
        let expr = Expr::Seq(
            bindings,
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Var("x".to_string())),
                Box::new(Expr::Var("y".to_string())),
            )),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_seq_with_functions() {
        let env = Environment::new();
        let bindings = vec![(
            "double".to_string(),
            Expr::Fun(
                "x".to_string(),
                Box::new(Expr::BinOp(
                    BinOp::Mul,
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Int(2)),
                )),
            ),
        )];
        let expr = Expr::Seq(
            bindings,
            Box::new(Expr::App(
                Box::new(Expr::Var("double".to_string())),
                Box::new(Expr::Int(21)),
            )),
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(42)));
    }

    #[test]
    fn test_eval_seq_scoping() {
        let env = Environment::new();
        // let x = 10; let y = x + 5; y
        let bindings = vec![
            ("x".to_string(), Expr::Int(10)),
            (
                "y".to_string(),
                Expr::BinOp(
                    BinOp::Add,
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Int(5)),
                ),
            ),
        ];
        let expr = Expr::Seq(bindings, Box::new(Expr::Var("y".to_string())));
        assert_eq!(eval(&expr, &env), Ok(Value::Int(15)));
    }

    #[test]
    fn test_extract_bindings_seq() {
        let bindings = vec![
            ("x".to_string(), Expr::Int(1)),
            ("y".to_string(), Expr::Int(2)),
        ];
        let expr = Expr::Seq(bindings, Box::new(Expr::Int(0)));
        let env = Environment::new();
        let result_env = extract_bindings(&expr, &env).unwrap();
        assert_eq!(result_env.lookup("x"), Some(&Value::Int(1)));
        assert_eq!(result_env.lookup("y"), Some(&Value::Int(2)));
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
        let expr = Expr::TupleProj(
            Box::new(Expr::Tuple(vec![Expr::Int(10), Expr::Int(20)])),
            0,
        );
        assert_eq!(eval(&expr, &env), Ok(Value::Int(10)));
    }

    #[test]
    fn test_eval_tuple_proj_second() {
        let env = Environment::new();
        let expr = Expr::TupleProj(
            Box::new(Expr::Tuple(vec![Expr::Int(10), Expr::Int(20)])),
            1,
        );
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
        let expr = Expr::TupleProj(
            Box::new(Expr::Tuple(vec![Expr::Int(10), Expr::Int(20)])),
            2,
        );
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
        let pattern = Pattern::Tuple(vec![Pattern::Var("x".to_string()), Pattern::Var("y".to_string())]);
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
            Pattern::Tuple(vec![Pattern::Var("a".to_string()), Pattern::Var("b".to_string())]),
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
                    Pattern::Tuple(vec![Pattern::Var("x".to_string()), Pattern::Var("y".to_string())]),
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

