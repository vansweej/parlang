//! Core elaboration of dictionary-explicit programs (M2.3c), the output of
//! `crate::dictionary::insert_dictionaries`.
//!
//! - A class `C` becomes the Core data type `$Dict_C p` with one
//!   constructor `$Dict_C` whose fields are the method types in class
//!   order. Method type variables other than the class parameter are bound
//!   by a `Forall` inside the field (rank-2 fields), named `m0`, `m1`, and
//!   so on, so they never shadow `p` or an elaborator type variable.
//! - The built-in `Eq` class is registered lazily, the first time a
//!   `$Dict_Eq` type is converted, so a program without dictionaries
//!   elaborates to exactly the same Core as before. Every referenced
//!   built-in dictionary `$dict_Eq_<Base>` is bound outermost to a
//!   `$Dict_Eq` constructor over the primitive `eq`.
//! - An instance dictionary `$dict_<Class>_<Tycon>` is a `TyLam` over the
//!   instance variables around a monomorphic `letrec`, so its method
//!   bodies may reference the dictionary itself. With a context the
//!   `letrec` binds the dictionary function; without one it binds a
//!   function of `Unit` that is applied to `()`. A self-reference at a
//!   different instantiation (polymorphic recursion) is unsupported.
//! - A context-free instance whose method field eagerly evaluates a
//!   self-reference (a non-lambda method body calling its own dictionary)
//!   diverges, exactly like the surface evaluator's encoding. This is not
//!   detected.
//! - A method occurrence `FieldAccess(dict, m)` is a `Case` selecting the
//!   field, then one `TyApp` per extra method variable. The instantiation
//!   is recovered by one-way matching the class signature against the
//!   occurrence type (`crate::dictionary::method_renaming`), which is total
//!   because every quantified method variable occurs in the signature. The
//!   signature's variables are first renamed apart from the matched types
//!   (`fresh_scheme`), so a shared type-variable id cannot alias them.
//!
//! Constrained functions elaborate to `TyLam`s around `$d` dictionary
//! lambdas and stay non-kernel (rule `R5`) until a Core to Core
//! specialiser exists (ADR 0010).

use parlang_core::Type as CoreType;
use parlang_core::{builder, BaseType, DataCon, DataType, Kind, Term};

use super::{unsupported, Elab, ElabError};
use crate::dictionary::{dictionary_type_name, method_renaming};
use crate::typechecker::{apply_subst, Substitution};
use crate::typed_ast::{TypedExpr, TypedExprKind};
use crate::types::{Type, TypeScheme, TypeVar};

/// Name prefix of every instance dictionary binding.
const INSTANCE_PREFIX: &str = "$dict_";

/// Name of the built-in `Eq` dictionary type and of its constructor.
const EQ_DICT: &str = "$Dict_Eq";

/// The instance dictionary whose value is being elaborated.
#[derive(Clone)]
pub(super) struct SelfDict {
    /// The dictionary binding name, for example `$dict_Eq_Nat`.
    name: String,
    /// Core type arguments of a self-reference at the same instantiation.
    vars: Vec<CoreType>,
    /// Whether the `letrec` binds a function of `Unit` (no context).
    context_free: bool,
}

/// Whether `name` binds an instance dictionary.
pub(super) fn is_instance_dictionary(name: &str) -> bool {
    name.starts_with(INSTANCE_PREFIX)
}

/// The base type of a built-in `Eq` dictionary name.
fn builtin_eq_base(name: &str) -> Option<BaseType> {
    match name {
        "$dict_Eq_Int" => Some(BaseType::Int),
        "$dict_Eq_Bool" => Some(BaseType::Bool),
        "$dict_Eq_Char" => Some(BaseType::Char),
        "$dict_Eq_Float" => Some(BaseType::Float),
        _ => None,
    }
}

/// The scheme of the built-in method `eq : a -> a -> Bool`, with the class
/// parameter first, as the type checker seeds it.
fn eq_method_scheme() -> TypeScheme {
    let param = Type::Var(TypeVar(0));
    let ty = Type::Fun(
        Box::new(param.clone()),
        Box::new(Type::Fun(Box::new(param), Box::new(Type::Bool))),
    );
    TypeScheme {
        vars: vec![TypeVar(0)],
        ty,
        constraints: Vec::new(),
    }
}

/// The largest type-variable id occurring in `ty`, or `None` without any.
fn max_var_id(ty: &Type) -> Option<usize> {
    match ty {
        Type::Var(TypeVar(n)) => Some(*n),
        Type::VarApp(TypeVar(n), args) => args
            .iter()
            .filter_map(max_var_id)
            .chain(std::iter::once(*n))
            .max(),
        Type::Fun(from, to) => max_var_id(from).max(max_var_id(to)),
        Type::SumType(_, items) | Type::Tuple(items) => items.iter().filter_map(max_var_id).max(),
        Type::Record(fields) => fields.values().filter_map(max_var_id).max(),
        Type::Int | Type::Bool | Type::Char | Type::Float | Type::Unit => None,
    }
}

/// `scheme` with its quantified variables renamed to ids above every id of
/// `scheme`, `head` and `actual`, so `method_renaming` never confuses a
/// signature variable with a variable of `head` or `actual`. Also returns
/// the renamed extra (non-parameter) variables in their original order.
fn fresh_scheme(scheme: &TypeScheme, head: &Type, actual: &Type) -> (TypeScheme, Vec<TypeVar>) {
    let base = [max_var_id(&scheme.ty), max_var_id(head), max_var_id(actual)]
        .into_iter()
        .flatten()
        .chain(scheme.vars.iter().map(|var| var.0))
        .max()
        .map_or(0, |max| max.saturating_add(1));
    let vars: Vec<TypeVar> = (0..scheme.vars.len())
        .map(|offset| TypeVar(base.saturating_add(offset)))
        .collect();
    let subst: Substitution = scheme
        .vars
        .iter()
        .cloned()
        .zip(vars.iter().cloned().map(Type::Var))
        .collect();
    let extras = vars.get(1..).unwrap_or_default().to_vec();
    let renamed = TypeScheme {
        vars,
        ty: apply_subst(&subst, &scheme.ty),
        constraints: Vec::new(),
    };
    (renamed, extras)
}

impl Elab {
    /// The Core name of surface type variable `n`: an override from
    /// `var_names` first, else `t<n>` when an enclosing `TyLam` binds it.
    pub(super) fn tyvar_name(&self, n: usize) -> Option<String> {
        self.var_names
            .get(&n)
            .cloned()
            .or_else(|| self.scope.contains(&n).then(|| format!("t{n}")))
    }

    /// Registers the built-in `$Dict_Eq` data type the first time `name` is
    /// `$Dict_Eq`; any other name is left alone.
    pub(super) fn ensure_dict_type(&mut self, name: &str) -> Result<(), ElabError> {
        if name != EQ_DICT || self.data_kinds.contains_key(name) {
            return Ok(());
        }
        let param = CoreType::Var("p".to_string());
        let field = builder::fun_ty(
            param.clone(),
            builder::fun_ty(param, CoreType::Base(BaseType::Bool)),
        );
        self.register_data(DataType {
            name: EQ_DICT.to_string(),
            params: vec!["p".to_string()],
            param_kinds: vec![Kind::Star],
            ctors: vec![DataCon {
                name: EQ_DICT.to_string(),
                fields: vec![field],
            }],
        })?;
        self.classes.insert(
            EQ_DICT.to_string(),
            vec![("eq".to_string(), eq_method_scheme())],
        );
        Ok(())
    }

    /// Registers the dictionary data type `$Dict_<class>` of a user class:
    /// parameter `p` of kind `param_kind`, one field per method in order.
    pub(super) fn register_class(
        &mut self,
        class: &str,
        param_kind: &Kind,
        methods: &[(String, TypeScheme)],
    ) -> Result<(), ElabError> {
        let dict_type = dictionary_type_name(class);
        let mut fields = Vec::with_capacity(methods.len());
        for (_, scheme) in methods {
            fields.push(self.method_field_type(scheme)?);
        }
        self.register_data(DataType {
            name: dict_type.clone(),
            params: vec!["p".to_string()],
            param_kinds: vec![param_kind.clone()],
            ctors: vec![DataCon {
                name: dict_type.clone(),
                fields,
            }],
        })?;
        self.classes.insert(dict_type, methods.to_vec());
        Ok(())
    }

    /// The Core field type of one method: its signature with the class
    /// parameter named `p` and each extra variable bound by a `Forall`
    /// named `m<i>`, the first extra variable outermost.
    fn method_field_type(&mut self, scheme: &TypeScheme) -> Result<CoreType, ElabError> {
        let Some((param, extras)) = scheme.vars.split_first() else {
            return Err(unsupported(
                "class method",
                "method scheme without a class parameter",
            ));
        };
        let kinds = crate::kind_check::infer_type_var_kinds(&self.data_kinds, &scheme.ty);
        let binders: Vec<String> = (0..extras.len()).map(|i| format!("m{i}")).collect();
        let saved = self.var_names.clone();
        self.var_names.insert(param.0, "p".to_string());
        for (extra, binder) in extras.iter().zip(&binders) {
            self.var_names.insert(extra.0, binder.clone());
        }
        let converted = self.conv_type(&scheme.ty);
        self.var_names = saved;
        let mut ty = converted?;
        for (extra, binder) in extras.iter().zip(&binders).rev() {
            let kind = kinds.get(extra).cloned().unwrap_or(Kind::Star);
            ty = builder::forall_k(binder, kind, ty)?;
        }
        Ok(ty)
    }

    /// Elaborates a variable naming a dictionary: a self-reference of the
    /// instance dictionary being elaborated, or a built-in `Eq` dictionary,
    /// which is recorded so `wrap_builtin_dicts` binds it. `None` for every
    /// other name.
    pub(super) fn elab_dict_var(
        &mut self,
        name: &str,
        type_args: &[Type],
    ) -> Result<Option<Term>, ElabError> {
        if let Some(current) = self
            .self_dict
            .clone()
            .filter(|current| current.name == name)
        {
            let mut args = Vec::with_capacity(type_args.len());
            for ty in type_args {
                args.push(self.conv_type(ty)?);
            }
            if args != current.vars {
                return Err(unsupported(
                    "instance dictionary",
                    &format!("{name} is used at another instantiation (polymorphic recursion)"),
                ));
            }
            let reference = builder::var(name)?;
            if current.context_free {
                return Ok(Some(builder::app(reference, builder::unit())));
            }
            return Ok(Some(reference));
        }
        if builtin_eq_base(name).is_some() {
            self.ensure_dict_type(EQ_DICT)?;
            self.builtin_dicts.insert(name.to_string());
        }
        Ok(None)
    }

    /// Binds every referenced built-in `Eq` dictionary around `body`, as a
    /// `$Dict_Eq` constructor over the primitive `eq` at its base type.
    pub(super) fn wrap_builtin_dicts(&mut self, body: Term) -> Result<Term, ElabError> {
        let mut acc = body;
        for name in std::mem::take(&mut self.builtin_dicts).into_iter().rev() {
            let Some(base) = builtin_eq_base(&name) else {
                continue;
            };
            let ty = CoreType::Base(base);
            let eq = builder::con("eq", vec![builder::var("$x")?, builder::var("$y")?])?;
            let method = builder::lam("$x", ty.clone(), builder::lam("$y", ty.clone(), eq)?)?;
            let dict = builder::con_at(EQ_DICT, vec![ty], vec![method])?;
            acc = builder::let_(&name, dict, acc)?;
        }
        Ok(acc)
    }

    /// Elaborates the instance dictionary binding `name`: `scheme` holds the
    /// instance variables over the dictionary type, and `value` is a record,
    /// or a function of the context dictionaries returning one.
    pub(super) fn elab_instance_dict(
        &mut self,
        name: &str,
        scheme: &TypeScheme,
        value: &TypedExpr,
    ) -> Result<Term, ElabError> {
        let depth = self.scope.len();
        self.scope.extend(scheme.vars.iter().map(|var| var.0));
        let result = self.instance_letrec(name, scheme, value);
        self.scope.truncate(depth);
        self.self_dict = None;
        let mut term = result?;
        let kinds = crate::kind_check::infer_type_var_kinds(&self.data_kinds, &scheme.ty);
        for var in scheme.vars.iter().rev() {
            let kind = kinds.get(var).cloned().unwrap_or(Kind::Star);
            term = builder::tylam_k(&format!("t{}", var.0), kind, term)?;
        }
        Ok(term)
    }

    /// The monomorphic `letrec` binding the instance dictionary `name`, so
    /// method bodies can reference it (see the module documentation).
    fn instance_letrec(
        &mut self,
        name: &str,
        scheme: &TypeScheme,
        value: &TypedExpr,
    ) -> Result<Term, ElabError> {
        let dict_ty = self.conv_type(&scheme.ty)?;
        let context_free = !matches!(value.kind, TypedExprKind::Fun(..));
        self.self_dict = Some(SelfDict {
            name: name.to_string(),
            vars: scheme
                .vars
                .iter()
                .map(|var| CoreType::Var(format!("t{}", var.0)))
                .collect(),
            context_free,
        });
        let body = self.elab_expr(value)?;
        if !context_free {
            return Ok(builder::letrec(name, dict_ty, body, builder::var(name)?)?);
        }
        let unit_ty = CoreType::Base(BaseType::Unit);
        let param = self.fresh_name("u");
        let thunk = builder::lam(&param, unit_ty.clone(), body)?;
        let ann = builder::fun_ty(unit_ty, dict_ty);
        let applied = builder::app(builder::var(name)?, builder::unit());
        Ok(builder::letrec(name, ann, thunk, applied)?)
    }

    /// Elaborates a dictionary record of type `dict_type args` into the
    /// dictionary constructor, one method field per class method in order.
    pub(super) fn elab_dict_record(
        &mut self,
        dict_type: &str,
        args: &[Type],
        fields: &[(String, TypedExpr)],
    ) -> Result<Term, ElabError> {
        self.ensure_dict_type(dict_type)?;
        let [head] = args else {
            return Err(unsupported(
                "dictionary",
                "dictionary type without exactly one argument",
            ));
        };
        let Some(methods) = self.classes.get(dict_type).cloned() else {
            return Err(unsupported(
                "dictionary",
                &format!("unknown dictionary type {dict_type}"),
            ));
        };
        if methods.len() != fields.len() {
            return Err(unsupported(
                "dictionary",
                "dictionary fields do not match the class methods",
            ));
        }
        let head_ty = self.conv_type(head)?;
        let mut values = Vec::with_capacity(fields.len());
        for ((method, scheme), (field, value)) in methods.iter().zip(fields) {
            if method != field {
                return Err(unsupported(
                    "dictionary",
                    "dictionary fields are not in class order",
                ));
            }
            values.push(self.elab_method_body(scheme, head, value)?);
        }
        Ok(builder::con_at(dict_type, vec![head_ty], values)?)
    }

    /// Elaborates one instance method `body` against its class `scheme` at
    /// `head`. Head-variable images take the Core names of the instance
    /// variables, and each extra variable's image becomes a fresh `TyLam`
    /// binder, in the field's `Forall` order.
    fn elab_method_body(
        &mut self,
        scheme: &TypeScheme,
        head: &Type,
        body: &TypedExpr,
    ) -> Result<Term, ElabError> {
        let mismatch = || {
            unsupported(
                "instance method",
                "method body does not match its class signature",
            )
        };
        let (renamed, fresh_extras) = fresh_scheme(scheme, head, &body.ty);
        let renaming = method_renaming(&renamed, head, &body.ty).ok_or_else(mismatch)?;
        let mut overrides = Vec::new();
        for (var, image) in &renaming {
            if fresh_extras.contains(var) {
                continue;
            }
            if let (Type::Var(image_var), Some(core_name)) = (image, self.tyvar_name(var.0)) {
                overrides.push((image_var.0, core_name));
            }
        }
        let kinds = crate::kind_check::infer_type_var_kinds(&self.data_kinds, &scheme.ty);
        let extras = scheme.vars.get(1..).unwrap_or_default();
        let mut binders = Vec::with_capacity(extras.len());
        for (extra, fresh_extra) in extras.iter().zip(&fresh_extras) {
            let Some(Type::Var(image_var)) = renaming.get(fresh_extra) else {
                return Err(mismatch());
            };
            let binder = self.fresh_name("m");
            overrides.push((image_var.0, binder.clone()));
            binders.push((binder, kinds.get(extra).cloned().unwrap_or(Kind::Star)));
        }
        let saved = self.var_names.clone();
        self.var_names.extend(overrides);
        let result = self.elab_expr(body);
        self.var_names = saved;
        let mut term = result?;
        for (binder, kind) in binders.into_iter().rev() {
            term = builder::tylam_k(&binder, kind, term)?;
        }
        Ok(term)
    }

    /// Elaborates `inner.field` at occurrence type `ty`: a method selection
    /// when `inner` is a dictionary, else a record field access.
    pub(super) fn elab_projection(
        &mut self,
        ty: &Type,
        inner: &TypedExpr,
        field: &str,
    ) -> Result<Term, ElabError> {
        if let Type::SumType(dict_type, args) = &inner.ty {
            return self.elab_method_select(ty, dict_type, args, inner, field);
        }
        self.elab_field_access(inner, field)
    }

    /// Selects method `field` from dictionary `inner` of type
    /// `dict_type args`, then applies the method's extra type variables as
    /// instantiated by the occurrence type `ty`.
    fn elab_method_select(
        &mut self,
        ty: &Type,
        dict_type: &str,
        args: &[Type],
        inner: &TypedExpr,
        field: &str,
    ) -> Result<Term, ElabError> {
        self.ensure_dict_type(dict_type)?;
        let selection_error = |reason: &str| unsupported("method selection", reason);
        let [head] = args else {
            return Err(selection_error(
                "dictionary type without exactly one argument",
            ));
        };
        let Some(methods) = self.classes.get(dict_type) else {
            return Err(selection_error(&format!(
                "unknown dictionary type {dict_type}"
            )));
        };
        let count = methods.len();
        let Some((position, scheme)) = methods
            .iter()
            .enumerate()
            .find(|(_, (method, _))| method == field)
            .map(|(index, (_, scheme))| (index, scheme.clone()))
        else {
            return Err(selection_error(&format!(
                "no method {field} in {dict_type}"
            )));
        };
        let (renamed, fresh_extras) = fresh_scheme(&scheme, head, ty);
        let Some(renaming) = method_renaming(&renamed, head, ty) else {
            return Err(selection_error(
                "occurrence type does not match the class signature",
            ));
        };
        let scrut = self.elab_expr(inner)?;
        let binders: Vec<String> = (0..count).map(|_| self.fresh_name("f")).collect();
        let refs: Vec<&str> = binders.iter().map(String::as_str).collect();
        let Some(selected) = binders.get(position).map(String::as_str) else {
            return Err(selection_error("method position out of range"));
        };
        let mut term = builder::case_(
            scrut,
            vec![builder::alt(dict_type, &refs, builder::var(selected)?)?],
        );
        for fresh_extra in &fresh_extras {
            let Some(instance) = renaming.get(fresh_extra) else {
                return Err(selection_error("extra method variable not instantiated"));
            };
            term = builder::tyapp(term, self.conv_type(instance)?);
        }
        Ok(term)
    }
}
