//! `parlang-core` — intentionally empty skeleton crate.
//!
//! This is a deliberate placeholder reserved for a future Arc B compiler core.
//! It currently exposes only [`version`]; the `parlang-driver` crate does NOT
//! depend on it yet. This is scaffolding, not dead weight.

pub mod base_type;
pub mod term;

pub use base_type::BaseType;
pub use term::{Lit, Term};

// core-modules: phase1
/// Returns the crate's semantic version string.
#[must_use]
pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::version;

    #[test]
    fn version_is_non_empty() {
        assert!(!version().is_empty());
    }
}
