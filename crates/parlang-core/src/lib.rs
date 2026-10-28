//! `parlang-core` — the `ParLang` compiler core (Arc B IR + Arc C VM).
//!
//! Defines the small-Core [`Term`] IR (Arc B) and a strict, call-by-value
//! tree-walking VM (Arc C) that evaluates it: see [`eval`], [`Value`],
//! [`Environment`], and [`EvalError`]. Branching and primitives use reserved
//! constructor names dispatched by the VM (`if`, `+`, `-`, `*`, `/`, `<`,
//! `eq`); recursion uses [`Value::RecClosure`]. Strings are `List Char` under
//! ADR 0002's `Cons`/`Nil` encoding (see `docs/adr/0004-core-char-and-strings.md`).
//! The evaluation model is
//! specified in `docs/CORE_OPERATIONAL_SEMANTICS.md` and
//! `docs/adr/0001-arc-c-eval-model.md`. The `parlang-driver` crate does NOT
//! depend on this crate yet.

pub mod base_type;
pub mod term;

pub use base_type::BaseType;
pub use term::{Alt, Lit, Term};

pub mod display;
pub mod dot;

pub use dot::core_to_dot;

pub mod builder;
pub mod error;

pub use error::{BuildError, BuildResult};

pub mod eval;
pub use eval::{eval, Environment, EvalError, EvalResult, Value};

pub mod prelude;
pub use prelude::{prelude, PreludeError};
// core-modules: phase5
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
