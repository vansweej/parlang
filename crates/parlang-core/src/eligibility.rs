//! Kernel-eligibility predicate over elaborated Core (roadmap slice 8).
//!
//! [`check_function`] decides whether a top-level function of an elaborated
//! program may be compiled to a data-parallel kernel. Rule ids are stable and
//! never renumbered; see `docs/adr/0010-kernel-eligibility-predicate.md`.

use std::fmt::{self, Display, Formatter};

use crate::lint::LintEnv;
use crate::term::Term;
use crate::ty::Type;

/// The `fresh_name` prefix the match compiler uses for join points, without
/// the leading `$`: join points are named `$j0`, `$j1`, and so on.
pub const JOIN_POINT_TAG: &str = "j";

/// Returns `true` when `name` is a match-compiler join point: `$`, then
/// [`JOIN_POINT_TAG`], then one or more ASCII digits.
#[must_use]
pub fn is_join_point(name: &str) -> bool {
    name.strip_prefix('$')
        .and_then(|rest| rest.strip_prefix(JOIN_POINT_TAG))
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// A kernel-eligibility rule. Ids are stable and never renumbered.
///
/// `R6` is reserved for future SOAC operator arguments (`map`, `reduce`,
/// `scan`); it deliberately has no variant, so nothing can claim to check it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// `R1`: every checked type is a kernel type.
    KernelTypes,
    /// `R2`: first-order code with saturated direct calls only.
    FirstOrder,
    /// `R3`: only self tail recursion.
    Recursion,
    /// `R4`: only non-recursive data types with kernel-type fields.
    Data,
    /// `R5`: no polymorphism left uninstantiated.
    Polymorphism,
}

impl Rule {
    /// Returns the stable rule id, for example `"R1"`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Rule::KernelTypes => "R1",
            Rule::FirstOrder => "R2",
            Rule::Recursion => "R3",
            Rule::Data => "R4",
            Rule::Polymorphism => "R5",
        }
    }

    /// Returns the stable human-readable rule name, for example `"data"`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Rule::KernelTypes => "kernel types",
            Rule::FirstOrder => "first-order",
            Rule::Recursion => "recursion",
            Rule::Data => "data",
            Rule::Polymorphism => "polymorphism",
        }
    }
}

/// One rule violation: the rule, the function it occurs in, and the
/// `Term::to_text` rendering of the offending node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The violated rule.
    pub rule: Rule,
    /// The function (top-level or self-loop binder) containing the node.
    pub function: String,
    /// The offending node, rendered in full.
    pub node: String,
}

/// The outcome of an eligibility check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// The function and all its callees satisfy every rule.
    Eligible,
    /// Every violation found, in deterministic pre-order.
    Rejected(Vec<Violation>),
}

/// An error that prevents an eligibility check from running at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EligibilityError {
    /// No binding of the requested name exists on the root `Let` spine.
    UnknownFunction(String),
    /// The input Core does not have the shape the elaborator produces.
    Malformed(String),
}

impl Display for EligibilityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EligibilityError::UnknownFunction(name) => write!(f, "unknown function: {name}"),
            EligibilityError::Malformed(msg) => write!(f, "malformed Core: {msg}"),
        }
    }
}

impl std::error::Error for EligibilityError {}

/// Returns the value of the innermost binding of `name` on the root `Let`
/// spine of `root`, which is the binding the program body sees.
#[must_use]
pub fn find_binding<'a>(root: &'a Term, name: &str) -> Option<&'a Term> {
    let mut found = None;
    let mut cursor = root;
    while let Term::Let(binder, value, body) = cursor {
        if binder == name {
            found = Some(value.as_ref());
        }
        cursor = body.as_ref();
    }
    found
}

/// Checks whether top-level function `name` of `root` is kernel-eligible.
///
/// `instantiation` supplies type arguments for a generalised function. The
/// rules are not implemented yet: any function found is reported eligible.
///
/// # Errors
///
/// Returns [`EligibilityError::UnknownFunction`] when no binding of `name`
/// exists on the root `Let` spine.
pub fn check_function(
    root: &Term,
    env: &LintEnv,
    name: &str,
    instantiation: &[Type],
) -> Result<Report, EligibilityError> {
    let _ = (env, instantiation);
    find_binding(root, name)
        .map(|_| Report::Eligible)
        .ok_or_else(|| EligibilityError::UnknownFunction(name.to_string()))
}
