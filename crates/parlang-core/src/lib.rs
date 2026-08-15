//! `parlang-core` — intentionally empty skeleton crate.
//!
//! This is a deliberate placeholder reserved for a future Arc B compiler core.
//! It currently exposes only [`version`]; the `parlang-driver` crate does NOT
//! depend on it yet. This is scaffolding, not dead weight.

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
