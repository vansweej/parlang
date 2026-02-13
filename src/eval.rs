/// Evaluator/Interpreter for the ParLang language
/// This module implements the runtime evaluation of ParLang expressions

use crate::ast::{BinOp, Expr};
use std::collections::HashMap;
use std::fmt;

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
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EvalError::UnboundVariable(name) => write!(f, "Unbound variable: {}", name),
            EvalError::TypeError(msg) => write!(f, "Type error: {}", msg),
            EvalError::DivisionByZero => write!(f, "Division by zero"),
        }
    }
}

impl std::error::Error for EvalError {}

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
}
