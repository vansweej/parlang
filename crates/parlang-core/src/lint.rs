//! Core Lint: a minimal synthesis-only type checker (type well-formedness
//! plus type synthesis, with no separate checking mode) for the explicitly
//! typed Core `Term` IR.
//!
//! This module performs no inference: every `Term` node either carries
//! enough annotation to synthesize its type directly, or its type is
//! determined by its children's synthesized types. See
//! `docs/adr/0007-core-explicit-typing-and-lint.md` for the design rationale.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use crate::term::{Alt, Term};
use crate::ty::Type;

/// A single data constructor: its name and the types of its fields.
///
/// Field types may mention the owning [`DataType`]'s parameters as
/// `Type::Var`.
#[derive(Debug, Clone, PartialEq)]
pub struct DataCon {
    /// The constructor's name, e.g. `"Cons"`.
    pub name: String,
    /// The constructor's field types, in order.
    pub fields: Vec<Type>,
}

/// A data type declaration: its name, type parameters, and constructors.
#[derive(Debug, Clone, PartialEq)]
pub struct DataType {
    /// The type's name, e.g. `"List"`.
    pub name: String,
    /// The type's parameters, e.g. `["a"]` for `List a`.
    pub params: Vec<String>,
    /// The type's constructors.
    pub ctors: Vec<DataCon>,
}

/// The environment Core Lint checks a term against: in-scope data types,
/// the constructor-to-type-name index, the term-variable context stack,
/// and the in-scope type-variable stack.
#[derive(Debug, Clone, Default)]
pub struct LintEnv {
    /// Known data types, keyed by type name.
    types: BTreeMap<String, DataType>,
    /// Index from constructor name to the name of its owning data type.
    ctor_owner: BTreeMap<String, String>,
    /// Term-variable context stack: `(name, type)` pairs, most recent last.
    terms: Vec<(String, Type)>,
    /// In-scope type-variable stack, most recent last.
    tyvars: Vec<String>,
}

impl LintEnv {
    /// Creates an empty environment.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates and registers a data type declaration, returning the
    /// updated environment.
    ///
    /// Unlike [`LintEnv::with_data`], which trusts its input, this checks
    /// that the type name is not already registered, that the type
    /// parameters are pairwise distinct, that every constructor name is
    /// unique both within the declaration and across already-registered
    /// types, and that every field type is well-formed with exactly the
    /// declaration's parameters in scope as type variables. The declared
    /// type itself is in scope while its field types are checked, so
    /// recursive types such as `List a` are accepted.
    ///
    /// # Errors
    ///
    /// Returns [`LintError::DuplicateDataType`],
    /// [`LintError::DuplicateTypeParam`], or [`LintError::DuplicateDataCon`]
    /// for the corresponding name clash, or the first well-formedness error
    /// (`UnboundTypeVar`, `UnknownType`, `ArityMismatch`, `ShadowedTypeVar`)
    /// found in a field type.
    pub fn try_with_data(self, data: DataType) -> Result<Self, LintError> {
        if self.types.contains_key(&data.name) {
            return Err(LintError::DuplicateDataType { name: data.name });
        }
        for (index, param) in data.params.iter().enumerate() {
            if data.params[..index].contains(param) {
                return Err(LintError::DuplicateTypeParam {
                    ty: data.name.clone(),
                    param: param.clone(),
                });
            }
        }
        for (index, ctor) in data.ctors.iter().enumerate() {
            let repeated = data.ctors[..index]
                .iter()
                .any(|earlier| earlier.name == ctor.name);
            if repeated || self.ctor_owner.contains_key(&ctor.name) {
                return Err(LintError::DuplicateDataCon {
                    ctor: ctor.name.clone(),
                });
            }
        }
        let mut scratch = Self {
            types: self.types.clone(),
            tyvars: data.params.clone(),
            ..Self::default()
        };
        scratch.types.insert(data.name.clone(), data.clone());
        for ctor in &data.ctors {
            for field in &ctor.fields {
                well_formed(&mut scratch, field)?;
            }
        }
        Ok(self.with_data(data))
    }

    /// Registers a data type declaration WITHOUT validating it, returning
    /// the updated environment.
    ///
    /// This trusts its input: a duplicate type or constructor name silently
    /// replaces the earlier registration, and field types are not checked.
    /// Use [`LintEnv::try_with_data`] for declarations not already known to
    /// be well-formed, such as user data declarations.
    #[must_use]
    pub fn with_data(mut self, dt: DataType) -> Self {
        for ctor in &dt.ctors {
            self.ctor_owner.insert(ctor.name.clone(), dt.name.clone());
        }
        self.types.insert(dt.name.clone(), dt);
        self
    }

    /// Binds a free term variable to a type, returning the updated
    /// environment. Useful for linting open terms, e.g. code referring to
    /// prelude bindings such as `not`.
    #[must_use]
    pub fn with_term(mut self, name: &str, ty: Type) -> Self {
        self.terms.push((name.to_string(), ty));
        self
    }

    /// Looks up a registered data type by its type name, for example `List`.
    #[must_use]
    pub fn data_type(&self, name: &str) -> Option<&DataType> {
        self.types.get(name)
    }

    /// Looks up the name of the data type that declares constructor `ctor`,
    /// for example `List` for `Cons`.
    #[must_use]
    pub fn owner_of(&self, ctor: &str) -> Option<&str> {
        self.ctor_owner.get(ctor).map(String::as_str)
    }

    /// Pushes a term-variable binding onto the context stack.
    fn push_term(&mut self, name: &str, ty: Type) {
        self.terms.push((name.to_string(), ty));
    }

    /// Pops the most recently pushed term-variable binding.
    fn pop_term(&mut self) {
        self.terms.pop();
    }

    /// Pushes a type variable onto the in-scope stack.
    fn push_tyvar(&mut self, name: &str) {
        self.tyvars.push(name.to_string());
    }

    /// Pops the most recently pushed type variable.
    fn pop_tyvar(&mut self) {
        self.tyvars.pop();
    }
}

/// An error raised while Core Lint checks a term.
#[derive(Debug, Clone, PartialEq)]
pub enum LintError {
    /// A term variable was referenced but not bound in scope.
    UnboundTermVar(String),
    /// A type variable was referenced but not bound in scope.
    UnboundTypeVar(String),
    /// A `Forall` or `TyLam` binder shadowed an already in-scope type variable.
    ShadowedTypeVar(String),
    /// A type constructor name was not a known data type.
    UnknownType(String),
    /// A constructor name was not registered in the environment.
    UnknownConstructor(String),
    /// An application's callee did not synthesize a function type.
    NotAFunction(Type),
    /// An application argument's type did not match the expected domain.
    ArgTypeMismatch {
        /// The expected argument type.
        expected: Type,
        /// The synthesized argument type.
        found: Type,
    },
    /// A constructor application supplied the wrong number of type arguments.
    TypeArgCountMismatch {
        /// The constructor name.
        ctor: String,
        /// The expected number of type arguments.
        expected: usize,
        /// The number of type arguments found.
        found: usize,
    },
    /// A constructor application supplied the wrong number of fields.
    FieldCountMismatch {
        /// The constructor name.
        ctor: String,
        /// The expected number of fields.
        expected: usize,
        /// The number of fields found.
        found: usize,
    },
    /// A type-constructor application supplied the wrong number of arguments.
    ArityMismatch {
        /// The type constructor name.
        tycon: String,
        /// The expected arity.
        expected: usize,
        /// The arity found.
        found: usize,
    },
    /// A reserved primitive constructor was given explicit type arguments.
    ReservedPrimWithTypeArgs(String),
    /// A primitive received the wrong number of value arguments.
    PrimArgCount {
        /// The primitive's name.
        prim: String,
        /// The expected argument count.
        expected: usize,
        /// The argument count found.
        found: usize,
    },
    /// A primitive argument had an unexpected type.
    PrimArgMismatch {
        /// The primitive's name.
        prim: String,
        /// The expected argument type.
        expected: Type,
        /// The synthesized argument type.
        found: Type,
    },
    /// A type application's callee did not synthesize a `Forall` type.
    NotAForall(Type),
    /// A case scrutinee's type did not resolve to a known data type.
    NotADataType(Type),
    /// A `Case` term did not cover all of the scrutinee type's constructors.
    NonExhaustiveCase {
        /// The names of the constructors missing an alternative.
        missing: Vec<String>,
    },
    /// A `Case` term had more than one alternative for the same constructor.
    DuplicateAlt(String),
    /// A `Case` alternative named a constructor from a different data type.
    ForeignAlt {
        /// The constructor name.
        ctor: String,
        /// The scrutinee's actual data type name.
        data_type: String,
    },
    /// A `Case` alternative bound the wrong number of fields.
    BinderCountMismatch {
        /// The constructor name.
        ctor: String,
        /// The expected number of binders.
        expected: usize,
        /// The number of binders found.
        found: usize,
    },
    /// A `Case` term had no alternatives.
    EmptyCase,
    /// Two branches of a conditional construct had different types.
    BranchTypeMismatch {
        /// The first branch's type.
        expected: Type,
        /// The mismatching branch's type.
        found: Type,
    },
    /// A `LetRec`'s bound value was not a `Lam`.
    LetRecValueNotLam,
    /// A `LetRec`'s declared type did not match its value's synthesized type.
    LetRecAnnotationMismatch {
        /// The declared (annotation) type.
        expected: Type,
        /// The value's synthesized type.
        found: Type,
    },
    /// `eq` was applied to a non-base type.
    NonBaseEq(Type),
    /// `eq` was applied to two different base types.
    EqTypeMismatch {
        /// The left operand's type.
        left: Type,
        /// The right operand's type.
        right: Type,
    },
    /// `LintEnv::try_with_data` was given a data type whose name is already
    /// registered.
    DuplicateDataType {
        /// The data type name.
        name: String,
    },
    /// A data type declaration listed the same type parameter more than once.
    DuplicateTypeParam {
        /// The data type name.
        ty: String,
        /// The repeated parameter name.
        param: String,
    },
    /// A data constructor name was repeated within one declaration, or is
    /// already owned by a registered data type.
    DuplicateDataCon {
        /// The constructor name.
        ctor: String,
    },
}

impl Display for LintError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnboundTermVar(n) => write!(f, "unbound term variable: {n}"),
            Self::UnboundTypeVar(n) => write!(f, "unbound type variable: {n}"),
            Self::ShadowedTypeVar(n) => write!(f, "shadowed type variable: {n}"),
            Self::UnknownType(n) => write!(f, "unknown type: {n}"),
            Self::UnknownConstructor(n) => write!(f, "unknown constructor: {n}"),
            Self::NotAFunction(t) => write!(f, "not a function: {t}"),
            Self::ArgTypeMismatch { expected, found } => {
                write!(
                    f,
                    "argument type mismatch: expected {expected}, found {found}"
                )
            }
            Self::TypeArgCountMismatch {
                ctor,
                expected,
                found,
            } => write!(
                f,
                "constructor {ctor}: expected {expected} type arguments, found {found}"
            ),
            Self::FieldCountMismatch {
                ctor,
                expected,
                found,
            } => write!(
                f,
                "constructor {ctor}: expected {expected} fields, found {found}"
            ),
            Self::ArityMismatch {
                tycon,
                expected,
                found,
            } => write!(
                f,
                "type constructor {tycon}: expected {expected} arguments, found {found}"
            ),
            Self::ReservedPrimWithTypeArgs(n) => {
                write!(f, "reserved primitive {n} given explicit type arguments")
            }
            Self::PrimArgCount {
                prim,
                expected,
                found,
            } => write!(
                f,
                "primitive {prim}: expected {expected} arguments, found {found}"
            ),
            Self::PrimArgMismatch {
                prim,
                expected,
                found,
            } => {
                write!(
                    f,
                    "primitive {prim}: argument mismatch: expected {expected}, found {found}"
                )
            }
            Self::NotAForall(t) => write!(f, "not a forall: {t}"),
            Self::NotADataType(t) => write!(f, "not a data type: {t}"),
            Self::NonExhaustiveCase { missing } => {
                write!(f, "non-exhaustive case, missing: {}", missing.join(", "))
            }
            Self::DuplicateAlt(c) => write!(f, "duplicate alternative for constructor: {c}"),
            Self::ForeignAlt { ctor, data_type } => {
                write!(f, "constructor {ctor} does not belong to type {data_type}")
            }
            Self::BinderCountMismatch {
                ctor,
                expected,
                found,
            } => write!(
                f,
                "alternative for {ctor}: expected {expected} binders, found {found}"
            ),
            Self::EmptyCase => write!(f, "case expression has no alternatives"),
            Self::BranchTypeMismatch { expected, found } => {
                write!(
                    f,
                    "branch type mismatch: expected {expected}, found {found}"
                )
            }
            Self::LetRecValueNotLam => write!(f, "letrec value must be a lambda"),
            Self::LetRecAnnotationMismatch { expected, found } => write!(
                f,
                "letrec annotation mismatch: expected {expected}, found {found}"
            ),
            Self::NonBaseEq(t) => write!(f, "eq applied to non-base type: {t}"),
            Self::EqTypeMismatch { left, right } => {
                write!(f, "eq type mismatch: {left} vs {right}")
            }
            Self::DuplicateDataType { name } => write!(f, "duplicate data type: {name}"),
            Self::DuplicateTypeParam { ty, param } => {
                write!(f, "data type {ty}: duplicate type parameter {param}")
            }
            Self::DuplicateDataCon { ctor } => write!(f, "duplicate data constructor: {ctor}"),
        }
    }
}

impl std::error::Error for LintError {}

/// Flattens a left-nested `App` spine into its head and ordered arguments.
fn spine(ty: &Type) -> (&Type, Vec<&Type>) {
    let mut args = Vec::new();
    let mut head = ty;
    while let Type::App(f, a) = head {
        args.push(a.as_ref());
        head = f;
    }
    args.reverse();
    (head, args)
}

/// Checks that `ty` is well-formed under `env`: every free type variable is
/// in scope, and every type-constructor application has the correct arity.
///
/// # Errors
///
/// Returns a [`LintError`] describing the first well-formedness violation.
fn well_formed(env: &mut LintEnv, ty: &Type) -> Result<(), LintError> {
    match ty {
        Type::Base(_) => Ok(()),
        Type::Var(n) => {
            if env.tyvars.contains(n) {
                Ok(())
            } else {
                Err(LintError::UnboundTypeVar(n.clone()))
            }
        }
        Type::Fun(a, b) => {
            well_formed(env, a)?;
            well_formed(env, b)
        }
        Type::Forall(a, body) => {
            if env.tyvars.contains(a) {
                return Err(LintError::ShadowedTypeVar(a.clone()));
            }
            env.push_tyvar(a);
            let result = well_formed(env, body);
            env.pop_tyvar();
            result
        }
        Type::Con(_) | Type::App(_, _) => {
            let (head, args) = spine(ty);
            let Type::Con(name) = head else {
                // Core currently has only kind `*`, so application heads must
                // be registered type constructors rather than type variables
                // or other kinds of type.
                return Err(LintError::ArityMismatch {
                    tycon: head.to_string(),
                    expected: 0,
                    found: args.len(),
                });
            };
            let dt = env
                .types
                .get(name)
                .cloned()
                .ok_or_else(|| LintError::UnknownType(name.clone()))?;
            if args.len() != dt.params.len() {
                return Err(LintError::ArityMismatch {
                    tycon: name.clone(),
                    expected: dt.params.len(),
                    found: args.len(),
                });
            }
            for arg in args {
                well_formed(env, arg)?;
            }
            Ok(())
        }
    }
}

/// Synthesizes the type of `term` under `env`.
///
/// # Errors
///
/// Returns the first [`LintError`] encountered.
fn synth(env: &mut LintEnv, term: &Term) -> Result<Type, LintError> {
    match term {
        Term::Var(n) => env
            .terms
            .iter()
            .rev()
            .find(|(name, _)| name == n)
            .map(|(_, ty)| ty.clone())
            .ok_or_else(|| LintError::UnboundTermVar(n.clone())),
        Term::Lit(l) => Ok(Type::Base(l.base_type())),
        Term::Lam(x, ty, body) => {
            well_formed(env, ty)?;
            env.push_term(x, ty.clone());
            let result = synth(env, body);
            env.pop_term();
            let tb = result?;
            Ok(Type::Fun(Box::new(ty.clone()), Box::new(tb)))
        }
        Term::App(f, a) => synth_app(env, f, a),
        Term::Let(x, v, b) => synth_let(env, x, v, b),
        Term::LetRec(x, ann, v, b) => synth_letrec(env, x, ann, v, b),
        Term::TyLam(a, t) => synth_tylam(env, a, t),
        Term::TyApp(t, ty) => synth_tyapp(env, t, ty),
        Term::Con(name, ty_args, args) => synth_con(env, name, ty_args, args),
        Term::Case(scrutinee, alts) => synth_case(env, scrutinee, alts),
    }
}

fn synth_app(env: &mut LintEnv, f: &Term, a: &Term) -> Result<Type, LintError> {
    let tf = synth(env, f)?;
    let Type::Fun(dom, cod) = tf else {
        return Err(LintError::NotAFunction(tf));
    };
    let ta = synth(env, a)?;
    if !ta.alpha_eq(&dom) {
        return Err(LintError::ArgTypeMismatch {
            expected: *dom,
            found: ta,
        });
    }
    Ok(*cod)
}

fn synth_let(env: &mut LintEnv, x: &str, v: &Term, b: &Term) -> Result<Type, LintError> {
    let tv = synth(env, v)?;
    env.push_term(x, tv);
    let result = synth(env, b);
    env.pop_term();
    result
}

fn synth_letrec(
    env: &mut LintEnv,
    x: &str,
    ann: &Type,
    v: &Term,
    b: &Term,
) -> Result<Type, LintError> {
    well_formed(env, ann)?;
    if !matches!(v, Term::Lam(..)) {
        return Err(LintError::LetRecValueNotLam);
    }
    env.push_term(x, ann.clone());
    let result = (|| {
        let tv = synth(env, v)?;
        if !tv.alpha_eq(ann) {
            return Err(LintError::LetRecAnnotationMismatch {
                expected: ann.clone(),
                found: tv,
            });
        }
        synth(env, b)
    })();
    env.pop_term();
    result
}

fn synth_tylam(env: &mut LintEnv, a: &str, t: &Term) -> Result<Type, LintError> {
    if env.tyvars.contains(&a.to_string()) {
        return Err(LintError::ShadowedTypeVar(a.to_string()));
    }
    env.push_tyvar(a);
    let result = synth(env, t);
    env.pop_tyvar();
    let tt = result?;
    Ok(Type::Forall(a.to_string(), Box::new(tt)))
}

fn synth_tyapp(env: &mut LintEnv, t: &Term, ty: &Type) -> Result<Type, LintError> {
    let tt = synth(env, t)?;
    let Type::Forall(a, body) = tt else {
        return Err(LintError::NotAForall(tt));
    };
    well_formed(env, ty)?;
    Ok(body.subst(&a, ty))
}

const RESERVED_PRIMS: &[&str] = &["if", "+", "-", "*", "/", "<", "eq"];

fn synth_con(
    env: &mut LintEnv,
    name: &str,
    ty_args: &[Type],
    args: &[Term],
) -> Result<Type, LintError> {
    if RESERVED_PRIMS.contains(&name) {
        if !ty_args.is_empty() {
            return Err(LintError::ReservedPrimWithTypeArgs(name.to_string()));
        }
        synth_prim(env, name, args)
    } else {
        synth_data_con(env, name, ty_args, args)
    }
}

fn synth_prim(env: &mut LintEnv, name: &str, args: &[Term]) -> Result<Type, LintError> {
    match name {
        "if" => {
            if args.len() != 3 {
                return Err(LintError::PrimArgCount {
                    prim: name.to_string(),
                    expected: 3,
                    found: args.len(),
                });
            }
            let tc = synth(env, &args[0])?;
            if tc != Type::Base(crate::base_type::BaseType::Bool) {
                return Err(LintError::PrimArgMismatch {
                    prim: name.to_string(),
                    expected: Type::Base(crate::base_type::BaseType::Bool),
                    found: tc,
                });
            }
            let t1 = synth(env, &args[1])?;
            let t2 = synth(env, &args[2])?;
            if !t1.alpha_eq(&t2) {
                return Err(LintError::BranchTypeMismatch {
                    expected: t1,
                    found: t2,
                });
            }
            Ok(t1)
        }
        "+" | "-" | "*" | "/" => synth_arith(env, name, args),
        "<" => synth_lt(env, name, args),
        "eq" => {
            if args.len() != 2 {
                return Err(LintError::PrimArgCount {
                    prim: name.to_string(),
                    expected: 2,
                    found: args.len(),
                });
            }
            let t0 = synth(env, &args[0])?;
            let t1 = synth(env, &args[1])?;
            if !matches!(t0, Type::Base(_)) {
                return Err(LintError::NonBaseEq(t0));
            }
            if !matches!(t1, Type::Base(_)) {
                return Err(LintError::NonBaseEq(t1));
            }
            if t0 != t1 {
                return Err(LintError::EqTypeMismatch {
                    left: t0,
                    right: t1,
                });
            }
            Ok(Type::Base(crate::base_type::BaseType::Bool))
        }
        _ => Err(LintError::UnknownConstructor(name.to_string())),
    }
}

/// Types `+`, `-`, `*`, `/`: both operands `Int` or both `Float`; the result
/// is the operand type. On mismatch, `expected` is the first operand's type
/// when it is numeric, else `Int`.
fn synth_arith(env: &mut LintEnv, name: &str, args: &[Term]) -> Result<Type, LintError> {
    if args.len() != 2 {
        return Err(LintError::PrimArgCount {
            prim: name.to_string(),
            expected: 2,
            found: args.len(),
        });
    }
    let t0 = synth(env, &args[0])?;
    let t1 = synth(env, &args[1])?;
    let numeric = matches!(
        t0,
        Type::Base(crate::base_type::BaseType::Int | crate::base_type::BaseType::Float)
    );
    if !numeric {
        return Err(LintError::PrimArgMismatch {
            prim: name.to_string(),
            expected: Type::Base(crate::base_type::BaseType::Int),
            found: t0,
        });
    }
    if t1 != t0 {
        return Err(LintError::PrimArgMismatch {
            prim: name.to_string(),
            expected: t0,
            found: t1,
        });
    }
    Ok(t0)
}

/// Types `<`: both operands `Int`, both `Char`, or both `Float`; the result
/// is `Bool`.
fn synth_lt(env: &mut LintEnv, name: &str, args: &[Term]) -> Result<Type, LintError> {
    if args.len() != 2 {
        return Err(LintError::PrimArgCount {
            prim: name.to_string(),
            expected: 2,
            found: args.len(),
        });
    }
    let t0 = synth(env, &args[0])?;
    let t1 = synth(env, &args[1])?;
    let ordered = matches!(
        t0,
        Type::Base(
            crate::base_type::BaseType::Int
                | crate::base_type::BaseType::Char
                | crate::base_type::BaseType::Float
        )
    );
    if !ordered || t1 != t0 {
        return Err(LintError::PrimArgMismatch {
            prim: name.to_string(),
            expected: t0,
            found: t1,
        });
    }
    Ok(Type::Base(crate::base_type::BaseType::Bool))
}

fn synth_data_con(
    env: &mut LintEnv,
    name: &str,
    ty_args: &[Type],
    args: &[Term],
) -> Result<Type, LintError> {
    let owner = env
        .ctor_owner
        .get(name)
        .cloned()
        .ok_or_else(|| LintError::UnknownConstructor(name.to_string()))?;
    let dt = env
        .types
        .get(&owner)
        .cloned()
        .ok_or_else(|| LintError::UnknownType(owner.clone()))?;

    if ty_args.len() != dt.params.len() {
        return Err(LintError::TypeArgCountMismatch {
            ctor: name.to_string(),
            expected: dt.params.len(),
            found: ty_args.len(),
        });
    }
    for ty_arg in ty_args {
        well_formed(env, ty_arg)?;
    }

    let dc = dt
        .ctors
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| LintError::UnknownConstructor(name.to_string()))?;

    if args.len() != dc.fields.len() {
        return Err(LintError::FieldCountMismatch {
            ctor: name.to_string(),
            expected: dc.fields.len(),
            found: args.len(),
        });
    }

    for (field, arg) in dc.fields.iter().zip(args) {
        let field_substituted = substitute_params(&dt.params, ty_args, field);
        let arg_ty = synth(env, arg)?;
        if !arg_ty.alpha_eq(&field_substituted) {
            return Err(LintError::ArgTypeMismatch {
                expected: field_substituted,
                found: arg_ty,
            });
        }
    }

    let head = Type::Con(dt.name.clone());
    Ok(ty_args.iter().cloned().fold(head, |acc, ty_arg| {
        Type::App(Box::new(acc), Box::new(ty_arg))
    }))
}

/// Substitutes every `params[i] := ty_args[i]` into `ty` simultaneously, so a
/// type argument that mentions a parameter name is never rewritten again.
fn substitute_params(params: &[String], ty_args: &[Type], ty: &Type) -> Type {
    let mapping: Vec<(&str, &Type)> = params.iter().map(String::as_str).zip(ty_args).collect();
    ty.subst_many(&mapping)
}

fn synth_case(env: &mut LintEnv, scrutinee: &Term, alts: &[Alt]) -> Result<Type, LintError> {
    let (first_alt, remaining_alts) = alts.split_first().ok_or(LintError::EmptyCase)?;

    let scrut_ty = synth(env, scrutinee)?;
    let (head, ty_args_refs) = spine(&scrut_ty);
    let Type::Con(tyname) = head else {
        return Err(LintError::NotADataType(scrut_ty.clone()));
    };
    let dt = env
        .types
        .get(tyname)
        .cloned()
        .ok_or_else(|| LintError::NotADataType(scrut_ty.clone()))?;
    let ty_args: Vec<Type> = ty_args_refs.into_iter().cloned().collect();

    // Exhaustiveness / duplicate / foreign-ctor checks.
    let mut seen: Vec<&str> = Vec::new();
    for alt in alts {
        if seen.contains(&alt.con.as_str()) {
            return Err(LintError::DuplicateAlt(alt.con.clone()));
        }
        if !dt.ctors.iter().any(|c| c.name == alt.con) {
            return Err(LintError::ForeignAlt {
                ctor: alt.con.clone(),
                data_type: dt.name.clone(),
            });
        }
        seen.push(&alt.con);
    }
    let missing: Vec<String> = dt
        .ctors
        .iter()
        .filter(|c| !seen.contains(&c.name.as_str()))
        .map(|c| c.name.clone())
        .collect();
    if !missing.is_empty() {
        return Err(LintError::NonExhaustiveCase { missing });
    }

    let expected = synth_alt(env, &dt, &ty_args, first_alt)?;
    for alt in remaining_alts {
        let body_ty = synth_alt(env, &dt, &ty_args, alt)?;
        if !expected.alpha_eq(&body_ty) {
            return Err(LintError::BranchTypeMismatch {
                expected: expected.clone(),
                found: body_ty,
            });
        }
    }

    Ok(expected)
}

fn synth_alt(
    env: &mut LintEnv,
    dt: &DataType,
    ty_args: &[Type],
    alt: &Alt,
) -> Result<Type, LintError> {
    let dc = dt
        .ctors
        .iter()
        .find(|ctor| ctor.name == alt.con)
        .ok_or_else(|| LintError::UnknownConstructor(alt.con.clone()))?;
    if alt.binders.len() != dc.fields.len() {
        return Err(LintError::BinderCountMismatch {
            ctor: alt.con.clone(),
            expected: dc.fields.len(),
            found: alt.binders.len(),
        });
    }
    for (binder, field) in alt.binders.iter().zip(&dc.fields) {
        env.push_term(binder, substitute_params(&dt.params, ty_args, field));
    }
    let result = synth(env, &alt.body);
    for _ in &alt.binders {
        env.pop_term();
    }
    result
}

/// Checks `term` against `env`, returning its synthesized type.
///
/// # Errors
///
/// Returns the first [`LintError`] encountered while checking `term`.
pub fn lint(term: &Term, env: &LintEnv) -> Result<Type, LintError> {
    let mut local = env.clone();
    synth(&mut local, term)
}

/// Builds a [`LintEnv`] with the single `List` data type registered:
/// `data List a = Nil | Cons a (List a)`.
#[must_use]
pub fn prelude_env() -> LintEnv {
    let list = DataType {
        name: "List".to_string(),
        params: vec!["a".to_string()],
        ctors: vec![
            DataCon {
                name: "Nil".to_string(),
                fields: vec![],
            },
            DataCon {
                name: "Cons".to_string(),
                fields: vec![
                    Type::Var("a".to_string()),
                    Type::App(
                        Box::new(Type::Con("List".to_string())),
                        Box::new(Type::Var("a".to_string())),
                    ),
                ],
            },
        ],
    };
    LintEnv::new().with_data(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_type::BaseType;
    use crate::builder::{
        alt, app, bool_, case_, char_, con, con_at, fun_ty, int, lam, let_, letrec, tvar, tyapp,
        tylam, var,
    };

    #[test]
    fn lint_int_literal() {
        let env = LintEnv::new();
        let ty = lint(&int(42), &env).expect("int literal should lint");
        assert_eq!(ty, Type::Base(BaseType::Int));
    }

    #[test]
    fn lint_unbound_var() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = var("x")?;
        assert_eq!(
            lint(&term, &env),
            Err(LintError::UnboundTermVar("x".to_string()))
        );
        Ok(())
    }

    #[test]
    fn lint_identity_lambda() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = lam("x", BaseType::Int, var("x")?)?;
        let ty = lint(&term, &env).expect("identity should lint");
        assert_eq!(
            ty,
            Type::Fun(
                Box::new(Type::Base(BaseType::Int)),
                Box::new(Type::Base(BaseType::Int))
            )
        );
        Ok(())
    }

    #[test]
    fn app_of_identity_lints_to_int() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let id = lam("x", BaseType::Int, var("x")?)?;
        let term = app(id, int(5));
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Int)));
        Ok(())
    }

    #[test]
    fn app_with_wrong_arg_type_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let id = lam("x", BaseType::Int, var("x")?)?;
        let term = app(id, bool_(true));
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::ArgTypeMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn let_binding_lints_to_int() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = let_("x", int(5), var("x")?)?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Int)));
        Ok(())
    }

    fn factorial_term() -> Result<Term, Box<dyn std::error::Error>> {
        let body = con(
            "*",
            vec![
                var("n")?,
                app(var("fac")?, con("-", vec![var("n")?, int(1)])?),
            ],
        )?;
        let fac_lam = lam("n", BaseType::Int, body)?;
        Ok(letrec(
            "fac",
            fun_ty(Type::Base(BaseType::Int), Type::Base(BaseType::Int)),
            fac_lam,
            app(var("fac")?, int(3)),
        )?)
    }

    #[test]
    fn letrec_lints_to_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = factorial_term()?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Int)));
        Ok(())
    }

    #[test]
    fn letrec_wrong_annotation_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let fac_lam = lam("n", BaseType::Int, int(1))?;
        let term = letrec(
            "fac",
            fun_ty(Type::Base(BaseType::Int), Type::Base(BaseType::Bool)),
            fac_lam,
            var("fac")?,
        )?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::LetRecAnnotationMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn letrec_non_lambda_value_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = letrec("x", Type::Base(BaseType::Int), int(5), var("x")?)?;
        assert_eq!(lint(&term, &env), Err(LintError::LetRecValueNotLam));
        Ok(())
    }

    #[test]
    fn if_lints_to_int() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("if", vec![bool_(true), int(1), int(2)])?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Int)));
        Ok(())
    }

    #[test]
    fn if_branch_type_mismatch() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("if", vec![bool_(true), int(1), bool_(false)])?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::BranchTypeMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn if_non_bool_cond_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("if", vec![int(1), int(1), int(2)])?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::PrimArgMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn add_int_int_lints_to_int() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("+", vec![int(1), int(2)])?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Int)));
        Ok(())
    }

    #[test]
    fn add_int_bool_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("+", vec![int(1), bool_(true)])?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::PrimArgMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn prim_arg_mismatch_names_the_primitive() -> Result<(), Box<dyn std::error::Error>> {
        let term = con("+", vec![int(1), bool_(true)])?;
        assert!(matches!(
            lint(&term, &LintEnv::new()),
            Err(LintError::PrimArgMismatch { prim, .. }) if prim == "+"
        ));
        Ok(())
    }

    #[test]
    fn add_with_one_arg_errors_on_count() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("+", vec![int(1)])?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::PrimArgCount { .. })
        ));
        Ok(())
    }

    #[test]
    fn float_arith_lints_to_float() -> Result<(), Box<dyn std::error::Error>> {
        use crate::builder::float;
        for op in ["+", "-", "*", "/"] {
            let term = con(op, vec![float(1.5), float(2.0)])?;
            assert_eq!(
                lint(&term, &LintEnv::new()),
                Ok(Type::Base(BaseType::Float))
            );
        }
        Ok(())
    }

    #[test]
    fn mixed_int_float_arith_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        use crate::builder::float;
        let int_first = con("+", vec![int(1), float(2.0)])?;
        let float_first = con("*", vec![float(2.0), int(1)])?;
        assert!(matches!(
            lint(&int_first, &LintEnv::new()),
            Err(LintError::PrimArgMismatch { .. })
        ));
        assert!(matches!(
            lint(&float_first, &LintEnv::new()),
            Err(LintError::PrimArgMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn lt_float_float_lints_to_bool() -> Result<(), Box<dyn std::error::Error>> {
        use crate::builder::float;
        let term = con("<", vec![float(1.5), float(2.5)])?;
        assert_eq!(lint(&term, &LintEnv::new()), Ok(Type::Base(BaseType::Bool)));
        Ok(())
    }

    #[test]
    fn lt_mixed_int_float_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        use crate::builder::float;
        let term = con("<", vec![int(1), float(2.5)])?;
        assert!(matches!(
            lint(&term, &LintEnv::new()),
            Err(LintError::PrimArgMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn lt_char_char_lints_to_bool() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("<", vec![char_('a'), char_('b')])?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Bool)));
        Ok(())
    }

    #[test]
    fn eq_int_int_lints_to_bool() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("eq", vec![int(1), int(2)])?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Bool)));
        Ok(())
    }

    #[test]
    fn eq_int_bool_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con("eq", vec![int(1), bool_(true)])?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::EqTypeMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn eq_on_non_base_data_values_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let env = prelude_env();
        let nil1 = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let nil2 = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let term = con("eq", vec![nil1, nil2])?;
        assert!(matches!(lint(&term, &env), Err(LintError::NonBaseEq(_))));
        Ok(())
    }

    #[test]
    fn reserved_prim_with_type_args_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = con_at(
            "if",
            vec![Type::Base(BaseType::Int)],
            vec![bool_(true), int(1), int(2)],
        )?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::ReservedPrimWithTypeArgs(_))
        ));
        Ok(())
    }

    #[test]
    fn tylam_lints_to_forall() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = tylam("a", lam("x", tvar("a")?, var("x")?)?)?;
        let ty = lint(&term, &env)?;
        assert!(matches!(ty, Type::Forall(_, _)));
        Ok(())
    }

    #[test]
    fn tyapp_instantiates_forall() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let poly = tylam("a", lam("x", tvar("a")?, var("x")?)?)?;
        let term = tyapp(poly, Type::Base(BaseType::Int));
        assert_eq!(
            lint(&term, &env),
            Ok(Type::Fun(
                Box::new(Type::Base(BaseType::Int)),
                Box::new(Type::Base(BaseType::Int))
            ))
        );
        Ok(())
    }

    #[test]
    fn tyapp_on_non_forall_rejected() {
        let env = LintEnv::new();
        let term = tyapp(int(1), Type::Base(BaseType::Int));
        assert!(matches!(lint(&term, &env), Err(LintError::NotAForall(_))));
    }

    #[test]
    fn nested_tylam_shadowing_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new();
        let term = tylam("a", tylam("a", var("x")?)?)?;
        assert_eq!(
            lint(&term, &env),
            Err(LintError::ShadowedTypeVar("a".to_string()))
        );
        Ok(())
    }

    #[test]
    fn type_var_headed_application_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let ill_kinded_binder = Type::App(
            Box::new(Type::Var("a".to_string())),
            Box::new(Type::Base(BaseType::Int)),
        );
        let term = tylam("a", lam("x", ill_kinded_binder, int(1))?)?;
        assert!(matches!(
            lint(&term, &LintEnv::new()),
            Err(LintError::ArityMismatch { .. })
        ));
        Ok(())
    }

    fn list_env() -> LintEnv {
        let list = DataType {
            name: "List".to_string(),
            params: vec!["a".to_string()],
            ctors: vec![
                DataCon {
                    name: "Nil".to_string(),
                    fields: vec![],
                },
                DataCon {
                    name: "Cons".to_string(),
                    fields: vec![
                        Type::Var("a".to_string()),
                        Type::App(
                            Box::new(Type::Con("List".to_string())),
                            Box::new(Type::Var("a".to_string())),
                        ),
                    ],
                },
            ],
        };
        LintEnv::new().with_data(list)
    }

    #[test]
    fn con_lints_to_list_int() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let nil = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let term = con_at("Cons", vec![Type::Base(BaseType::Int)], vec![int(1), nil])?;
        let expected = Type::App(
            Box::new(Type::Con("List".to_string())),
            Box::new(Type::Base(BaseType::Int)),
        );
        assert_eq!(lint(&term, &env), Ok(expected));
        Ok(())
    }

    #[test]
    fn con_wrong_field_type_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let nil = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let term = con_at(
            "Cons",
            vec![Type::Base(BaseType::Int)],
            vec![bool_(true), nil],
        )?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::ArgTypeMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn con_wrong_type_arg_count_mismatches() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let nil = con_at("Nil", vec![], vec![])?;
        let term = con_at("Cons", vec![], vec![int(1), nil])?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::TypeArgCountMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn con_unknown_constructor_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let term = con_at("Foo", vec![Type::Base(BaseType::Int)], vec![int(1)])?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::UnknownConstructor(_))
        ));
        Ok(())
    }

    #[test]
    fn stale_ctor_owner_is_an_error_not_a_panic() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env().with_data(DataType {
            name: "List".to_string(),
            params: vec!["a".to_string()],
            ctors: vec![DataCon {
                name: "Empty".to_string(),
                fields: vec![],
            }],
        });
        let term = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::UnknownConstructor(_))
        ));
        Ok(())
    }

    fn list_int_case(nil_body: Term, cons_body: Term) -> Result<Term, Box<dyn std::error::Error>> {
        let nil = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let cons = con_at("Cons", vec![Type::Base(BaseType::Int)], vec![int(1), nil])?;
        let scrutinee = let_("xs", cons, var("xs")?)?;
        Ok(case_(
            scrutinee,
            vec![
                alt("Cons", &["h", "t"], cons_body)?,
                alt("Nil", &[], nil_body)?,
            ],
        ))
    }

    #[test]
    fn case_list_int_lints_to_int() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let term = list_int_case(int(0), var("h")?)?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Int)));
        Ok(())
    }

    #[test]
    fn case_missing_nil_alt_non_exhaustive() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let nil = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let cons = con_at("Cons", vec![Type::Base(BaseType::Int)], vec![int(1), nil])?;
        let scrutinee = let_("xs", cons, var("xs")?)?;
        let term = case_(scrutinee, vec![alt("Cons", &["h", "t"], var("h")?)?]);
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::NonExhaustiveCase { .. })
        ));
        Ok(())
    }

    #[test]
    fn case_foreign_alt_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let nil = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let cons = con_at("Cons", vec![Type::Base(BaseType::Int)], vec![int(1), nil])?;
        let scrutinee = let_("xs", cons, var("xs")?)?;
        let term = case_(
            scrutinee,
            vec![
                alt("Cons", &["h", "t"], var("h")?)?,
                alt("Foo", &[], int(0))?,
            ],
        );
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::ForeignAlt { .. })
        ));
        Ok(())
    }

    #[test]
    fn case_empty_alts_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let nil = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let term = case_(nil, vec![]);
        assert_eq!(lint(&term, &env), Err(LintError::EmptyCase));
        Ok(())
    }

    #[test]
    fn case_binder_count_mismatch_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let env = list_env();
        let nil = con_at("Nil", vec![Type::Base(BaseType::Int)], vec![])?;
        let cons = con_at(
            "Cons",
            vec![Type::Base(BaseType::Int)],
            vec![int(1), nil.clone()],
        )?;
        let scrutinee = let_("xs", cons, var("xs")?)?;
        let term = case_(
            scrutinee,
            vec![alt("Cons", &["only"], int(0))?, alt("Nil", &[], int(0))?],
        );
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::BinderCountMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn prelude_not_lints_to_bool_arrow() -> Result<(), Box<dyn std::error::Error>> {
        let not_term = lam(
            "b",
            BaseType::Bool,
            con("if", vec![var("b")?, bool_(false), bool_(true)])?,
        )?;
        assert_eq!(
            lint(&not_term, &LintEnv::new()),
            Ok(Type::Fun(
                Box::new(Type::Base(BaseType::Bool)),
                Box::new(Type::Base(BaseType::Bool))
            ))
        );
        Ok(())
    }

    fn list_decl() -> DataType {
        DataType {
            name: "List".to_string(),
            params: vec!["a".to_string()],
            ctors: vec![
                DataCon {
                    name: "Nil".to_string(),
                    fields: vec![],
                },
                DataCon {
                    name: "Cons".to_string(),
                    fields: vec![
                        Type::Var("a".to_string()),
                        Type::App(
                            Box::new(Type::Con("List".to_string())),
                            Box::new(Type::Var("a".to_string())),
                        ),
                    ],
                },
            ],
        }
    }

    fn single_ctor_decl(name: &str, params: &[&str], fields: Vec<Type>) -> DataType {
        DataType {
            name: name.to_string(),
            params: params.iter().map(|p| (*p).to_string()).collect(),
            ctors: vec![DataCon {
                name: format!("Mk{name}"),
                fields,
            }],
        }
    }

    #[test]
    fn try_with_data_accepts_recursive_list() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new().try_with_data(list_decl())?;
        let term = list_int_case(int(0), var("h")?)?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Int)));
        Ok(())
    }

    #[test]
    fn try_with_data_accepts_two_param_result() -> Result<(), Box<dyn std::error::Error>> {
        let result_decl = DataType {
            name: "Result".to_string(),
            params: vec!["a".to_string(), "b".to_string()],
            ctors: vec![
                DataCon {
                    name: "Ok".to_string(),
                    fields: vec![Type::Var("a".to_string())],
                },
                DataCon {
                    name: "Err".to_string(),
                    fields: vec![Type::Var("b".to_string())],
                },
            ],
        };
        let env = LintEnv::new().try_with_data(result_decl)?;
        let term = con_at(
            "Err",
            vec![Type::Base(BaseType::Int), Type::Base(BaseType::Bool)],
            vec![bool_(true)],
        )?;
        let expected = Type::App(
            Box::new(Type::App(
                Box::new(Type::Con("Result".to_string())),
                Box::new(Type::Base(BaseType::Int)),
            )),
            Box::new(Type::Base(BaseType::Bool)),
        );
        assert_eq!(lint(&term, &env), Ok(expected));
        Ok(())
    }

    #[test]
    fn try_with_data_accepts_prelude_env_content() -> Result<(), Box<dyn std::error::Error>> {
        let list = prelude_env()
            .types
            .get("List")
            .cloned()
            .ok_or("prelude_env must register List")?;
        assert!(LintEnv::new().try_with_data(list).is_ok());
        Ok(())
    }

    #[test]
    fn try_with_data_rejects_duplicate_type() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new().try_with_data(list_decl())?;
        assert_eq!(
            env.try_with_data(list_decl()).map(|_| ()),
            Err(LintError::DuplicateDataType {
                name: "List".to_string()
            })
        );
        Ok(())
    }

    #[test]
    fn try_with_data_rejects_duplicate_type_param() {
        let decl = single_ctor_decl(
            "Pair",
            &["a", "a"],
            vec![Type::Var("a".to_string()), Type::Var("a".to_string())],
        );
        assert_eq!(
            LintEnv::new().try_with_data(decl).map(|_| ()),
            Err(LintError::DuplicateTypeParam {
                ty: "Pair".to_string(),
                param: "a".to_string()
            })
        );
    }

    #[test]
    fn try_with_data_rejects_duplicate_ctor_within_decl() {
        let decl = DataType {
            name: "T".to_string(),
            params: vec![],
            ctors: vec![
                DataCon {
                    name: "A".to_string(),
                    fields: vec![],
                },
                DataCon {
                    name: "A".to_string(),
                    fields: vec![],
                },
            ],
        };
        assert_eq!(
            LintEnv::new().try_with_data(decl).map(|_| ()),
            Err(LintError::DuplicateDataCon {
                ctor: "A".to_string()
            })
        );
    }

    #[test]
    fn try_with_data_rejects_ctor_owned_by_other_type() -> Result<(), Box<dyn std::error::Error>> {
        let env = LintEnv::new().try_with_data(list_decl())?;
        let decl = DataType {
            name: "Other".to_string(),
            params: vec![],
            ctors: vec![DataCon {
                name: "Nil".to_string(),
                fields: vec![],
            }],
        };
        assert_eq!(
            env.try_with_data(decl).map(|_| ()),
            Err(LintError::DuplicateDataCon {
                ctor: "Nil".to_string()
            })
        );
        Ok(())
    }

    #[test]
    fn try_with_data_rejects_unbound_field_var() {
        let decl = single_ctor_decl("Box", &["a"], vec![Type::Var("b".to_string())]);
        assert_eq!(
            LintEnv::new().try_with_data(decl).map(|_| ()),
            Err(LintError::UnboundTypeVar("b".to_string()))
        );
    }

    #[test]
    fn try_with_data_rejects_unknown_field_type() {
        let decl = single_ctor_decl("Holder", &[], vec![Type::Con("Missing".to_string())]);
        assert_eq!(
            LintEnv::new().try_with_data(decl).map(|_| ()),
            Err(LintError::UnknownType("Missing".to_string()))
        );
    }

    #[test]
    fn try_with_data_rejects_field_arity_mismatch() {
        let decl = single_ctor_decl("Bad", &["a"], vec![Type::Con("Bad".to_string())]);
        assert_eq!(
            LintEnv::new().try_with_data(decl).map(|_| ()),
            Err(LintError::ArityMismatch {
                tycon: "Bad".to_string(),
                expected: 1,
                found: 0
            })
        );
    }

    #[test]
    fn registration_errors_display_their_names() {
        let dup_type = LintError::DuplicateDataType {
            name: "List".to_string(),
        };
        let dup_param = LintError::DuplicateTypeParam {
            ty: "Pair".to_string(),
            param: "a".to_string(),
        };
        let dup_ctor = LintError::DuplicateDataCon {
            ctor: "Nil".to_string(),
        };
        assert_eq!(dup_type.to_string(), "duplicate data type: List");
        assert_eq!(
            dup_param.to_string(),
            "data type Pair: duplicate type parameter a"
        );
        assert_eq!(dup_ctor.to_string(), "duplicate data constructor: Nil");
    }

    fn pair_decl() -> DataType {
        single_ctor_decl(
            "Pair",
            &["a", "b"],
            vec![Type::Var("a".to_string()), Type::Var("b".to_string())],
        )
    }

    fn pair_app(first: Type, second: Type) -> Type {
        Type::App(
            Box::new(Type::App(
                Box::new(Type::Con("Pair".to_string())),
                Box::new(first),
            )),
            Box::new(second),
        )
    }

    #[test]
    fn con_substitutes_type_params_simultaneously() -> Result<(), Box<dyn std::error::Error>> {
        // /\b. \x:b. MkPair @b @Int x 1  :  forall b. b -> Pair b Int
        let env = LintEnv::new().try_with_data(pair_decl())?;
        let pair = con_at(
            "MkPair",
            vec![tvar("b")?, Type::Base(BaseType::Int)],
            vec![var("x")?, int(1)],
        )?;
        let term = tylam("b", lam("x", tvar("b")?, pair)?)?;
        let ty = lint(&term, &env)?;
        let expected = Type::Forall(
            "b".to_string(),
            Box::new(Type::Fun(
                Box::new(Type::Var("b".to_string())),
                Box::new(pair_app(
                    Type::Var("b".to_string()),
                    Type::Base(BaseType::Int),
                )),
            )),
        );
        assert!(ty.alpha_eq(&expected), "unexpected type: {ty}");
        Ok(())
    }

    #[test]
    fn con_rejects_int_for_type_var_named_like_a_param() -> Result<(), Box<dyn std::error::Error>> {
        // /\b. MkPair @b @Int 1 1 is ill-typed: the first field has type b.
        let env = LintEnv::new().try_with_data(pair_decl())?;
        let pair = con_at(
            "MkPair",
            vec![tvar("b")?, Type::Base(BaseType::Int)],
            vec![int(1), int(1)],
        )?;
        let term = tylam("b", pair)?;
        assert!(matches!(
            lint(&term, &env),
            Err(LintError::ArgTypeMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn case_binds_fields_with_simultaneous_substitution() -> Result<(), Box<dyn std::error::Error>>
    {
        // /\b. \p:Pair b Int. case p of MkPair(x, y) -> x  :  forall b. Pair b Int -> b
        let env = LintEnv::new().try_with_data(pair_decl())?;
        let scrutinee_ty = pair_app(tvar("b")?, Type::Base(BaseType::Int));
        let body = case_(var("p")?, vec![alt("MkPair", &["x", "y"], var("x")?)?]);
        let term = tylam("b", lam("p", scrutinee_ty.clone(), body)?)?;
        let ty = lint(&term, &env)?;
        let expected = Type::Forall(
            "b".to_string(),
            Box::new(Type::Fun(
                Box::new(scrutinee_ty),
                Box::new(Type::Var("b".to_string())),
            )),
        );
        assert!(ty.alpha_eq(&expected), "unexpected type: {ty}");
        Ok(())
    }

    #[test]
    fn prelude_env_case_list_int_lints_to_int() -> Result<(), Box<dyn std::error::Error>> {
        let env = prelude_env();
        let term = list_int_case(int(0), var("h")?)?;
        assert_eq!(lint(&term, &env), Ok(Type::Base(BaseType::Int)));
        Ok(())
    }
}
