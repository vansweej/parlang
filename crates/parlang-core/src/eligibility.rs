//! Kernel-eligibility predicate over elaborated Core (roadmap slice 8).
//!
//! [`check_function`] decides whether a top-level function of an elaborated
//! program may be compiled to a data-parallel kernel. Rule ids are stable and
//! never renumbered; see `docs/adr/0010-kernel-eligibility-predicate.md`.

use std::collections::{BTreeMap, BTreeSet};
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

/// The leading structure of a spine-bound function value.
struct FunctionShape<'a> {
    /// Number of leading `TyLam` binders.
    tylams: usize,
    /// Leading parameter binder types with their `Lam` nodes, outermost first.
    params: Vec<(&'a Type, &'a Term)>,
}

/// Peels leading `TyLam`s, unwraps a self-loop `LetRec(g, _, inner, Var g)`,
/// and collects the leading `Lam` parameters of `value`.
fn split_function(value: &Term) -> FunctionShape<'_> {
    let mut tylams: usize = 0;
    let mut cursor = value;
    while let Term::TyLam(_, body) = cursor {
        tylams = tylams.saturating_add(1);
        cursor = body.as_ref();
    }
    if let Term::LetRec(binder, _, inner, body) = cursor {
        if matches!(body.as_ref(), Term::Var(name) if name == binder) {
            cursor = inner.as_ref();
        }
    }
    let mut params = Vec::new();
    while let Term::Lam(_, ty, body) = cursor {
        params.push((ty, cursor));
        cursor = body.as_ref();
    }
    FunctionShape { tylams, params }
}

/// Splits a type application spine into its head and arguments, in order.
fn type_spine(ty: &Type) -> (&Type, Vec<&Type>) {
    let mut args = Vec::new();
    let mut head = ty;
    while let Type::App(func, arg) = head {
        args.push(arg.as_ref());
        head = func.as_ref();
    }
    args.reverse();
    (head, args)
}

/// Returns `true` when `ty` is a kernel type: a base type, or a kernel data
/// type applied to kernel types. `Var`, `Fun` and `Forall` are never kernel.
fn is_kernel_type(ty: &Type, env: &LintEnv) -> bool {
    match type_spine(ty) {
        (Type::Base(_), args) => args.is_empty(),
        (Type::Con(name), args) => {
            is_kernel_data(name, env) && args.iter().all(|arg| is_kernel_type(arg, env))
        }
        _ => false,
    }
}

/// Adds every type-constructor name inside `ty` to `heads`; returns `false`
/// when `ty` contains a `Fun` or `Forall`.
fn collect_con_heads<'a>(ty: &'a Type, heads: &mut BTreeSet<&'a str>) -> bool {
    match ty {
        Type::Base(_) | Type::Var(_) => true,
        Type::Con(name) => {
            heads.insert(name.as_str());
            true
        }
        Type::App(func, arg) => collect_con_heads(func, heads) && collect_con_heads(arg, heads),
        Type::Fun(..) | Type::Forall(..) => false,
    }
}

/// Returns `true` when the data type `name` is registered, no field reachable
/// from it holds a `Fun` or `Forall`, and its reachable type graph is acyclic.
fn is_kernel_data<'a>(name: &'a str, env: &'a LintEnv) -> bool {
    let mut edges: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut pending = vec![name];
    while let Some(current) = pending.pop() {
        if edges.contains_key(current) {
            continue;
        }
        let Some(data) = env.data_type(current) else {
            return false;
        };
        let mut heads = BTreeSet::new();
        for field in data.ctors.iter().flat_map(|ctor| ctor.fields.iter()) {
            if !collect_con_heads(field, &mut heads) {
                return false;
            }
        }
        pending.extend(heads.iter().copied());
        edges.insert(current, heads);
    }
    is_acyclic(&edges)
}

/// Kahn's algorithm: returns `true` when the graph `edges` has no cycle.
fn is_acyclic(edges: &BTreeMap<&str, BTreeSet<&str>>) -> bool {
    let mut incoming: BTreeMap<&str, usize> = edges.keys().map(|key| (*key, 0)).collect();
    for target in edges.values().flatten() {
        if let Some(count) = incoming.get_mut(target) {
            *count = count.saturating_add(1);
        }
    }
    let mut ready: Vec<&str> = incoming
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(key, _)| *key)
        .collect();
    let mut removed: usize = 0;
    while let Some(node) = ready.pop() {
        removed = removed.saturating_add(1);
        for target in edges.get(node).into_iter().flatten() {
            if let Some(count) = incoming.get_mut(target) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    ready.push(*target);
                }
            }
        }
    }
    removed == edges.len()
}

/// Builds a violation of `rule` in `function` at `node`.
fn violation(rule: Rule, function: &str, node: &Term) -> Violation {
    Violation {
        rule,
        function: function.to_string(),
        node: node.to_text(),
    }
}

/// Checks whether top-level function `name` of `root` is kernel-eligible.
///
/// `instantiation` supplies type arguments for a generalised function; a
/// `TyLam` count that differs from its length is an `R5` violation. Each
/// leading parameter whose type is not a kernel type is one `R1` violation.
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
    let value = find_binding(root, name)
        .ok_or_else(|| EligibilityError::UnknownFunction(name.to_string()))?;
    let shape = split_function(value);
    let mut violations = Vec::new();
    if shape.tylams != instantiation.len() {
        violations.push(violation(Rule::Polymorphism, name, value));
    }
    for &(ty, node) in &shape.params {
        if !is_kernel_type(ty, env) {
            violations.push(violation(Rule::KernelTypes, name, node));
        }
    }
    if violations.is_empty() {
        Ok(Report::Eligible)
    } else {
        Ok(Report::Rejected(violations))
    }
}
