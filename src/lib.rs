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
pub mod parser;
pub mod eval;
pub mod dot;
pub mod types;
pub mod typechecker;
pub mod exhaustiveness;

// Re-export commonly used types and functions
pub use ast::{Expr, BinOp};
pub use parser::parse;
pub use eval::{eval, extract_bindings, Value, Environment, EvalError};
pub use types::{Type, TypeScheme, TypeVar, RowVar};
pub use typechecker::{typecheck, TypeError, TypeEnv};
pub use exhaustiveness::{check_exhaustiveness, ExhaustivenessResult};
