//! Kernel-eligibility predicate over elaborated Core (roadmap slice 8).
//!
//! [`check_function`] decides whether a top-level function of an elaborated
//! program may be compiled to a data-parallel kernel. Rule ids are stable and
//! never renumbered; see `docs/adr/0010-kernel-eligibility-predicate.md`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::{self, Display, Formatter};

use crate::lint::{LintEnv, RESERVED_PRIMS};
use crate::term::{Alt, Term};
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
    /// The self-loop `LetRec` binder, when the value is a self loop.
    self_binder: Option<&'a str>,
    /// Leading parameters as `(name, type, Lam node)`, outermost first.
    params: Vec<(&'a str, &'a Type, &'a Term)>,
    /// The term under the leading parameters.
    body: &'a Term,
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
    let mut self_binder = None;
    if let Term::LetRec(binder, _, inner, body) = cursor {
        if matches!(body.as_ref(), Term::Var(name) if name == binder) {
            self_binder = Some(binder.as_str());
            cursor = inner.as_ref();
        }
    }
    let mut params = Vec::new();
    while let Term::Lam(param, ty, body) = cursor {
        params.push((param.as_str(), ty, cursor));
        cursor = body.as_ref();
    }
    FunctionShape {
        tylams,
        self_binder,
        params,
        body: cursor,
    }
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

/// What a variable name refers to inside a checked function body.
#[derive(Clone, Copy)]
enum Head {
    /// A parameter, `Case` binder or non-join `Let` binder.
    Local,
    /// A join point: its arity and whether its `Let` is in tail position.
    Join(usize, bool),
    /// The self-loop binder with its arity.
    SelfCall(usize),
    /// An earlier spine binding with its arity (`0` for constants).
    Callee(usize),
}

/// Pre-order rule walker over one function body. `Let` binders, join
/// points and `Case` binders are lexically scoped: each is visible only in
/// its body and the previous scope is restored afterwards.
struct Walker<'a> {
    env: &'a LintEnv,
    function: &'a str,
    self_call: Option<(&'a str, usize)>,
    joins: BTreeMap<&'a str, (usize, bool)>,
    locals: BTreeSet<&'a str>,
    callees: BTreeMap<&'a str, usize>,
    used: Vec<&'a str>,
    violations: Vec<Violation>,
}

impl<'a> Walker<'a> {
    /// Creates a walker for `function`; later `earlier` bindings win.
    fn new(env: &'a LintEnv, function: &'a str, earlier: &[(&'a str, &'a Term)]) -> Self {
        let mut callees = BTreeMap::new();
        for &(binder, value) in earlier {
            callees.insert(binder, split_function(value).params.len());
        }
        Walker {
            env,
            function,
            self_call: None,
            joins: BTreeMap::new(),
            locals: BTreeSet::new(),
            callees,
            used: Vec::new(),
            violations: Vec::new(),
        }
    }

    fn report(&mut self, rule: Rule, node: &Term) {
        self.violations.push(violation(rule, self.function, node));
    }

    /// Binds leading parameters as locals; one `R1` per non-kernel binder.
    fn bind_params(&mut self, shape: &FunctionShape<'a>) {
        for &(param, ty, node) in &shape.params {
            if !is_kernel_type(ty, self.env) {
                self.report(Rule::KernelTypes, node);
            }
            self.locals.insert(param);
        }
    }

    /// Resolves a callee arity, recording first references in order.
    fn resolve_callee(&mut self, name: &'a str) -> Option<usize> {
        let arity = self.callees.get(name).copied()?;
        if !self.used.contains(&name) {
            self.used.push(name);
        }
        Some(arity)
    }

    fn classify(&mut self, name: &'a str) -> Result<Head, EligibilityError> {
        if self.locals.contains(name) {
            return Ok(Head::Local);
        }
        if let Some(&(arity, join_tail)) = self.joins.get(name) {
            return Ok(Head::Join(arity, join_tail));
        }
        if let Some((_, arity)) = self.self_call.filter(|&(binder, _)| binder == name) {
            return Ok(Head::SelfCall(arity));
        }
        self.resolve_callee(name)
            .map(Head::Callee)
            .ok_or_else(|| EligibilityError::Malformed(format!("unbound variable {name}")))
    }

    /// Walks `term`; `tail` is whether it is in tail position of the function.
    /// A non-leading `Lam`, `LetRec` or `TyLam` is reported without descending
    /// into it, so one offending node yields one violation, not a cascade.
    fn walk(&mut self, term: &'a Term, tail: bool) -> Result<(), EligibilityError> {
        match term {
            Term::Var(name) => self.walk_var(term, name),
            Term::App(..) => self.walk_app(term, tail),
            Term::Let(name, value, body) => self.walk_let(name, value, body, tail),
            Term::Con(name, ty_args, args) => self.walk_con(term, name, ty_args, args, tail),
            Term::Case(scrutinee, alts) => self.walk_case(term, scrutinee, alts, tail),
            Term::TyApp(inner, ty) => {
                if !is_kernel_type(ty, self.env) {
                    self.report(Rule::KernelTypes, term);
                }
                self.walk(inner, tail)
            }
            Term::Lam(..) => {
                self.report(Rule::FirstOrder, term);
                Ok(())
            }
            Term::LetRec(..) => {
                self.report(Rule::Recursion, term);
                Ok(())
            }
            Term::TyLam(..) => {
                self.report(Rule::Polymorphism, term);
                Ok(())
            }
            // Every base type, including Float, Char and Unit, is a kernel type.
            Term::Lit(_) => Ok(()),
        }
    }

    fn walk_var(&mut self, term: &Term, name: &'a str) -> Result<(), EligibilityError> {
        match self.classify(name)? {
            Head::Local | Head::Callee(0) => {}
            Head::Join(..) | Head::SelfCall(_) | Head::Callee(_) => {
                self.report(Rule::FirstOrder, term);
            }
        }
        Ok(())
    }

    fn walk_app(&mut self, term: &'a Term, tail: bool) -> Result<(), EligibilityError> {
        let mut args = Vec::new();
        let mut head = term;
        while let Term::App(func, arg) = head {
            args.push(arg.as_ref());
            head = func.as_ref();
        }
        args.reverse();
        while let Term::TyApp(inner, ty) = head {
            if !is_kernel_type(ty, self.env) {
                self.report(Rule::KernelTypes, head);
            }
            head = inner.as_ref();
        }
        self.check_call(term, head, args.len(), tail)?;
        for arg in args {
            self.walk(arg, false)?;
        }
        Ok(())
    }

    /// A join call is a jump only when it is in tail position or its `Let`
    /// is not; any other join call is a local function call (`R2`).
    fn check_call(
        &mut self,
        term: &Term,
        head: &'a Term,
        count: usize,
        tail: bool,
    ) -> Result<(), EligibilityError> {
        let Term::Var(name) = head else {
            self.report(Rule::FirstOrder, term);
            return Ok(());
        };
        match self.classify(name)? {
            Head::SelfCall(arity) => {
                if count != arity {
                    self.report(Rule::FirstOrder, term);
                }
                if !tail {
                    self.report(Rule::Recursion, term);
                }
            }
            Head::Join(arity, join_tail) if count == arity && (tail || !join_tail) => {}
            Head::Callee(arity) if arity > 0 && count == arity => {}
            Head::Local | Head::Join(..) | Head::Callee(_) => self.report(Rule::FirstOrder, term),
        }
        Ok(())
    }

    /// A join body inherits this `Let`'s tail flag: the match compiler emits
    /// join calls only at decision-tree leaves, which are in tail position
    /// relative to the match, so a join body is in tail position iff the
    /// match is. Locals and joins are restored after the body.
    fn walk_let(
        &mut self,
        name: &'a str,
        value: &'a Term,
        body: &'a Term,
        tail: bool,
    ) -> Result<(), EligibilityError> {
        let saved_locals = self.locals.clone();
        let saved_joins = self.joins.clone();
        if is_join_point(name) && matches!(value, Term::Lam(..)) {
            let shape = split_function(value);
            self.bind_params(&shape);
            self.walk(shape.body, tail)?;
            self.locals.clone_from(&saved_locals);
            self.joins.insert(name, (shape.params.len(), tail));
        } else {
            self.walk(value, false)?;
            self.locals.insert(name);
        }
        let result = self.walk(body, tail);
        self.locals = saved_locals;
        self.joins = saved_joins;
        result
    }

    fn walk_con(
        &mut self,
        term: &Term,
        name: &str,
        ty_args: &[Type],
        args: &'a [Term],
        tail: bool,
    ) -> Result<(), EligibilityError> {
        if name == "if" {
            for (index, arg) in args.iter().enumerate() {
                self.walk(arg, tail && index > 0)?;
            }
            return Ok(());
        }
        if !RESERVED_PRIMS.contains(&name) {
            let env = self.env;
            if !ty_args.iter().all(|ty| is_kernel_type(ty, env)) {
                self.report(Rule::KernelTypes, term);
            }
            if !env
                .owner_of(name)
                .is_some_and(|owner| is_kernel_data(owner, env))
            {
                self.report(Rule::Data, term);
            }
        }
        for arg in args {
            self.walk(arg, false)?;
        }
        Ok(())
    }

    /// Each alternative's binders are visible only in its own body.
    fn walk_case(
        &mut self,
        term: &Term,
        scrutinee: &'a Term,
        alts: &'a [Alt],
        tail: bool,
    ) -> Result<(), EligibilityError> {
        self.walk(scrutinee, false)?;
        let env = self.env;
        let owner = alts.first().and_then(|alt| env.owner_of(&alt.con));
        if !owner.is_some_and(|name| is_kernel_data(name, env)) {
            self.report(Rule::Data, term);
        }
        for alt in alts {
            let saved = self.locals.clone();
            self.locals.extend(alt.binders.iter().map(String::as_str));
            let result = self.walk(&alt.body, tail);
            self.locals = saved;
            result?;
        }
        Ok(())
    }
}

/// Collects the root `Let` spine bindings of `root`, outermost first.
fn spine_bindings(root: &Term) -> Vec<(&str, &Term)> {
    let mut bindings = Vec::new();
    let mut cursor = root;
    while let Term::Let(binder, value, body) = cursor {
        bindings.push((binder.as_str(), value.as_ref()));
        cursor = body.as_ref();
    }
    bindings
}

/// Checks the spine binding at `index` against the bindings before it.
fn check_binding<'a>(
    bindings: &[(&'a str, &'a Term)],
    index: usize,
    env: &'a LintEnv,
    instantiation: &[Type],
) -> Result<(Vec<Violation>, Vec<&'a str>), EligibilityError> {
    let (Some(&(name, value)), Some(earlier)) = (bindings.get(index), bindings.get(..index)) else {
        return Err(EligibilityError::Malformed(format!(
            "no spine binding {index}"
        )));
    };
    let shape = split_function(value);
    let mut walker = Walker::new(env, name, earlier);
    if shape.tylams != instantiation.len() {
        walker.report(Rule::Polymorphism, value);
    }
    walker.bind_params(&shape);
    walker.self_call = shape.self_binder.map(|binder| (binder, shape.params.len()));
    walker.walk(shape.body, true)?;
    Ok((walker.violations, walker.used))
}

/// Checks the binding at `index` and, breadth-first in first-reference
/// order, every spine binding it references. Memoised by binding index: a
/// name can denote different bindings at different spine prefixes.
///
/// Callees are checked with an empty instantiation in this slice, so a
/// generalised (`TyLam`-wrapped) callee is an `R5` violation even when the
/// caller instantiates it at kernel types. Per-instantiation checking, by
/// substituting the instantiation into binder types with
/// `Type::subst_many`, is future work.
fn check_with_callees(
    bindings: &[(&str, &Term)],
    index: usize,
    env: &LintEnv,
    instantiation: &[Type],
) -> Result<Vec<Violation>, EligibilityError> {
    let none: &[Type] = &[];
    let mut violations = Vec::new();
    let mut checked = BTreeSet::new();
    let mut queue = VecDeque::from([(index, instantiation)]);
    while let Some((current, types)) = queue.pop_front() {
        if !checked.insert(current) {
            continue;
        }
        let (found, used) = check_binding(bindings, current, env, types)?;
        violations.extend(found);
        let earlier = bindings.get(..current).unwrap_or_default();
        for callee in used {
            if let Some(position) = earlier.iter().rposition(|&(binder, _)| binder == callee) {
                queue.push_back((position, none));
            }
        }
    }
    Ok(violations)
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
/// Rules: `R1` binder, type-argument and parameter types are kernel types;
/// `R2` calls are saturated direct calls and no non-leading lambda appears;
/// `R3` recursion is self tail recursion only; `R4` constructors and cases
/// use non-recursive kernel data; `R5` the `TyLam` count equals the length
/// of `instantiation` and no inner `TyLam` appears. Every earlier spine
/// binding the function references is checked the same way, transitively,
/// and its violations are appended after the function's own.
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
    let bindings = spine_bindings(root);
    let index = bindings
        .iter()
        .rposition(|&(binder, _)| binder == name)
        .ok_or_else(|| EligibilityError::UnknownFunction(name.to_string()))?;
    let violations = check_with_callees(&bindings, index, env, instantiation)?;
    if violations.is_empty() {
        Ok(Report::Eligible)
    } else {
        Ok(Report::Rejected(violations))
    }
}
