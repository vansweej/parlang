//! Core kinds (milestone 2, slice M2.1a).
//!
//! A [`Kind`] classifies a Core [`crate::ty::Type`]: `*` is the kind of
//! types that have values, and `k1 -> k2` is the kind of a type function
//! such as `List : * -> *`. Core is explicitly typed, so kinds are always
//! given and never inferred (see `docs/adr/0011-core-kinds.md`).

use std::fmt::{self, Display, Formatter};

/// The kind of a Core type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// The kind `*` of value-carrying types such as `Int` or `List Int`.
    Star,
    /// The kind `dom -> cod` of a type function such as `List`.
    Arrow(Box<Kind>, Box<Kind>),
}

impl Kind {
    /// Builds the arrow kind `dom -> cod`.
    #[must_use]
    pub fn arrow(dom: Kind, cod: Kind) -> Kind {
        Kind::Arrow(Box::new(dom), Box::new(cod))
    }

    /// Returns `true` when this kind is `*`.
    #[must_use]
    pub fn is_star(&self) -> bool {
        matches!(self, Kind::Star)
    }

    /// Builds the kind of a type constructor whose parameters have the
    /// given kinds, for example `* -> * -> *` for two `*` parameters.
    #[must_use]
    pub fn of_constructor(params: &[Kind]) -> Kind {
        params
            .iter()
            .rev()
            .fold(Kind::Star, |acc, param| Kind::arrow(param.clone(), acc))
    }
}

impl Display for Kind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Star => write!(f, "*"),
            Kind::Arrow(dom, cod) => {
                if matches!(dom.as_ref(), Kind::Arrow(..)) {
                    write!(f, "({dom}) -> {cod}")
                } else {
                    write!(f, "{dom} -> {cod}")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_renders_as_asterisk() {
        assert_eq!(Kind::Star.to_string(), "*");
        assert!(Kind::Star.is_star());
    }

    #[test]
    fn arrow_is_right_associative() {
        let kind = Kind::arrow(Kind::Star, Kind::arrow(Kind::Star, Kind::Star));
        assert_eq!(kind.to_string(), "* -> * -> *");
        assert!(!kind.is_star());
    }

    #[test]
    fn arrow_domain_is_parenthesised() {
        let kind = Kind::arrow(Kind::arrow(Kind::Star, Kind::Star), Kind::Star);
        assert_eq!(kind.to_string(), "(* -> *) -> *");
    }

    #[test]
    fn of_constructor_folds_params_right() {
        assert_eq!(Kind::of_constructor(&[]), Kind::Star);
        let hkt = Kind::of_constructor(&[Kind::arrow(Kind::Star, Kind::Star), Kind::Star]);
        assert_eq!(hkt.to_string(), "(* -> *) -> * -> *");
    }
}
