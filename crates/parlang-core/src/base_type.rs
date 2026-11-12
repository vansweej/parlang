//! Monomorphic base types for the Core language (Arc B, B2).
//!
//! These are the ground types the Core term AST is annotated with. They are
//! deliberately monomorphic: polymorphism and type inference are deferred to a
//! later arc, and this enum forms the fixed seam those arcs build on.
//!
//! Core carries a `Char` base type — a Unicode scalar value (a code point),
//! like Haskell's `Char`. Strings in Core are `List Char` under ADR 0002's
//! `Cons`/`Nil` encoding, not a primitive string value; see
//! `docs/adr/0004-core-char-and-strings.md`.

use std::fmt::{self, Display, Formatter};

/// A monomorphic base type in the Core language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseType {
    /// 64-bit signed integer type.
    Int,
    /// Boolean type.
    Bool,
    /// 64-bit floating-point type.
    Float,
    /// The unit type.
    Unit,
    /// A Unicode scalar value (code point), matching Haskell's `Char`.
    Char,
}

impl BaseType {
    /// Returns the capitalized spelling of this base type (GHC/Haskell style).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BaseType::Int => "Int",
            BaseType::Bool => "Bool",
            BaseType::Float => "Float",
            BaseType::Unit => "Unit",
            BaseType::Char => "Char",
        }
    }
}

impl Display for BaseType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = self.as_str();
        write!(f, "{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_matches_display() {
        let all = [
            BaseType::Int,
            BaseType::Bool,
            BaseType::Float,
            BaseType::Unit,
            BaseType::Char,
        ];
        for bt in all {
            assert_eq!(bt.as_str(), bt.to_string(), "mismatch for {bt:?}");
        }
    }

    #[test]
    fn spellings_are_capitalized() {
        let cases = [
            (BaseType::Int, "Int"),
            (BaseType::Bool, "Bool"),
            (BaseType::Float, "Float"),
            (BaseType::Unit, "Unit"),
            (BaseType::Char, "Char"),
        ];
        for (bt, want) in cases {
            assert_eq!(bt.as_str(), want, "wrong spelling for {bt:?}");
        }
    }
}
