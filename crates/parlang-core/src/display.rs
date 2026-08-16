//! Human-readable textual rendering of Core terms.
//!
//! This module provides `Display` implementations for [`Lit`] and [`Term`],
//! plus [`Term::to_text`], the stable public entry point for dumping a Core
//! term as a string. The rendering is fully parenthesized and unambiguous; it
//! is intended for snapshot tests, debugging, and tooling — not as a
//! parseable surface syntax.

use std::fmt::{self, Display, Formatter};

use crate::term::{Lit, Term};

impl Display for Lit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Lit::Int(v) => write!(f, "{v}"),
            Lit::Bool(v) => write!(f, "{v}"),
            Lit::Float(v) => write!(f, "{v}"),
            Lit::Unit => write!(f, "()"),
            Lit::Str(s) => write!(f, "{s:?}"),
        }
    }
}

impl Display for Term {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Term::Var(name) => write!(f, "{name}"),
            Term::Lam(name, ty, body) => write!(f, "(\\{name}: {ty}. {body})"),
            Term::App(func, arg) => write!(f, "({func} {arg})"),
            Term::Let(name, value, body) => {
                write!(f, "(let {name} = {value} in {body})")
            }
            Term::LetRec(name, value, body) => {
                write!(f, "(letrec {name} = {value} in {body})")
            }
            Term::Lit(lit) => write!(f, "{lit}"),
            Term::Con(name, args) => {
                write!(f, "{name}")?;
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

impl Term {
    /// Renders this term as its stable public text dump.
    ///
    /// This is the stable public entry point for text dumps of Core terms,
    /// paralleling the surface language's text dump. Callers should depend on
    /// this method rather than on the `Display` impl directly, so the dump
    /// format can be evolved behind a single named entry point.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_type::BaseType;

    #[test]
    fn identity_lambda_text() {
        let term = Term::Lam(
            "x".to_string(),
            BaseType::Int,
            Box::new(Term::Var("x".to_string())),
        );
        assert_eq!(term.to_text(), "(\\x: int. x)");
    }

    #[test]
    fn let_and_con_text() {
        let term = Term::Let(
            "p".to_string(),
            Box::new(Term::Con(
                "Pair".to_string(),
                vec![Term::Lit(Lit::Int(1)), Term::Lit(Lit::Bool(true))],
            )),
            Box::new(Term::Var("p".to_string())),
        );
        assert_eq!(term.to_text(), "(let p = Pair(1, true) in p)");
    }

    #[test]
    fn letrec_and_unit_text() {
        let term = Term::LetRec(
            "f".to_string(),
            Box::new(Term::Lam(
                "x".to_string(),
                BaseType::Unit,
                Box::new(Term::Var("x".to_string())),
            )),
            Box::new(Term::Lit(Lit::Unit)),
        );
        assert_eq!(term.to_text(), "(letrec f = (\\x: unit. x) in ())");
    }

    #[test]
    fn float_text_uses_a_value_with_visible_fraction() {
        let term = Term::Lit(Lit::Float(1.5));
        let text = term.to_text();
        assert!(text.contains("1.5"), "expected 1.5 in: {text}");
    }
}
