//! Monomorphic base types for the Core language (Arc B, B2).
//!
//! These are the ground types the Core term AST is annotated with. They are
//! deliberately monomorphic: polymorphism and type inference are deferred to a
//! later arc, and this enum forms the fixed seam those arcs build on.
//!
//! ADR note: `String` has no surface counterpart yet (Arc F seam). It exists in
//! the Core model so that string literals have a well-defined base type and so
//! the elaborator introduced in Arc F has a stable target to lower into, but no
//! surface syntax produces it today.

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
    /// The string type (no surface counterpart yet; Arc F seam).
    String,
}

impl BaseType {
    /// Returns the lowercase spelling of this base type.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BaseType::Int => "int",
            BaseType::Bool => "bool",
            BaseType::Float => "float",
            BaseType::Unit => "unit",
            BaseType::String => "string",
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
            BaseType::String,
        ];
        for bt in all {
            assert_eq!(bt.as_str(), bt.to_string(), "mismatch for {bt:?}");
        }
    }

    #[test]
    fn spellings_are_lowercase() {
        let all = [
            BaseType::Int,
            BaseType::Bool,
            BaseType::Float,
            BaseType::Unit,
            BaseType::String,
        ];
        for bt in all {
            let s = bt.as_str();
            assert_eq!(s, s.to_lowercase(), "not lowercase for {bt:?}");
        }
    }
}