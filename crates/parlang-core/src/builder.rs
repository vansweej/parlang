//! Tiny, dependency-free Core builder (smart constructors) for Arc B, B4.
//!
//! Rationale: a full `combine`-based textual Core parser would add a
//! dependency to a currently zero-dependency crate and is unnecessary — the
//! only Arc B client is hand-written test code. Smart constructors are
//! lighter, dependency-free, and sufficient. A textual parser can be added
//! later if a real need appears.
//!
//! Fallible constructors (empty names) return [`BuildResult`]; no
//! `unwrap`/`panic!` is used in library code.

use crate::base_type::BaseType;
use crate::error::{BuildError, BuildResult};
use crate::term::{Lit, Term};

/// A term variable.
///
/// # Errors
/// Returns [`BuildError::EmptyName`] if `name` is empty.
pub fn var(name: &str) -> BuildResult<Term> {
    if name.is_empty() {
        return Err(BuildError::EmptyName);
    }
    Ok(Term::Var(name.to_string()))
}

/// A typed lambda abstraction.
///
/// # Errors
/// Returns [`BuildError::EmptyName`] if `name` is empty.
pub fn lam(name: &str, ty: BaseType, body: Term) -> BuildResult<Term> {
    if name.is_empty() {
        return Err(BuildError::EmptyName);
    }
    Ok(Term::Lam(name.to_string(), ty, Box::new(body)))
}

/// A single application `func arg`.
#[must_use]
pub fn app(func: Term, arg: Term) -> Term {
    Term::App(Box::new(func), Box::new(arg))
}

/// Left-associative application of `func` to `args`, i.e. `((f a) b) c`.
#[must_use]
pub fn apps(func: Term, args: Vec<Term>) -> Term {
    args.into_iter().fold(func, app)
}

/// A non-recursive `let`.
///
/// # Errors
/// Returns [`BuildError::EmptyName`] if `name` is empty.
pub fn let_(name: &str, value: Term, body: Term) -> BuildResult<Term> {
    if name.is_empty() {
        return Err(BuildError::EmptyName);
    }
    Ok(Term::Let(name.to_string(), Box::new(value), Box::new(body)))
}

/// A recursive `letrec`.
///
/// # Errors
/// Returns [`BuildError::EmptyName`] if `name` is empty.
pub fn letrec(name: &str, value: Term, body: Term) -> BuildResult<Term> {
    if name.is_empty() {
        return Err(BuildError::EmptyName);
    }
    Ok(Term::LetRec(
        name.to_string(),
        Box::new(value),
        Box::new(body),
    ))
}

/// A constructor application.
///
/// # Errors
/// Returns [`BuildError::EmptyName`] if `name` is empty.
pub fn con(name: &str, args: Vec<Term>) -> BuildResult<Term> {
    if name.is_empty() {
        return Err(BuildError::EmptyName);
    }
    Ok(Term::Con(name.to_string(), args))
}

/// An integer literal.
#[must_use]
pub fn int(n: i64) -> Term {
    Term::Lit(Lit::Int(n))
}

/// A boolean literal.
#[must_use]
pub fn bool_(b: bool) -> Term {
    Term::Lit(Lit::Bool(b))
}

/// A float literal.
#[must_use]
pub fn float(x: f64) -> Term {
    Term::Lit(Lit::Float(x))
}

/// The unit literal.
#[must_use]
pub fn unit() -> Term {
    Term::Lit(Lit::Unit)
}

/// A string literal.
#[must_use]
pub fn str_(s: &str) -> Term {
    Term::Lit(Lit::Str(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_name_rejected() {
        assert_eq!(var(""), Err(BuildError::EmptyName));
        assert_eq!(lam("", BaseType::Int, unit()), Err(BuildError::EmptyName));
        assert_eq!(let_("", unit(), unit()), Err(BuildError::EmptyName));
        assert_eq!(letrec("", unit(), unit()), Err(BuildError::EmptyName));
        assert_eq!(con("", vec![]), Err(BuildError::EmptyName));
    }

    #[test]
    fn identity_builds() -> Result<(), BuildError> {
        let id = lam("x", BaseType::Int, var("x")?)?;
        assert_eq!(
            id,
            Term::Lam(
                "x".to_string(),
                BaseType::Int,
                Box::new(Term::Var("x".to_string()))
            )
        );
        Ok(())
    }

    #[test]
    fn apps_is_left_associative() -> Result<(), BuildError> {
        let built = apps(var("f")?, vec![int(1), int(2)]);
        let expected = app(app(var("f")?, int(1)), int(2));
        assert_eq!(built, expected);
        Ok(())
    }

    #[test]
    fn con_holds_args() -> Result<(), BuildError> {
        let built = con("Pair", vec![int(1), unit()])?;
        assert_eq!(
            built,
            Term::Con(
                "Pair".to_string(),
                vec![Term::Lit(Lit::Int(1)), Term::Lit(Lit::Unit)]
            )
        );
        Ok(())
    }
}
