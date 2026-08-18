#![recursion_limit = "512"]

/// `ParLang`: A small ML-alike functional language written in Rust
///
/// This library provides:
/// - AST definitions for the language
/// - Parser using the combine parser combinator library
/// - Evaluator/interpreter for executing programs
/// - Optional Hindley-Milner type inference system
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
///
/// # Type Checking Example
///
/// ```
/// use parlang::{parse, typecheck};
///
/// let program = "fun x -> x + 1";
/// let expr = parse(program).expect("Parse error");
/// let ty = typecheck(&expr).expect("Type error");
/// println!("Type: {}", ty); // prints "Type: Int -> Int"
/// ```
pub mod ast;
pub mod dot;
pub mod eval;
pub mod exhaustiveness;
pub mod parser;
pub mod typechecker;
pub mod types;

// Re-export commonly used types and functions
pub use ast::{BinOp, Expr};
pub use eval::{eval, extract_bindings, Environment, EvalError, Value};
pub use exhaustiveness::{check_exhaustiveness, ExhaustivenessResult};
pub use parser::parse;
pub use typechecker::{typecheck, TypeEnv, TypeError};
pub use types::{Type, TypeScheme, TypeVar};
