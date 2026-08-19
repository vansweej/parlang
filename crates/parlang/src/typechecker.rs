/// Hindley-Milner type inference implementation
use crate::ast::{BinOp, Decl, Expr, Program};
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
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            bindings: HashMap::new(),
            next_var: 0,
            type_aliases: HashMap::new(),
            constructors: HashMap::new(),
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

    /// Bind a variable to a type scheme
    pub fn bind(&mut self, name: String, scheme: TypeScheme) {
        self.bindings.insert(name, scheme);
    }

    /// Extend environment with a monomorphic binding
    #[must_use]
    pub fn extend(&self, name: String, ty: Type) -> Self {
        let mut new_env = self.clone();
        new_env.bind(name, TypeScheme { vars: vec![], ty });
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

    /// Generalize a type by quantifying free type variables
    pub fn generalize(&self, ty: &Type) -> TypeScheme {
        let free_in_env = self.free_vars();
        let free_in_type = free_type_vars(ty);

        let mut quantified: Vec<TypeVar> = free_in_type.difference(&free_in_env).cloned().collect();
        quantified.sort();

        TypeScheme {
            vars: quantified,
            ty: ty.clone(),
        }
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

    /// Look up constructor information
    pub fn lookup_constructor(&self, name: &str) -> Option<&ConstructorInfo> {
        self.constructors.get(name)
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
            if name1 != name2 {
                return Err(TypeError::UnificationError(
                    Box::new(t1.clone()),
                    Box::new(t2.clone()),
                ));
            }

            if args1.len() != args2.len() {
                return Err(TypeError::UnificationError(
                    Box::new(t1.clone()),
                    Box::new(t2.clone()),
                ));
            }

            // Unify all type arguments
            let mut subst = HashMap::new();
            for (type_arg1, type_arg2) in args1.iter().zip(args2.iter()) {
                let type_arg1 = apply_subst(&subst, type_arg1);
                let type_arg2 = apply_subst(&subst, type_arg2);
                let s = unify(&type_arg1, &type_arg2)?;
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

/// Convert a `TypeAnnotation` to a Type, resolving names to concrete types
fn resolve_type_annotation(
    ty_ann: &crate::ast::TypeAnnotation,
    env: &mut TypeEnv,
) -> Result<Type, TypeError> {
    match ty_ann {
        crate::ast::TypeAnnotation::Concrete(name) => {
            // Check if it's a basic type
            match name.as_str() {
                "Int" => Ok(Type::Int),
                "Bool" => Ok(Type::Bool),
                "Char" => Ok(Type::Char),
                "Float" => Ok(Type::Float),
                _ => {
                    // Try to resolve as type alias
                    env.resolve_type_alias(name)
                        .ok_or_else(|| TypeError::UnboundVariable(name.clone()))
                }
            }
        }
        crate::ast::TypeAnnotation::Var(_name) => {
            // Type variables in annotations become fresh type variables
            // This allows polymorphic annotations like: fun (x : a) -> x
            Ok(env.fresh_var())
        }
        crate::ast::TypeAnnotation::Fun(arg, ret) => {
            let arg_ty = resolve_type_annotation(arg, env)?;
            let ret_ty = resolve_type_annotation(ret, env)?;
            Ok(Type::Fun(Box::new(arg_ty), Box::new(ret_ty)))
        }
        crate::ast::TypeAnnotation::App(name, _args) => {
            // For now, we don't support applied types in annotations
            // This would require tracking type constructors
            Err(TypeError::UnboundVariable(format!(
                "Applied type not yet supported in annotations: {name}"
            )))
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
) -> Result<(Type, Substitution), TypeError> {
    let (left_ty, s1) = infer(left, env)?;
    advance_next_var_from_inference(env, &left_ty, &s1);
    let mut env1 = env.clone();
    apply_subst_env(&s1, &mut env1);

    let (right_ty, s2) = infer(right, &mut env1)?;
    let left_ty = apply_subst(&s2, &left_ty);

    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
            infer_arith_binop(left_ty, &right_ty, &s1, &s2)
        }
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            infer_ordering_binop(left_ty, &right_ty, &s1, &s2)
        }
        BinOp::Eq | BinOp::Neq => {
            // Equality works on any type, but both sides must match
            let s3 = unify(&left_ty, &right_ty)?;
            let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
            Ok((Type::Bool, subst))
        }
    }
}

/// Type inference for `let name [: Ty] = value in body` (extracted from
/// `infer` to keep its line count down).
fn infer_let(
    name: &str,
    ty_ann_opt: Option<&crate::ast::TypeAnnotation>,
    value: &Expr,
    body: &Expr,
    env: &mut TypeEnv,
) -> Result<(Type, Substitution), TypeError> {
    let (value_ty, s1) = infer(value, env)?;
    advance_next_var_from_inference(env, &value_ty, &s1);

    // If there's a type annotation, check it matches the inferred type
    if let Some(ty_ann) = ty_ann_opt {
        let annotated_ty = resolve_type_annotation(ty_ann, env)?;
        let s_ann = unify(&value_ty, &annotated_ty)?;
        let s1 = compose_subst(&s_ann, &s1);

        let mut env1 = env.clone();
        apply_subst_env(&s1, &mut env1);

        let unified_ty = apply_subst(&s1, &value_ty);
        let scheme = env1.generalize(&unified_ty);
        env1.bind(name.to_string(), scheme);

        let (body_ty, s2) = infer(body, &mut env1)?;
        advance_next_var_from_inference(&mut env1, &body_ty, &s2);
        env.next_var = env.next_var.max(env1.next_var);

        let subst = compose_subst(&s2, &s1);
        Ok((body_ty, subst))
    } else {
        let mut env1 = env.clone();
        apply_subst_env(&s1, &mut env1);

        // Generalize the type (let-polymorphism)
        let scheme = env1.generalize(&value_ty);
        env1.bind(name.to_string(), scheme);

        let (body_ty, s2) = infer(body, &mut env1)?;
        advance_next_var_from_inference(&mut env1, &body_ty, &s2);
        env.next_var = env.next_var.max(env1.next_var);

        let subst = compose_subst(&s2, &s1);
        Ok((body_ty, subst))
    }
}

/// Type inference for `record_expr.field_name` (extracted from `infer` to
/// keep its line count down).
fn infer_field_access(
    record_expr: &Expr,
    field_name: &str,
    env: &mut TypeEnv,
) -> Result<(Type, Substitution), TypeError> {
    // Infer the type of the record expression
    let (record_ty, s1) = infer(record_expr, env)?;

    // Apply substitution to get concrete record type
    let record_ty = apply_subst(&s1, &record_ty);

    match record_ty {
        Type::Record(fields) => {
            // Look up the field type
            if let Some(field_ty) = fields.get(field_name) {
                Ok((field_ty.clone(), s1))
            } else {
                let available: Vec<String> = fields.keys().cloned().collect();
                Err(TypeError::FieldNotFound(field_name.to_string(), available))
            }
        }
        _ => Err(TypeError::RecordExpected(format!("{record_ty}"))),
    }
}

/// Type inference for a sum-type constructor application (extracted from
/// `infer` to keep its line count down).
fn infer_constructor(
    name: &str,
    args: &[Expr],
    env: &mut TypeEnv,
) -> Result<(Type, Substitution), TypeError> {
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

        for arg in args {
            let (arg_ty, s) = infer(arg, env)?;
            subst = compose_subst(&s, &subst);
            arg_types.push(apply_subst(&subst, &arg_ty));
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
        Ok((result_ty, subst))
    } else {
        // Constructor not registered - return a fresh type variable
        // This maintains backward compatibility
        Ok((env.fresh_var(), HashMap::new()))
    }
}

/// Type inference for `if cond then then_br else else_br` (extracted from
/// `infer` to keep its line count down).
fn infer_if(
    cond: &Expr,
    then_br: &Expr,
    else_br: &Expr,
    env: &mut TypeEnv,
) -> Result<(Type, Substitution), TypeError> {
    let (cond_ty, s1) = infer(cond, env)?;
    let s2 = unify(&cond_ty, &Type::Bool)?;
    advance_next_var_from_inference(env, &cond_ty, &s1);

    let mut env1 = env.clone();
    apply_subst_env(&compose_subst(&s2, &s1), &mut env1);

    let (then_ty, s3) = infer(then_br, &mut env1)?;
    advance_next_var_from_inference(&mut env1, &then_ty, &s3);

    let mut env2 = env1.clone();
    apply_subst_env(&s3, &mut env2);

    let (else_ty, s4) = infer(else_br, &mut env2)?;
    advance_next_var_from_inference(&mut env2, &else_ty, &s4);
    env.next_var = env.next_var.max(env2.next_var);

    let then_ty = apply_subst(&s4, &then_ty);
    let s5 = unify(&then_ty, &else_ty)?;

    let result_ty = apply_subst(&s5, &then_ty);
    let subst = compose_subst(
        &s5,
        &compose_subst(&s4, &compose_subst(&s3, &compose_subst(&s2, &s1))),
    );

    Ok((result_ty, subst))
}

/// Type inference for function application `func arg` (extracted from
/// `infer` to keep its line count down).
fn infer_app(
    func: &Expr,
    arg: &Expr,
    env: &mut TypeEnv,
) -> Result<(Type, Substitution), TypeError> {
    let (func_ty, s1) = infer(func, env)?;
    advance_next_var_from_inference(env, &func_ty, &s1);

    let mut env1 = env.clone();
    apply_subst_env(&s1, &mut env1);

    let (arg_ty, s2) = infer(arg, &mut env1)?;
    advance_next_var_from_inference(&mut env1, &arg_ty, &s2);

    let func_ty = apply_subst(&s2, &func_ty);
    let result_ty = env1.fresh_var();

    let s3 = unify(
        &func_ty,
        &Type::Fun(Box::new(arg_ty), Box::new(result_ty.clone())),
    )?;

    let result_ty = apply_subst(&s3, &result_ty);
    let subst = compose_subst(&s3, &compose_subst(&s2, &s1));

    Ok((result_ty, subst))
}

/// Type inference for `rec name -> body` via fixpoint typing (extracted from
/// `infer` to keep its line count down).
fn infer_rec(
    name: &str,
    body: &Expr,
    env: &mut TypeEnv,
) -> Result<(Type, Substitution), TypeError> {
    // For recursive functions, we use fixpoint typing:
    // 1. Generate a fresh type variable for the recursive function
    // 2. Add it to the environment before checking the body
    // 3. Infer the type of the body with the recursive name bound
    // 4. Unify the inferred type with the assumed type

    let rec_ty = env.fresh_var();
    let mut extended_env = env.extend(name.to_string(), rec_ty.clone());

    let (body_ty, subst) = infer(body, &mut extended_env)?;
    advance_next_var_from_inference(&mut extended_env, &body_ty, &subst);
    env.next_var = env.next_var.max(extended_env.next_var);

    // The body type should be the same as the recursive function type
    // (after applying the substitution from inferring the body)
    let rec_ty = apply_subst(&subst, &rec_ty);
    let s2 = unify(&rec_ty, &body_ty)?;

    let final_ty = apply_subst(&s2, &body_ty);
    let final_subst = compose_subst(&s2, &subst);

    Ok((final_ty, final_subst))
}

/// Type inference for `fun param [: Ty] -> body` (extracted from `infer` to
/// keep its line count down).
fn infer_fun(
    param: &str,
    ty_ann_opt: Option<&crate::ast::TypeAnnotation>,
    body: &Expr,
    env: &mut TypeEnv,
) -> Result<(Type, Substitution), TypeError> {
    // Use annotated type if provided, otherwise create fresh variable
    let param_ty = if let Some(ty_ann) = ty_ann_opt {
        resolve_type_annotation(ty_ann, env)?
    } else {
        env.fresh_var()
    };

    let mut env1 = env.clone();
    env1 = env1.extend(param.to_string(), param_ty.clone());

    let (body_ty, s1) = infer(body, &mut env1)?;
    advance_next_var_from_inference(&mut env1, &body_ty, &s1);
    env.next_var = env.next_var.max(env1.next_var);
    let param_ty = apply_subst(&s1, &param_ty);

    Ok((Type::Fun(Box::new(param_ty), Box::new(body_ty)), s1))
}

/// Type inference for expressions
///
/// # Errors
///
/// Returns a `TypeError` if `expr` contains an unbound variable, a type
/// mismatch that cannot be unified, an occurs-check failure, a constructor
/// applied with the wrong number of arguments, or any other static typing
/// violation detected during inference.
pub fn infer(expr: &Expr, env: &mut TypeEnv) -> Result<(Type, Substitution), TypeError> {
    match expr {
        Expr::Int(_) => Ok((Type::Int, HashMap::new())),

        Expr::Bool(_) => Ok((Type::Bool, HashMap::new())),

        Expr::Char(_) => Ok((Type::Char, HashMap::new())),

        Expr::Float(_) => Ok((Type::Float, HashMap::new())),

        Expr::Var(name) => {
            let ty = env
                .lookup(name)
                .ok_or_else(|| TypeError::UnboundVariable(name.clone()))?;
            Ok((ty, HashMap::new()))
        }

        Expr::BinOp(op, left, right) => infer_binop(*op, left, right, env),

        Expr::If(cond, then_br, else_br) => infer_if(cond, then_br, else_br, env),

        Expr::Let(name, ty_ann_opt, value, body) => {
            infer_let(name, ty_ann_opt.as_ref(), value, body, env)
        }

        Expr::Fun(param, ty_ann_opt, body) => infer_fun(param, ty_ann_opt.as_ref(), body, env),

        Expr::App(func, arg) => infer_app(func, arg, env),

        Expr::Rec(name, body) => infer_rec(name, body, env),

        Expr::Tuple(elements) => {
            // Empty tuple is the unit type ()
            if elements.is_empty() {
                return Ok((Type::Unit, HashMap::new()));
            }
            // For non-empty tuples, return a type variable for now
            // (full tuple type implementation is planned)
            Ok((env.fresh_var(), HashMap::new()))
        }

        Expr::TupleProj(_, _) => {
            // For now, return a type variable for tuple projection
            Ok((env.fresh_var(), HashMap::new()))
        }

        Expr::Match(_, _) => {
            // For now, return a type variable for pattern matching
            Ok((env.fresh_var(), HashMap::new()))
        }

        Expr::Load(_, _) => {
            // For now, return a type variable for load expressions
            Ok((env.fresh_var(), HashMap::new()))
        }

        Expr::TypeAlias(name, ty_expr, body) => {
            // Resolve the type expression to a Type
            let ty = resolve_type_expr(ty_expr, env)?;

            // Define the type alias in the environment
            let mut new_env = env.clone();
            new_env.define_type_alias(name.clone(), ty);

            // Infer the type of the body with the extended environment
            infer(body, &mut new_env)
        }

        Expr::Record(fields) => {
            // Infer types for all field expressions
            let mut field_types = HashMap::new();
            let mut subst = HashMap::new();

            for (name, expr) in fields {
                let (ty, s) = infer(expr, env)?;

                // Apply accumulated substitution to the type
                let ty = apply_subst(&subst, &ty);

                // Compose substitutions
                subst = compose_subst(&s, &subst);

                // Apply substitution to environment for next field
                apply_subst_env(&s, env);

                field_types.insert(name.clone(), ty);
            }

            Ok((Type::Record(field_types), subst))
        }

        Expr::FieldAccess(record_expr, field_name) => {
            infer_field_access(record_expr, field_name, env)
        }

        Expr::TypeDef {
            name,
            type_params,
            constructors,
            body,
        } => {
            // Register constructors in the environment
            for (ctor_name, payload_types) in constructors {
                let info = ConstructorInfo {
                    type_params: type_params.clone(),
                    payload_types: payload_types.clone(),
                    sum_type_name: name.clone(),
                };
                env.register_constructor(ctor_name.clone(), info);
            }

            // Type check the body with constructors available
            infer(body, env)
        }

        Expr::Constructor(name, args) => infer_constructor(name, args, env),
    }
}

/// Public API for type checking
///
/// # Errors
///
/// Returns a `TypeError` if `expr` fails to type-check; see [`infer`] for the
/// specific error conditions.
pub fn typecheck(expr: &Expr) -> Result<Type, TypeError> {
    let mut env = TypeEnv::new();
    let (ty, subst) = infer(expr, &mut env)?;
    Ok(apply_subst(&subst, &ty))
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

/// Type check a top-level program, threading generalized declarations left to right.
///
/// # Errors
///
/// Returns a `TypeError` if a declaration or the trailing body fails to type-check.
pub fn typecheck_program(program: &Program) -> Result<Type, TypeError> {
    let mut env = TypeEnv::new();
    typecheck_program_with_env(program, &mut env)
}

/// Type check a top-level program and persist its declarations in `env` on success.
///
/// # Errors
///
/// Returns a `TypeError` if a declaration or the trailing body fails to type-check.
/// The supplied environment is unchanged on error.
pub fn typecheck_program_with_env(program: &Program, env: &mut TypeEnv) -> Result<Type, TypeError> {
    let mut program_env = env.clone();

    for decl in &program.decls {
        match decl {
            Decl::Let {
                name,
                ty_ann,
                value,
                ..
            } => {
                let (value_ty, mut subst) = infer(value, &mut program_env)?;
                advance_next_var_from_inference(&mut program_env, &value_ty, &subst);
                if let Some(ty_ann) = ty_ann {
                    let annotated_ty = resolve_type_annotation(ty_ann, &mut program_env)?;
                    let annotation_subst = unify(&value_ty, &annotated_ty)?;
                    subst = compose_subst(&annotation_subst, &subst);
                }
                apply_subst_env(&subst, &mut program_env);
                let value_ty = apply_subst(&subst, &value_ty);
                let scheme = program_env.generalize(&value_ty);
                program_env.bind(name.clone(), scheme);
            }
        }
    }

    let ty = match &program.body {
        Some(body) => {
            let (ty, subst) = infer(body, &mut program_env)?;
            Ok(apply_subst(&subst, &ty))
        }
        None => Ok(Type::Int),
    }?;
    *env = program_env;
    Ok(ty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_expr as parse;

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
