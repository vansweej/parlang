//! `ParLang` Core prelude.
//!
//! This module builds a small standard environment of Core-level helper
//! functions (currently just `not`) on top of the reserved primitive
//! constructors implemented by the evaluator.

use crate::base_type::BaseType;
use crate::builder::{bool_, con, lam, var};
use crate::error::BuildError;
use crate::eval::{eval, Environment, EvalError};

/// An error that can occur while building the `ParLang` Core prelude.
#[derive(Debug, Clone, PartialEq)]
pub enum PreludeError {
    /// A term failed to build (see [`BuildError`]).
    Build(BuildError),
    /// A term failed to evaluate (see [`EvalError`]).
    Eval(EvalError),
}

impl std::fmt::Display for PreludeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Build(e) => write!(f, "prelude build error: {e}"),
            Self::Eval(e) => write!(f, "prelude eval error: {e}"),
        }
    }
}

impl std::error::Error for PreludeError {}

impl From<BuildError> for PreludeError {
    fn from(e: BuildError) -> Self {
        Self::Build(e)
    }
}

impl From<EvalError> for PreludeError {
    fn from(e: EvalError) -> Self {
        Self::Eval(e)
    }
}

/// Builds the `ParLang` Core prelude environment.
///
/// # Errors
///
/// Returns `PreludeError::Build` if a term fails to build and
/// `PreludeError::Eval` if a seed term fails to evaluate.
pub fn prelude() -> Result<Environment, PreludeError> {
    let not_term = lam(
        "b",
        BaseType::Bool,
        con("if", vec![var("b")?, bool_(false), bool_(true)])?,
    )?;

    let env = Environment::new();
    let not_value = eval(&not_term, &env)?;
    let env = env.extend("not", not_value);

    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::app;
    use crate::eval::Value;

    #[test]
    fn prelude_builds() -> Result<(), PreludeError> {
        let env = prelude()?;
        assert!(env.lookup("not").is_some());
        assert!(env.lookup("isEmpty").is_none());
        assert!(env.lookup("nonEmpty").is_none());
        Ok(())
    }

    #[test]
    fn not_true_is_false() -> Result<(), PreludeError> {
        let env = prelude()?;
        let t = app(var("not")?, bool_(true));
        assert_eq!(eval(&t, &env), Ok(Value::Bool(false)));
        Ok(())
    }

    #[test]
    fn from_build_error_converts() {
        assert_eq!(
            PreludeError::from(BuildError::EmptyName),
            PreludeError::Build(BuildError::EmptyName)
        );
    }
}
