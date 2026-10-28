//! System-F-shaped Core types (roadmap slice 6b).
//!
//! The Core `Type` enum is the type-level counterpart to the `Term` IR. It is
//! deliberately System-F-shaped so that dictionary-passing and higher-kinded
//! types stay reachable without a later restructure (see
//! `docs/adr/0005-core-type-enum.md`).
//!
//! Shape:
//! - `Base` absorbs the monomorphic `BaseType` as a leaf, so literals, VM
//!   primitives and the lowercase golden rendering stay keyed on `BaseType`.
//! - `Var` is a NAMED type variable, matching `Term`'s named binders for
//!   pedagogical legibility.
//! - `Fun` is a dedicated right-associative arrow.
//! - `App` is binary type-constructor application (GHC `AppTy`-style), which
//!   admits variable-headed application `f a` so higher-kinded types need no
//!   restructure; `Con` is a nullary type-constructor reference.
//! - `Forall` is a single-binder System-F universal. Binders are implicitly
//!   kind `*`; a kind slot is deferred to milestone 2.
//!
//! No kinds and no term-level changes are introduced in this slice.

use std::fmt::{self, Display, Formatter};

use crate::base_type::BaseType;

/// A System-F-shaped Core type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// A monomorphic base type leaf (`int`, `bool`, `float`, `unit`, `char`).
    Base(BaseType),
    /// A named type variable, for example `a`.
    Var(String),
    /// A nullary type-constructor reference, for example `List`.
    Con(String),
    /// A function type `domain -> codomain`.
    Fun(Box<Type>, Box<Type>),
    /// Binary type-constructor application `head arg`.
    App(Box<Type>, Box<Type>),
    /// A single-binder universal `forall a. body`.
    Forall(String, Box<Type>),
}

impl From<BaseType> for Type {
    fn from(bt: BaseType) -> Self {
        Type::Base(bt)
    }
}

impl Type {
    /// Renders a sub-type as the left operand of an application,
    /// parenthesising it when it is not an atom or an application.
    fn fmt_app_head(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Type::Base(_) | Type::Var(_) | Type::Con(_) | Type::App(..) => write!(f, "{self}"),
            Type::Fun(..) | Type::Forall(..) => write!(f, "({self})"),
        }
    }

    /// Renders a sub-type as an application argument: nested applications,
    /// arrows and foralls are parenthesised so juxtaposition stays unambiguous.
    fn fmt_app_arg(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Type::Base(_) | Type::Var(_) | Type::Con(_) => write!(f, "{self}"),
            Type::App(..) | Type::Fun(..) | Type::Forall(..) => write!(f, "({self})"),
        }
    }

    /// Renders a sub-type in arrow-domain position, parenthesising arrows (for
    /// right-associativity) and foralls.
    fn fmt_arrow_dom(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Type::Fun(..) | Type::Forall(..) => write!(f, "({self})"),
            Type::Base(_) | Type::Var(_) | Type::Con(_) | Type::App(..) => write!(f, "{self}"),
        }
    }

    /// Returns the free type variables of this type in deterministic
    /// (lexicographic) order. A `Forall` binder shadows its binder name inside
    /// its body. `Base` and `Con` leaves contribute no variables.
    #[must_use]
    pub fn free_vars(&self) -> std::collections::BTreeSet<String> {
        let mut acc = std::collections::BTreeSet::new();
        self.collect_free_vars(&mut acc);
        acc
    }

    fn collect_free_vars(&self, acc: &mut std::collections::BTreeSet<String>) {
        match self {
            Type::Base(_) | Type::Con(_) => {}
            Type::Var(name) => {
                acc.insert(name.clone());
            }
            Type::Fun(a, b) | Type::App(a, b) => {
                a.collect_free_vars(acc);
                b.collect_free_vars(acc);
            }
            Type::Forall(binder, body) => {
                let mut inner = std::collections::BTreeSet::new();
                body.collect_free_vars(&mut inner);
                inner.remove(binder);
                acc.extend(inner);
            }
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Type::Base(bt) => write!(f, "{bt}"),
            Type::Var(name) | Type::Con(name) => write!(f, "{name}"),
            Type::Fun(dom, cod) => {
                dom.fmt_arrow_dom(f)?;
                write!(f, " -> {cod}")
            }
            Type::App(head, arg) => {
                head.fmt_app_head(f)?;
                write!(f, " ")?;
                arg.fmt_app_arg(f)
            }
            Type::Forall(binder, body) => write!(f, "forall {binder}. {body}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_renders_lowercase() {
        assert_eq!(Type::Base(BaseType::Int).to_string(), "int");
        assert_eq!(Type::from(BaseType::Unit).to_string(), "unit");
    }

    #[test]
    fn arrow_is_right_associative() {
        // a -> b -> c  ==  a -> (b -> c), no parens on the codomain.
        let ty = Type::Fun(
            Box::new(Type::Var("a".to_string())),
            Box::new(Type::Fun(
                Box::new(Type::Var("b".to_string())),
                Box::new(Type::Var("c".to_string())),
            )),
        );
        assert_eq!(ty.to_string(), "a -> b -> c");
    }

    #[test]
    fn arrow_domain_is_parenthesised() {
        // (a -> b) -> c
        let ty = Type::Fun(
            Box::new(Type::Fun(
                Box::new(Type::Var("a".to_string())),
                Box::new(Type::Var("b".to_string())),
            )),
            Box::new(Type::Var("c".to_string())),
        );
        assert_eq!(ty.to_string(), "(a -> b) -> c");
    }

    #[test]
    fn application_is_juxtaposition_and_left_associative() {
        // List a
        let list_a = Type::App(
            Box::new(Type::Con("List".to_string())),
            Box::new(Type::Var("a".to_string())),
        );
        assert_eq!(list_a.to_string(), "List a");
        // Either a b  ==  (Either a) b, no parens on a left application head.
        let either_a_b = Type::App(
            Box::new(Type::App(
                Box::new(Type::Con("Either".to_string())),
                Box::new(Type::Var("a".to_string())),
            )),
            Box::new(Type::Var("b".to_string())),
        );
        assert_eq!(either_a_b.to_string(), "Either a b");
    }

    #[test]
    fn nested_application_argument_is_parenthesised() {
        // List (Option a)  -- not the surface's lossy "List Option a".
        let ty = Type::App(
            Box::new(Type::Con("List".to_string())),
            Box::new(Type::App(
                Box::new(Type::Con("Option".to_string())),
                Box::new(Type::Var("a".to_string())),
            )),
        );
        assert_eq!(ty.to_string(), "List (Option a)");
    }

    #[test]
    fn variable_headed_application_renders() {
        // f a  -- HKT-readiness: variable-headed application is representable.
        let ty = Type::App(
            Box::new(Type::Var("f".to_string())),
            Box::new(Type::Var("a".to_string())),
        );
        assert_eq!(ty.to_string(), "f a");
    }

    #[test]
    fn forall_body_extends_right() {
        // forall a. a -> List a
        let ty = Type::Forall(
            "a".to_string(),
            Box::new(Type::Fun(
                Box::new(Type::Var("a".to_string())),
                Box::new(Type::App(
                    Box::new(Type::Con("List".to_string())),
                    Box::new(Type::Var("a".to_string())),
                )),
            )),
        );
        assert_eq!(ty.to_string(), "forall a. a -> List a");
    }

    #[test]
    fn forall_as_arrow_domain_is_parenthesised() {
        // (forall a. a) -> int
        let ty = Type::Fun(
            Box::new(Type::Forall(
                "a".to_string(),
                Box::new(Type::Var("a".to_string())),
            )),
            Box::new(Type::Base(BaseType::Int)),
        );
        assert_eq!(ty.to_string(), "(forall a. a) -> int");
    }

    #[test]
    fn free_vars_are_sorted_and_deduplicated() {
        // b -> a -> b  =>  {a, b}
        let ty = Type::Fun(
            Box::new(Type::Var("b".to_string())),
            Box::new(Type::Fun(
                Box::new(Type::Var("a".to_string())),
                Box::new(Type::Var("b".to_string())),
            )),
        );
        let fv: Vec<String> = ty.free_vars().into_iter().collect();
        assert_eq!(fv, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn forall_binder_is_not_free() {
        // forall a. a -> b  =>  {b}
        let ty = Type::Forall(
            "a".to_string(),
            Box::new(Type::Fun(
                Box::new(Type::Var("a".to_string())),
                Box::new(Type::Var("b".to_string())),
            )),
        );
        let fv: Vec<String> = ty.free_vars().into_iter().collect();
        assert_eq!(fv, vec!["b".to_string()]);
    }

    #[test]
    fn base_and_con_have_no_free_vars() {
        assert!(Type::Base(BaseType::Int).free_vars().is_empty());
        assert!(Type::Con("List".to_string()).free_vars().is_empty());
    }
}
