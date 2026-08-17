//! `ParLang` Core prelude.
//!
//! This module builds a small standard environment of Core-level helper
//! functions (e.g. `not`, `isEmpty`, `nonEmpty`) on top of the reserved
//! primitive constructors implemented by the evaluator.

use crate::base_type::BaseType;
use crate::builder::{app, bool_, con, int, lam, var};
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
    let is_empty_term = lam(
        "s",
        BaseType::String,
        con("eq", vec![con("strlen", vec![var("s")?])?, int(0)])?,
    )?;
    let non_empty_term = lam(
        "s",
        BaseType::String,
        app(var("not")?, app(var("isEmpty")?, var("s")?)),
    )?;

    let env = Environment::new();
    let not_value = eval(&not_term, &env)?;
    let env = env.extend("not", not_value);
    let is_empty_value = eval(&is_empty_term, &env)?;
    let env = env.extend("isEmpty", is_empty_value);
    let non_empty_value = eval(&non_empty_term, &env)?;
    let env = env.extend("nonEmpty", non_empty_value);

    Ok(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Value;

    #[test]
    fn prelude_builds() -> Result<(), PreludeError> {
        let env = prelude()?;
        assert!(env.lookup("not").is_some());
        assert!(env.lookup("isEmpty").is_some());
        assert!(env.lookup("nonEmpty").is_some());
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
