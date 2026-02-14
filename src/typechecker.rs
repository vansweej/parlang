/// Hindley-Milner type inference implementation
use crate::ast::{BinOp, Expr};
use crate::types::{Type, TypeScheme, TypeVar, RowVar};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Sum type constructor information
#[derive(Debug, Clone)]
pub struct ConstructorInfo {
    /// Type parameters (e.g., ["a", "b"] for Either a b)
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
    next_row_var: usize,
    type_aliases: HashMap<String, Type>,
    /// Constructor information: maps constructor name to its type info
    constructors: HashMap<String, ConstructorInfo>,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            bindings: HashMap::new(),
            next_var: 0,
            next_row_var: 0,
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

    /// Generate a fresh row variable
    /// 
    /// Row variables represent "the rest of the fields" in record types.
    /// They enable row polymorphism, allowing functions to work with records
    /// that have at least certain fields.
    /// 
    /// # Example
    /// ```
    /// // When type-checking: fun r -> r.age
    /// // We create a row variable to represent unknown fields:
    /// // Type: { age: t0 | r0 } -> t0
    /// ```
    pub fn fresh_row_var(&mut self) -> RowVar {
        let row_var = RowVar(self.next_row_var);
        self.next_row_var += 1;
        row_var
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
    pub fn extend(&self, name: String, ty: Type) -> Self {
        let mut new_env = self.clone();
        new_env.bind(name, TypeScheme { vars: vec![], row_vars: vec![], ty });
        new_env
    }

    /// Instantiate a type scheme by replacing quantified variables with fresh ones
    fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        if scheme.vars.is_empty() && scheme.row_vars.is_empty() {
            return scheme.ty.clone();
        }

        let mut subst = HashMap::new();
        for var in &scheme.vars {
            subst.insert(var.clone(), self.fresh_var());
        }
        
        let mut row_subst = HashMap::new();
        for row_var in &scheme.row_vars {
            row_subst.insert(row_var.clone(), Type::Row(self.fresh_row_var()));
        }
        
        let ty_after_subst = apply_subst(&subst, &scheme.ty);
        apply_row_subst(&row_subst, &ty_after_subst)
    }

    /// Generalize a type by quantifying free type variables and row variables
    pub fn generalize(&self, ty: &Type) -> TypeScheme {
        let free_in_env = self.free_vars();
        let free_in_type = free_type_vars(ty);

        let mut quantified: Vec<TypeVar> = free_in_type
            .difference(&free_in_env)
            .cloned()
            .collect();
        quantified.sort();

        let free_row_in_env = self.free_row_vars();
        let free_row_in_type = free_row_vars(ty);

        let mut quantified_rows: Vec<RowVar> = free_row_in_type
            .difference(&free_row_in_env)
            .cloned()
            .collect();
        quantified_rows.sort();

        TypeScheme {
            vars: quantified,
            row_vars: quantified_rows,
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

    /// Get free row variables in the environment
    fn free_row_vars(&self) -> HashSet<RowVar> {
        self.bindings
            .values()
            .flat_map(|scheme| {
                let mut free = free_row_vars(&scheme.ty);
                for row_var in &scheme.row_vars {
                    free.remove(row_var);
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
    pub fn register_constructor(
        &mut self,
        constructor_name: String,
        info: ConstructorInfo,
    ) {
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
                new_fields.insert(
                    name.clone(),
                    apply_subst_with_visited(subst, ty, visited),
                );
            }
            Type::Record(new_fields)
        }
        Type::RecordRow(fields, row_var) => {
            let mut new_fields = HashMap::new();
            for (name, ty) in fields {
                new_fields.insert(
                    name.clone(),
                    apply_subst_with_visited(subst, ty, visited),
                );
            }
            Type::RecordRow(new_fields, row_var.clone())
        }
        Type::Row(row_var) => Type::Row(row_var.clone()),
        Type::SumType(name, args) => {
            let new_args = args
                .iter()
                .map(|arg| apply_subst_with_visited(subst, arg, visited))
                .collect();
            Type::SumType(name.clone(), new_args)
        }
        Type::Array(elem_ty, size) => {
            let new_elem_ty = apply_subst_with_visited(subst, elem_ty, visited);
            Type::Array(Box::new(new_elem_ty), *size)
        }
        Type::Ref(inner_ty) => {
            let new_inner_ty = apply_subst_with_visited(subst, inner_ty, visited);
            Type::Ref(Box::new(new_inner_ty))
        }
    }
}

/// Row substitution (maps RowVar to Type)
/// 
/// Row substitutions map row variables to concrete types, allowing us to
/// resolve row polymorphic types to concrete record types during unification.
type RowSubstitution = HashMap<RowVar, Type>;

/// Apply row substitution to a type
/// 
/// This function applies row variable substitutions to types, which is essential
/// for row polymorphism. When we have a type like `{ age: Int | r0 }` and a
/// substitution `r0 -> { name: Int }`, we can merge them to get
/// `{ age: Int, name: Int }`.
/// 
/// # Arguments
/// * `subst` - The row substitution mapping row variables to types
/// * `ty` - The type to apply the substitution to
/// 
/// # Returns
/// The type with row variables substituted
fn apply_row_subst(subst: &RowSubstitution, ty: &Type) -> Type {
    match ty {
        Type::Int | Type::Bool | Type::Char | Type::Float | Type::Unit | Type::Var(_) => ty.clone(),
        Type::Fun(arg, ret) => Type::Fun(
            Box::new(apply_row_subst(subst, arg)),
            Box::new(apply_row_subst(subst, ret)),
        ),
        Type::Record(fields) => {
            let mut new_fields = HashMap::new();
            for (name, field_ty) in fields {
                new_fields.insert(name.clone(), apply_row_subst(subst, field_ty));
            }
            Type::Record(new_fields)
        }
        Type::RecordRow(fields, row_var) => {
            let mut new_fields = HashMap::new();
            for (name, field_ty) in fields {
                new_fields.insert(name.clone(), apply_row_subst(subst, field_ty));
            }
            // If there's a substitution for this row variable, apply it
            if let Some(row_ty) = subst.get(row_var) {
                // Merge fields with the substituted row
                match row_ty {
                    Type::Record(row_fields) => {
                        // Merge new_fields with row_fields
                        let mut merged = row_fields.clone();
                        merged.extend(new_fields);
                        Type::Record(merged)
                    }
                    Type::RecordRow(row_fields, new_row_var) => {
                        // Merge new_fields with row_fields, keeping the new row variable
                        let mut merged = row_fields.clone();
                        merged.extend(new_fields);
                        Type::RecordRow(merged, new_row_var.clone())
                    }
                    Type::Row(new_row_var) => {
                        // Keep the fields, replace the row variable
                        Type::RecordRow(new_fields, new_row_var.clone())
                    }
                    _ => Type::RecordRow(new_fields, row_var.clone()),
                }
            } else {
                Type::RecordRow(new_fields, row_var.clone())
            }
        }
        Type::Row(row_var) => {
            // If there's a substitution for this row variable, use it
            if let Some(row_ty) = subst.get(row_var) {
                row_ty.clone()
            } else {
                ty.clone()
            }
        }
        Type::SumType(name, args) => {
            let new_args = args.iter().map(|arg| apply_row_subst(subst, arg)).collect();
            Type::SumType(name.clone(), new_args)
        }
        Type::Array(elem_ty, size) => {
            let new_elem_ty = apply_row_subst(subst, elem_ty);
            Type::Array(Box::new(new_elem_ty), *size)
        }
        Type::Ref(inner_ty) => {
            let new_inner_ty = apply_row_subst(subst, inner_ty);
            Type::Ref(Box::new(new_inner_ty))
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
        Type::RecordRow(fields, _row_var) => {
            let mut set = HashSet::new();
            for ty in fields.values() {
                set.extend(free_type_vars(ty));
            }
            set
        }
        Type::Row(_) => HashSet::new(),
        Type::SumType(_name, args) => {
            let mut set = HashSet::new();
            for arg in args {
                set.extend(free_type_vars(arg));
            }
            set
        }
        Type::Array(elem_ty, _size) => {
            free_type_vars(elem_ty)
        }
        Type::Ref(inner_ty) => {
            free_type_vars(inner_ty)
        }
    }
}

/// Get free row variables in a type
/// 
/// Row variables that appear in a type but are not bound by any quantifier
/// are considered "free". This function collects all such free row variables.
/// 
/// # Example
/// For the type `{ age: Int | r0 }`, this returns `{r0}`.
/// For the type `forall r0. { age: Int | r0 }`, after instantiation r0 is bound.
fn free_row_vars(ty: &Type) -> HashSet<RowVar> {
    match ty {
        Type::Int | Type::Bool | Type::Char | Type::Float | Type::Unit | Type::Var(_) | Type::Record(_) => HashSet::new(),
        Type::RecordRow(fields, row_var) => {
            let mut set = HashSet::new();
            set.insert(row_var.clone());
            for field_ty in fields.values() {
                set.extend(free_row_vars(field_ty));
            }
            set
        }
        Type::Row(row_var) => {
            let mut set = HashSet::new();
            set.insert(row_var.clone());
            set
        }
        Type::Fun(arg, ret) => {
            let mut set = free_row_vars(arg);
            set.extend(free_row_vars(ret));
            set
        }
        Type::SumType(_name, args) => {
            let mut set = HashSet::new();
            for arg in args {
                set.extend(free_row_vars(arg));
            }
            set
        }
        Type::Array(elem_ty, _size) => {
            free_row_vars(elem_ty)
        }
        Type::Ref(inner_ty) => {
            free_row_vars(inner_ty)
        }
    }
}

/// Convert TypeAnnotation to Type
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
        crate::ast::TypeAnnotation::Fun(arg, ret) => {
            Type::Fun(
                Box::new(type_annotation_to_type(arg, type_param_map, env)),
                Box::new(type_annotation_to_type(ret, type_param_map, env)),
            )
        }
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
    UnificationError(Type, Type),
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
                write!(f, "Field '{field}' not found. Available fields: {available:?}")
            }
            TypeError::RecordExpected(got) => {
                write!(f, "Expected record type, got {got}")
            }
            TypeError::RecordFieldMismatch => {
                write!(f, "Record types have different fields")
            }
            TypeError::ConstructorArityMismatch(name, expected, actual) => {
                write!(f, "Constructor '{name}' expects {expected} arguments, but got {actual}")
            }
        }
    }
}

impl std::error::Error for TypeError {}

/// Unification algorithm
fn unify(t1: &Type, t2: &Type) -> Result<Substitution, TypeError> {
    match (t1, t2) {
        (Type::Int, Type::Int) | (Type::Bool, Type::Bool) | (Type::Char, Type::Char) | (Type::Float, Type::Float) | (Type::Unit, Type::Unit) => Ok(HashMap::new()),

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

        // Unify closed record with row-polymorphic record
        // This handles cases like: { x: Int, y: Int } ~ { x: Int | r0 }
        // The closed record must have at least the fields in the row-polymorphic record
        (Type::Record(fields), Type::RecordRow(row_fields, row_var))
        | (Type::RecordRow(row_fields, row_var), Type::Record(fields)) => {
            // The closed record must have at least the fields in row_fields
            let mut subst = HashMap::new();
            
            // Unify the common fields
            for (name, row_ty) in row_fields {
                match fields.get(name) {
                    Some(field_ty) => {
                        let row_ty = apply_subst(&subst, row_ty);
                        let field_ty = apply_subst(&subst, field_ty);
                        let s = unify(&row_ty, &field_ty)?;
                        subst = compose_subst(&s, &subst);
                    }
                    None => {
                        return Err(TypeError::FieldNotFound(name.clone(), 
                            fields.keys().cloned().collect()));
                    }
                }
            }
            
            // The row variable should represent the remaining fields
            let mut remaining = fields.clone();
            for name in row_fields.keys() {
                remaining.remove(name);
            }
            
            // Bind the row variable to the remaining fields
            let mut row_subst = HashMap::new();
            row_subst.insert(row_var.clone(), Type::Record(remaining));
            
            // Compose the substitutions - we need to convert row_subst to regular subst
            // For now, we'll just return the type substitution
            Ok(subst)
        }

        // Unify two row-polymorphic records
        // This handles cases like: { x: Int | r0 } ~ { y: Int | r1 }
        // We need to unify common fields and handle the row variables appropriately
        (Type::RecordRow(fields1, row1), Type::RecordRow(fields2, row2)) => {
            // Find common fields and unify them
            let mut subst = HashMap::new();
            let mut fields1_only = HashMap::new();
            let mut fields2_only = HashMap::new();
            
            // Collect fields only in fields1
            for (name, ty) in fields1 {
                if !fields2.contains_key(name) {
                    fields1_only.insert(name.clone(), ty.clone());
                }
            }
            
            // Collect fields only in fields2
            for (name, ty) in fields2 {
                if !fields1.contains_key(name) {
                    fields2_only.insert(name.clone(), ty.clone());
                }
            }
            
            // Unify common fields
            for (name, ty1) in fields1 {
                if let Some(ty2) = fields2.get(name) {
                    let ty1 = apply_subst(&subst, ty1);
                    let ty2 = apply_subst(&subst, ty2);
                    let s = unify(&ty1, &ty2)?;
                    subst = compose_subst(&s, &subst);
                }
            }
            
            // Now we need to handle the row variables
            // row1 should contain fields2_only + some rest
            // row2 should contain fields1_only + some rest
            // For simplicity, if both have the same row variable, they unify
            if row1 == row2 {
                Ok(subst)
            } else {
                // More complex case: bind one row variable to include the other's unique fields
                // For now, we'll bind row1 to contain fields2_only and row2
                if !fields2_only.is_empty() {
                    // Can't easily represent this with current substitution system
                    // This would require row variable constraints which is complex
                    // For now, require exact match
                    Err(TypeError::RecordFieldMismatch)
                } else if !fields1_only.is_empty() {
                    Err(TypeError::RecordFieldMismatch)
                } else {
                    // No unique fields on either side, just unify the row variables
                    // by binding row1 to row2
                    Ok(subst)
                }
            }
        }

        // Unify Row with Row
        (Type::Row(r1), Type::Row(r2)) => {
            if r1 == r2 {
                Ok(HashMap::new())
            } else {
                // Row variables can be unified, but we don't have row substitution in our
                // regular substitution. For now, we'll accept them as compatible.
                Ok(HashMap::new())
            }
        }

        // Unify Row with Record or RecordRow
        (Type::Row(_row), Type::Record(_fields)) |
        (Type::Record(_fields), Type::Row(_row)) => {
            // A row variable can unify with a closed record
            // This would bind the row variable to that record
            Ok(HashMap::new())
        }

        (Type::Row(_row), Type::RecordRow(_fields, _row_var)) |
        (Type::RecordRow(_fields, _row_var), Type::Row(_row)) => {
            // A row variable can unify with a row-polymorphic record
            Ok(HashMap::new())
        }

        (Type::SumType(name1, args1), Type::SumType(name2, args2)) => {
            // Sum types must have the same name and same number of type arguments
            if name1 != name2 {
                return Err(TypeError::UnificationError(t1.clone(), t2.clone()));
            }
            
            if args1.len() != args2.len() {
                return Err(TypeError::UnificationError(t1.clone(), t2.clone()));
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

        _ => Err(TypeError::UnificationError(t1.clone(), t2.clone())),
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

/// Convert a TypeExpr to a Type, resolving any aliases
fn resolve_type_expr(ty_expr: &crate::ast::TypeExpr, env: &TypeEnv) -> Result<Type, TypeError> {
    match ty_expr {
        crate::ast::TypeExpr::Int => Ok(Type::Int),
        crate::ast::TypeExpr::Bool => Ok(Type::Bool),
        crate::ast::TypeExpr::Fun(arg, ret) => {
            let arg_ty = resolve_type_expr(arg, env)?;
            let ret_ty = resolve_type_expr(ret, env)?;
            Ok(Type::Fun(Box::new(arg_ty), Box::new(ret_ty)))
        }
        crate::ast::TypeExpr::Alias(name) => {
            env.resolve_type_alias(name)
                .ok_or_else(|| TypeError::UnboundVariable(name.clone()))
        }
    }
}

/// Convert a TypeAnnotation to a Type, resolving names to concrete types
fn resolve_type_annotation(ty_ann: &crate::ast::TypeAnnotation, env: &mut TypeEnv) -> Result<Type, TypeError> {
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
            Err(TypeError::UnboundVariable(format!("Applied type not yet supported in annotations: {}", name)))
        }
    }
}

/// Type inference for expressions
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

        Expr::BinOp(op, left, right) => {
            let (left_ty, s1) = infer(left, env)?;
            let mut env1 = env.clone();
            apply_subst_env(&s1, &mut env1);

            let (right_ty, s2) = infer(right, &mut env1)?;
            let left_ty = apply_subst(&s2, &left_ty);

            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    // Arithmetic operations work on Int and Float
                    // Check if left type is Int or Float
                    match &left_ty {
                        Type::Int => {
                            let s3 = unify(&right_ty, &Type::Int)?;
                            let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                            return Ok((Type::Int, subst));
                        }
                        Type::Float => {
                            let s3 = unify(&right_ty, &Type::Float)?;
                            let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                            return Ok((Type::Float, subst));
                        }
                        Type::Var(_) => {
                            // Try to unify with right type first
                            let s3 = unify(&left_ty, &right_ty)?;
                            let unified_ty = apply_subst(&s3, &left_ty);
                            
                            // Now check if unified type is Int or Float
                            match &unified_ty {
                                Type::Int | Type::Float => {
                                    let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                                    return Ok((unified_ty, subst));
                                }
                                Type::Var(_) => {
                                    // Still a type variable, default to Int for arithmetic operations
                                    let s4 = unify(&unified_ty, &Type::Int)?;
                                    let subst = compose_subst(&s4, &compose_subst(&s3, &compose_subst(&s2, &s1)));
                                    return Ok((Type::Int, subst));
                                }
                                _ => {
                                    return Err(TypeError::UnificationError(
                                        unified_ty,
                                        Type::Int,
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(TypeError::UnificationError(
                                left_ty,
                                Type::Int,
                            ));
                        }
                    }
                }
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    // Ordering comparisons work for Int, Char, and Float
                    // Check if left type is Int, Char, or Float
                    match &left_ty {
                        Type::Int => {
                            let s3 = unify(&right_ty, &Type::Int)?;
                            let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                            return Ok((Type::Bool, subst));
                        }
                        Type::Char => {
                            let s3 = unify(&right_ty, &Type::Char)?;
                            let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                            return Ok((Type::Bool, subst));
                        }
                        Type::Float => {
                            let s3 = unify(&right_ty, &Type::Float)?;
                            let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                            return Ok((Type::Bool, subst));
                        }
                        Type::Var(_) => {
                            // Try to unify with right type first
                            let s3 = unify(&left_ty, &right_ty)?;
                            let unified_ty = apply_subst(&s3, &left_ty);
                            
                            // Now check if unified type is Int, Char, or Float
                            match &unified_ty {
                                Type::Int | Type::Char | Type::Float => {
                                    let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                                    return Ok((Type::Bool, subst));
                                }
                                Type::Var(_) => {
                                    // Still a type variable, default to Int for ordering operations
                                    let s4 = unify(&unified_ty, &Type::Int)?;
                                    let subst = compose_subst(&s4, &compose_subst(&s3, &compose_subst(&s2, &s1)));
                                    return Ok((Type::Bool, subst));
                                }
                                _ => {
                                    return Err(TypeError::UnificationError(
                                        unified_ty,
                                        Type::Int,
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(TypeError::UnificationError(
                                left_ty,
                                Type::Int,
                            ));
                        }
                    }
                }
                BinOp::Eq | BinOp::Neq => {
                    // Equality works on any type, but both sides must match
                    let s3 = unify(&left_ty, &right_ty)?;
                    let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                    return Ok((Type::Bool, subst));
                }
            }
        }

        Expr::If(cond, then_br, else_br) => {
            let (cond_ty, s1) = infer(cond, env)?;
            let s2 = unify(&cond_ty, &Type::Bool)?;

            let mut env1 = env.clone();
            apply_subst_env(&compose_subst(&s2, &s1), &mut env1);

            let (then_ty, s3) = infer(then_br, &mut env1)?;

            let mut env2 = env1.clone();
            apply_subst_env(&s3, &mut env2);

            let (else_ty, s4) = infer(else_br, &mut env2)?;

            let then_ty = apply_subst(&s4, &then_ty);
            let s5 = unify(&then_ty, &else_ty)?;

            let result_ty = apply_subst(&s5, &then_ty);
            let subst = compose_subst(
                &s5,
                &compose_subst(&s4, &compose_subst(&s3, &compose_subst(&s2, &s1))),
            );

            Ok((result_ty, subst))
        }

        Expr::Let(name, ty_ann_opt, value, body) => {
            let (value_ty, s1) = infer(value, env)?;

            // If there's a type annotation, check it matches the inferred type
            if let Some(ty_ann) = ty_ann_opt {
                let annotated_ty = resolve_type_annotation(ty_ann, env)?;
                let s_ann = unify(&value_ty, &annotated_ty)?;
                let s1 = compose_subst(&s_ann, &s1);
                
                let mut env1 = env.clone();
                apply_subst_env(&s1, &mut env1);
                
                let unified_ty = apply_subst(&s1, &value_ty);
                let scheme = env1.generalize(&unified_ty);
                env1.bind(name.clone(), scheme);

                let (body_ty, s2) = infer(body, &mut env1)?;

                let subst = compose_subst(&s2, &s1);
                Ok((body_ty, subst))
            } else {
                let mut env1 = env.clone();
                apply_subst_env(&s1, &mut env1);

                // Generalize the type (let-polymorphism)
                let scheme = env1.generalize(&value_ty);
                env1.bind(name.clone(), scheme);

                let (body_ty, s2) = infer(body, &mut env1)?;

                let subst = compose_subst(&s2, &s1);
                Ok((body_ty, subst))
            }
        }

        Expr::Fun(param, ty_ann_opt, body) => {
            // Use annotated type if provided, otherwise create fresh variable
            let param_ty = if let Some(ty_ann) = ty_ann_opt {
                resolve_type_annotation(ty_ann, env)?
            } else {
                env.fresh_var()
            };
            
            let mut env1 = env.clone();
            env1 = env1.extend(param.clone(), param_ty.clone());

            let (body_ty, s1) = infer(body, &mut env1)?;
            let param_ty = apply_subst(&s1, &param_ty);

            Ok((Type::Fun(Box::new(param_ty), Box::new(body_ty)), s1))
        }

        Expr::App(func, arg) => {
            let (func_ty, s1) = infer(func, env)?;

            let mut env1 = env.clone();
            apply_subst_env(&s1, &mut env1);

            let (arg_ty, s2) = infer(arg, &mut env1)?;

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

        Expr::Rec(name, body) => {
            // For recursive functions, we use fixpoint typing:
            // 1. Generate a fresh type variable for the recursive function
            // 2. Add it to the environment before checking the body
            // 3. Infer the type of the body with the recursive name bound
            // 4. Unify the inferred type with the assumed type
            
            let rec_ty = env.fresh_var();
            let mut extended_env = env.extend(name.clone(), rec_ty.clone());
            
            let (body_ty, subst) = infer(body, &mut extended_env)?;
            
            // The body type should be the same as the recursive function type
            // (after applying the substitution from inferring the body)
            let rec_ty = apply_subst(&subst, &rec_ty);
            let s2 = unify(&rec_ty, &body_ty)?;
            
            let final_ty = apply_subst(&s2, &body_ty);
            let final_subst = compose_subst(&s2, &subst);
            
            Ok((final_ty, final_subst))
        }

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

        Expr::Seq(_, _) => {
            // For now, return a type variable for sequential expressions
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
            // Infer the type of the record expression
            let (record_ty, s1) = infer(record_expr, env)?;
            
            // Apply substitution to get concrete record type
            let record_ty = apply_subst(&s1, &record_ty);
            
            match record_ty {
                Type::Record(fields) => {
                    // Look up the field type
                    match fields.get(field_name) {
                        Some(field_ty) => Ok((field_ty.clone(), s1)),
                        None => {
                            let available: Vec<String> = fields.keys().cloned().collect();
                            Err(TypeError::FieldNotFound(field_name.clone(), available))
                        }
                    }
                }
                Type::RecordRow(fields, _) => {
                    // Look up the field type in the known fields
                    match fields.get(field_name) {
                        Some(field_ty) => Ok((field_ty.clone(), s1)),
                        None => {
                            // Field might be in the row variable, but we can't know for sure
                            // For now, report an error with available fields
                            let available: Vec<String> = fields.keys().cloned().collect();
                            Err(TypeError::FieldNotFound(field_name.clone(), available))
                        }
                    }
                }
                Type::Var(_) => {
                    // Polymorphic record - create a fresh type variable for the field
                    // Use row polymorphism: create a record type with at least this field
                    let field_ty = env.fresh_var();
                    let row_var = env.fresh_row_var();
                    
                    // Create a record type with at least this field plus other fields (row variable)
                    let mut fields = HashMap::new();
                    fields.insert(field_name.clone(), field_ty.clone());
                    let record_with_field = Type::RecordRow(fields, row_var);
                    
                    // Unify with the record type
                    let s2 = unify(&record_ty, &record_with_field)?;
                    let subst = compose_subst(&s2, &s1);
                    
                    Ok((field_ty, subst))
                }
                Type::Row(row_var) => {
                    // A row variable on its own - we need to constrain it to have this field
                    // Create a fresh type variable for the field type
                    let field_ty = env.fresh_var();
                    let new_row_var = env.fresh_row_var();
                    
                    // Create a record type with this field
                    let mut fields = HashMap::new();
                    fields.insert(field_name.clone(), field_ty.clone());
                    let record_with_field = Type::RecordRow(fields, new_row_var);
                    
                    // Unify the row variable with this record type
                    let row_ty = Type::Row(row_var.clone());
                    let s2 = unify(&row_ty, &record_with_field)?;
                    let subst = compose_subst(&s2, &s1);
                    
                    Ok((field_ty, subst))
                }
                _ => {
                    Err(TypeError::RecordExpected(format!("{record_ty}")))
                }
            }
        }
        
        Expr::TypeDef { name, type_params, constructors, body } => {
            // Register constructors in the environment
            for (ctor_name, _payload_types) in constructors {
                let info = ConstructorInfo {
                    type_params: type_params.clone(),
                    payload_types: _payload_types.clone(),
                    sum_type_name: name.clone(),
                };
                env.register_constructor(ctor_name.clone(), info);
            }
            
            // Type check the body with constructors available
            infer(body, env)
        }
        
        Expr::Constructor(name, args) => {
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
                        name.clone(),
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
                let type_args: Vec<Type> = info.type_params
                    .iter()
                    .map(|param| {
                        apply_subst(&subst, &type_param_map[param])
                    })
                    .collect();
                
                let result_ty = Type::SumType(info.sum_type_name.clone(), type_args);
                Ok((result_ty, subst))
            } else {
                // Constructor not registered - return a fresh type variable
                // This maintains backward compatibility
                Ok((env.fresh_var(), HashMap::new()))
            }
        }
        
        Expr::Array(elements) => {
            if elements.is_empty() {
                // Empty array - use fresh type variable for element type
                let elem_ty = env.fresh_var();
                Ok((Type::Array(Box::new(elem_ty), 0), HashMap::new()))
            } else {
                // Infer type of first element
                let (first_ty, mut subst) = infer(&elements[0], env)?;
                
                // Check that all other elements have the same type
                for elem in &elements[1..] {
                    let (elem_ty, s) = infer(elem, env)?;
                    subst = compose_subst(&s, &subst);
                    let s2 = unify(&apply_subst(&subst, &first_ty), &apply_subst(&subst, &elem_ty))?;
                    subst = compose_subst(&s2, &subst);
                }
                
                let final_elem_ty = apply_subst(&subst, &first_ty);
                let size = elements.len();
                Ok((Type::Array(Box::new(final_elem_ty), size), subst))
            }
        }
        
        Expr::ArrayIndex(arr_expr, index_expr) => {
            // Infer types of array and index
            let (arr_ty, s1) = infer(arr_expr, env)?;
            let (index_ty, s2) = infer(index_expr, env)?;
            let mut subst = compose_subst(&s2, &s1);
            
            // Index must be Int
            let s3 = unify(&apply_subst(&subst, &index_ty), &Type::Int)?;
            subst = compose_subst(&s3, &subst);
            
            // Array must be Array type
            let elem_ty = env.fresh_var();
            // Array size is not validated during type inference - it's a runtime property
            // We use 0 as a placeholder since the actual size will be checked at runtime
            let size_var = 0;
            let expected_arr_ty = Type::Array(Box::new(elem_ty.clone()), size_var);
            
            // We need special handling for array unification because size may differ
            // Extract the element type from the array
            let arr_ty_subst = apply_subst(&subst, &arr_ty);
            match arr_ty_subst {
                Type::Array(actual_elem_ty, _size) => {
                    let s4 = unify(&elem_ty, &actual_elem_ty)?;
                    subst = compose_subst(&s4, &subst);
                    Ok((apply_subst(&subst, &actual_elem_ty), subst))
                }
                Type::Var(_) => {
                    // If it's still a type variable, unify with array type
                    let s4 = unify(&arr_ty_subst, &expected_arr_ty)?;
                    subst = compose_subst(&s4, &subst);
                    Ok((apply_subst(&subst, &elem_ty), subst))
                }
                _ => {
                    Err(TypeError::UnificationError(
                        arr_ty_subst,
                        expected_arr_ty
                    ))
                }
            }
        }
        
        Expr::Ref(expr) => {
            // Type of ref expr is Ref T where T is the type of expr
            let (ty, subst) = infer(expr, env)?;
            Ok((Type::Ref(Box::new(ty)), subst))
        }
        
        Expr::Deref(expr) => {
            // Type of !ref_expr is T where ref_expr has type Ref T
            let (ref_ty, subst) = infer(expr, env)?;
            
            // Create a fresh type variable for the inner type
            let inner_ty = env.fresh_var();
            let expected_ref_ty = Type::Ref(Box::new(inner_ty.clone()));
            
            // Unify the inferred type with Ref inner_ty
            let ref_ty_subst = apply_subst(&subst, &ref_ty);
            let s2 = match &ref_ty_subst {
                Type::Ref(actual_inner) => {
                    unify(&inner_ty, actual_inner)?
                }
                Type::Var(_) => {
                    unify(&ref_ty_subst, &expected_ref_ty)?
                }
                _ => {
                    return Err(TypeError::UnificationError(
                        ref_ty_subst,
                        expected_ref_ty
                    ));
                }
            };
            
            let final_subst = compose_subst(&s2, &subst);
            Ok((apply_subst(&final_subst, &inner_ty), final_subst))
        }
        
        Expr::RefAssign(ref_expr, value_expr) => {
            // Type check: ref_expr must have type Ref T, value_expr must have type T
            // Result type is unit ()
            let (ref_ty, s1) = infer(ref_expr, env)?;
            let (val_ty, s2) = infer(value_expr, env)?;
            let mut subst = compose_subst(&s2, &s1);
            
            // Extract the inner type from the reference
            let ref_ty_subst = apply_subst(&subst, &ref_ty);
            let inner_ty = match &ref_ty_subst {
                Type::Ref(inner) => inner.as_ref().clone(),
                Type::Var(_) => {
                    // If it's a type variable, create a fresh variable for the inner type
                    let fresh_inner = env.fresh_var();
                    let expected_ref_ty = Type::Ref(Box::new(fresh_inner.clone()));
                    let s3 = unify(&ref_ty_subst, &expected_ref_ty)?;
                    subst = compose_subst(&s3, &subst);
                    fresh_inner
                }
                _ => {
                    return Err(TypeError::UnificationError(
                        ref_ty_subst,
                        Type::Ref(Box::new(env.fresh_var()))
                    ));
                }
            };
            
            // Unify the value type with the inner type of the reference
            let val_ty_subst = apply_subst(&subst, &val_ty);
            let s3 = unify(&val_ty_subst, &apply_subst(&subst, &inner_ty))?;
            subst = compose_subst(&s3, &subst);
            
            // Return unit type
            Ok((Type::Unit, subst))
        }
    }
}

/// Public API for type checking
pub fn typecheck(expr: &Expr) -> Result<Type, TypeError> {
    let mut env = TypeEnv::new();
    let (ty, subst) = infer(expr, &mut env)?;
    Ok(apply_subst(&subst, &ty))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

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
        assert_eq!(
            check("if false then true else false").unwrap(),
            Type::Bool
        );
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
        assert!(matches!(
            check("x + 1"),
            Err(TypeError::UnboundVariable(_))
        ));
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
