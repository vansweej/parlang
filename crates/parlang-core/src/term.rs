//! Core term representation for the `ParLang` compiler core (Arc B).
//!
//! This module defines the small-Core term language: [`Term`] and [`Lit`].
//! It is a *desugared* intermediate representation, not the surface AST —
//! `let` and `letrec` are kept as distinct constructors (per SPJ 1987 ch.3-6,
//! which motivates an explicit `let`/`letrec` split in a small functional
//! core).
//!
//! # ADR: named binders vs. De Bruijn indices
//!
//! Arc B represents binders with symbolic names ([`Term::Var`], and the
//! binder-name fields of [`Term::Lam`]/[`Term::Let`]/[`Term::LetRec`]). This is
//! TAPL Section 6 ("Nameless Representation of Terms", p.97) *option (1)*:
//! symbolic names with explicit, capture-avoiding renaming performed on the
//! fly. We deliberately do NOT adopt option (2) — names under the Barendregt
//! convention — because TAPL notes that convention is not stable under
//! substitution: every substitution step must be followed by a renaming step
//! to restore the invariant.
//!
//! Why option (1) for Arc B: the only consumers of the Core AST in this arc
//! are the B4 builder (which constructs terms) and the B3 dumps (DOT and
//! text, which only read structure). Neither performs substitution, so named
//! binders carry no correctness cost here and keep the AST and its dumps
//! directly legible.
//!
//! ## Exit criteria
//!
//! This choice is revisitable and is inherited by Arc C. Arc B performs no
//! substitution, so the renaming cost is latent. Once Arc C's evaluator
//! performs beta-reduction / substitution it will incur exactly the
//! on-the-fly capture-avoiding renaming cost TAPL describes for option (1).
//! The canonical exit is TAPL Section 6 *option (3)*, a nameless (De Bruijn)
//! representation requiring no renaming, using the shifting and substitution
//! operations of TAPL Section 6.2 (p.80) and the beta-reduction-with-shift
//! rule of TAPL Section 6.3 (p.81). The `let`/`letrec` split follows SPJ 1987
//! ch.3-6 and is orthogonal to this binder decision.

use crate::base_type::BaseType;

/// A literal value in a Core term.
#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    /// A 64-bit signed integer literal.
    Int(i64),
    /// A boolean literal.
    Bool(bool),
    /// A 64-bit floating-point literal.
    Float(f64),
    /// The unit literal.
    Unit,
    /// A string literal (no surface counterpart yet; Arc F seam).
    Str(String),
}

impl Lit {
    /// Returns the base type of this literal.
    #[must_use]
    pub fn base_type(&self) -> BaseType {
        match self {
            Lit::Int(_) => BaseType::Int,
            Lit::Bool(_) => BaseType::Bool,
            Lit::Float(_) => BaseType::Float,
            Lit::Unit => BaseType::Unit,
            Lit::Str(_) => BaseType::String,
        }
    }
}

/// A Core term.
///
/// Binders are named (`String`); see the module documentation for the ADR
/// covering that choice.
#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// A variable reference by name.
    Var(String),
    /// A lambda abstraction: binder name, binder type, and body.
    Lam(String, BaseType, Box<Term>),
    /// An application of a function term to an argument term.
    App(Box<Term>, Box<Term>),
    /// A non-recursive `let`: binder name, bound value, and body.
    Let(String, Box<Term>, Box<Term>),
    /// A recursive `letrec`: binder name, bound value, and body.
    LetRec(String, Box<Term>, Box<Term>),
    /// A literal.
    Lit(Lit),
    /// A constructor application: constructor name and ordered arguments.
    Con(String, Vec<Term>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_base_type_maps_correctly() {
        assert_eq!(Lit::Int(0).base_type(), BaseType::Int);
        assert_eq!(Lit::Bool(true).base_type(), BaseType::Bool);
        assert_eq!(Lit::Float(1.5).base_type(), BaseType::Float);
        assert_eq!(Lit::Unit.base_type(), BaseType::Unit);
        assert_eq!(Lit::Str("hi".to_string()).base_type(), BaseType::String);
    }

    #[test]
    fn terms_are_structurally_comparable() {
        let a = Term::Lam(
            "x".to_string(),
            BaseType::Int,
            Box::new(Term::Var("x".to_string())),
        );
        let b = Term::Lam(
            "x".to_string(),
            BaseType::Int,
            Box::new(Term::Var("x".to_string())),
        );
        let c = Term::Var("y".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
