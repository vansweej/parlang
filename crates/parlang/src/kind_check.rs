//! Surface kind checking for data declarations and type annotations
//! (milestone 2, slice M2.1b).
//!
//! Every type has a kind: `*` for types with values and `k1 -> k2` for type
//! functions such as `List : * -> *`. Data declarations get their parameter
//! kinds inferred by unification over the whole declaration group, with
//! unsolved kinds defaulting to `*`. Annotations are checked against `*`,
//! each annotation type variable getting a kind of its own.

use crate::ast::TypeAnnotation;
use crate::typechecker::{TypeEnv, TypeError};
use crate::types::{Type, TypeVar};
use parlang_core::Kind;
use std::collections::{BTreeMap, HashMap};

/// A kind under inference: `*`, an arrow, or an unsolved kind variable.
#[derive(Debug, Clone, PartialEq, Eq)]
enum KindTerm {
    Star,
    Arrow(Box<KindTerm>, Box<KindTerm>),
    Var(usize),
}

impl KindTerm {
    fn arrow(dom: KindTerm, cod: KindTerm) -> KindTerm {
        KindTerm::Arrow(Box::new(dom), Box::new(cod))
    }

    /// The inference form of a solved kind.
    fn of_kind(kind: &Kind) -> KindTerm {
        match kind {
            Kind::Star => KindTerm::Star,
            Kind::Arrow(dom, cod) => {
                KindTerm::arrow(KindTerm::of_kind(dom), KindTerm::of_kind(cod))
            }
        }
    }

    fn var_id(&self) -> Option<usize> {
        match self {
            KindTerm::Var(id) => Some(*id),
            _ => None,
        }
    }
}

/// The kind `p1 -> ... -> pn -> result`.
fn arrows(params: &[KindTerm], result: KindTerm) -> KindTerm {
    params
        .iter()
        .rev()
        .fold(result, |acc, param| KindTerm::arrow(param.clone(), acc))
}

/// Kind-variable solutions, indexed by `KindTerm::Var` id.
#[derive(Default)]
struct KindSolver {
    solutions: Vec<Option<KindTerm>>,
}

impl KindSolver {
    fn fresh(&mut self) -> KindTerm {
        let id = self.solutions.len();
        self.solutions.push(None);
        KindTerm::Var(id)
    }

    /// Follow solved kind variables at the root of `kind`.
    fn resolve(&self, kind: &KindTerm) -> KindTerm {
        let mut current = kind.clone();
        while let Some(next) = current
            .var_id()
            .and_then(|id| self.solutions.get(id).cloned().flatten())
        {
            current = next;
        }
        current
    }

    fn occurs(&self, id: usize, kind: &KindTerm) -> bool {
        match self.resolve(kind) {
            KindTerm::Star => false,
            KindTerm::Var(other) => other == id,
            KindTerm::Arrow(dom, cod) => self.occurs(id, &dom) || self.occurs(id, &cod),
        }
    }

    /// Unify two kinds, returning `false` when they cannot be made equal.
    fn unify(&mut self, left: &KindTerm, right: &KindTerm) -> bool {
        match (self.resolve(left), self.resolve(right)) {
            (KindTerm::Star, KindTerm::Star) => true,
            (KindTerm::Var(id), other) | (other, KindTerm::Var(id)) => self.bind(id, other),
            (KindTerm::Arrow(dom, cod), KindTerm::Arrow(other_dom, other_cod)) => {
                self.unify(&dom, &other_dom) && self.unify(&cod, &other_cod)
            }
            _ => false,
        }
    }

    fn bind(&mut self, id: usize, kind: KindTerm) -> bool {
        if kind.var_id() == Some(id) {
            return true;
        }
        if self.occurs(id, &kind) {
            return false;
        }
        match self.solutions.get_mut(id) {
            Some(slot) => {
                *slot = Some(kind);
                true
            }
            // Ids only come from `fresh`, so the slot always exists.
            None => false,
        }
    }

    /// The solved form of `kind`, defaulting unsolved variables to `*`.
    fn finish(&self, kind: &KindTerm) -> Kind {
        match self.resolve(kind) {
            KindTerm::Arrow(dom, cod) => Kind::arrow(self.finish(&dom), self.finish(&cod)),
            KindTerm::Star | KindTerm::Var(_) => Kind::Star,
        }
    }
}

/// Where the checked annotation occurs, which selects unknown-name errors.
#[derive(Clone, Copy)]
enum Site {
    Payload,
    Annotation,
}

/// Kind checker over surface type annotations.
struct KindChecker<'env> {
    env: &'env TypeEnv,
    site: Site,
    solver: KindSolver,
    /// Parameter kinds of the data types declared in the current group.
    group: HashMap<String, Vec<KindTerm>>,
    /// Kinds of the type variables in scope.
    vars: HashMap<String, KindTerm>,
}

impl<'env> KindChecker<'env> {
    fn new(env: &'env TypeEnv, site: Site) -> Self {
        KindChecker {
            env,
            site,
            solver: KindSolver::default(),
            group: HashMap::new(),
            vars: HashMap::new(),
        }
    }

    fn var_kind(&mut self, name: &str) -> KindTerm {
        let solver = &mut self.solver;
        self.vars
            .entry(name.to_string())
            .or_insert_with(|| solver.fresh())
            .clone()
    }

    /// Remaining parameter kinds of an alias target: a partially applied data
    /// type keeps its unapplied parameters, any other target has kind `*`.
    fn alias_params(&self, target: &Type) -> Vec<KindTerm> {
        let Type::SumType(data, args) = target else {
            return Vec::new();
        };
        self.env
            .data_kinds(data)
            .and_then(|kinds| kinds.get(args.len()..))
            .map(|rest| rest.iter().map(KindTerm::of_kind).collect())
            .unwrap_or_default()
    }

    /// Parameter kinds of the type constructor `name`; `bare` is `true`
    /// when it is not applied to any argument.
    fn head_params(&self, name: &str, bare: bool) -> Result<Vec<KindTerm>, TypeError> {
        if matches!(name, "Int" | "Bool" | "Char" | "Float") {
            return Ok(Vec::new());
        }
        if let Some(params) = self.group.get(name) {
            return Ok(params.clone());
        }
        if let Some(kinds) = self.env.data_kinds(name) {
            return Ok(kinds.iter().map(KindTerm::of_kind).collect());
        }
        match (self.site, self.env.resolve_type_alias(name)) {
            (Site::Payload, Some(_)) => Err(TypeError::TypeAliasInDataPayload(name.to_string())),
            (Site::Payload, None) => Err(TypeError::UnboundVariable(format!(
                "Unknown type constructor in data declaration: {name}"
            ))),
            (Site::Annotation, Some(target)) if bare => Ok(self.alias_params(&target)),
            (Site::Annotation, None) if bare => Err(TypeError::UnboundVariable(name.to_string())),
            (Site::Annotation, _) => Err(TypeError::UnboundVariable(format!(
                "Unknown type constructor in annotation: {name}"
            ))),
        }
    }

    /// Unify `found` with `expected`, reporting a `KindMismatch` for `subject`.
    fn expect(
        &mut self,
        subject: &str,
        found: &KindTerm,
        expected: &KindTerm,
    ) -> Result<(), TypeError> {
        if self.solver.unify(found, expected) {
            return Ok(());
        }
        Err(TypeError::KindMismatch(
            subject.to_string(),
            self.solver.finish(expected).to_string(),
            self.solver.finish(found).to_string(),
        ))
    }

    /// Check that `ann` has kind `expected`.
    fn check(&mut self, ann: &TypeAnnotation, expected: &KindTerm) -> Result<(), TypeError> {
        match ann {
            TypeAnnotation::Concrete(name) => self.check_con(name, &[], expected),
            TypeAnnotation::App(name, args) => self.check_con(name, args, expected),
            TypeAnnotation::Var(name) => {
                let found = self.var_kind(name);
                self.expect(name, &found, expected)
            }
            TypeAnnotation::VarApp(name, args) => {
                let mut arg_kinds = Vec::with_capacity(args.len());
                for arg in args {
                    let kind = self.solver.fresh();
                    self.check(arg, &kind)?;
                    arg_kinds.push(kind);
                }
                let found = self.var_kind(name);
                self.expect(name, &found, &arrows(&arg_kinds, expected.clone()))
            }
            TypeAnnotation::Fun(arg, ret) => {
                self.check(arg, &KindTerm::Star)?;
                self.check(ret, &KindTerm::Star)?;
                self.expect(&ann.to_string(), &KindTerm::Star, expected)
            }
        }
    }

    /// Check the type constructor `name` applied to `args` against `expected`.
    ///
    /// Over-application, and under-application where `*` is expected, keep
    /// the arity error `TypeConstructorArityMismatch`.
    fn check_con(
        &mut self,
        name: &str,
        args: &[TypeAnnotation],
        expected: &KindTerm,
    ) -> Result<(), TypeError> {
        let params = self.head_params(name, args.is_empty())?;
        let arity_error =
            || TypeError::TypeConstructorArityMismatch(name.to_string(), params.len(), args.len());
        if args.len() > params.len() {
            return Err(arity_error());
        }
        for (arg, kind) in args.iter().zip(&params) {
            self.check(arg, kind)?;
        }
        let found = arrows(params.get(args.len()..).unwrap_or_default(), KindTerm::Star);
        if self.solver.unify(&found, expected) {
            return Ok(());
        }
        if self.solver.resolve(expected) == KindTerm::Star {
            return Err(arity_error());
        }
        Err(TypeError::KindMismatch(
            name.to_string(),
            self.solver.finish(expected).to_string(),
            self.solver.finish(&found).to_string(),
        ))
    }
}

/// One `data` declaration whose parameter kinds are being inferred.
pub(crate) struct DataSig<'decl> {
    /// Declared type name.
    pub(crate) name: &'decl str,
    /// Type parameter names, in order.
    pub(crate) params: &'decl [String],
    /// Constructors with their payload field annotations.
    pub(crate) ctors: &'decl [(String, Vec<TypeAnnotation>)],
}

/// Infer the parameter kinds of a group of mutually recursive `data`
/// declarations, checking that every payload field has kind `*`.
///
/// Unsolved parameter kinds default to `*`. Returns one entry per
/// declaration, in order.
///
/// # Errors
///
/// Returns `TypeError::KindMismatch` for an ill-kinded field,
/// `TypeError::TypeConstructorArityMismatch` for a constructor applied to too
/// many arguments or left unapplied where `*` is expected, and the
/// unknown-name errors of payload checking.
pub(crate) fn infer_data_kinds(
    env: &TypeEnv,
    decls: &[DataSig<'_>],
) -> Result<Vec<(String, Vec<Kind>)>, TypeError> {
    let mut checker = KindChecker::new(env, Site::Payload);
    for decl in decls {
        let params: Vec<KindTerm> = decl.params.iter().map(|_| checker.solver.fresh()).collect();
        checker.group.insert(decl.name.to_string(), params);
    }
    for decl in decls {
        let params = checker.group.get(decl.name).cloned().unwrap_or_default();
        checker.vars = decl.params.iter().cloned().zip(params).collect();
        for (_, fields) in decl.ctors {
            for field in fields {
                checker.check(field, &KindTerm::Star)?;
            }
        }
    }
    Ok(decls
        .iter()
        .map(|decl| {
            let params = checker
                .group
                .get(decl.name)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let kinds = params
                .iter()
                .map(|kind| checker.solver.finish(kind))
                .collect();
            (decl.name.to_string(), kinds)
        })
        .collect())
}

/// Check that the annotation `ann` has kind `*`; its type variables get
/// kinds of their own, shared across the annotation.
///
/// # Errors
///
/// Returns the errors of `infer_data_kinds`, with annotation wording for
/// unknown type names.
pub(crate) fn check_annotation_kinds(env: &TypeEnv, ann: &TypeAnnotation) -> Result<(), TypeError> {
    KindChecker::new(env, Site::Annotation).check(ann, &KindTerm::Star)
}

/// Infer the kind of the class parameter `param` from the method
/// signatures in `methods`, checking that every signature has kind `*`.
///
/// The parameter's kind is shared by all signatures; every other type
/// variable gets a fresh kind per signature. An unsolved parameter kind
/// defaults to `*`, so `class Container f where { empty : f Int }` gives
/// `f : * -> *`.
///
/// # Errors
///
/// Returns the errors of `check_annotation_kinds`, e.g. `KindMismatch` for
/// `m : f -> f Int`.
pub(crate) fn infer_class_param_kind(
    env: &TypeEnv,
    param: &str,
    methods: &[(String, TypeAnnotation)],
) -> Result<Kind, TypeError> {
    let mut checker = KindChecker::new(env, Site::Annotation);
    let param_kind = checker.solver.fresh();
    for (_, sig) in methods {
        checker.vars = HashMap::from([(param.to_string(), param_kind.clone())]);
        checker.check(sig, &KindTerm::Star)?;
    }
    Ok(checker.solver.finish(&param_kind))
}

/// Kind inference over the type variables of one resolved surface `Type`.
struct VarKinds<'data> {
    /// Parameter kinds of every known data type, keyed by type name.
    data_kinds: &'data BTreeMap<String, Vec<Kind>>,
    solver: KindSolver,
    /// Kinds of the type variables seen so far.
    vars: BTreeMap<TypeVar, KindTerm>,
}

impl VarKinds<'_> {
    fn var_kind(&mut self, var: &TypeVar) -> KindTerm {
        let solver = &mut self.solver;
        self.vars
            .entry(var.clone())
            .or_insert_with(|| solver.fresh())
            .clone()
    }

    /// Constrain `ty` to have kind `expected`. A failed unification is
    /// ignored: it cannot happen on a type-checked program, and Core Lint
    /// re-checks every kind.
    fn visit(&mut self, ty: &Type, expected: &KindTerm) {
        match ty {
            Type::Int | Type::Bool | Type::Char | Type::Float | Type::Unit => {
                self.solver.unify(&KindTerm::Star, expected);
            }
            Type::Fun(arg, ret) => {
                self.visit(arg, &KindTerm::Star);
                self.visit(ret, &KindTerm::Star);
                self.solver.unify(&KindTerm::Star, expected);
            }
            Type::Tuple(items) => {
                for item in items {
                    self.visit(item, &KindTerm::Star);
                }
                self.solver.unify(&KindTerm::Star, expected);
            }
            Type::Record(fields) => {
                for field in fields.values() {
                    self.visit(field, &KindTerm::Star);
                }
                self.solver.unify(&KindTerm::Star, expected);
            }
            Type::Var(var) => {
                let found = self.var_kind(var);
                self.solver.unify(&found, expected);
            }
            Type::SumType(name, args) => {
                let params: Vec<KindTerm> = self
                    .data_kinds
                    .get(name)
                    .map(|kinds| kinds.iter().map(KindTerm::of_kind).collect())
                    .unwrap_or_default();
                for (index, arg) in args.iter().enumerate() {
                    let kind = params.get(index).cloned().unwrap_or(KindTerm::Star);
                    self.visit(arg, &kind);
                }
                let rest = params.get(args.len()..).unwrap_or_default();
                let found = arrows(rest, KindTerm::Star);
                self.solver.unify(&found, expected);
            }
            Type::VarApp(var, args) => {
                let mut arg_kinds = Vec::with_capacity(args.len());
                for arg in args {
                    let kind = self.solver.fresh();
                    self.visit(arg, &kind);
                    arg_kinds.push(kind);
                }
                let found = self.var_kind(var);
                self.solver
                    .unify(&found, &arrows(&arg_kinds, expected.clone()));
            }
        }
    }
}

/// Infer the kind of every type variable occurring in `ty`, given the
/// parameter kinds `data_kinds` of the data types it mentions.
///
/// `ty` is checked against `*`. A partially applied data type gets the
/// kind of its remaining parameters, and an applied variable `f a` gets an
/// arrow kind. Unsolved kinds default to `*`. Unknown data names and
/// unification failures cannot occur on a type-checked program; they are
/// ignored rather than reported, leaving Core Lint as the safety net.
pub(crate) fn infer_type_var_kinds(
    data_kinds: &BTreeMap<String, Vec<Kind>>,
    ty: &Type,
) -> BTreeMap<TypeVar, Kind> {
    let mut inference = VarKinds {
        data_kinds,
        solver: KindSolver::default(),
        vars: BTreeMap::new(),
    };
    inference.visit(ty, &KindTerm::Star);
    inference
        .vars
        .iter()
        .map(|(var, kind)| (var.clone(), inference.solver.finish(kind)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The parameter kinds of `Wrap f a`: `[* -> *, *]`.
    fn wrap_kinds() -> BTreeMap<String, Vec<Kind>> {
        let mut data_kinds = BTreeMap::new();
        data_kinds.insert(
            "Wrap".to_string(),
            vec![Kind::arrow(Kind::Star, Kind::Star), Kind::Star],
        );
        data_kinds
    }

    /// `Wrap f a -> f a` gives `f : * -> *` and `a : *`.
    #[test]
    fn infer_type_var_kinds_wrap_scheme() {
        let head = TypeVar(0);
        let elem = TypeVar(1);
        let wrap = Type::SumType(
            "Wrap".to_string(),
            vec![Type::Var(head.clone()), Type::Var(elem.clone())],
        );
        let applied = Type::VarApp(head.clone(), vec![Type::Var(elem.clone())]);
        let ty = Type::Fun(Box::new(wrap), Box::new(applied));
        let kinds = infer_type_var_kinds(&wrap_kinds(), &ty);
        let star_to_star = Kind::arrow(Kind::Star, Kind::Star);
        assert_eq!(kinds.get(&head), Some(&star_to_star));
        assert_eq!(kinds.get(&elem), Some(&Kind::Star));
    }

    /// A variable applied to two arguments gets `* -> * -> *`.
    #[test]
    fn infer_type_var_kinds_binary_application() {
        let head = TypeVar(3);
        let ty = Type::VarApp(head.clone(), vec![Type::Int, Type::Bool]);
        let kinds = infer_type_var_kinds(&BTreeMap::new(), &ty);
        let expected = Kind::arrow(Kind::Star, Kind::arrow(Kind::Star, Kind::Star));
        assert_eq!(kinds.get(&head), Some(&expected));
    }

    /// Variables under an unknown data name, or used at `*`, default to `*`.
    #[test]
    fn infer_type_var_kinds_defaults_to_star() {
        let elem = TypeVar(5);
        let ty = Type::SumType("Mystery".to_string(), vec![Type::Var(elem.clone())]);
        let kinds = infer_type_var_kinds(&wrap_kinds(), &ty);
        assert_eq!(kinds.get(&elem), Some(&Kind::Star));
    }
}
