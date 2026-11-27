//! `parlang-core` — the `ParLang` compiler core (Arc B IR + Arc C VM).
//!
//! Defines the small-Core [`Term`] IR (Arc B): an explicitly typed,
//! System-F-style calculus with type abstraction and application
//! (`TyLam`/`TyApp`), checked by Core Lint (the `lint` module, see
//! [`LintEnv`] and [`LintError`]). Evaluation is strict and call-by-value and
//! runs on a CEK machine (the `machine` module, ADR 0008) that can also
//! record a step-by-step [`trace`]: see [`eval`], [`run`], [`Value`],
//! [`Environment`], and [`EvalError`]. Branching and primitives use reserved
//! constructor names dispatched by the machine (`if`, `+`, `-`, `*`, `/`, `<`,
//! `eq`); recursion uses [`Value::RecClosure`]. Strings are `List Char` under
//! ADR 0002's `Cons`/`Nil` encoding (see `docs/adr/0004-core-char-and-strings.md`).
//! The evaluation model is
//! specified in `docs/CORE_OPERATIONAL_SEMANTICS.md` and
//! `docs/adr/0001-arc-c-eval-model.md`. No other workspace crate depends on
//! this crate yet; the surface-to-Core elaborator lands in roadmap slice 7.

pub mod base_type;
pub mod term;
pub mod ty;

pub use base_type::BaseType;
pub use term::{Alt, Lit, Term};
pub use ty::Type;

pub mod display;
pub mod dot;

pub use dot::core_to_dot;

pub mod builder;
pub mod error;

pub use error::{BuildError, BuildResult};

pub mod eval;
pub use eval::{eval, Environment, EvalError, EvalResult, Value};
pub mod machine;
pub mod readback;
pub mod value_display;
pub use machine::{reduction_terms, run, trace, Machine, StepKind, StepOutcome, TraceStep};

pub mod prelude;
pub use prelude::{prelude, PreludeError};

pub mod lint;
pub use lint::{lint, prelude_env, DataCon, DataType, LintEnv, LintError};
pub mod eligibility;
pub use eligibility::{check_function, EligibilityError, Report, Rule, Violation};
pub mod render;
pub use render::render_value;
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
