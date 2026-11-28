//! Translates a type-checked `TypedProgram` into an explicitly typed
//! `parlang_core::Term`, using `parlang_core::builder`.
//!
//! `check_elaborated` runs Core Lint on the result.
//!
//! Unsupported constructs, including every `match` until slice 7e, return
//! `ElabError::Unsupported` and never panic.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use parlang_core::Type as CoreType;
use parlang_core::{builder, BaseType, BuildError, DataCon, DataType, LintEnv, LintError, Term};

use crate::ast::{BinOp, TypeAnnotation};
use crate::typed_ast::{TypedDecl, TypedExpr, TypedExprKind as K, TypedProgram};
use crate::types::{Type, TypeScheme, TypeVar};

mod dictionary;
mod match_compile;

/// The result of elaborating a `TypedProgram` into Core.
#[derive(Debug, Clone)]
pub struct Elaborated {
    /// The elaborated Core term.
    pub term: Term,
    /// The data environment the term needs: user declarations plus
    /// synthesized tuple and record types.
    pub env: LintEnv,
    /// The surface program type converted to Core, with residual variables
    /// defaulted to `Unit`.
    pub ty: CoreType,
}

/// An error raised while elaborating a `TypedProgram` into Core.
#[derive(Debug, Clone, PartialEq)]
pub enum ElabError {
    /// A construct not yet supported by Core elaboration.
    Unsupported {
        /// Name of the unsupported construct.
        construct: String,
        /// Reason it is not supported.
        reason: String,
    },
    /// An invalid data declaration was encountered while registering types.
    Data(LintError),
    /// The elaborated term failed Core Lint.
    Lint(LintError),
    /// The Core-checked type does not match the expected surface type.
    TypeMismatch {
        /// The expected surface type, converted to Core.
        surface: CoreType,
        /// The type Core Lint actually synthesized.
        core: CoreType,
    },
    /// A term failed to build.
    Build(BuildError),
}

impl From<BuildError> for ElabError {
    fn from(e: BuildError) -> Self {
        ElabError::Build(e)
    }
}

impl fmt::Display for ElabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElabError::Unsupported { construct, reason } => {
                write!(
                    f,
                    "{construct} is not supported by Core elaboration yet: {reason}"
                )
            }
            ElabError::Data(e) => write!(f, "invalid data declaration: {e}"),
            ElabError::Lint(e) => write!(f, "Core Lint failed: {e}"),
            ElabError::TypeMismatch { surface, core } => {
                write!(f, "Core type {core} does not match surface type {surface}")
            }
            ElabError::Build(e) => write!(f, "Core build error: {e}"),
        }
    }
}

impl std::error::Error for ElabError {}

/// Builds an `Unsupported` error for `construct` with the given `reason`.
fn unsupported(construct: &str, reason: &str) -> ElabError {
    ElabError::Unsupported {
        construct: construct.to_string(),
        reason: reason.to_string(),
    }
}

/// Mutable elaboration state threaded through the translation.
#[derive(Default)]
struct Elab {
    /// The Core data environment being built.
    env: LintEnv,
    /// Parameter kinds of every registered data type, keyed by type name.
    data_kinds: BTreeMap<String, Vec<parlang_core::Kind>>,
    /// Constructors of each registered data type as `(name, arity)`, in declaration order.
    data_ctors: BTreeMap<String, Vec<(String, usize)>>,
    /// Owning data type name of every registered constructor.
    ctor_owner: BTreeMap<String, String>,
    /// Surface type-variable ids bound by enclosing `TyLam`s.
    scope: Vec<usize>,
    /// Counter for fresh `$`-prefixed binder names.
    fresh: usize,
    /// Core names overriding surface type variables, consulted before `scope`.
    var_names: BTreeMap<usize, String>,
    /// Superclasses and method schemes of each registered dictionary type,
    /// keyed by its name.
    classes: BTreeMap<String, dictionary::ClassDict>,
    /// Built-in `Eq` dictionaries referenced so far, bound by `elab_program`.
    builtin_dicts: BTreeSet<String>,
    /// The instance dictionary being elaborated, for self-references.
    self_dict: Option<dictionary::SelfDict>,
}

impl Elab {
    /// Registers `dt` in the Core `LintEnv`, recording its parameter kinds.
    fn register_data(&mut self, dt: DataType) -> Result<(), ElabError> {
        self.data_kinds
            .insert(dt.name.clone(), dt.param_kinds.clone());
        let ctors: Vec<(String, usize)> = dt
            .ctors
            .iter()
            .map(|c| (c.name.clone(), c.fields.len()))
            .collect();
        for (cname, _) in &ctors {
            self.ctor_owner.insert(cname.clone(), dt.name.clone());
        }
        self.data_ctors.insert(dt.name.clone(), ctors);
        self.env = std::mem::take(&mut self.env)
            .try_with_data(dt)
            .map_err(ElabError::Data)?;
        Ok(())
    }

    fn conv_type(&mut self, ty: &Type) -> Result<CoreType, ElabError> {
        match ty {
            Type::Int => Ok(CoreType::Base(BaseType::Int)),
            Type::Bool => Ok(CoreType::Base(BaseType::Bool)),
            Type::Char => Ok(CoreType::Base(BaseType::Char)),
            Type::Float => Ok(CoreType::Base(BaseType::Float)),
            Type::Unit => Ok(CoreType::Base(BaseType::Unit)),
            Type::Fun(a, b) => Ok(CoreType::Fun(
                Box::new(self.conv_type(a)?),
                Box::new(self.conv_type(b)?),
            )),
            Type::Var(TypeVar(n)) => Ok(self
                .tyvar_name(*n)
                .map_or(CoreType::Base(BaseType::Unit), CoreType::Var)),
            // Partial application is allowed: Core Lint checks its kind.
            Type::SumType(name, args) => {
                self.ensure_dict_type(name)?;
                if let Some(kinds) = self.data_kinds.get(name) {
                    if args.len() > kinds.len() {
                        return Err(unsupported(
                            "type",
                            &format!(
                                "data type {name} applied to {} arguments, expects at most {}",
                                args.len(),
                                kinds.len()
                            ),
                        ));
                    }
                }
                let mut acc = CoreType::Con(name.clone());
                for arg in args {
                    acc = CoreType::App(Box::new(acc), Box::new(self.conv_type(arg)?));
                }
                Ok(acc)
            }
            Type::VarApp(TypeVar(n), args) => {
                let Some(head) = self.tyvar_name(*n) else {
                    return Err(unsupported("type", "residual higher-kinded type variable"));
                };
                let mut acc = CoreType::Var(head);
                for arg in args {
                    acc = CoreType::App(Box::new(acc), Box::new(self.conv_type(arg)?));
                }
                Ok(acc)
            }
            Type::Tuple(items) => self.conv_tuple_type(items),
            Type::Record(fields) => self.conv_record_type(fields),
        }
    }

    /// Converts a surface `TypeAnnotation` to a Core `Type`, mirroring the
    /// surface `type_annotation_to_type`.
    fn conv_ann(ann: &TypeAnnotation) -> CoreType {
        match ann {
            TypeAnnotation::Concrete(name) => match name.as_str() {
                "Int" => CoreType::Base(BaseType::Int),
                "Bool" => CoreType::Base(BaseType::Bool),
                "Char" => CoreType::Base(BaseType::Char),
                "Float" => CoreType::Base(BaseType::Float),
                other => CoreType::Con(other.to_string()),
            },
            TypeAnnotation::Var(p) => CoreType::Var(p.clone()),
            TypeAnnotation::Fun(a, b) => {
                CoreType::Fun(Box::new(Self::conv_ann(a)), Box::new(Self::conv_ann(b)))
            }
            TypeAnnotation::App(n, args) => {
                let mut acc = CoreType::Con(n.clone());
                for arg in args {
                    acc = CoreType::App(Box::new(acc), Box::new(Self::conv_ann(arg)));
                }
                acc
            }
            TypeAnnotation::VarApp(p, args) => {
                let mut acc = CoreType::Var(p.clone());
                for arg in args {
                    acc = CoreType::App(Box::new(acc), Box::new(Self::conv_ann(arg)));
                }
                acc
            }
        }
    }

    /// Generates a fresh `$`-prefixed binder name using `prefix`.
    ///
    /// The `$` prefix cannot clash with surface identifiers, and builders
    /// reject only empty names.
    fn fresh_name(&mut self, prefix: &str) -> String {
        let name = format!("${prefix}{}", self.fresh);
        self.fresh += 1;
        name
    }

    /// Returns the synthetic tuple type-constructor name for `arity`, e.g.
    /// `(,)` for arity 2 and `(,,)` for arity 3.
    fn tuple_type_name(arity: usize) -> String {
        format!("({})", ",".repeat(arity.saturating_sub(1)))
    }

    /// Returns the synthetic record type-constructor name for a sorted field
    /// list, e.g. `{age,name}`.
    fn record_type_name(sorted: &[String]) -> String {
        format!("{{{}}}", sorted.join(","))
    }

    /// Registers a synthetic data type named `name` with `arity` type
    /// parameters and a single constructor of the same name, if not already
    /// registered.
    fn ensure_synthetic(&mut self, name: &str, arity: usize) -> Result<(), ElabError> {
        if self.data_kinds.contains_key(name) {
            return Ok(());
        }
        let params: Vec<String> = (1..=arity).map(|i| format!("p{i}")).collect();
        let fields: Vec<CoreType> = params.iter().map(|p| CoreType::Var(p.clone())).collect();
        let dt = DataType {
            name: name.to_string(),
            params,
            param_kinds: vec![parlang_core::Kind::Star; arity],
            ctors: vec![DataCon {
                name: name.to_string(),
                fields,
            }],
        };
        self.register_data(dt)
    }

    /// Ensures a synthetic tuple type of `arity` is registered and returns
    /// its name.
    fn ensure_tuple(&mut self, arity: usize) -> Result<String, ElabError> {
        let name = Self::tuple_type_name(arity);
        self.ensure_synthetic(&name, arity)?;
        Ok(name)
    }

    /// Ensures a synthetic record type over `sorted` field names is
    /// registered and returns its name.
    fn ensure_record(&mut self, sorted: &[String]) -> Result<String, ElabError> {
        let name = Self::record_type_name(sorted);
        self.ensure_synthetic(&name, sorted.len())?;
        Ok(name)
    }

    /// Converts a surface tuple type of arity two or more to Core, via a
    /// synthetic tuple data type.
    fn conv_tuple_type(&mut self, items: &[Type]) -> Result<CoreType, ElabError> {
        if items.len() < 2 {
            return Err(unsupported("tuple type", "tuple of arity below two"));
        }
        let name = self.ensure_tuple(items.len())?;
        let mut acc = CoreType::Con(name);
        for item in items {
            let item_ty = self.conv_type(item)?;
            acc = CoreType::App(Box::new(acc), Box::new(item_ty));
        }
        Ok(acc)
    }

    /// Converts a surface record type to Core, via a synthetic record data
    /// type keyed by sorted field names.
    fn conv_record_type(
        &mut self,
        fields: &std::collections::HashMap<String, Type>,
    ) -> Result<CoreType, ElabError> {
        let mut sorted: Vec<String> = fields.keys().cloned().collect();
        sorted.sort();
        let name = self.ensure_record(&sorted)?;
        let mut acc = CoreType::Con(name);
        for field_name in &sorted {
            let field_ty = self.conv_type(&fields[field_name])?;
            acc = CoreType::App(Box::new(acc), Box::new(field_ty));
        }
        Ok(acc)
    }

    /// Elaborates a tuple literal into a synthetic tuple constructor
    /// application.
    fn elab_tuple(&mut self, ty: &Type, items: &[TypedExpr]) -> Result<Term, ElabError> {
        let Type::Tuple(component_types) = ty else {
            return Err(unsupported("tuple", "tuple without tuple type"));
        };
        let name = self.ensure_tuple(items.len())?;
        let mut type_args = Vec::with_capacity(component_types.len());
        for component_ty in component_types {
            type_args.push(self.conv_type(component_ty)?);
        }
        let mut values = Vec::with_capacity(items.len());
        for item in items {
            values.push(self.elab_expr(item)?);
        }
        Ok(builder::con_at(&name, type_args, values)?)
    }

    /// Elaborates `inner.index` (0-based) via a single-alt `Case` over the
    /// synthetic tuple constructor.
    fn elab_tuple_proj(&mut self, inner: &TypedExpr, index: usize) -> Result<Term, ElabError> {
        let Type::Tuple(ts) = &inner.ty else {
            return Err(unsupported("tuple projection", "projection of a non-tuple"));
        };
        let name = self.ensure_tuple(ts.len())?;
        let scrut = self.elab_expr(inner)?;
        let binders: Vec<String> = (0..ts.len()).map(|_| self.fresh_name("p")).collect();
        let refs: Vec<&str> = binders.iter().map(String::as_str).collect();
        let Some(selected) = binders.get(index).map(String::as_str) else {
            return Err(unsupported("tuple projection", "projection out of range"));
        };
        Ok(builder::case_(
            scrut,
            vec![builder::alt(&name, &refs, builder::var(selected)?)?],
        ))
    }

    /// Elaborates a record literal into a synthetic record constructor
    /// application, with fields in sorted order.
    fn elab_record(
        &mut self,
        ty: &Type,
        fields: &[(String, TypedExpr)],
    ) -> Result<Term, ElabError> {
        if let Type::SumType(dict_type, args) = ty {
            return self.elab_dict_record(dict_type, args, fields);
        }
        let Type::Record(map) = ty else {
            return Err(unsupported("record", "record without record type"));
        };
        let mut names: Vec<&String> = fields.iter().map(|(n, _)| n).collect();
        names.sort();
        let sorted: Vec<String> = names.iter().map(|n| (*n).clone()).collect();
        let ctor_name = self.ensure_record(&sorted)?;

        let mut type_args = Vec::with_capacity(sorted.len());
        for field_name in &sorted {
            let Some(field_ty) = map.get(field_name) else {
                return Err(unsupported("record", "field missing from record type"));
            };
            type_args.push(self.conv_type(field_ty)?);
        }

        // Evaluating in sorted order rather than source order changes
        // evaluation order, which is safe because Core is pure.
        let mut values = Vec::with_capacity(sorted.len());
        for field_name in &sorted {
            let value_expr = fields
                .iter()
                .find(|(n, _)| n == field_name)
                .map(|(_, e)| e)
                .ok_or_else(|| unsupported("record", "field missing from record value"))?;
            values.push(self.elab_expr(value_expr)?);
        }

        Ok(builder::con_at(&ctor_name, type_args, values)?)
    }

    /// Elaborates `inner.field` via a single-alt `Case` over the synthetic
    /// record constructor.
    fn elab_field_access(&mut self, inner: &TypedExpr, field: &str) -> Result<Term, ElabError> {
        let Type::Record(map) = &inner.ty else {
            return Err(unsupported(
                "field access",
                "field access on non-record type",
            ));
        };
        let mut sorted: Vec<String> = map.keys().cloned().collect();
        sorted.sort();
        let ctor_name = self.ensure_record(&sorted)?;
        let Some(position) = sorted.iter().position(|n| n == field) else {
            return Err(unsupported(
                "field access",
                "field access on non-record type",
            ));
        };
        let scrut = self.elab_expr(inner)?;
        let binders: Vec<String> = sorted.iter().map(|_| self.fresh_name("f")).collect();
        let refs: Vec<&str> = binders.iter().map(String::as_str).collect();
        let selected = binders[position].as_str();
        Ok(builder::case_(
            scrut,
            vec![builder::alt(&ctor_name, &refs, builder::var(selected)?)?],
        ))
    }

    /// Elaborates a user data-constructor application with its explicit
    /// instantiation type arguments.
    fn elab_constructor(
        &mut self,
        ty: &Type,
        name: &str,
        args: &[TypedExpr],
    ) -> Result<Term, ElabError> {
        let Type::SumType(_, sum_args) = ty else {
            return Err(unsupported("constructor", "constructor without data type"));
        };
        let mut type_args = Vec::with_capacity(sum_args.len());
        for arg in sum_args {
            type_args.push(self.conv_type(arg)?);
        }
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.elab_expr(arg)?);
        }
        Ok(builder::con_at(name, type_args, values)?)
    }

    /// Registers the data declaration `name` with its typechecker-inferred
    /// `param_kinds`, parallel to `params`.
    fn data_decl(
        &mut self,
        name: &str,
        params: &[String],
        param_kinds: &[parlang_core::Kind],
        ctors: &[(String, Vec<TypeAnnotation>)],
    ) -> Result<(), ElabError> {
        let dt = DataType {
            name: name.to_string(),
            params: params.to_vec(),
            param_kinds: param_kinds.to_vec(),
            ctors: ctors
                .iter()
                .map(|(ctor_name, anns)| DataCon {
                    name: ctor_name.clone(),
                    fields: anns.iter().map(Self::conv_ann).collect(),
                })
                .collect(),
        };
        self.register_data(dt)
    }

    fn elab_expr(&mut self, expr: &TypedExpr) -> Result<Term, ElabError> {
        match &expr.kind {
            K::Int(n) => Ok(builder::int(*n)),
            K::Bool(b) => Ok(builder::bool_(*b)),
            K::Char(c) => Ok(builder::char_(*c)),
            K::Float(x) => Ok(builder::float(*x)),
            K::Var(name, type_args) => self.elab_var(name, type_args),
            K::Fun(param, _, body) => self.elab_fun(&expr.ty, param, body),
            K::App(func, arg) => self.elab_app(func, arg),
            K::Let(name, _, scheme, value, body) => self.elab_let(name, scheme, value, body),
            K::Rec(name, body) => self.elab_rec(&expr.ty, name, body),
            K::If(cond, then_e, else_e) => self.elab_if(cond, then_e, else_e),
            K::BinOp(op, lhs, rhs) => self.elab_binop(*op, lhs, rhs),
            K::TypeAlias(_, _, body) => self.elab_expr(body),
            K::TypeDef {
                name,
                type_params,
                param_kinds,
                constructors,
                body,
            } => self.elab_typedef(name, type_params, param_kinds, constructors, body),
            K::Tuple(items) if items.is_empty() => Ok(builder::unit()),
            K::Tuple(items) => self.elab_tuple(&expr.ty, items),
            K::TupleProj(inner, index) => self.elab_tuple_proj(inner, *index),
            K::Record(fields) => self.elab_record(&expr.ty, fields),
            K::FieldAccess(inner, field) => self.elab_projection(&expr.ty, inner, field),
            K::Constructor(name, args) => self.elab_constructor(&expr.ty, name, args),
            K::Match(scrut, arms) => self.compile_match(scrut, arms),
            K::Load(..) => Err(unsupported(
                "load",
                "load is not type-checked; typed modules come later",
            )),
        }
    }

    fn elab_var(&mut self, name: &str, type_args: &[Type]) -> Result<Term, ElabError> {
        if let Some(term) = self.elab_dict_var(name, type_args)? {
            return Ok(term);
        }
        let mut term = builder::var(name)?;
        for ty in type_args {
            let core_ty = self.conv_type(ty)?;
            term = builder::tyapp(term, core_ty);
        }
        Ok(term)
    }

    fn elab_fun(&mut self, ty: &Type, param: &str, body: &TypedExpr) -> Result<Term, ElabError> {
        let Type::Fun(dom, _) = ty else {
            return Err(unsupported("fun", "lambda without function type"));
        };
        let dom_ty = self.conv_type(dom)?;
        let body_term = self.elab_expr(body)?;
        Ok(builder::lam(param, dom_ty, body_term)?)
    }

    fn elab_app(&mut self, func: &TypedExpr, arg: &TypedExpr) -> Result<Term, ElabError> {
        let f2 = self.elab_expr(func)?;
        let a2 = self.elab_expr(arg)?;
        Ok(builder::app(f2, a2))
    }

    /// Wraps the elaborated `value` in one `TyLam` per generalized variable of
    /// `scheme`, with the first variable ending up outermost. Each binder's
    /// kind is inferred from how its variable is used in `scheme.ty`. A
    /// scheme that still has class constraints is rejected: run
    /// `crate::dictionary::insert_dictionaries` before elaborating.
    fn elab_generalized(
        &mut self,
        scheme: &TypeScheme,
        value: &TypedExpr,
    ) -> Result<Term, ElabError> {
        if !scheme.constraints.is_empty() {
            return Err(unsupported(
                "constrained binding",
                &format!("{scheme} still needs dictionary passing; insert dictionaries first"),
            ));
        }
        let depth = self.scope.len();
        for v in &scheme.vars {
            self.scope.push(v.0);
        }
        let result = self.elab_expr(value);
        self.scope.truncate(depth);
        let mut term = result?;
        let kinds = crate::kind_check::infer_type_var_kinds(&self.data_kinds, &scheme.ty);
        for v in scheme.vars.iter().rev() {
            let kind = kinds.get(v).cloned().unwrap_or(parlang_core::Kind::Star);
            term = builder::tylam_k(&format!("t{}", v.0), kind, term)?;
        }
        Ok(term)
    }

    fn elab_let(
        &mut self,
        name: &str,
        scheme: &TypeScheme,
        value: &TypedExpr,
        body: &TypedExpr,
    ) -> Result<Term, ElabError> {
        let value_term = self.elab_generalized(scheme, value)?;
        let body_term = self.elab_expr(body)?;
        Ok(builder::let_(name, value_term, body_term)?)
    }

    fn elab_rec(&mut self, ty: &Type, name: &str, body: &TypedExpr) -> Result<Term, ElabError> {
        let lam_term = self.elab_expr(body)?;
        if !matches!(lam_term, Term::Lam(..)) {
            return Err(unsupported("rec", "recursive binding of a non-function"));
        }
        let ann = self.conv_type(ty)?;
        Ok(builder::letrec(name, ann, lam_term, builder::var(name)?)?)
    }

    fn elab_if(
        &mut self,
        cond: &TypedExpr,
        then_e: &TypedExpr,
        else_e: &TypedExpr,
    ) -> Result<Term, ElabError> {
        let c2 = self.elab_expr(cond)?;
        let t2 = self.elab_expr(then_e)?;
        let e2 = self.elab_expr(else_e)?;
        Ok(builder::con("if", vec![c2, t2, e2])?)
    }

    /// Returns the Core primitive name for an arithmetic operator.
    fn arith_name(op: BinOp) -> Option<&'static str> {
        match op {
            BinOp::Add => Some("+"),
            BinOp::Sub => Some("-"),
            BinOp::Mul => Some("*"),
            BinOp::Div => Some("/"),
            _ => None,
        }
    }

    fn elab_binop(
        &mut self,
        op: BinOp,
        lhs: &TypedExpr,
        rhs: &TypedExpr,
    ) -> Result<Term, ElabError> {
        if matches!(op, BinOp::Eq | BinOp::Neq) {
            let lhs_core = self.conv_type(&lhs.ty)?;
            if !matches!(lhs_core, CoreType::Base(_)) {
                let sym = if matches!(op, BinOp::Eq) { "==" } else { "!=" };
                return Err(unsupported(
                    sym,
                    &format!("equality on non-base type {}", lhs.ty),
                ));
            }
        }

        let l2 = self.elab_expr(lhs)?;
        let r2 = self.elab_expr(rhs)?;

        if let Some(p) = Self::arith_name(op) {
            return Ok(builder::con(p, vec![l2, r2])?);
        }

        // The operand swap below is safe because Core is pure: evaluating the
        // two operands in reversed order has no observable side effect.
        match op {
            BinOp::Lt => Ok(builder::con("<", vec![l2, r2])?),
            BinOp::Gt => Ok(builder::con("<", vec![r2, l2])?),
            BinOp::Le => {
                let gt = builder::con("<", vec![r2, l2])?;
                Ok(builder::con(
                    "if",
                    vec![gt, builder::bool_(false), builder::bool_(true)],
                )?)
            }
            BinOp::Ge => {
                let lt = builder::con("<", vec![l2, r2])?;
                Ok(builder::con(
                    "if",
                    vec![lt, builder::bool_(false), builder::bool_(true)],
                )?)
            }
            BinOp::Eq => Ok(builder::con("eq", vec![l2, r2])?),
            BinOp::Neq => {
                let eq = builder::con("eq", vec![l2, r2])?;
                Ok(builder::con(
                    "if",
                    vec![eq, builder::bool_(false), builder::bool_(true)],
                )?)
            }
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                Err(unsupported("binop", "unexpected operator"))
            }
        }
    }

    fn elab_typedef(
        &mut self,
        name: &str,
        params: &[String],
        param_kinds: &[parlang_core::Kind],
        ctors: &[(String, Vec<TypeAnnotation>)],
        body: &TypedExpr,
    ) -> Result<Term, ElabError> {
        self.data_decl(name, params, param_kinds, ctors)?;
        self.elab_expr(body)
    }

    fn elab_program(&mut self, program: &TypedProgram) -> Result<Term, ElabError> {
        let mut lets: Vec<(String, Term)> = Vec::new();
        for decl in &program.decls {
            match decl {
                TypedDecl::Data {
                    name,
                    type_params,
                    param_kinds,
                    constructors,
                    ..
                } => {
                    self.data_decl(name, type_params, param_kinds, constructors)?;
                }
                // Surface types never contain alias names: `TypeEnv` resolves
                // aliases to their underlying `Type` before inference
                // records them on any node, so there is nothing to elaborate.
                TypedDecl::TypeAlias { .. } => {}
                TypedDecl::Class {
                    name,
                    param_kind,
                    superclasses,
                    methods,
                    ..
                } => self.register_class(name, param_kind, superclasses, methods)?,
                TypedDecl::Instance { .. } => {
                    return Err(unsupported(
                        "instance declaration",
                        "insert dictionaries before elaborating",
                    ));
                }
                TypedDecl::Let {
                    name,
                    scheme,
                    value,
                    ..
                } => {
                    let value_term = if dictionary::is_instance_dictionary(name) {
                        self.elab_instance_dict(name, scheme, value)?
                    } else {
                        self.elab_generalized(scheme, value)?
                    };
                    lets.push((name.clone(), value_term));
                }
            }
        }

        let mut acc = match &program.body {
            Some(b) => self.elab_expr(b)?,
            None => builder::int(0),
        };
        for (name, value) in lets.into_iter().rev() {
            acc = builder::let_(&name, value, acc)?;
        }
        self.wrap_builtin_dicts(acc)
    }
}

/// Elaborates a type-checked `program` into Core.
///
/// # Errors
///
/// Returns an `ElabError` if `program` uses a construct not yet supported by
/// Core elaboration, or if a Core term fails to build.
pub fn elaborate_program(program: &TypedProgram) -> Result<Elaborated, ElabError> {
    let mut elab = Elab::default();
    let term = elab.elab_program(program)?;
    let ty = elab.conv_type(&program.ty)?;
    Ok(Elaborated {
        term,
        env: elab.env,
        ty,
    })
}

/// Runs Core Lint on `elab` and checks its synthesized type against the
/// expected surface-derived type.
///
/// # Errors
///
/// Returns an `ElabError` if Core Lint rejects the term, or if the
/// synthesized type does not match the expected surface type.
pub fn check_elaborated(elab: &Elaborated) -> Result<CoreType, ElabError> {
    let core = parlang_core::lint(&elab.term, &elab.env).map_err(ElabError::Lint)?;
    if !core.alpha_eq(&elab.ty) {
        return Err(ElabError::TypeMismatch {
            surface: elab.ty.clone(),
            core,
        });
    }
    Ok(core)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses and type-checks `src`, returning its `TypedProgram`.
    fn typed(src: &str) -> TypedProgram {
        let program = crate::parser::parse_program(src).expect("parse");
        crate::typechecker::typecheck_program_typed(&program).expect("typecheck")
    }

    /// Elaborates `src` into Core.
    fn elab_src(src: &str) -> Result<Elaborated, ElabError> {
        elaborate_program(&typed(src))
    }

    /// Elaborates, Lint-checks, and evaluates `src`, returning its `Value`.
    fn run_src(src: &str) -> parlang_core::Value {
        let e = elab_src(src).expect("elaborate");
        let checked = check_elaborated(&e);
        assert!(checked.is_ok(), "check_elaborated: {checked:?}");
        parlang_core::eval(&e.term, &parlang_core::Environment::new()).expect("eval")
    }

    /// `1 + 2 * 3 - 4 / 2` evaluates to `5` under Core arithmetic.
    #[test]
    fn int_arithmetic() {
        assert_eq!(run_src("1 + 2 * 3 - 4 / 2"), parlang_core::Value::Int(5));
    }

    /// `1.5 + 2.0` evaluates to `3.5` as a `Float`.
    #[test]
    fn float_arithmetic() {
        assert_eq!(run_src("1.5 + 2.0"), parlang_core::Value::Float(3.5));
    }

    /// Integer comparisons `<`, `>`, `<=`, `>=`, `==`, `!=` elaborate correctly.
    #[test]
    fn comparisons_int() {
        let cases = [
            ("1 < 2", true),
            ("2 > 1", true),
            ("2 <= 2", true),
            ("3 >= 4", false),
            ("1 == 1", true),
            ("1 != 1", false),
        ];
        for (src, expected) in cases {
            assert_eq!(run_src(src), parlang_core::Value::Bool(expected), "{src}");
        }
    }

    /// Char comparisons `<`, `>`, `<=`, `>=`, `==`, `!=` elaborate correctly.
    #[test]
    fn comparisons_char() {
        let cases = [
            ("'a' < 'b'", true),
            ("'a' > 'b'", false),
            ("'a' <= 'a'", true),
            ("'b' >= 'a'", true),
            ("'a' == 'b'", false),
            ("'a' != 'b'", true),
        ];
        for (src, expected) in cases {
            assert_eq!(run_src(src), parlang_core::Value::Bool(expected), "{src}");
        }
    }

    /// `<=` on `Float` rewrites via `<` and `if` and evaluates correctly.
    #[test]
    fn float_less_equal_rewrites() {
        assert_eq!(run_src("1.5 <= 2.5"), parlang_core::Value::Bool(true));
    }

    /// `if 1 < 2 then 10 else 20` evaluates to `10`.
    #[test]
    fn if_expression() {
        assert_eq!(
            run_src("if 1 < 2 then 10 else 20"),
            parlang_core::Value::Int(10)
        );
    }

    /// Lambda application `(fun x -> x + 1) 41` evaluates to `42`.
    #[test]
    fn lambda_application() {
        assert_eq!(run_src("(fun x -> x + 1) 41"), parlang_core::Value::Int(42));
    }

    /// A `letrec`-based factorial of `5` evaluates to `120`.
    #[test]
    fn letrec_factorial() {
        let src = "let fact = rec f -> fun n -> if n == 0 then 1 else n * f (n - 1) in fact 5";
        assert_eq!(run_src(src), parlang_core::Value::Int(120));
    }

    /// A polymorphic `let id` is elaborated with `TyLam`/`TyApp` and runs.
    #[test]
    fn polymorphic_let_core_text() {
        let src = "let id = fun x -> x in if id true then id 1 else 0";
        assert_eq!(run_src(src), parlang_core::Value::Int(1));
        let e = elab_src(src).expect("elaborate");
        let text = e.term.to_text();
        assert!(text.contains("@Int"), "{text}");
        assert!(text.contains("@Bool"), "{text}");
        assert!(text.contains("/\\"), "{text}");
    }

    /// A user `data` declaration is registered and the body still runs.
    #[test]
    fn user_data_declaration_registers() {
        assert_eq!(
            run_src("data Color = Red | Green;\n1"),
            parlang_core::Value::Int(1)
        );
    }

    /// Declarations without a trailing body elaborate to `0 : Int`.
    #[test]
    fn decls_without_body_yield_zero() {
        let e = elab_src("let x = 5;\nlet y = x + 1;").expect("elaborate");
        assert_eq!(check_elaborated(&e), Ok(CoreType::Base(BaseType::Int)));
    }

    /// A residual, never-instantiated type variable defaults to `Unit`.
    #[test]
    fn residual_type_var_defaults_to_unit() {
        let e = elab_src("let id = fun x -> x in id").expect("elaborate");
        assert_eq!(
            check_elaborated(&e),
            Ok(CoreType::Fun(
                Box::new(CoreType::Base(BaseType::Unit)),
                Box::new(CoreType::Base(BaseType::Unit))
            ))
        );
    }

    /// `load` is rejected with `ElabError::Unsupported` naming the construct.
    #[test]
    fn load_is_unsupported() {
        assert!(matches!(
            elab_src("load \"x.par\" in 1"),
            Err(ElabError::Unsupported { ref construct, .. }) if construct == "load"
        ));
    }

    /// A polymorphic `id` used at two types builds a `(,)` tuple via `TyApp`.
    #[test]
    fn polymorphic_let_tuple() {
        let src = "let id = fun x -> x in (id 1, id true)";
        assert_eq!(
            run_src(src),
            parlang_core::Value::Con(
                "(,)".to_string(),
                vec![parlang_core::Value::Int(1), parlang_core::Value::Bool(true)]
            )
        );
        let text = elab_src(src).expect("elaborate").term.to_text();
        assert!(text.contains("@Int"), "{text}");
        assert!(text.contains("/\\"), "{text}");
    }

    /// Tuple projection `(10, 20, 30).1` selects the middle component.
    #[test]
    fn tuple_projection() {
        assert_eq!(run_src("(10, 20, 30).1"), parlang_core::Value::Int(20));
    }

    /// A `let`-bound record literal supports field access.
    #[test]
    fn record_construction_and_access() {
        assert_eq!(
            run_src("let r = {name: 1, age: 2} in r.age"),
            parlang_core::Value::Int(2)
        );
    }

    /// A record value is a `{age,name}` constructor with fields in sorted order.
    #[test]
    fn record_value_sorted_fields() {
        assert_eq!(
            run_src("{name: 1, age: 2}"),
            parlang_core::Value::Con(
                "{age,name}".to_string(),
                vec![parlang_core::Value::Int(2), parlang_core::Value::Int(1)]
            )
        );
    }

    /// A user data constructor elaborates with its type argument.
    #[test]
    fn data_constructor_with_type_args() {
        assert_eq!(
            run_src("data Option a = Some a | None;\nSome 5"),
            parlang_core::Value::Con("Some".to_string(), vec![parlang_core::Value::Int(5)])
        );
    }

    /// A nullary constructor's residual type argument defaults to `Unit` and lints.
    #[test]
    fn nullary_constructor_defaults_type_arg() {
        assert_eq!(
            run_src("data Option a = Some a | None;\nNone"),
            parlang_core::Value::Con("None".to_string(), vec![])
        );
    }

    /// A string literal elaborates to a `List Char` built from `Cons`/`Nil`.
    #[test]
    fn string_literal_is_char_list() {
        use parlang_core::Value as V;
        let nil = V::Con("Nil".to_string(), vec![]);
        let b = V::Con("Cons".to_string(), vec![V::Char('b'), nil]);
        let ab = V::Con("Cons".to_string(), vec![V::Char('a'), b]);
        assert_eq!(run_src("data List a = Nil | Cons a (List a);\n\"ab\""), ab);
    }

    /// A binding with an `Eq` constraint needs dictionary passing, so
    /// elaboration rejects it instead of dropping the constraint.
    #[test]
    fn constrained_binding_is_unsupported() {
        assert!(matches!(
            elab_src("let f = fun x -> fun y -> x == y;\n0"),
            Err(ElabError::Unsupported { ref construct, .. }) if construct == "constrained binding"
        ));
        assert!(matches!(
            elab_src("let g = fun x -> fun y -> x == y in 0"),
            Err(ElabError::Unsupported { ref construct, .. }) if construct == "constrained binding"
        ));
    }

    /// Declarations of the higher-kinded example `hkt.par`.
    const HKT_DECLS: &str = "data Option a = None | Some a;\n\
        data List a = Nil | Cons a (List a);\n\
        data Wrap f a = MkWrap (f a);\n\
        let unwrap = fun w -> match w with | MkWrap x -> x;\n";

    /// `unwrap (MkWrap (Some 1))` runs on Core to `Some(1)`.
    #[test]
    fn hkt_unwrap_runs_on_core() {
        use parlang_core::Value as V;
        let src = format!("{HKT_DECLS}unwrap (MkWrap (Some 1))");
        assert_eq!(run_src(&src), V::Con("Some".to_string(), vec![V::Int(1)]));
    }

    /// `Wrap` instantiated at the partial `List` runs on Core.
    #[test]
    fn hkt_wrap_list_runs_on_core() {
        use parlang_core::Value as V;
        let src = format!("{HKT_DECLS}MkWrap (Cons 1 Nil)");
        let nil = V::Con("Nil".to_string(), vec![]);
        let list = V::Con("Cons".to_string(), vec![V::Int(1), nil]);
        assert_eq!(run_src(&src), V::Con("MkWrap".to_string(), vec![list]));
    }

    /// `Wrap` is registered with its inferred parameter kinds.
    #[test]
    fn hkt_data_decl_registers_param_kinds() {
        use parlang_core::Kind;
        let e = elab_src(&format!("{HKT_DECLS}0")).expect("elaborate");
        let wrap = e.env.data_type("Wrap").expect("Wrap registered");
        let expected = vec![Kind::arrow(Kind::Star, Kind::Star), Kind::Star];
        assert_eq!(wrap.param_kinds, expected);
    }

    /// A higher-kinded parameter passed on through `Wrap f Int` (no `f a`
    /// in the payload) gets kind `* -> *` and runs on Core.
    #[test]
    fn hkt_parameter_through_data_application() {
        use parlang_core::{Kind, Value as V};
        let decls = format!("{HKT_DECLS}data T f = MkT (Wrap f Int);\n");
        let e = elab_src(&format!("{decls}0")).expect("elaborate");
        let t = e.env.data_type("T").expect("T registered");
        assert_eq!(t.param_kinds, vec![Kind::arrow(Kind::Star, Kind::Star)]);
        let some = V::Con("Some".to_string(), vec![V::Int(1)]);
        let wrapped = V::Con("MkWrap".to_string(), vec![some]);
        assert_eq!(
            run_src(&format!("{decls}MkT (MkWrap (Some 1))")),
            V::Con("MkT".to_string(), vec![wrapped])
        );
    }

    /// A recursive higher-kinded field `f (Fix f)` runs on Core.
    #[test]
    fn hkt_fix_runs_on_core() {
        use parlang_core::Value as V;
        let src = format!("{HKT_DECLS}data Fix f = In (f (Fix f));\nIn (Some (In None))");
        let none = V::Con("None".to_string(), vec![]);
        let inner = V::Con("In".to_string(), vec![none]);
        let some = V::Con("Some".to_string(), vec![inner]);
        assert_eq!(run_src(&src), V::Con("In".to_string(), vec![some]));
    }

    /// `unwrap` binds its head variable with a `* -> *` `TyLam`.
    #[test]
    fn hkt_unwrap_tylam_is_higher_kinded() {
        let e = elab_src(&format!("{HKT_DECLS}0")).expect("elaborate");
        let text = e.term.to_text();
        assert!(text.contains(" : * -> *). "), "{text}");
    }

    /// A partially applied data type converts; over-application is rejected.
    #[test]
    fn partial_sum_type_converts_and_over_application_is_unsupported() {
        let mut elab = Elab::default();
        let ctors = vec![(
            "Some".to_string(),
            vec![TypeAnnotation::Var("a".to_string())],
        )];
        let params = vec!["a".to_string()];
        let kinds = vec![parlang_core::Kind::Star];
        elab.data_decl("Option", &params, &kinds, &ctors)
            .expect("register Option");
        let partial = Type::SumType("Option".to_string(), vec![]);
        assert_eq!(
            elab.conv_type(&partial),
            Ok(CoreType::Con("Option".to_string()))
        );
        let over = Type::SumType("Option".to_string(), vec![Type::Int, Type::Int]);
        assert!(matches!(
            elab.conv_type(&over),
            Err(ElabError::Unsupported { .. })
        ));
    }

    /// An applied type variable converts in scope and is rejected out of scope.
    #[test]
    fn var_app_in_scope_converts_and_residual_is_unsupported() {
        let mut elab = Elab::default();
        let applied = Type::VarApp(TypeVar(7), vec![Type::Int]);
        assert!(matches!(
            elab.conv_type(&applied),
            Err(ElabError::Unsupported { ref reason, .. }) if reason.contains("residual")
        ));
        elab.scope.push(7);
        let expected = CoreType::App(
            Box::new(CoreType::Var("t7".to_string())),
            Box::new(CoreType::Base(BaseType::Int)),
        );
        assert_eq!(elab.conv_type(&applied), Ok(expected));
    }

    /// A never-instantiated higher-kinded use is an error, never silent.
    #[test]
    fn residual_higher_kinded_use_is_an_error() {
        let src = format!("{HKT_DECLS}(fun w -> 0) unwrap");
        let outcome = elab_src(&src).and_then(|e| check_elaborated(&e));
        assert!(outcome.is_err(), "{outcome:?}");
    }
}
