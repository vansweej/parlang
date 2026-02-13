/// Evaluator/Interpreter for the ParLang language
/// This module implements the runtime evaluation of ParLang expressions

use crate::ast::{BinOp, Expr};
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
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Closure(param, _, _) => write!(f, "<function {}>", param),
        }
    }
}

/// Environment for variable bindings
#[derive(Debug, Clone, PartialEq)]
pub struct Environment {
    bindings: HashMap<String, Value>,
}

impl Environment {
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

    pub fn extend(&self, name: String, value: Value) -> Self {
        let mut new_env = self.clone();
        new_env.bind(name, value);
        new_env
    }

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
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EvalError::UnboundVariable(name) => write!(f, "Unbound variable: {}", name),
            EvalError::TypeError(msg) => write!(f, "Type error: {}", msg),
            EvalError::DivisionByZero => write!(f, "Division by zero"),
            EvalError::LoadError(msg) => write!(f, "Load error: {}", msg),
        }
    }
}

impl std::error::Error for EvalError {}

/// Extract bindings from nested let expressions
/// This walks through the AST and extracts all top-level let bindings
fn extract_bindings(expr: &Expr, env: &Environment) -> Result<Environment, EvalError> {
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
                .map_err(|e| EvalError::LoadError(format!("Failed to read file '{}': {}", filepath, e)))?;
            let lib_expr = crate::parser::parse(&content)
                .map_err(|e| EvalError::LoadError(format!("Failed to parse file '{}': {}", filepath, e)))?;
            
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

/// Evaluate an expression in an environment
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
                _ => Err(EvalError::TypeError(
                    "Application requires a function".to_string(),
                )),
            }
        }
        
        Expr::Load(filepath, body) => {
            // Read the file contents
            let content = fs::read_to_string(Path::new(filepath))
                .map_err(|e| EvalError::LoadError(format!("Failed to read file '{}': {}", filepath, e)))?;
            
            // Parse the file contents
            let lib_expr = crate::parser::parse(&content)
                .map_err(|e| EvalError::LoadError(format!("Failed to parse file '{}': {}", filepath, e)))?;
            
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
            "Type error in binary operation: {:?}",
            op
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
        let env: Environment = Default::default();
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
        assert_eq!(format!("{}", closure), "<function x>");
    }

    // Test EvalError Display implementation
    #[test]
    fn test_eval_error_display_unbound_var() {
        let err = EvalError::UnboundVariable("x".to_string());
        assert_eq!(format!("{}", err), "Unbound variable: x");
    }

    #[test]
    fn test_eval_error_display_type_error() {
        let err = EvalError::TypeError("test error".to_string());
        assert_eq!(format!("{}", err), "Type error: test error");
    }

    #[test]
    fn test_eval_error_display_division_by_zero() {
        let err = EvalError::DivisionByZero;
        assert_eq!(format!("{}", err), "Division by zero");
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
        use std::io::Write;
        
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
        assert_eq!(format!("{}", err), "Load error: test load error");
    }
}
