//! Error type for Core construction (Arc B, B4).
//!
//! A dedicated error enum keeps library code free of `unwrap`/`panic!`.

use std::fmt::{self, Display, Formatter};

/// An error raised while building a Core term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// A binder or constructor name was empty.
    EmptyName,
}

impl Display for BuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::EmptyName => write!(f, "name must not be empty"),
        }
    }
}

impl std::error::Error for BuildError {}

/// Convenient `Result` alias for Core builder operations.
pub type BuildResult<T> = Result<T, BuildError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_message_is_stable() {
        let err = BuildError::EmptyName;
        assert_eq!(err.to_string(), "name must not be empty");
    }
}
