/// ParLang: A small ML-alike functional language written in Rust
/// 
/// This library provides:
/// - AST definitions for the language
/// - Parser using the combine parser combinator library
/// - Evaluator/interpreter for executing programs
/// 
/// # Example
/// 
/// ```
/// use parlang::{parse, eval, Environment};
/// 
/// let program = "let x = 42 in x + 1";
/// let expr = parse(program).expect("Parse error");
/// let env = Environment::new();
/// let result = eval(&expr, &env).expect("Evaluation error");
/// println!("Result: {}", result); // prints "Result: 43"
/// ```

pub mod ast;
pub mod parser;
pub mod eval;
pub mod dot;

// Re-export commonly used types and functions
pub use ast::{Expr, BinOp};
pub use parser::parse;
pub use eval::{eval, extract_bindings, Value, Environment, EvalError};
