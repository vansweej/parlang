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
            Lit::Char(c) => write!(f, "{c:?}"),
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
            Term::LetRec(name, ann, value, body) => {
                write!(f, "(letrec {name}: {ann} = {value} in {body})")
            }
            Term::Lit(lit) => write!(f, "{lit}"),
            Term::Con(name, ty_args, args) => {
                write!(f, "{name}")?;
                for ty in ty_args {
                    write!(f, " @{ty}")?;
                }
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
            Term::TyLam(binder, body) => write!(f, "(/\\{binder}. {body})"),
            Term::TyApp(term, ty) => write!(f, "({term} @{ty})"),
            Term::Case(scrutinee, alts) => {
                write!(f, "(case {scrutinee} of ")?;
                for (i, alt) in alts.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", alt.con)?;
                    if !alt.binders.is_empty() {
                        write!(f, "({})", alt.binders.join(", "))?;
                    }
                    write!(f, " -> {}", alt.body)?;
                }
                write!(f, ")")
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
    use crate::term::Alt;

    #[test]
    fn identity_lambda_text() {
        let term = Term::Lam(
            "x".to_string(),
            crate::ty::Type::Base(BaseType::Int),
            Box::new(Term::Var("x".to_string())),
        );
        assert_eq!(term.to_text(), "(\\x: Int. x)");
    }

    #[test]
    fn let_and_con_text() {
        let term = Term::Let(
            "p".to_string(),
            Box::new(Term::Con(
                "Pair".to_string(),
                vec![],
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
            crate::ty::Type::Fun(
                Box::new(crate::ty::Type::Base(BaseType::Unit)),
                Box::new(crate::ty::Type::Base(BaseType::Unit)),
            ),
            Box::new(Term::Lam(
                "x".to_string(),
                crate::ty::Type::Base(BaseType::Unit),
                Box::new(Term::Var("x".to_string())),
            )),
            Box::new(Term::Lit(Lit::Unit)),
        );
        assert_eq!(
            term.to_text(),
            "(letrec f: Unit -> Unit = (\\x: Unit. x) in ())"
        );
    }

    #[test]
    fn float_text_uses_a_value_with_visible_fraction() {
        let term = Term::Lit(Lit::Float(1.5));
        let text = term.to_text();
        assert!(text.contains("1.5"), "expected 1.5 in: {text}");
    }

    #[test]
    fn case_text_renders_alts() {
        let term = Term::Case(
            Box::new(Term::Var("s".to_string())),
            vec![
                Alt {
                    con: "Cons".to_string(),
                    binders: vec!["x".to_string(), "xs".to_string()],
                    body: Term::Var("x".to_string()),
                },
                Alt {
                    con: "Nil".to_string(),
                    binders: vec![],
                    body: Term::Lit(Lit::Int(0)),
                },
            ],
        );
        assert_eq!(term.to_text(), "(case s of Cons(x, xs) -> x | Nil -> 0)");
    }

    #[test]
    fn tylam_tyapp_render_ascii() {
        let tl = Term::TyLam("a".to_string(), Box::new(Term::Var("x".to_string())));
        assert_eq!(tl.to_text(), "(/\\a. x)");
        let ta = Term::TyApp(
            Box::new(Term::Var("f".to_string())),
            crate::ty::Type::Base(BaseType::Int),
        );
        assert_eq!(ta.to_text(), "(f @Int)");
    }

    #[test]
    fn typed_con_renders_at_type_args() {
        let term = Term::Con(
            "Just".to_string(),
            vec![crate::ty::Type::Base(BaseType::Int)],
            vec![Term::Lit(Lit::Int(1))],
        );
        assert_eq!(term.to_text(), "Just @Int(1)");
    }

    #[test]
    fn empty_ty_arg_con_is_byte_identical() {
        let term = Term::Con(
            "Pair".to_string(),
            vec![],
            vec![Term::Lit(Lit::Int(1)), Term::Lit(Lit::Bool(true))],
        );
        assert_eq!(term.to_text(), "Pair(1, true)");
    }
}
