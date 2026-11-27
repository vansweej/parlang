/// Hindley-Milner type inference implementation
use crate::ast::{BinOp, Decl, Expr, Literal, Pattern, Program};
use crate::typed_ast::{
    TypedDecl, TypedExpr, TypedExprKind, TypedMatchArm, TypedPattern, TypedPatternKind,
    TypedProgram,
};
use crate::types::{Type, TypeScheme, TypeVar};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Sum type constructor information
#[derive(Debug, Clone)]
pub struct ConstructorInfo {
    /// Type parameters (e.g., `["a", "b"]` for `Either a b`)
    pub type_params: Vec<String>,
    /// Payload types for this constructor
    pub payload_types: Vec<crate::ast::TypeAnnotation>,
    /// Name of the sum type this constructor belongs to
    pub sum_type_name: String,
}

/// Type environment (Γ) mapping variables to type schemes
#[derive(Debug, Clone)]
pub struct TypeEnv {
    bindings: HashMap<String, TypeScheme>,
    next_var: usize,
    type_aliases: HashMap<String, Type>,
    /// Constructor information: maps constructor name to its type info
    constructors: HashMap<String, ConstructorInfo>,
    /// Declared data types: maps type name to its parameter kinds
    data_kinds: HashMap<String, Vec<parlang_core::Kind>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            bindings: HashMap::new(),
            next_var: 0,
            type_aliases: HashMap::new(),
            constructors: HashMap::new(),
            data_kinds: HashMap::new(),
        }
    }

    /// Generate a fresh type variable
    pub fn fresh_var(&mut self) -> Type {
        let var = Type::Var(TypeVar(self.next_var));
        self.next_var += 1;
        var
    }

    /// Advance the fresh-variable counter past an existing type variable.
    fn advance_past(&mut self, var: &TypeVar) {
        self.next_var = self.next_var.max(var.0.saturating_add(1));
    }

    /// Look up a variable and instantiate its type scheme
    pub fn lookup(&mut self, name: &str) -> Option<Type> {
        let scheme = self.bindings.get(name)?.clone();
        Some(self.instantiate(&scheme))
    }

    /// Look up a variable, instantiate its type scheme, and also return the
    /// fresh instantiation types, one per quantified variable in
    /// `TypeScheme::vars` order (empty for a monomorphic binding).
    pub fn lookup_instantiated(&mut self, name: &str) -> Option<(Type, Vec<Type>)> {
        let scheme = self.bindings.get(name)?.clone();
        Some(self.instantiate_with_args(&scheme))
    }

    /// Bind a variable to a type scheme
    pub fn bind(&mut self, name: String, scheme: TypeScheme) {
        self.bindings.insert(name, scheme);
    }

    /// Extend environment with a monomorphic binding
    #[must_use]
    pub fn extend(&self, name: String, ty: Type) -> Self {
        let mut new_env = self.clone();
        new_env.bind(name, TypeScheme::mono(ty));
        new_env
    }

    /// Instantiate a type scheme by replacing quantified variables with fresh ones
    fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        if scheme.vars.is_empty() {
            return scheme.ty.clone();
        }

        let mut subst = HashMap::new();
        for var in &scheme.vars {
            subst.insert(var.clone(), self.fresh_var());
        }

        apply_subst(&subst, &scheme.ty)
    }

    /// Like `instantiate`, but also returns the fresh type variables used,
    /// in `TypeScheme::vars` order. Issues the same fresh variables in the
    /// same order as `instantiate`.
    fn instantiate_with_args(&mut self, scheme: &TypeScheme) -> (Type, Vec<Type>) {
        let args: Vec<Type> = scheme.vars.iter().map(|_| self.fresh_var()).collect();
        let subst: Substitution = scheme
            .vars
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        (apply_subst(&subst, &scheme.ty), args)
    }

    /// Generalize a type by quantifying free type variables
    pub fn generalize(&self, ty: &Type) -> TypeScheme {
        let free_in_env = self.free_vars();
        let free_in_type = free_type_vars(ty);

        let mut quantified: Vec<TypeVar> = free_in_type.difference(&free_in_env).cloned().collect();
        quantified.sort();

        TypeScheme::poly(quantified, ty.clone())
    }

    /// Get free type variables in the environment
    fn free_vars(&self) -> HashSet<TypeVar> {
        self.bindings
            .values()
            .flat_map(|scheme| {
                let mut free = free_type_vars(&scheme.ty);
                for var in &scheme.vars {
                    free.remove(var);
                }
                free
            })
            .collect()
    }

    /// Define a type alias
    pub fn define_type_alias(&mut self, name: String, ty: Type) {
        self.type_aliases.insert(name, ty);
    }

    /// Resolve a type alias by name
    pub fn resolve_type_alias(&self, name: &str) -> Option<Type> {
        self.type_aliases.get(name).cloned()
    }

    /// Register a constructor for a sum type
    pub fn register_constructor(&mut self, constructor_name: String, info: ConstructorInfo) {
        self.constructors.insert(constructor_name, info);
    }

    /// Constructors of the sum type `type_name` with their arities, sorted by name.
    pub(crate) fn constructors_of(&self, type_name: &str) -> Vec<(String, usize)> {
        let mut ctors: Vec<(String, usize)> = self
            .constructors
            .iter()
            .filter(|(_, info)| info.sum_type_name == type_name)
            .map(|(name, info)| (name.clone(), info.payload_types.len()))
            .collect();
        ctors.sort();
        ctors
    }

    /// Look up constructor information
    pub fn lookup_constructor(&self, name: &str) -> Option<&ConstructorInfo> {
        self.constructors.get(name)
    }

    /// Record the parameter kinds of the data type `name`.
    pub fn define_data_kinds(&mut self, name: String, kinds: Vec<parlang_core::Kind>) {
        self.data_kinds.insert(name, kinds);
    }

    /// Parameter kinds of the declared data type `name`, if any.
    pub fn data_kinds(&self, name: &str) -> Option<&[parlang_core::Kind]> {
        self.data_kinds.get(name).map(Vec::as_slice)
    }

    /// Number of type parameters of the declared data type `name`, if any.
    pub fn data_arity(&self, name: &str) -> Option<usize> {
        self.data_kinds.get(name).map(Vec::len)
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Substitution map from type variables to types
type Substitution = HashMap<TypeVar, Type>;

/// Apply type substitution to a type
///
/// This is a wrapper around `apply_subst_with_visited` that handles the common case
/// of applying a substitution without needing to track visited variables. It prevents
/// infinite recursion when dealing with cyclic type references.
///
/// # Arguments
/// * `subst` - The substitution mapping type variables to types
/// * `ty` - The type to apply the substitution to
///
/// # Returns
/// A new type with all substitutable type variables replaced
fn apply_subst(subst: &Substitution, ty: &Type) -> Type {
    apply_subst_with_visited(subst, ty, &mut HashSet::new())
}

/// Apply substitution to a type with cycle detection
fn apply_subst_with_visited(
    subst: &Substitution,
    ty: &Type,
    visited: &mut HashSet<TypeVar>,
) -> Type {
    match ty {
        Type::Int | Type::Bool | Type::Char | Type::Float | Type::Unit => ty.clone(),
        Type::Var(v) => {
            if visited.contains(v) {
                // Cycle detected, return the variable as-is
                return ty.clone();
            }
            if let Some(t) = subst.get(v) {
                visited.insert(v.clone());
                let result = apply_subst_with_visited(subst, t, visited);
                visited.remove(v);
                result
            } else {
                ty.clone()
            }
        }
        Type::Fun(arg, ret) => Type::Fun(
            Box::new(apply_subst_with_visited(subst, arg, visited)),
            Box::new(apply_subst_with_visited(subst, ret, visited)),
        ),
        Type::Record(fields) => {
            let mut new_fields = HashMap::new();
            for (name, ty) in fields {
                new_fields.insert(name.clone(), apply_subst_with_visited(subst, ty, visited));
            }
            Type::Record(new_fields)
        }
        Type::SumType(name, args) => {
            let new_args = args
                .iter()
                .map(|arg| apply_subst_with_visited(subst, arg, visited))
                .collect();
            Type::SumType(name.clone(), new_args)
        }
        Type::VarApp(var, args) => subst_var_app(subst, var, args, visited),
        Type::Tuple(elements) => Type::Tuple(
            elements
                .iter()
                .map(|elem| apply_subst_with_visited(subst, elem, visited))
                .collect(),
        ),
    }
}

/// Substitute inside the applied type variable `var args`, re-flattening the
/// result when `var` is bound to a data type or another applied variable.
fn subst_var_app(
    subst: &Substitution,
    var: &TypeVar,
    args: &[Type],
    visited: &mut HashSet<TypeVar>,
) -> Type {
    let new_args: Vec<Type> = args
        .iter()
        .map(|arg| apply_subst_with_visited(subst, arg, visited))
        .collect();
    // Kinding invariant: applied variables come from kind-checked annotations
    // and payloads, and unification binds an applied variable's head only in
    // `unify_var_app`, to a type constructor. Any other binding would be
    // ill-kinded, so it is kept unsubstituted instead of dropping arguments.
    match apply_subst_with_visited(subst, &Type::Var(var.clone()), visited) {
        Type::Var(head) => Type::VarApp(head, new_args),
        Type::VarApp(head, mut prefix) => {
            prefix.extend(new_args);
            Type::VarApp(head, prefix)
        }
        Type::SumType(name, mut prefix) => {
            prefix.extend(new_args);
            Type::SumType(name, prefix)
        }
        _ => Type::VarApp(var.clone(), new_args),
    }
}

/// Get free type variables in a type
///
/// A type variable is "free" if it appears in the type but is not bound by any
/// quantifier. This function recursively traverses a type and collects all free
/// type variables.
///
/// # Arguments
/// * `ty` - The type to analyze
///
/// # Returns
/// A set of all free type variables in the type
///
/// # Example
/// - For `Int -> Int`: returns `{}`
/// - For `t0 -> t1`: returns `{t0, t1}`
/// - For `{ age: t0 }`: returns `{t0}`
fn free_type_vars(ty: &Type) -> HashSet<TypeVar> {
    match ty {
        Type::Int | Type::Bool | Type::Char | Type::Float | Type::Unit => HashSet::new(),
        Type::Var(v) => {
            let mut set = HashSet::new();
            set.insert(v.clone());
            set
        }
        Type::Fun(arg, ret) => {
            let mut set = free_type_vars(arg);
            set.extend(free_type_vars(ret));
            set
        }
        Type::Record(fields) => {
            let mut set = HashSet::new();
            for ty in fields.values() {
                set.extend(free_type_vars(ty));
            }
            set
        }
        Type::SumType(_name, args) => {
            let mut set = HashSet::new();
            for arg in args {
                set.extend(free_type_vars(arg));
            }
            set
        }
        Type::VarApp(var, args) => {
            let mut set = HashSet::new();
            set.insert(var.clone());
            for arg in args {
                set.extend(free_type_vars(arg));
            }
            set
        }
        Type::Tuple(elements) => {
            let mut set = HashSet::new();
            for elem in elements {
                set.extend(free_type_vars(elem));
            }
            set
        }
    }
}

/// Convert `TypeAnnotation` to Type
/// This is used when processing sum type definitions
fn type_annotation_to_type(
    annotation: &crate::ast::TypeAnnotation,
    type_param_map: &HashMap<String, Type>,
    env: &mut TypeEnv,
) -> Type {
    match annotation {
        crate::ast::TypeAnnotation::Concrete(name) => {
            match name.as_str() {
                "Int" => Type::Int,
                "Bool" => Type::Bool,
                "Char" => Type::Char,
                "Float" => Type::Float,
                _ => {
                    // User-defined sum type (not a built-in primitive)
                    // Treat as a sum type with no arguments
                    Type::SumType(name.clone(), vec![])
                }
            }
        }
        crate::ast::TypeAnnotation::Var(name) => {
            // Look up the type variable in the parameter map
            type_param_map.get(name).cloned().unwrap_or_else(|| {
                // Type parameter not found in map - generate a fresh variable
                // This handles the case where a type parameter is used but not declared
                env.fresh_var()
            })
        }
        crate::ast::TypeAnnotation::Fun(arg, ret) => Type::Fun(
            Box::new(type_annotation_to_type(arg, type_param_map, env)),
            Box::new(type_annotation_to_type(ret, type_param_map, env)),
        ),
        crate::ast::TypeAnnotation::App(name, args) => {
            let arg_types: Vec<Type> = args
                .iter()
                .map(|arg| type_annotation_to_type(arg, type_param_map, env))
                .collect();
            Type::SumType(name.clone(), arg_types)
        }
        crate::ast::TypeAnnotation::VarApp(name, args) => {
            let head = type_param_map
                .get(name)
                .cloned()
                .unwrap_or_else(|| env.fresh_var());
            let arg_types: Vec<Type> = args
                .iter()
                .map(|arg| type_annotation_to_type(arg, type_param_map, env))
                .collect();
            // Parameter-map heads are type variables, so the fallback is unused.
            crate::types::apply_type_args(head.clone(), arg_types).unwrap_or(head)
        }
    }
}

/// Type checking errors
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    UnboundVariable(String),
    UnificationError(Box<Type>, Box<Type>),
    OccursCheckFailed(TypeVar, Type),
    RecursionRequiresAnnotation,
    /// Field not found in record type: field name, available fields
    FieldNotFound(String, Vec<String>),
    /// Expected record type but got something else
    RecordExpected(String),
    /// Record type field mismatch during unification
    RecordFieldMismatch,
    /// Constructor applied with wrong number of arguments: constructor name, expected, actual
    ConstructorArityMismatch(String, usize, usize),
    /// A cycle was found while resolving top-level type aliases.
    TypeAliasCycle(Vec<String>),
    /// A top-level data type or type alias was declared more than once.
    DuplicateTypeName(String),
    /// A top-level constructor name was declared more than once.
    DuplicateConstructor(String),
    /// Tuple projection index was out of range: (index, arity).
    TupleIndexOutOfBounds(usize, usize),
    /// Expected a tuple type but got something else.
    TupleExpected(String),
    /// A constructor was used without any `data` declaration defining it.
    UnknownConstructor(String),
    /// A type constructor applied to the wrong number of type arguments:
    /// type name, expected, actual (expected is 0 for base types).
    TypeConstructorArityMismatch(String, usize, usize),
    /// A type alias was used in a data constructor payload, which is not
    /// supported yet because payload annotations are not alias-expanded.
    TypeAliasInDataPayload(String),
    /// A type whose kind does not fit its position: type, expected kind,
    /// found kind (e.g. `Int` where `* -> *` is expected).
    KindMismatch(String, String, String),
    /// A `match` whose arms do not cover every value; `missing` holds witness patterns.
    NonExhaustiveMatch {
        missing: Vec<String>,
    },
    /// TEMPORARY (M2.2a): `class`/`instance` declarations parse but are not
    /// typechecked yet; the payload names the declaration, e.g. `class Eq`.
    /// M2.2b replaces this with real class and instance typing.
    ClassesNotSupportedYet(String),
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeError::UnboundVariable(name) => {
                write!(f, "Unbound variable: {name}")
            }
            TypeError::UnificationError(t1, t2) => {
                write!(f, "Cannot unify types: {t1} and {t2}")
            }
            TypeError::OccursCheckFailed(var, ty) => {
                write!(f, "Occurs check failed: t{} occurs in {ty}", var.0)
            }
            TypeError::RecursionRequiresAnnotation => {
                write!(f, "Recursive functions require type annotations")
            }
            TypeError::FieldNotFound(field, available) => {
                write!(
                    f,
                    "Field '{field}' not found. Available fields: {available:?}"
                )
            }
            TypeError::RecordExpected(got) => {
                write!(f, "Expected record type, got {got}")
            }
            TypeError::RecordFieldMismatch => {
                write!(f, "Record types have different fields")
            }
            TypeError::ConstructorArityMismatch(name, expected, actual) => {
                write!(
                    f,
                    "Constructor '{name}' expects {expected} arguments, but got {actual}"
                )
            }
            TypeError::TypeAliasCycle(names) => {
                write!(f, "Type alias cycle detected: {}", names.join(" -> "))
            }
            TypeError::DuplicateTypeName(name) => {
                write!(f, "Duplicate type name: {name}")
            }
            TypeError::DuplicateConstructor(name) => {
                write!(f, "Duplicate constructor: {name}")
            }
            TypeError::TupleIndexOutOfBounds(index, arity) => {
                write!(
                    f,
                    "Tuple index {index} out of bounds for tuple of arity {arity}"
                )
            }
            TypeError::TupleExpected(got) => {
                write!(f, "Expected tuple type, got {got}")
            }
            TypeError::UnknownConstructor(name) => {
                write!(f, "Unknown constructor: {name}")
            }
            TypeError::NonExhaustiveMatch { missing } => {
                write!(f, "Non-exhaustive match: missing {}", missing.join(", "))
            }
            TypeError::TypeConstructorArityMismatch(name, expected, actual) => {
                write!(
                    f,
                    "Type constructor '{name}' expects {expected} type arguments, but got {actual}"
                )
            }
            TypeError::TypeAliasInDataPayload(name) => {
                write!(f, "Type alias in data payload is not supported yet: {name}")
            }
            TypeError::KindMismatch(ty, expected, found) => {
                write!(
                    f,
                    "Kind mismatch for {ty}: expected kind {expected}, found {found}"
                )
            }
            TypeError::ClassesNotSupportedYet(what) => {
                write!(f, "type classes are not supported yet: {what}")
            }
        }
    }
}

impl std::error::Error for TypeError {}

/// Unification algorithm
fn unify(t1: &Type, t2: &Type) -> Result<Substitution, TypeError> {
    match (t1, t2) {
        (Type::Int, Type::Int)
        | (Type::Bool, Type::Bool)
        | (Type::Char, Type::Char)
        | (Type::Float, Type::Float)
        | (Type::Unit, Type::Unit) => Ok(HashMap::new()),

        (Type::Var(v), t) | (t, Type::Var(v)) => bind_var(v.clone(), t.clone()),

        (Type::Fun(a1, r1), Type::Fun(a2, r2)) => {
            let s1 = unify(a1, a2)?;
            let r1_subst = apply_subst(&s1, r1);
            let r2_subst = apply_subst(&s1, r2);
            let s2 = unify(&r1_subst, &r2_subst)?;
            Ok(compose_subst(&s2, &s1))
        }

        (Type::Record(fields1), Type::Record(fields2)) => {
            // Both records must have the same fields
            if fields1.len() != fields2.len() {
                return Err(TypeError::RecordFieldMismatch);
            }

            let mut subst = HashMap::new();

            for (name, ty1) in fields1 {
                match fields2.get(name) {
                    Some(ty2) => {
                        let ty1 = apply_subst(&subst, ty1);
                        let ty2 = apply_subst(&subst, ty2);
                        let s = unify(&ty1, &ty2)?;
                        subst = compose_subst(&s, &subst);
                    }
                    None => {
                        return Err(TypeError::RecordFieldMismatch);
                    }
                }
            }

            Ok(subst)
        }

        (Type::SumType(name1, args1), Type::SumType(name2, args2)) => {
            // Sum types must have the same name and same number of type arguments
            if name1 != name2 || args1.len() != args2.len() {
                return Err(TypeError::UnificationError(
                    Box::new(t1.clone()),
                    Box::new(t2.clone()),
                ));
            }
            unify_pairwise(args1, args2)
        }

        (Type::VarApp(var, args), other) | (other, Type::VarApp(var, args)) => {
            unify_var_app(var, args, other, t1, t2)
        }

        (Type::Tuple(left), Type::Tuple(right)) => {
            if left.len() != right.len() {
                return Err(TypeError::UnificationError(
                    Box::new(t1.clone()),
                    Box::new(t2.clone()),
                ));
            }
            let mut subst = HashMap::new();
            for (left_elem, right_elem) in left.iter().zip(right.iter()) {
                let left_elem = apply_subst(&subst, left_elem);
                let right_elem = apply_subst(&subst, right_elem);
                let s = unify(&left_elem, &right_elem)?;
                subst = compose_subst(&s, &subst);
            }
            Ok(subst)
        }

        _ => Err(TypeError::UnificationError(
            Box::new(t1.clone()),
            Box::new(t2.clone()),
        )),
    }
}

/// Unify two equally long type lists pairwise, threading the substitution.
fn unify_pairwise(left: &[Type], right: &[Type]) -> Result<Substitution, TypeError> {
    let mut subst = HashMap::new();
    for (left_ty, right_ty) in left.iter().zip(right) {
        let s = unify(
            &apply_subst(&subst, left_ty),
            &apply_subst(&subst, right_ty),
        )?;
        subst = compose_subst(&s, &subst);
    }
    Ok(subst)
}

/// Unify the applied type variable `var args` with `other`.
///
/// `other` must be an application with at least as many arguments: `var` is
/// bound to `other` without its last `args.len()` arguments (a partially
/// applied data type or applied variable) and the remaining arguments are
/// unified pairwise. A shorter applied variable on the other side swaps the
/// roles. `t1` and `t2` are the original pair, used for error reporting.
fn unify_var_app(
    var: &TypeVar,
    args: &[Type],
    other: &Type,
    t1: &Type,
    t2: &Type,
) -> Result<Substitution, TypeError> {
    let mismatch = || TypeError::UnificationError(Box::new(t1.clone()), Box::new(t2.clone()));
    let (head, other_args) = match other {
        Type::SumType(name, other_args) => (Type::SumType(name.clone(), vec![]), other_args),
        Type::VarApp(other_var, other_args) => (Type::Var(other_var.clone()), other_args),
        _ => return Err(mismatch()),
    };
    if other_args.len() < args.len() {
        return match other {
            Type::VarApp(other_var, _) => {
                let swapped = Type::VarApp(var.clone(), args.to_vec());
                unify_var_app(other_var, other_args, &swapped, t1, t2)
            }
            _ => Err(mismatch()),
        };
    }
    let (prefix, suffix) = other_args.split_at(other_args.len() - args.len());
    let head = crate::types::apply_type_args(head, prefix.to_vec()).ok_or_else(mismatch)?;
    let bound = bind_var(var.clone(), head)?;
    let left: Vec<Type> = args.iter().map(|arg| apply_subst(&bound, arg)).collect();
    let right: Vec<Type> = suffix.iter().map(|arg| apply_subst(&bound, arg)).collect();
    let rest = unify_pairwise(&left, &right)?;
    Ok(compose_subst(&rest, &bound))
}

/// Bind a type variable to a type
fn bind_var(var: TypeVar, ty: Type) -> Result<Substitution, TypeError> {
    if let Type::Var(v) = &ty {
        if v == &var {
            return Ok(HashMap::new());
        }
    }

    // Occurs check
    if free_type_vars(&ty).contains(&var) {
        return Err(TypeError::OccursCheckFailed(var, ty));
    }

    let mut subst = HashMap::new();
    subst.insert(var, ty);
    Ok(subst)
}

/// Compose two substitutions
fn compose_subst(s1: &Substitution, s2: &Substitution) -> Substitution {
    let mut result = s2.clone();
    for (var, ty) in s1 {
        result.insert(var.clone(), apply_subst(s1, ty));
    }
    result
}

/// Apply substitution to type environment
fn apply_subst_env(subst: &Substitution, env: &mut TypeEnv) {
    for scheme in env.bindings.values_mut() {
        scheme.ty = apply_subst(subst, &scheme.ty);
    }
}

/// Convert a `TypeExpr` to a Type, resolving any aliases
fn resolve_type_expr(ty_expr: &crate::ast::TypeExpr, env: &TypeEnv) -> Result<Type, TypeError> {
    match ty_expr {
        crate::ast::TypeExpr::Int => Ok(Type::Int),
        crate::ast::TypeExpr::Bool => Ok(Type::Bool),
        crate::ast::TypeExpr::Fun(arg, ret) => {
            let arg_ty = resolve_type_expr(arg, env)?;
            let ret_ty = resolve_type_expr(ret, env)?;
            Ok(Type::Fun(Box::new(arg_ty), Box::new(ret_ty)))
        }
        crate::ast::TypeExpr::Alias(name) => env
            .resolve_type_alias(name)
            .ok_or_else(|| TypeError::UnboundVariable(name.clone())),
    }
}

fn resolve_program_type_alias(
    name: &str,
    aliases: &HashMap<String, crate::ast::TypeExpr>,
    env: &TypeEnv,
    chain: &mut Vec<String>,
) -> Result<Type, TypeError> {
    if let Some(cycle_start) = chain.iter().position(|current| current == name) {
        let mut cycle = chain[cycle_start..].to_vec();
        cycle.push(name.to_string());
        return Err(TypeError::TypeAliasCycle(cycle));
    }

    let Some(ty_expr) = aliases.get(name) else {
        return env
            .resolve_type_alias(name)
            .ok_or_else(|| TypeError::UnboundVariable(name.to_string()));
    };

    chain.push(name.to_string());
    let result = resolve_program_type_expr(ty_expr, aliases, env, chain);
    chain.pop();
    result
}

fn resolve_program_type_expr(
    ty_expr: &crate::ast::TypeExpr,
    aliases: &HashMap<String, crate::ast::TypeExpr>,
    env: &TypeEnv,
    chain: &mut Vec<String>,
) -> Result<Type, TypeError> {
    match ty_expr {
        crate::ast::TypeExpr::Int => Ok(Type::Int),
        crate::ast::TypeExpr::Bool => Ok(Type::Bool),
        crate::ast::TypeExpr::Fun(arg, ret) => Ok(Type::Fun(
            Box::new(resolve_program_type_expr(arg, aliases, env, chain)?),
            Box::new(resolve_program_type_expr(ret, aliases, env, chain)?),
        )),
        crate::ast::TypeExpr::Alias(name) => resolve_program_type_alias(name, aliases, env, chain),
    }
}

/// Convert a `TypeAnnotation` to a Type, resolving names to concrete types.
///
/// Type variables are shared within this one annotation: every occurrence of
/// the same name becomes the same fresh, flexible type variable.
fn resolve_type_annotation(
    ty_ann: &crate::ast::TypeAnnotation,
    env: &mut TypeEnv,
) -> Result<Type, TypeError> {
    crate::kind_check::check_annotation_kinds(env, ty_ann)?;
    let mut vars = HashMap::new();
    resolve_annotation_with_vars(ty_ann, env, &mut vars)
}

/// Resolve `ty_ann` like `resolve_type_annotation`, reusing the type variables
/// already allocated for this annotation in `vars`.
fn resolve_annotation_with_vars(
    ty_ann: &crate::ast::TypeAnnotation,
    env: &mut TypeEnv,
    vars: &mut HashMap<String, Type>,
) -> Result<Type, TypeError> {
    match ty_ann {
        crate::ast::TypeAnnotation::Concrete(name) => {
            // Check if it's a basic type
            match name.as_str() {
                "Int" => Ok(Type::Int),
                "Bool" => Ok(Type::Bool),
                "Char" => Ok(Type::Char),
                "Float" => Ok(Type::Float),
                // Kinds were checked by `resolve_type_annotation`, so a bare
                // parameterised data type here is a partial application.
                _ if env.data_arity(name).is_some() => Ok(Type::SumType(name.clone(), vec![])),
                // Try to resolve as type alias
                _ => env
                    .resolve_type_alias(name)
                    .ok_or_else(|| TypeError::UnboundVariable(name.clone())),
            }
        }
        crate::ast::TypeAnnotation::Var(name) => {
            // One fresh variable per name within this annotation, so
            // `fun (f : a -> a) -> f` links the argument and result types.
            Ok(vars
                .entry(name.clone())
                .or_insert_with(|| env.fresh_var())
                .clone())
        }
        crate::ast::TypeAnnotation::Fun(arg, ret) => {
            let arg_ty = resolve_annotation_with_vars(arg, env, vars)?;
            let ret_ty = resolve_annotation_with_vars(ret, env, vars)?;
            Ok(Type::Fun(Box::new(arg_ty), Box::new(ret_ty)))
        }
        crate::ast::TypeAnnotation::App(name, args) => {
            let arg_types = args
                .iter()
                .map(|arg| resolve_annotation_with_vars(arg, env, vars))
                .collect::<Result<Vec<Type>, TypeError>>()?;
            Ok(Type::SumType(name.clone(), arg_types))
        }
        crate::ast::TypeAnnotation::VarApp(name, args) => {
            let head = vars
                .entry(name.clone())
                .or_insert_with(|| env.fresh_var())
                .clone();
            let applied = args
                .iter()
                .map(|arg| resolve_annotation_with_vars(arg, env, vars))
                .collect::<Result<Vec<Type>, TypeError>>()?;
            // Annotation heads are type variables, so the fallback is unused.
            Ok(crate::types::apply_type_args(head.clone(), applied).unwrap_or(head))
        }
    }
}

/// Type inference for a binary operator application, given the already-inferred
/// left/right types are not yet known — infers both operands, then dispatches
/// on the operator to determine the result type (extracted from `infer` to
/// keep that function's line count manageable).
/// Type inference for `+ - * /`, given the already-inferred operand types and
/// substitutions (extracted from `infer_binop` to keep its line count down).
fn infer_arith_binop(
    left_ty: Type,
    right_ty: &Type,
    s1: &Substitution,
    s2: &Substitution,
) -> Result<(Type, Substitution), TypeError> {
    // Arithmetic operations work on Int and Float
    // Check if left type is Int or Float
    match &left_ty {
        Type::Int => {
            let s3 = unify(right_ty, &Type::Int)?;
            let subst = compose_subst(&s3, &compose_subst(s2, s1));
            Ok((Type::Int, subst))
        }
        Type::Float => {
            let s3 = unify(right_ty, &Type::Float)?;
            let subst = compose_subst(&s3, &compose_subst(s2, s1));
            Ok((Type::Float, subst))
        }
        Type::Var(_) => {
            // Try to unify with right type first
            let s3 = unify(&left_ty, right_ty)?;
            let unified_ty = apply_subst(&s3, &left_ty);

            // Now check if unified type is Int or Float
            match &unified_ty {
                Type::Int | Type::Float => {
                    let subst = compose_subst(&s3, &compose_subst(s2, s1));
                    Ok((unified_ty, subst))
                }
                Type::Var(_) => {
                    // Still a type variable, default to Int for arithmetic operations
                    let s4 = unify(&unified_ty, &Type::Int)?;
                    let subst = compose_subst(&s4, &compose_subst(&s3, &compose_subst(s2, s1)));
                    Ok((Type::Int, subst))
                }
                _ => Err(TypeError::UnificationError(
                    Box::new(unified_ty),
                    Box::new(Type::Int),
                )),
            }
        }
        _ => Err(TypeError::UnificationError(
            Box::new(left_ty),
            Box::new(Type::Int),
        )),
    }
}

/// Type inference for `< <= > >=`, given the already-inferred operand types
/// and substitutions (extracted from `infer_binop` to keep its line count down).
fn infer_ordering_binop(
    left_ty: Type,
    right_ty: &Type,
    s1: &Substitution,
    s2: &Substitution,
) -> Result<(Type, Substitution), TypeError> {
    // Ordering comparisons work for Int, Char, and Float
    // Check if left type is Int, Char, or Float
    match &left_ty {
        Type::Int => {
            let s3 = unify(right_ty, &Type::Int)?;
            let subst = compose_subst(&s3, &compose_subst(s2, s1));
            Ok((Type::Bool, subst))
        }
        Type::Char => {
            let s3 = unify(right_ty, &Type::Char)?;
            let subst = compose_subst(&s3, &compose_subst(s2, s1));
            Ok((Type::Bool, subst))
        }
        Type::Float => {
            let s3 = unify(right_ty, &Type::Float)?;
            let subst = compose_subst(&s3, &compose_subst(s2, s1));
            Ok((Type::Bool, subst))
        }
        Type::Var(_) => {
            // Try to unify with right type first
            let s3 = unify(&left_ty, right_ty)?;
            let unified_ty = apply_subst(&s3, &left_ty);

            // Now check if unified type is Int, Char, or Float
            match &unified_ty {
                Type::Int | Type::Char | Type::Float => {
                    let subst = compose_subst(&s3, &compose_subst(s2, s1));
                    Ok((Type::Bool, subst))
                }
                Type::Var(_) => {
                    // Still a type variable, default to Int for ordering operations
                    let s4 = unify(&unified_ty, &Type::Int)?;
                    let subst = compose_subst(&s4, &compose_subst(&s3, &compose_subst(s2, s1)));
                    Ok((Type::Bool, subst))
                }
                _ => Err(TypeError::UnificationError(
                    Box::new(unified_ty),
                    Box::new(Type::Int),
                )),
            }
        }
        _ => Err(TypeError::UnificationError(
            Box::new(left_ty),
            Box::new(Type::Int),
        )),
    }
}

fn infer_binop(
    op: BinOp,
    left: &Expr,
    right: &Expr,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let (left_typed, s1) = infer_typed(left, env)?;
    advance_next_var_from_inference(env, &left_typed.ty, &s1);
    let mut env1 = env.clone();
    apply_subst_env(&s1, &mut env1);

    let (right_typed, s2) = infer_typed(right, &mut env1)?;
    advance_next_var_from_inference(&mut env1, &right_typed.ty, &s2);
    env.next_var = env.next_var.max(env1.next_var);
    let left_ty = apply_subst(&s2, &left_typed.ty);

    let (result_ty, subst) = match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
            infer_arith_binop(left_ty, &right_typed.ty, &s1, &s2)?
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            infer_ordering_binop(left_ty, &right_typed.ty, &s1, &s2)?
        }
        BinOp::Eq | BinOp::Neq => {
            // Equality works on any type, but both sides must match
            let s3 = unify(&left_ty, &right_typed.ty)?;
            let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
            (Type::Bool, subst)
        }
    };

    Ok((
        TypedExpr {
            kind: TypedExprKind::BinOp(op, Box::new(left_typed), Box::new(right_typed)),
            ty: result_ty,
        },
        subst,
    ))
}

/// Type inference for `let name [: Ty] = value in body` (extracted from
/// `infer` to keep its line count down).
fn infer_let(
    name: &str,
    ty_ann_opt: Option<&crate::ast::TypeAnnotation>,
    value: &Expr,
    body: &Expr,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let (value_typed, s1) = infer_typed(value, env)?;
    advance_next_var_from_inference(env, &value_typed.ty, &s1);

    if let Some(ty_ann) = ty_ann_opt {
        let annotated_ty = resolve_type_annotation(ty_ann, env)?;
        let s_ann = unify(&value_typed.ty, &annotated_ty)?;
        let s1 = compose_subst(&s_ann, &s1);

        let mut env1 = env.clone();
        apply_subst_env(&s1, &mut env1);

        let unified_ty = apply_subst(&s1, &value_typed.ty);
        let scheme = env1.generalize(&unified_ty);
        env1.bind(name.to_string(), scheme.clone());

        let (body_typed, s2) = infer_typed(body, &mut env1)?;
        advance_next_var_from_inference(&mut env1, &body_typed.ty, &s2);
        env.next_var = env.next_var.max(env1.next_var);

        let subst = compose_subst(&s2, &s1);
        let ty = body_typed.ty.clone();
        Ok((
            TypedExpr {
                kind: TypedExprKind::Let(
                    name.to_string(),
                    ty_ann_opt.cloned(),
                    scheme,
                    Box::new(value_typed),
                    Box::new(body_typed),
                ),
                ty,
            },
            subst,
        ))
    } else {
        let mut env1 = env.clone();
        apply_subst_env(&s1, &mut env1);

        let scheme = env1.generalize(&apply_subst(&s1, &value_typed.ty));
        env1.bind(name.to_string(), scheme.clone());

        let (body_typed, s2) = infer_typed(body, &mut env1)?;
        advance_next_var_from_inference(&mut env1, &body_typed.ty, &s2);
        env.next_var = env.next_var.max(env1.next_var);

        let subst = compose_subst(&s2, &s1);
        let ty = body_typed.ty.clone();
        Ok((
            TypedExpr {
                kind: TypedExprKind::Let(
                    name.to_string(),
                    ty_ann_opt.cloned(),
                    scheme,
                    Box::new(value_typed),
                    Box::new(body_typed),
                ),
                ty,
            },
            subst,
        ))
    }
}

/// Type inference for `record_expr.field_name` (extracted from `infer` to
/// keep its line count down).
fn infer_field_access(
    record_expr: &Expr,
    field_name: &str,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let (record_typed, s1) = infer_typed(record_expr, env)?;
    let record_ty = apply_subst(&s1, &record_typed.ty);

    let field_ty = match record_ty {
        Type::Record(ref fields) => {
            if let Some(field_ty) = fields.get(field_name) {
                field_ty.clone()
            } else {
                let available: Vec<String> = fields.keys().cloned().collect();
                return Err(TypeError::FieldNotFound(field_name.to_string(), available));
            }
        }
        _ => return Err(TypeError::RecordExpected(format!("{record_ty}"))),
    };

    Ok((
        TypedExpr {
            kind: TypedExprKind::FieldAccess(Box::new(record_typed), field_name.to_string()),
            ty: field_ty,
        },
        s1,
    ))
}

/// Type inference for tuple projection `tuple_expr.index`. The tuple
/// expression is inferred and its substitution applied; a `Type::Tuple`
/// yields the element type at `index` (or `TupleIndexOutOfBounds`), and any
/// other type — including an unresolved type variable or `Unit` — is a
/// `TupleExpected` error, mirroring the `RecordExpected` precedent for field
/// access.
fn infer_tuple_proj(
    tuple_expr: &Expr,
    index: usize,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let (tuple_typed, s1) = infer_typed(tuple_expr, env)?;
    let tuple_ty = apply_subst(&s1, &tuple_typed.ty);

    let element_ty = match tuple_ty {
        Type::Tuple(ref elements) => match elements.get(index) {
            Some(element_ty) => element_ty.clone(),
            None => return Err(TypeError::TupleIndexOutOfBounds(index, elements.len())),
        },
        other => return Err(TypeError::TupleExpected(format!("{other}"))),
    };

    Ok((
        TypedExpr {
            kind: TypedExprKind::TupleProj(Box::new(tuple_typed), index),
            ty: element_ty,
        },
        s1,
    ))
}

/// Type inference for a sum-type constructor application (extracted from
/// `infer` to keep its line count down).
fn infer_constructor(
    name: &str,
    args: &[Expr],
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    // Look up constructor information and clone it to avoid borrow issues
    if let Some(info) = env.lookup_constructor(name).cloned() {
        // Create a mapping from type parameters to fresh type variables
        let mut type_param_map = HashMap::new();
        for param in &info.type_params {
            type_param_map.insert(param.clone(), env.fresh_var());
        }

        // Type check each argument
        let mut subst = HashMap::new();
        let mut arg_types = Vec::new();
        let mut typed_args = Vec::new();

        for arg in args {
            let (typed_arg, s) = infer_typed(arg, env)?;
            subst = compose_subst(&s, &subst);
            arg_types.push(apply_subst(&subst, &typed_arg.ty));
            typed_args.push(typed_arg);
        }

        // Check that the number of arguments matches
        if arg_types.len() != info.payload_types.len() {
            // Return an error for argument count mismatch
            return Err(TypeError::ConstructorArityMismatch(
                name.to_string(),
                info.payload_types.len(),
                arg_types.len(),
            ));
        }

        // Unify each argument with its expected type
        for (arg_ty, expected_annotation) in arg_types.iter().zip(&info.payload_types) {
            let expected_ty = type_annotation_to_type(expected_annotation, &type_param_map, env);
            let s = unify(arg_ty, &expected_ty)?;
            subst = compose_subst(&s, &subst);
        }

        // Create the result type
        let type_args: Vec<Type> = info
            .type_params
            .iter()
            .map(|param| apply_subst(&subst, &type_param_map[param]))
            .collect();

        let result_ty = Type::SumType(info.sum_type_name.clone(), type_args);
        Ok((
            TypedExpr {
                kind: TypedExprKind::Constructor(name.to_string(), typed_args),
                ty: result_ty,
            },
            subst,
        ))
    } else {
        // Constructor not declared by any `data` declaration: reject it before
        // inferring its arguments, so the constructor name is the reported error.
        Err(TypeError::UnknownConstructor(name.to_string()))
    }
}

/// Type a record pattern `{ f1: p1, ... }` against the scrutinee type.
/// A subset of the record's fields is permitted (the evaluator's record
/// pattern is partial). Any non-record scrutinee — including an unresolved
/// type variable — is a `RecordExpected` error.
fn infer_pattern_record(
    fields: &[(String, Pattern)],
    expected: &Type,
    env: &mut TypeEnv,
    bindings: &mut Vec<(String, Type)>,
) -> Result<(TypedPattern, Substitution), TypeError> {
    match expected {
        Type::Record(map) => {
            let mut subst = HashMap::new();
            let mut typed_fields = Vec::with_capacity(fields.len());
            for (field_name, sub_pattern) in fields {
                let Some(field_ty) = map.get(field_name) else {
                    let mut available: Vec<String> = map.keys().cloned().collect();
                    available.sort();
                    return Err(TypeError::FieldNotFound(field_name.clone(), available));
                };
                let field_ty = apply_subst(&subst, field_ty);
                let (typed_sub, s) = infer_pattern(sub_pattern, &field_ty, env, bindings)?;
                subst = compose_subst(&s, &subst);
                typed_fields.push((field_name.clone(), typed_sub));
            }
            Ok((
                typed_pattern(TypedPatternKind::Record(typed_fields), expected),
                subst,
            ))
        }
        other => Err(TypeError::RecordExpected(format!("{other}"))),
    }
}

/// Type a constructor pattern `Ctor p1 p2 ...` against the scrutinee type.
/// Registered constructors constrain the scrutinee to their sum type and
/// type their sub-patterns against the declared payloads. A constructor with
/// no `data` declaration is an `UnknownConstructor` error; string literals
/// desugar to `Cons`/`Nil`, so they need a `List` declaration in scope.
fn infer_pattern_constructor(
    name: &str,
    patterns: &[Pattern],
    expected: &Type,
    env: &mut TypeEnv,
    bindings: &mut Vec<(String, Type)>,
) -> Result<(TypedPattern, Substitution), TypeError> {
    if let Some(info) = env.lookup_constructor(name).cloned() {
        let type_args: Vec<Type> = info.type_params.iter().map(|_| env.fresh_var()).collect();
        let type_param_map: HashMap<String, Type> = info
            .type_params
            .iter()
            .cloned()
            .zip(type_args.iter().cloned())
            .collect();

        if patterns.len() != info.payload_types.len() {
            return Err(TypeError::ConstructorArityMismatch(
                name.to_string(),
                info.payload_types.len(),
                patterns.len(),
            ));
        }

        let sum_ty = Type::SumType(info.sum_type_name.clone(), type_args);
        let mut subst = unify(expected, &sum_ty)?;

        let mut items = Vec::with_capacity(patterns.len());
        for (sub_pattern, annotation) in patterns.iter().zip(&info.payload_types) {
            let payload = type_annotation_to_type(annotation, &type_param_map, env);
            let payload = apply_subst(&subst, &payload);
            let (typed_sub, s) = infer_pattern(sub_pattern, &payload, env, bindings)?;
            subst = compose_subst(&s, &subst);
            items.push(typed_sub);
        }
        let kind = TypedPatternKind::Constructor(name.to_string(), items);
        Ok((typed_pattern(kind, &sum_ty), subst))
    } else {
        Err(TypeError::UnknownConstructor(name.to_string()))
    }
}

/// Build a [`TypedPattern`] node of type `ty` (resolved later by
/// `resolve_pattern`).
fn typed_pattern(kind: TypedPatternKind, ty: &Type) -> TypedPattern {
    TypedPattern {
        kind,
        ty: ty.clone(),
    }
}

/// Apply `subst` to every `ty` in a typed pattern tree.
fn resolve_pattern(subst: &Substitution, pattern: &TypedPattern) -> TypedPattern {
    use TypedPatternKind as P;
    let kind = match &pattern.kind {
        P::Literal(_) | P::Var(_) | P::Wildcard => pattern.kind.clone(),
        P::Tuple(items) => P::Tuple(items.iter().map(|p| resolve_pattern(subst, p)).collect()),
        P::Record(fields) => P::Record(
            fields
                .iter()
                .map(|(name, p)| (name.clone(), resolve_pattern(subst, p)))
                .collect(),
        ),
        P::Constructor(name, args) => P::Constructor(
            name.clone(),
            args.iter().map(|p| resolve_pattern(subst, p)).collect(),
        ),
    };
    TypedPattern {
        kind,
        ty: apply_subst(subst, &pattern.ty),
    }
}

/// Type a pattern against the `expected` scrutinee type, collecting variable
/// bindings. Returns the typed pattern (types resolved later by
/// `resolve_pattern`) and the substitution produced by unifying the
/// pattern's shape with `expected`.
fn infer_pattern(
    pattern: &Pattern,
    expected: &Type,
    env: &mut TypeEnv,
    bindings: &mut Vec<(String, Type)>,
) -> Result<(TypedPattern, Substitution), TypeError> {
    match pattern {
        Pattern::Literal(lit) => {
            let lit_ty = match lit {
                Literal::Int(_) => Type::Int,
                Literal::Bool(_) => Type::Bool,
                Literal::Char(_) => Type::Char,
            };
            let s = unify(expected, &lit_ty)?;
            Ok((
                typed_pattern(TypedPatternKind::Literal(lit.clone()), &lit_ty),
                s,
            ))
        }
        Pattern::Var(name) => {
            bindings.push((name.clone(), expected.clone()));
            let kind = TypedPatternKind::Var(name.clone());
            Ok((typed_pattern(kind, expected), HashMap::new()))
        }
        Pattern::Wildcard => Ok((
            typed_pattern(TypedPatternKind::Wildcard, expected),
            HashMap::new(),
        )),
        Pattern::Tuple(patterns) => {
            if patterns.is_empty() {
                let s = unify(expected, &Type::Unit)?;
                let kind = TypedPatternKind::Tuple(Vec::new());
                return Ok((typed_pattern(kind, &Type::Unit), s));
            }
            let fresh: Vec<Type> = patterns.iter().map(|_| env.fresh_var()).collect();
            let mut subst = unify(expected, &Type::Tuple(fresh.clone()))?;
            let mut items = Vec::with_capacity(patterns.len());
            for (sub_pattern, fresh_ty) in patterns.iter().zip(&fresh) {
                let fresh_ty = apply_subst(&subst, fresh_ty);
                let (typed_sub, s) = infer_pattern(sub_pattern, &fresh_ty, env, bindings)?;
                subst = compose_subst(&s, &subst);
                items.push(typed_sub);
            }
            let kind = TypedPatternKind::Tuple(items);
            Ok((typed_pattern(kind, &Type::Tuple(fresh)), subst))
        }
        Pattern::Record(fields) => infer_pattern_record(fields, expected, env, bindings),
        Pattern::Constructor(name, patterns) => {
            infer_pattern_constructor(name, patterns, expected, env, bindings)
        }
    }
}

/// Checks that the arms of a `match` cover every value of the scrutinee type.
///
/// Resolves `scrut_ty` and each arm pattern under the final `subst`, then calls
/// `crate::match_coverage::check_match`; missing witnesses become
/// `TypeError::NonExhaustiveMatch`.
fn check_arm_coverage(
    subst: &Substitution,
    scrut_ty: &Type,
    arms: &[TypedMatchArm],
    env: &TypeEnv,
) -> Result<(), TypeError> {
    let resolved_ty = apply_subst(subst, scrut_ty);
    let resolved: Vec<TypedPattern> = arms
        .iter()
        .map(|arm| resolve_pattern(subst, &arm.pattern))
        .collect();
    let refs: Vec<&TypedPattern> = resolved.iter().collect();
    crate::match_coverage::check_match(&resolved_ty, &refs, env)
        .map_err(|missing| TypeError::NonExhaustiveMatch { missing })
}

/// Type inference for `match scrutinee with | p1 -> e1 | ...`. Each arm's
/// pattern is typed against the (substituted) scrutinee type, the pattern's
/// bindings extend a per-arm environment monomorphically, and every arm body
/// is unified into a single result type.
fn infer_match(
    scrutinee: &Expr,
    arms: &[(Pattern, Expr)],
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let (scrut_typed, s0) = infer_typed(scrutinee, env)?;
    advance_next_var_from_inference(env, &scrut_typed.ty, &s0);

    let result_ty = env.fresh_var();
    let mut subst = s0;
    let mut typed_arms = Vec::with_capacity(arms.len());

    for (pattern, body) in arms {
        let expected = apply_subst(&subst, &scrut_typed.ty);
        let mut bindings: Vec<(String, Type)> = Vec::new();
        let (arm_pattern, pattern_subst) = infer_pattern(pattern, &expected, env, &mut bindings)?;
        subst = compose_subst(&pattern_subst, &subst);

        let mut arm_env = env.clone();
        apply_subst_env(&subst, &mut arm_env);
        for (name, ty) in &bindings {
            arm_env = arm_env.extend(name.clone(), apply_subst(&subst, ty));
        }

        let (body_typed, body_subst) = infer_typed(body, &mut arm_env)?;
        advance_next_var_from_inference(&mut arm_env, &body_typed.ty, &body_subst);
        env.next_var = env.next_var.max(arm_env.next_var);
        subst = compose_subst(&body_subst, &subst);

        let arm_subst = unify(
            &apply_subst(&subst, &result_ty),
            &apply_subst(&subst, &body_typed.ty),
        )?;
        subst = compose_subst(&arm_subst, &subst);

        typed_arms.push(TypedMatchArm {
            pattern: arm_pattern,
            bindings,
            body: body_typed,
        });
    }

    check_arm_coverage(&subst, &scrut_typed.ty, &typed_arms, env)?;
    let ty = apply_subst(&subst, &result_ty);
    Ok((
        TypedExpr {
            kind: TypedExprKind::Match(Box::new(scrut_typed), typed_arms),
            ty,
        },
        subst,
    ))
}

/// Type inference for a tuple expression `(e1, e2, ...)`. The empty tuple is
/// the `Unit` type; otherwise each element is inferred left to right,
/// threading the accumulated substitution, and the result is `Type::Tuple`.
fn infer_tuple(
    elements: &[Expr],
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    if elements.is_empty() {
        return Ok((
            TypedExpr {
                kind: TypedExprKind::Tuple(vec![]),
                ty: Type::Unit,
            },
            HashMap::new(),
        ));
    }

    let mut subst = HashMap::new();
    let mut typed_elems = Vec::with_capacity(elements.len());

    for element in elements {
        let mut element_env = env.clone();
        apply_subst_env(&subst, &mut element_env);
        let (element_typed, element_subst) = infer_typed(element, &mut element_env)?;
        advance_next_var_from_inference(&mut element_env, &element_typed.ty, &element_subst);
        env.next_var = env.next_var.max(element_env.next_var);
        subst = compose_subst(&element_subst, &subst);
        typed_elems.push(element_typed);
    }

    let resolved: Vec<Type> = typed_elems
        .iter()
        .map(|e| apply_subst(&subst, &e.ty))
        .collect();

    Ok((
        TypedExpr {
            kind: TypedExprKind::Tuple(typed_elems),
            ty: Type::Tuple(resolved),
        },
        subst,
    ))
}

/// Type inference for `if cond then then_br else else_br` (extracted from
/// `infer` to keep its line count down).
fn infer_if(
    cond: &Expr,
    then_br: &Expr,
    else_br: &Expr,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let (cond_typed, s1) = infer_typed(cond, env)?;
    let s2 = unify(&cond_typed.ty, &Type::Bool)?;
    advance_next_var_from_inference(env, &cond_typed.ty, &s1);

    let mut env1 = env.clone();
    apply_subst_env(&compose_subst(&s2, &s1), &mut env1);

    let (then_typed, s3) = infer_typed(then_br, &mut env1)?;
    advance_next_var_from_inference(&mut env1, &then_typed.ty, &s3);

    let mut env2 = env1.clone();
    apply_subst_env(&s3, &mut env2);

    let (else_typed, s4) = infer_typed(else_br, &mut env2)?;
    advance_next_var_from_inference(&mut env2, &else_typed.ty, &s4);
    env.next_var = env.next_var.max(env2.next_var);

    let then_ty = apply_subst(&s4, &then_typed.ty);
    let s5 = unify(&then_ty, &else_typed.ty)?;

    let result_ty = apply_subst(&s5, &then_ty);
    let subst = compose_subst(
        &s5,
        &compose_subst(&s4, &compose_subst(&s3, &compose_subst(&s2, &s1))),
    );

    Ok((
        TypedExpr {
            kind: TypedExprKind::If(
                Box::new(cond_typed),
                Box::new(then_typed),
                Box::new(else_typed),
            ),
            ty: result_ty,
        },
        subst,
    ))
}

/// Type inference for function application `func arg` (extracted from
/// `infer` to keep its line count down).
fn infer_app(
    func: &Expr,
    arg: &Expr,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let (func_typed, s1) = infer_typed(func, env)?;
    advance_next_var_from_inference(env, &func_typed.ty, &s1);

    let mut env1 = env.clone();
    apply_subst_env(&s1, &mut env1);

    let (arg_typed, s2) = infer_typed(arg, &mut env1)?;
    advance_next_var_from_inference(&mut env1, &arg_typed.ty, &s2);
    env.next_var = env.next_var.max(env1.next_var);

    let func_ty = apply_subst(&s2, &func_typed.ty);
    let result_ty = env1.fresh_var();
    env.next_var = env.next_var.max(env1.next_var);

    let s3 = unify(
        &func_ty,
        &Type::Fun(Box::new(arg_typed.ty.clone()), Box::new(result_ty.clone())),
    )?;

    let result_ty = apply_subst(&s3, &result_ty);
    let subst = compose_subst(&s3, &compose_subst(&s2, &s1));

    Ok((
        TypedExpr {
            kind: TypedExprKind::App(Box::new(func_typed), Box::new(arg_typed)),
            ty: result_ty,
        },
        subst,
    ))
}

/// Type inference for `rec name -> body` via fixpoint typing (extracted from
/// `infer` to keep its line count down).
fn infer_rec(
    name: &str,
    body: &Expr,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    // For recursive functions, we use fixpoint typing:
    // 1. Generate a fresh type variable for the recursive function
    // 2. Add it to the environment before checking the body
    // 3. Infer the type of the body with the recursive name bound
    // 4. Unify the inferred type with the assumed type

    let rec_ty = env.fresh_var();
    let mut extended_env = env.extend(name.to_string(), rec_ty.clone());

    let (body_typed, subst) = infer_typed(body, &mut extended_env)?;
    advance_next_var_from_inference(&mut extended_env, &body_typed.ty, &subst);
    env.next_var = env.next_var.max(extended_env.next_var);

    // The body type should be the same as the recursive function type
    // (after applying the substitution from inferring the body)
    let rec_ty = apply_subst(&subst, &rec_ty);
    let s2 = unify(&rec_ty, &body_typed.ty)?;

    let final_ty = apply_subst(&s2, &body_typed.ty);
    let final_subst = compose_subst(&s2, &subst);

    Ok((
        TypedExpr {
            kind: TypedExprKind::Rec(name.to_string(), Box::new(body_typed)),
            ty: final_ty,
        },
        final_subst,
    ))
}

/// Type inference for `fun param [: Ty] -> body` (extracted from `infer` to
/// keep its line count down).
fn infer_fun(
    param: &str,
    ty_ann_opt: Option<&crate::ast::TypeAnnotation>,
    body: &Expr,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let param_ty = if let Some(ty_ann) = ty_ann_opt {
        resolve_type_annotation(ty_ann, env)?
    } else {
        env.fresh_var()
    };

    let mut env1 = env.clone();
    env1 = env1.extend(param.to_string(), param_ty.clone());

    let (body_typed, s1) = infer_typed(body, &mut env1)?;
    advance_next_var_from_inference(&mut env1, &body_typed.ty, &s1);
    env.next_var = env.next_var.max(env1.next_var);
    let param_ty = apply_subst(&s1, &param_ty);

    let fun_ty = Type::Fun(Box::new(param_ty), Box::new(body_typed.ty.clone()));
    Ok((
        TypedExpr {
            kind: TypedExprKind::Fun(param.to_string(), ty_ann_opt.cloned(), Box::new(body_typed)),
            ty: fun_ty,
        },
        s1,
    ))
}

/// Type inference for a record literal `{ f1 = e1, ... }` (extracted from
/// `infer` to keep its line count down).
fn infer_record(
    fields: &[(String, Expr)],
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let mut field_types = HashMap::new();
    let mut typed_fields = Vec::with_capacity(fields.len());
    let mut subst = HashMap::new();

    for (name, expr) in fields {
        let (expr_typed, s) = infer_typed(expr, env)?;
        let ty = apply_subst(&subst, &expr_typed.ty);
        subst = compose_subst(&s, &subst);
        apply_subst_env(&s, env);
        field_types.insert(name.clone(), ty);
        typed_fields.push((name.clone(), expr_typed));
    }

    Ok((
        TypedExpr {
            kind: TypedExprKind::Record(typed_fields),
            ty: Type::Record(field_types),
        },
        subst,
    ))
}

/// Type inference for a `type Name ... = ...` definition node (extracted from
/// `infer` to keep its line count down).
fn infer_type_def(
    name: &str,
    type_params: &[String],
    constructors: &[(String, Vec<crate::ast::TypeAnnotation>)],
    body: &Expr,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let sig = crate::kind_check::DataSig {
        name,
        params: type_params,
        ctors: constructors,
    };
    // Captured now: inferring `body` may redefine `name` in `env`.
    let mut param_kinds = Vec::new();
    for (data_name, kinds) in crate::kind_check::infer_data_kinds(env, std::slice::from_ref(&sig))?
    {
        if data_name == name {
            param_kinds.clone_from(&kinds);
        }
        env.define_data_kinds(data_name, kinds);
    }
    for (ctor_name, payload_types) in constructors {
        let info = ConstructorInfo {
            type_params: type_params.to_vec(),
            payload_types: payload_types.clone(),
            sum_type_name: name.to_string(),
        };
        env.register_constructor(ctor_name.clone(), info);
    }

    let (body_typed, subst) = infer_typed(body, env)?;
    let ty = body_typed.ty.clone();
    Ok((
        TypedExpr {
            kind: TypedExprKind::TypeDef {
                name: name.to_string(),
                type_params: type_params.to_vec(),
                param_kinds,
                constructors: constructors.to_vec(),
                body: Box::new(body_typed),
            },
            ty,
        },
        subst,
    ))
}

/// Type inference for `type Name = TyExpr in body` (extracted from `infer` to
/// keep its line count down).
fn infer_type_alias(
    name: &str,
    ty_expr: &crate::ast::TypeExpr,
    body: &Expr,
    env: &mut TypeEnv,
) -> Result<(TypedExpr, Substitution), TypeError> {
    let ty = resolve_type_expr(ty_expr, env)?;
    let mut new_env = env.clone();
    new_env.define_type_alias(name.to_string(), ty);

    let (body_typed, subst) = infer_typed(body, &mut new_env)?;
    advance_next_var_from_inference(&mut new_env, &body_typed.ty, &subst);
    env.next_var = env.next_var.max(new_env.next_var);

    let ty = body_typed.ty.clone();
    Ok((
        TypedExpr {
            kind: TypedExprKind::TypeAlias(name.to_string(), ty_expr.clone(), Box::new(body_typed)),
            ty,
        },
        subst,
    ))
}

/// Build a leaf typed node (literal / `Var` / resolved `Load`) with no
/// substitution to compose.
fn leaf(kind: TypedExprKind, ty: Type) -> (TypedExpr, Substitution) {
    (TypedExpr { kind, ty }, HashMap::new())
}

/// Type inference for expressions
///
/// # Errors
///
/// Returns a `TypeError` if `expr` contains an unbound variable, a type
/// mismatch that cannot be unified, an occurs-check failure, a constructor
/// applied with the wrong number of arguments, or any other static typing
/// violation detected during inference.
fn infer_typed(expr: &Expr, env: &mut TypeEnv) -> Result<(TypedExpr, Substitution), TypeError> {
    match expr {
        Expr::Int(n) => Ok(leaf(TypedExprKind::Int(*n), Type::Int)),
        Expr::Bool(b) => Ok(leaf(TypedExprKind::Bool(*b), Type::Bool)),
        Expr::Char(c) => Ok(leaf(TypedExprKind::Char(*c), Type::Char)),
        Expr::Float(x) => Ok(leaf(TypedExprKind::Float(*x), Type::Float)),
        Expr::Var(name) => {
            let (ty, args) = env
                .lookup_instantiated(name)
                .ok_or_else(|| TypeError::UnboundVariable(name.clone()))?;
            Ok(leaf(TypedExprKind::Var(name.clone(), args), ty))
        }
        Expr::BinOp(op, left, right) => infer_binop(*op, left, right, env),
        Expr::If(cond, then_br, else_br) => infer_if(cond, then_br, else_br, env),
        Expr::Let(name, ty_ann_opt, value, body) => {
            infer_let(name, ty_ann_opt.as_ref(), value, body, env)
        }
        Expr::Fun(param, ty_ann_opt, body) => infer_fun(param, ty_ann_opt.as_ref(), body, env),
        Expr::App(func, arg) => infer_app(func, arg, env),
        Expr::Rec(name, body) => infer_rec(name, body, env),
        Expr::Tuple(elements) => infer_tuple(elements, env),
        Expr::TupleProj(tuple_expr, index) => infer_tuple_proj(tuple_expr, *index, env),
        Expr::Match(scrutinee, arms) => infer_match(scrutinee, arms, env),
        Expr::Load(path, body) => Ok(leaf(
            TypedExprKind::Load(path.clone(), body.clone()),
            env.fresh_var(),
        )),
        Expr::TypeAlias(name, ty_expr, body) => infer_type_alias(name, ty_expr, body, env),
        Expr::Record(fields) => infer_record(fields, env),
        Expr::FieldAccess(record_expr, field_name) => {
            infer_field_access(record_expr, field_name, env)
        }
        Expr::TypeDef {
            name,
            type_params,
            constructors,
            body,
        } => infer_type_def(name, type_params, constructors, body, env),
        Expr::Constructor(name, args) => infer_constructor(name, args, env),
    }
}

/// Type inference for expressions (type-only wrapper over `infer_typed`).
///
/// # Errors
///
/// Returns a `TypeError` for any static typing violation; see `infer_typed`.
pub fn infer(expr: &Expr, env: &mut TypeEnv) -> Result<(Type, Substitution), TypeError> {
    infer_typed(expr, env).map(|(typed, subst)| (typed.ty, subst))
}

/// Apply `subst` to every `ty` in a typed tree (and every match-arm binding),
/// producing a fully resolved typed AST. `Load` payloads are copied
/// unchanged; only their node `ty` is resolved.
fn resolve_typed(subst: &Substitution, typed: &TypedExpr) -> TypedExpr {
    use TypedExprKind as K;
    let kind = match &typed.kind {
        K::Int(_) | K::Bool(_) | K::Char(_) | K::Float(_) | K::Load(_, _) => typed.kind.clone(),
        K::Var(name, args) => K::Var(
            name.clone(),
            args.iter().map(|arg| apply_subst(subst, arg)).collect(),
        ),
        K::BinOp(op, l, r) => K::BinOp(
            *op,
            Box::new(resolve_typed(subst, l)),
            Box::new(resolve_typed(subst, r)),
        ),
        K::If(c, t, e) => K::If(
            Box::new(resolve_typed(subst, c)),
            Box::new(resolve_typed(subst, t)),
            Box::new(resolve_typed(subst, e)),
        ),
        K::Let(n, ann, scheme, v, b) => K::Let(
            n.clone(),
            ann.clone(),
            resolve_scheme(subst, scheme),
            Box::new(resolve_typed(subst, v)),
            Box::new(resolve_typed(subst, b)),
        ),
        K::Fun(p, ann, b) => K::Fun(p.clone(), ann.clone(), Box::new(resolve_typed(subst, b))),
        K::App(f, a) => K::App(
            Box::new(resolve_typed(subst, f)),
            Box::new(resolve_typed(subst, a)),
        ),
        K::Rec(n, b) => K::Rec(n.clone(), Box::new(resolve_typed(subst, b))),
        K::Match(s, arms) => K::Match(
            Box::new(resolve_typed(subst, s)),
            arms.iter()
                .map(|arm| TypedMatchArm {
                    pattern: resolve_pattern(subst, &arm.pattern),
                    bindings: arm
                        .bindings
                        .iter()
                        .map(|(name, ty)| (name.clone(), apply_subst(subst, ty)))
                        .collect(),
                    body: resolve_typed(subst, &arm.body),
                })
                .collect(),
        ),
        K::Tuple(elems) => K::Tuple(elems.iter().map(|e| resolve_typed(subst, e)).collect()),
        K::TupleProj(t, i) => K::TupleProj(Box::new(resolve_typed(subst, t)), *i),
        K::TypeAlias(n, te, b) => {
            K::TypeAlias(n.clone(), te.clone(), Box::new(resolve_typed(subst, b)))
        }
        K::Record(fields) => K::Record(
            fields
                .iter()
                .map(|(name, e)| (name.clone(), resolve_typed(subst, e)))
                .collect(),
        ),
        K::FieldAccess(r, name) => K::FieldAccess(Box::new(resolve_typed(subst, r)), name.clone()),
        K::TypeDef {
            name,
            type_params,
            param_kinds,
            constructors,
            body,
        } => K::TypeDef {
            name: name.clone(),
            type_params: type_params.clone(),
            param_kinds: param_kinds.clone(),
            constructors: constructors.clone(),
            body: Box::new(resolve_typed(subst, body)),
        },
        K::Constructor(name, args) => K::Constructor(
            name.clone(),
            args.iter().map(|a| resolve_typed(subst, a)).collect(),
        ),
    };
    TypedExpr {
        kind,
        ty: apply_subst(subst, &typed.ty),
    }
}

/// Apply `subst` to a let-generalized `scheme`, leaving its quantified
/// variables untouched so they still name the `Type::Var`s of the value.
fn resolve_scheme(subst: &Substitution, scheme: &TypeScheme) -> TypeScheme {
    let mut inner = subst.clone();
    for var in &scheme.vars {
        inner.remove(var);
    }
    TypeScheme::poly(scheme.vars.clone(), apply_subst(&inner, &scheme.ty))
}

/// Type check an expression, returning the fully resolved typed AST.
///
/// # Errors
///
/// Returns a `TypeError` for any static typing violation; see [`infer`].
pub fn typecheck_typed(expr: &Expr) -> Result<TypedExpr, TypeError> {
    let mut env = TypeEnv::new();
    let (typed, subst) = infer_typed(expr, &mut env)?;
    Ok(resolve_typed(&subst, &typed))
}

/// Public API for type checking
///
/// # Errors
///
/// Returns a `TypeError` if `expr` fails to type-check; see [`infer`] for the
/// specific error conditions.
pub fn typecheck(expr: &Expr) -> Result<Type, TypeError> {
    Ok(typecheck_typed(expr)?.ty)
}

/// Synchronise a cloned environment's fresh-variable counter with an inference result.
///
/// Several inference helpers recurse through cloned `TypeEnv`s. Their fresh variables
/// must not be re-issued by a sibling inference branch that starts from the original
/// environment.
fn advance_next_var_from_inference(env: &mut TypeEnv, ty: &Type, subst: &Substitution) {
    for var in free_type_vars(ty) {
        env.advance_past(&var);
    }
    for (var, replacement) in subst {
        env.advance_past(var);
        for replacement_var in free_type_vars(replacement) {
            env.advance_past(&replacement_var);
        }
    }
}

/// Infer, annotation-check, generalize, and bind one top-level `let` into
/// `program_env`, returning the resolved typed value and its scheme.
fn infer_decl_let(
    name: &str,
    ty_ann: Option<&crate::ast::TypeAnnotation>,
    value: &Expr,
    program_env: &mut TypeEnv,
) -> Result<(TypedExpr, TypeScheme), TypeError> {
    let (typed_value, mut subst) = infer_typed(value, program_env)?;
    let value_ty = typed_value.ty.clone();
    advance_next_var_from_inference(program_env, &value_ty, &subst);
    if let Some(ty_ann) = ty_ann {
        let annotated_ty = resolve_type_annotation(ty_ann, program_env)?;
        let annotation_subst = unify(&value_ty, &annotated_ty)?;
        subst = compose_subst(&annotation_subst, &subst);
    }
    apply_subst_env(&subst, program_env);
    let resolved_value = resolve_typed(&subst, &typed_value);
    let value_ty = apply_subst(&subst, &value_ty);
    let scheme = program_env.generalize(&value_ty);
    program_env.bind(name.to_string(), scheme.clone());
    Ok((resolved_value, scheme))
}

/// Run the final declaration pass, binding each top-level `let` and collecting
/// the typed declarations in source order.
///
/// `data_kinds` holds the kinds inferred for every top-level `data`
/// declaration before any `let` is inferred, so a nested type inside a `let`
/// value cannot change them. `infer_data_kinds` returns an entry for every
/// declaration, so the empty default below is not reached.
fn collect_typed_decls(
    program: &Program,
    program_env: &mut TypeEnv,
    data_kinds: &HashMap<String, Vec<parlang_core::Kind>>,
) -> Result<Vec<TypedDecl>, TypeError> {
    let mut decls = Vec::with_capacity(program.decls.len());
    for decl in &program.decls {
        match decl {
            Decl::Let {
                name,
                ty_ann,
                value,
                doc,
            } => {
                let (value, scheme) = infer_decl_let(name, ty_ann.as_ref(), value, program_env)?;
                decls.push(TypedDecl::Let {
                    name: name.clone(),
                    ty_ann: ty_ann.clone(),
                    scheme,
                    value: Box::new(value),
                    doc: doc.clone(),
                });
            }
            Decl::Data {
                name,
                type_params,
                constructors,
                doc,
            } => decls.push(TypedDecl::Data {
                name: name.clone(),
                type_params: type_params.clone(),
                param_kinds: data_kinds.get(name).cloned().unwrap_or_default(),
                constructors: constructors.clone(),
                doc: doc.clone(),
            }),
            Decl::TypeAlias { name, ty_expr, doc } => decls.push(TypedDecl::TypeAlias {
                name: name.clone(),
                ty_expr: ty_expr.clone(),
                doc: doc.clone(),
            }),
            Decl::Class { name, .. } => {
                return Err(TypeError::ClassesNotSupportedYet(format!("class {name}")));
            }
            Decl::Instance { class, .. } => {
                return Err(TypeError::ClassesNotSupportedYet(format!(
                    "instance {class}"
                )));
            }
        }
    }
    Ok(decls)
}

/// Type check a program, returning the fully resolved typed program AST.
///
/// # Errors
///
/// Returns a `TypeError` if a declaration or the trailing body fails to type-check.
pub fn typecheck_program_typed(program: &Program) -> Result<TypedProgram, TypeError> {
    let mut env = TypeEnv::new();
    typecheck_program_typed_with_env(program, &mut env)
}

/// Type check a top-level program and persist its declarations in `env` on success.
///
/// # Errors
///
/// Returns a `TypeError` if a declaration or the trailing body fails to type-check.
/// The supplied environment is unchanged on error.
pub fn typecheck_program_with_env(program: &Program, env: &mut TypeEnv) -> Result<Type, TypeError> {
    Ok(typecheck_program_typed_with_env(program, env)?.ty)
}

/// Type check a top-level program, threading generalized declarations left to right.
///
/// # Errors
///
/// Returns a `TypeError` if a declaration or the trailing body fails to type-check.
pub fn typecheck_program(program: &Program) -> Result<Type, TypeError> {
    Ok(typecheck_program_typed(program)?.ty)
}

/// Infer the parameter kinds of every top-level `data` declaration as one
/// mutually recursive group, record them in `env`, and return them keyed by
/// type name.
fn define_program_data_kinds(
    program: &Program,
    env: &mut TypeEnv,
) -> Result<HashMap<String, Vec<parlang_core::Kind>>, TypeError> {
    let sigs: Vec<crate::kind_check::DataSig<'_>> = program
        .decls
        .iter()
        .filter_map(|decl| match decl {
            Decl::Data {
                name,
                type_params,
                constructors,
                ..
            } => Some(crate::kind_check::DataSig {
                name,
                params: type_params,
                ctors: constructors,
            }),
            _ => None,
        })
        .collect();
    let mut recorded = HashMap::new();
    for (name, kinds) in crate::kind_check::infer_data_kinds(env, &sigs)? {
        env.define_data_kinds(name.clone(), kinds.clone());
        recorded.insert(name, kinds);
    }
    Ok(recorded)
}

/// Type check a top-level program and persist its declarations in `env` on success,
/// returning the fully resolved typed program AST.
///
/// # Errors
///
/// Returns a `TypeError` if a declaration or the trailing body fails to type-check.
/// The supplied environment is unchanged on error.
pub fn typecheck_program_typed_with_env(
    program: &Program,
    env: &mut TypeEnv,
) -> Result<TypedProgram, TypeError> {
    let mut program_env = env.clone();

    let mut type_names = HashSet::new();
    let mut constructor_names = HashSet::new();
    let mut aliases = HashMap::new();

    for decl in &program.decls {
        match decl {
            Decl::Data {
                name, constructors, ..
            } => {
                if !type_names.insert(name.clone()) {
                    return Err(TypeError::DuplicateTypeName(name.clone()));
                }
                program_env.define_type_alias(name.clone(), Type::SumType(name.clone(), vec![]));
                for (constructor_name, _) in constructors {
                    if !constructor_names.insert(constructor_name.clone()) {
                        return Err(TypeError::DuplicateConstructor(constructor_name.clone()));
                    }
                }
            }
            Decl::TypeAlias { name, ty_expr, .. } => {
                if !type_names.insert(name.clone()) {
                    return Err(TypeError::DuplicateTypeName(name.clone()));
                }
                aliases.insert(name.clone(), ty_expr.clone());
            }
            Decl::Let { .. } => {}
            Decl::Class { name, .. } => {
                return Err(TypeError::ClassesNotSupportedYet(format!("class {name}")));
            }
            Decl::Instance { class, .. } => {
                return Err(TypeError::ClassesNotSupportedYet(format!(
                    "instance {class}"
                )));
            }
        }
    }

    for name in aliases.keys() {
        let ty = resolve_program_type_alias(name, &aliases, &program_env, &mut Vec::new())?;
        program_env.define_type_alias(name.clone(), ty);
    }

    let data_kinds = define_program_data_kinds(program, &mut program_env)?;
    for decl in &program.decls {
        if let Decl::Data {
            name,
            type_params,
            constructors,
            ..
        } = decl
        {
            for (constructor_name, payload_types) in constructors {
                program_env.register_constructor(
                    constructor_name.clone(),
                    ConstructorInfo {
                        type_params: type_params.clone(),
                        payload_types: payload_types.clone(),
                        sum_type_name: name.clone(),
                    },
                );
            }
        }
    }

    let decls = collect_typed_decls(program, &mut program_env, &data_kinds)?;

    let (typed_body, ty) = match &program.body {
        Some(body) => {
            let (typed, subst) = infer_typed(body, &mut program_env)?;
            let resolved = resolve_typed(&subst, &typed);
            let ty = resolved.ty.clone();
            (Some(resolved), ty)
        }
        None => (None, Type::Int),
    };

    *env = program_env;
    Ok(TypedProgram {
        decls,
        body: typed_body,
        ty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_expr as parse;

    #[test]
    fn unify_var_app_binds_head_to_partial_data_type() {
        let applied = Type::VarApp(TypeVar(0), vec![Type::Var(TypeVar(1))]);
        let option_int = Type::SumType("Option".to_string(), vec![Type::Int]);
        let subst = unify(&applied, &option_int).unwrap();
        assert_eq!(apply_subst(&subst, &applied), option_int);
        let head = apply_subst(&subst, &Type::Var(TypeVar(0)));
        assert_eq!(head, Type::SumType("Option".to_string(), vec![]));
    }

    #[test]
    fn unify_var_app_with_longer_var_app_binds_prefix() {
        let short = Type::VarApp(TypeVar(0), vec![Type::Int]);
        let long = Type::VarApp(TypeVar(1), vec![Type::Bool, Type::Int]);
        let subst = unify(&long, &short).unwrap();
        assert_eq!(apply_subst(&subst, &short), apply_subst(&subst, &long));
    }

    #[test]
    fn unify_var_app_rejects_base_types_and_cycles() {
        let applied = Type::VarApp(TypeVar(0), vec![Type::Int]);
        assert!(unify(&applied, &Type::Int).is_err());
        let longer = Type::VarApp(TypeVar(0), vec![Type::Bool, Type::Int]);
        assert!(matches!(
            unify(&applied, &longer),
            Err(TypeError::OccursCheckFailed(..))
        ));
    }

    fn check_typed(source: &str) -> Result<crate::typed_ast::TypedExpr, TypeError> {
        let expr = parse(source).unwrap();
        typecheck_typed(&expr)
    }

    #[test]
    fn test_typed_literal_root() {
        let typed = check_typed("42").unwrap();
        assert_eq!(typed.ty, Type::Int);
        assert!(matches!(
            typed.kind,
            crate::typed_ast::TypedExprKind::Int(42)
        ));
    }

    #[test]
    fn test_typed_app_resolves_func_child() {
        // modelled on test_infer_application
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("(fun x -> x + 1) 41").unwrap();
        assert_eq!(typed.ty, Type::Int);
        let K::App(func, _arg) = &typed.kind else {
            panic!("expected App")
        };
        assert_eq!(func.ty, Type::Fun(Box::new(Type::Int), Box::new(Type::Int)));
    }

    #[test]
    fn test_typed_polymorphic_let_value_keeps_var() {
        // modelled on test_infer_identity (fun x -> x) inside a let
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("let id = fun x -> x in id 1").unwrap();
        assert_eq!(typed.ty, Type::Int);
        let K::Let(_, _, _, value, _) = &typed.kind else {
            panic!("expected Let")
        };
        let K::Fun(_, _, body) = &value.kind else {
            panic!("expected Fun value")
        };
        let Type::Fun(param, ret) = &value.ty else {
            panic!("expected Fun type")
        };
        assert!(matches!(**param, Type::Var(_)), "param should stay a Var");
        assert_eq!(**param, **ret);
        assert_eq!(body.ty, **ret);
    }

    fn check_program(source: &str) -> crate::typed_ast::TypedProgram {
        let program = crate::parser::parse_program(source).unwrap();
        typecheck_program_typed(&program).unwrap()
    }

    #[test]
    fn test_typed_var_poly_instantiation_args() {
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("let id = fun x -> x in id 1").unwrap();
        let K::Let(.., body) = &typed.kind else {
            panic!("expected Let")
        };
        let K::App(func, _) = &body.kind else {
            panic!("expected App body")
        };
        let K::Var(name, args) = &func.kind else {
            panic!("expected Var func")
        };
        assert_eq!(name, "id");
        assert_eq!(args, &vec![Type::Int]);
    }

    #[test]
    fn test_typed_var_monomorphic_args_empty() {
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("(fun x -> x) 1").unwrap();
        let K::App(func, _) = &typed.kind else {
            panic!("expected App")
        };
        let K::Fun(_, _, body) = &func.kind else {
            panic!("expected Fun")
        };
        assert!(matches!(&body.kind, K::Var(n, args) if n == "x" && args.is_empty()));
        let typed = check_typed("let n = 1 in n").unwrap();
        let K::Let(.., body) = &typed.kind else {
            panic!("expected Let")
        };
        assert!(matches!(&body.kind, K::Var(n, args) if n == "n" && args.is_empty()));
    }

    #[test]
    fn test_typed_var_top_level_decl_args() {
        use crate::typed_ast::{TypedDecl, TypedExprKind as K};
        let program = check_program("let id = fun x -> x;\nid 'c'");
        let TypedDecl::Let { scheme, .. } = &program.decls[0] else {
            panic!("expected Let decl")
        };
        assert_eq!(scheme.vars.len(), 1);
        let body = program.body.unwrap();
        let K::App(func, _) = &body.kind else {
            panic!("expected App")
        };
        let K::Var(_, args) = &func.kind else {
            panic!("expected Var")
        };
        assert_eq!(args, &vec![Type::Char]);
    }

    /// A top-level `data` declaration carries its inferred parameter kinds.
    #[test]
    fn test_typed_data_decl_param_kinds() {
        use crate::typed_ast::TypedDecl;
        use parlang_core::Kind;
        let program = check_program("data Wrap f a = MkWrap (f a);\n1");
        let TypedDecl::Data { param_kinds, .. } = &program.decls[0] else {
            panic!("expected Data decl")
        };
        let expected = vec![Kind::arrow(Kind::Star, Kind::Star), Kind::Star];
        assert_eq!(param_kinds, &expected);
    }

    /// A nested same-name `data` in the body keeps the outer kinds intact.
    #[test]
    fn test_typed_type_def_kinds_survive_nested_redefinition() {
        use crate::typed_ast::TypedExprKind as K;
        use parlang_core::Kind;
        let typed = check_typed("data W f = MkW (f Int) in data W a = MkA a in 0").unwrap();
        let K::TypeDef {
            param_kinds, body, ..
        } = &typed.kind
        else {
            panic!("expected outer TypeDef")
        };
        assert_eq!(param_kinds, &vec![Kind::arrow(Kind::Star, Kind::Star)]);
        let K::TypeDef {
            param_kinds: inner, ..
        } = &body.kind
        else {
            panic!("expected inner TypeDef")
        };
        assert_eq!(inner, &vec![Kind::Star]);
    }

    #[test]
    fn test_typed_match_arm_bindings() {
        // tuple-pattern match: `match (1, 2) with | (a, b) -> a + b`
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("match (1, 2) with | (a, b) -> a + b").unwrap();
        let K::Match(_, arms) = &typed.kind else {
            panic!("expected Match")
        };
        assert!(!arms[0].bindings.is_empty());
        assert!(arms[0].bindings.iter().all(|(_, ty)| *ty == Type::Int));
    }

    #[test]
    fn test_typed_hidden_let_var_not_reissued() {
        // `(fun z -> 1 + (let f = fun x -> 2 in 3)) true`
        // Without the infer_binop next_var sync, the hidden `fun x -> 2` param var
        // is reissued as the app's result var and then bound to Int by the root
        // subst, corrupting the inner let-value function's parameter type.
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("(fun z -> 1 + (let f = fun x -> 2 in 3)) true").unwrap();
        let K::App(func, _arg) = &typed.kind else {
            panic!("expected App")
        };
        let K::Fun(_, _, body) = &func.kind else {
            panic!("expected Fun func")
        };
        let K::BinOp(_, _, right) = &body.kind else {
            panic!("expected BinOp body")
        };
        let K::Let(_, _, _, value, _) = &right.kind else {
            panic!("expected Let right")
        };
        let K::Fun(_, _, _) = &value.kind else {
            panic!("expected Fun value")
        };
        let Type::Fun(param, ret) = &value.ty else {
            panic!("expected Fun type")
        };
        assert!(matches!(**param, Type::Var(_)), "param should stay a Var");
        assert_eq!(**ret, Type::Int);
    }

    /// Walk `expr` and assert that every expression-level `Let` scheme
    /// quantifies exactly the free type vars of its resolved value type
    /// (valid for lets whose value captures no monomorphic outer variable).
    fn assert_closed_let_schemes(expr: &crate::typed_ast::TypedExpr) {
        use crate::typed_ast::TypedExprKind as K;
        match &expr.kind {
            K::Let(_, _, scheme, value, body) => {
                let vars: std::collections::HashSet<TypeVar> =
                    scheme.vars.iter().cloned().collect();
                assert_eq!(
                    free_type_vars(&value.ty),
                    vars,
                    "let scheme/value vars differ"
                );
                assert_closed_let_schemes(value);
                assert_closed_let_schemes(body);
            }
            K::Fun(_, _, inner) | K::Rec(_, inner) => assert_closed_let_schemes(inner),
            K::App(left, right) | K::BinOp(_, left, right) => {
                assert_closed_let_schemes(left);
                assert_closed_let_schemes(right);
            }
            _ => {}
        }
    }

    #[test]
    fn test_typed_let_scheme_one_var() {
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("let id = fun x -> x in id 1").unwrap();
        let K::Let(_, _, scheme, value, body) = &typed.kind else {
            panic!("expected Let")
        };
        assert_eq!(scheme.vars.len(), 1);
        let var = Type::Var(scheme.vars[0].clone());
        assert_eq!(value.ty, Type::Fun(Box::new(var.clone()), Box::new(var)));
        let K::App(func, _) = &body.kind else {
            panic!("expected App body")
        };
        let K::Var(_, args) = &func.kind else {
            panic!("expected Var func")
        };
        assert_eq!(args, &vec![Type::Int]);
    }

    #[test]
    fn test_typed_let_scheme_two_vars_args_order() {
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("let k = fun x -> fun y -> x in k 1 true").unwrap();
        let K::Let(_, _, scheme, _, body) = &typed.kind else {
            panic!("expected Let")
        };
        assert_eq!(scheme.vars.len(), 2);
        let Type::Fun(x_ty, _) = &scheme.ty else {
            panic!("expected Fun scheme")
        };
        let Type::Var(x_var) = &**x_ty else {
            panic!("expected Var param")
        };
        let pos_x = scheme.vars.iter().position(|v| v == x_var).unwrap();
        let K::App(inner, _) = &body.kind else {
            panic!("expected outer App")
        };
        let K::App(func, _) = &inner.kind else {
            panic!("expected inner App")
        };
        let K::Var(_, args) = &func.kind else {
            panic!("expected Var func")
        };
        assert_eq!(args.len(), 2);
        assert_eq!(args[pos_x], Type::Int);
        assert_eq!(args[1 - pos_x], Type::Bool);
    }

    #[test]
    fn test_typed_let_scheme_monomorphic() {
        use crate::typed_ast::TypedExprKind as K;
        let typed = check_typed("let n = 1 in n").unwrap();
        let K::Let(_, _, scheme, _, _) = &typed.kind else {
            panic!("expected Let")
        };
        assert!(scheme.vars.is_empty());
        assert_eq!(scheme.ty, Type::Int);
    }

    #[test]
    fn test_typed_let_schemes_match_value_vars() {
        let typed = check_typed("let id = fun x -> x in id 1").unwrap();
        assert_closed_let_schemes(&typed);
        let source = "let f = fun x -> let g = fun y -> y in g x in f 1";
        let typed = check_typed(source).unwrap();
        assert_closed_let_schemes(&typed);
    }

    #[test]
    fn test_typed_decl_scheme_matches_value_vars() {
        use crate::typed_ast::TypedDecl;
        let program = check_program("let id = fun x -> x;\nid 'c'");
        let TypedDecl::Let { scheme, value, .. } = &program.decls[0] else {
            panic!("expected Let decl")
        };
        assert_eq!(value.ty, scheme.ty);
        let vars: std::collections::HashSet<TypeVar> = scheme.vars.iter().cloned().collect();
        assert_eq!(free_type_vars(&value.ty), vars);
    }

    #[test]
    fn test_string_literal_without_list_decl_is_unknown_constructor() {
        // "ab" desugars to Cons 'a' (Cons 'b' Nil); with no `data List` it is rejected.
        let err = check_typed("\"ab\"");
        assert_eq!(err, Err(TypeError::UnknownConstructor("Cons".to_string())));
    }

    #[test]
    fn test_typed_constructor_pattern_types() {
        use crate::typed_ast::{TypedExprKind as K, TypedPatternKind as P};
        let program = check_program(
            "data Option a = Some a | None;\nmatch Some 1 with | Some n -> n | None -> 0",
        );
        let body = program.body.unwrap();
        let K::Match(_, arms) = &body.kind else {
            panic!("expected Match")
        };
        let option_int = Type::SumType("Option".to_string(), vec![Type::Int]);
        assert_eq!(arms[0].pattern.ty, option_int);
        let P::Constructor(name, subs) = &arms[0].pattern.kind else {
            panic!("expected Some pattern")
        };
        assert_eq!(name, "Some");
        assert_eq!(subs.len(), 1);
        assert!(matches!(&subs[0].kind, P::Var(n) if n == "n"));
        assert_eq!(subs[0].ty, Type::Int);
        assert_eq!(arms[1].pattern.ty, option_int);
        let P::Constructor(none_name, none_subs) = &arms[1].pattern.kind else {
            panic!("expected None pattern")
        };
        assert_eq!(none_name, "None");
        assert!(none_subs.is_empty());
    }

    #[test]
    fn test_typed_tuple_pattern_types() {
        use crate::typed_ast::{TypedExprKind as K, TypedPatternKind as P};
        let typed = check_typed("match (1, true) with | (a, _) -> a").unwrap();
        let K::Match(_, arms) = &typed.kind else {
            panic!("expected Match")
        };
        let pattern = &arms[0].pattern;
        assert_eq!(pattern.ty, Type::Tuple(vec![Type::Int, Type::Bool]));
        let P::Tuple(items) = &pattern.kind else {
            panic!("expected Tuple pattern")
        };
        assert!(matches!(&items[0].kind, P::Var(n) if n == "a"));
        assert_eq!(items[0].ty, Type::Int);
        assert!(matches!(items[1].kind, P::Wildcard));
        assert_eq!(items[1].ty, Type::Bool);
        let erased = Pattern::Tuple(vec![Pattern::Var("a".to_string()), Pattern::Wildcard]);
        assert_eq!(pattern.to_pattern(), erased);
    }

    #[test]
    fn test_typed_unit_pattern_type() {
        use crate::typed_ast::{TypedExprKind as K, TypedPatternKind as P};
        let typed = check_typed("match () with | () -> 1").unwrap();
        let K::Match(_, arms) = &typed.kind else {
            panic!("expected Match")
        };
        assert_eq!(arms[0].pattern.ty, Type::Unit);
        let P::Tuple(items) = &arms[0].pattern.kind else {
            panic!("expected unit Tuple pattern")
        };
        assert!(items.is_empty());
    }

    #[test]
    fn test_typed_literal_pattern_type() {
        use crate::typed_ast::{TypedExprKind as K, TypedPatternKind as P};
        let typed = check_typed("match 3 with | 0 -> 1 | _ -> 2").unwrap();
        let K::Match(_, arms) = &typed.kind else {
            panic!("expected Match")
        };
        assert!(matches!(arms[0].pattern.kind, P::Literal(Literal::Int(0))));
        assert_eq!(arms[0].pattern.ty, Type::Int);
        assert!(matches!(arms[1].pattern.kind, P::Wildcard));
        assert_eq!(arms[1].pattern.ty, Type::Int);
    }

    #[test]
    fn test_unknown_constructor_errors_before_args() {
        // The constructor is rejected before its arguments are inferred.
        let err = check_typed("Foo undefinedVar");
        assert_eq!(err, Err(TypeError::UnknownConstructor("Foo".to_string())));
    }

    #[test]
    fn test_constructor_args_do_not_share_app_result_vars() {
        // `f 1` and `g true` are independent applications inside constructor
        // arguments; their result types must be distinct type variables.
        let program = crate::parser::parse_program(
            "data Pair a b = Foo a b ; fun f -> fun g -> Foo (f 1) (g true)",
        )
        .unwrap();
        let ty = typecheck_program(&program).unwrap();
        let Type::Fun(f_ty, rest) = &ty else {
            panic!("expected Fun")
        };
        let Type::Fun(g_ty, _) = &**rest else {
            panic!("expected Fun for g")
        };
        let Type::Fun(_, f_ret) = &**f_ty else {
            panic!("expected f: Int -> _")
        };
        let Type::Fun(_, g_ret) = &**g_ty else {
            panic!("expected g: Bool -> _")
        };
        assert_ne!(
            f_ret, g_ret,
            "f and g must not share a result type variable"
        );
    }

    fn check(source: &str) -> Result<Type, TypeError> {
        let expr = parse(source).unwrap();
        typecheck(&expr)
    }

    #[test]
    fn test_infer_literals() {
        assert_eq!(check("42").unwrap(), Type::Int);
        assert_eq!(check("true").unwrap(), Type::Bool);
        assert_eq!(check("false").unwrap(), Type::Bool);
    }

    #[test]
    fn test_infer_arithmetic() {
        assert_eq!(check("1 + 2").unwrap(), Type::Int);
        assert_eq!(check("10 * 3").unwrap(), Type::Int);
        assert_eq!(check("20 - 5").unwrap(), Type::Int);
        assert_eq!(check("100 / 10").unwrap(), Type::Int);
    }

    #[test]
    fn test_infer_comparison() {
        assert_eq!(check("5 > 3").unwrap(), Type::Bool);
        assert_eq!(check("5 >= 3").unwrap(), Type::Bool);
        assert_eq!(check("5 < 3").unwrap(), Type::Bool);
        assert_eq!(check("5 <= 3").unwrap(), Type::Bool);
        assert_eq!(check("1 == 1").unwrap(), Type::Bool);
        assert_eq!(check("1 != 2").unwrap(), Type::Bool);
    }

    #[test]
    fn test_infer_if() {
        assert_eq!(check("if true then 1 else 2").unwrap(), Type::Int);
        assert_eq!(check("if 5 > 3 then 10 else 0").unwrap(), Type::Int);
        assert_eq!(check("if false then true else false").unwrap(), Type::Bool);
    }

    #[test]
    fn test_infer_function() {
        let ty = check("fun x -> x + 1").unwrap();
        assert!(matches!(ty, Type::Fun(_, _)));
        if let Type::Fun(arg, ret) = ty {
            assert_eq!(*arg, Type::Int);
            assert_eq!(*ret, Type::Int);
        }
    }

    #[test]
    fn test_infer_identity() {
        let ty = check("fun x -> x").unwrap();
        // Should infer: t0 -> t0 (some type variable)
        assert!(matches!(ty, Type::Fun(_, _)));
        if let Type::Fun(arg, ret) = ty {
            assert_eq!(arg, ret);
        }
    }

    #[test]
    fn test_infer_application() {
        assert_eq!(check("(fun x -> x + 1) 41").unwrap(), Type::Int);
    }

    #[test]
    fn test_infer_let_polymorphism() {
        // id should be polymorphic: forall a. a -> a
        let result = check("let id = fun x -> x in id 42");
        assert_eq!(result.unwrap(), Type::Int);

        let result = check("let id = fun x -> x in id true");
        assert_eq!(result.unwrap(), Type::Bool);
    }

    #[test]
    fn test_infer_let_polymorphism_multiple_uses() {
        // id is used at both Int and Bool types in the same expression
        let result = check("let id = fun x -> x in if id true then id 1 else id 2");
        assert_eq!(result.unwrap(), Type::Int);
    }

    #[test]
    fn test_error_type_mismatch() {
        assert!(check("1 + true").is_err());
        assert!(check("if 1 then 2 else 3").is_err());
        assert!(check("if true then 1 else false").is_err());
    }

    #[test]
    fn test_error_unbound_variable() {
        assert!(matches!(check("x + 1"), Err(TypeError::UnboundVariable(_))));
        assert!(matches!(check("y"), Err(TypeError::UnboundVariable(_))));
    }

    #[test]
    fn test_currying() {
        let ty = check("fun x -> fun y -> x + y").unwrap();
        // Should be: Int -> Int -> Int
        assert!(matches!(ty, Type::Fun(_, _)));
        if let Type::Fun(arg1, rest) = ty {
            assert_eq!(*arg1, Type::Int);
            assert!(matches!(*rest, Type::Fun(_, _)));
            if let Type::Fun(arg2, ret) = *rest {
                assert_eq!(*arg2, Type::Int);
                assert_eq!(*ret, Type::Int);
            }
        }
    }

    #[test]
    fn test_partial_application() {
        let ty = check("let add = fun x -> fun y -> x + y in add 5").unwrap();
        // Should be: Int -> Int
        assert!(matches!(ty, Type::Fun(_, _)));
        if let Type::Fun(arg, ret) = ty {
            assert_eq!(*arg, Type::Int);
            assert_eq!(*ret, Type::Int);
        }
    }

    #[test]
    fn test_complex_expression() {
        // Tests nested let, function application, and polymorphism
        let ty = check("let f = fun x -> x + 1 in let g = fun y -> y in g (f 10)").unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_higher_order_function() {
        // apply: (a -> b) -> a -> b
        let ty = check("fun f -> fun x -> f x").unwrap();
        assert!(matches!(ty, Type::Fun(_, _)));
    }

    #[test]
    fn test_compose_functions() {
        // compose specialized to Int functions
        let ty = check("fun f -> fun g -> fun x -> f (g x)").unwrap();
        assert!(matches!(ty, Type::Fun(_, _)));
    }

    #[test]
    fn test_const_function() {
        // const: a -> b -> a
        let ty = check("fun x -> fun y -> x").unwrap();
        assert!(matches!(ty, Type::Fun(_, _)));
    }

    #[test]
    fn test_boolean_function() {
        let ty = check("fun x -> if x then 1 else 0").unwrap();
        assert!(matches!(ty, Type::Fun(_, _)));
        if let Type::Fun(arg, ret) = ty {
            assert_eq!(*arg, Type::Bool);
            assert_eq!(*ret, Type::Int);
        }
    }

    #[test]
    fn test_nested_if() {
        let ty = check("if true then (if false then 1 else 2) else 3").unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_let_in_let() {
        let ty = check("let x = 10 in let y = 20 in x + y").unwrap();
        assert_eq!(ty, Type::Int);
    }

    #[test]
    fn test_equality_polymorphic() {
        // Equality should work on Int
        assert_eq!(check("1 == 2").unwrap(), Type::Bool);
        // Equality should work on Bool
        assert_eq!(check("true == false").unwrap(), Type::Bool);
    }

    #[test]
    fn test_rec_simple() {
        // Test that recursive functions are now supported
        let ty = check("rec f -> fun n -> if n == 0 then 1 else n").unwrap();
        assert_eq!(ty, Type::Fun(Box::new(Type::Int), Box::new(Type::Int)));
    }

    #[test]
    fn test_rec_factorial() {
        // Test factorial: rec f -> fun n -> if n == 0 then 1 else n * f (n - 1)
        let ty = check("rec f -> fun n -> if n == 0 then 1 else n * f (n - 1)").unwrap();
        assert_eq!(ty, Type::Fun(Box::new(Type::Int), Box::new(Type::Int)));
    }
}
