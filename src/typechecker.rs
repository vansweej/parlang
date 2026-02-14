/// Hindley-Milner type inference implementation
use crate::ast::{BinOp, Expr};
use crate::types::{Type, TypeScheme, TypeVar};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Type environment (Γ) mapping variables to type schemes
#[derive(Debug, Clone)]
pub struct TypeEnv {
    bindings: HashMap<String, TypeScheme>,
    next_var: usize,
    type_aliases: HashMap<String, Type>,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            bindings: HashMap::new(),
            next_var: 0,
            type_aliases: HashMap::new(),
        }
    }

    /// Generate a fresh type variable
    pub fn fresh_var(&mut self) -> Type {
        let var = Type::Var(TypeVar(self.next_var));
        self.next_var += 1;
        var
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

        let mut quantified: Vec<TypeVar> = free_in_type
            .difference(&free_in_env)
            .cloned()
            .collect();
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
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Substitution map from type variables to types
type Substitution = HashMap<TypeVar, Type>;

/// Apply substitution to a type
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
        Type::Int | Type::Bool => ty.clone(),
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
    }
}

/// Get free type variables in a type
fn free_type_vars(ty: &Type) -> HashSet<TypeVar> {
    match ty {
        Type::Int | Type::Bool => HashSet::new(),
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
        }
    }
}

impl std::error::Error for TypeError {}

/// Unification algorithm
fn unify(t1: &Type, t2: &Type) -> Result<Substitution, TypeError> {
    match (t1, t2) {
        (Type::Int, Type::Int) | (Type::Bool, Type::Bool) => Ok(HashMap::new()),

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

/// Type inference for expressions
pub fn infer(expr: &Expr, env: &mut TypeEnv) -> Result<(Type, Substitution), TypeError> {
    match expr {
        Expr::Int(_) => Ok((Type::Int, HashMap::new())),

        Expr::Bool(_) => Ok((Type::Bool, HashMap::new())),

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

            let (expected_arg, expected_ret) = match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => (Type::Int, Type::Int),
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => (Type::Int, Type::Bool),
                BinOp::Eq | BinOp::Neq => {
                    // Equality works on any type, but both sides must match
                    let s3 = unify(&left_ty, &right_ty)?;
                    let subst = compose_subst(&s3, &compose_subst(&s2, &s1));
                    return Ok((Type::Bool, subst));
                }
            };

            let s3 = unify(&left_ty, &expected_arg)?;
            let right_ty = apply_subst(&s3, &right_ty);
            let s4 = unify(&right_ty, &expected_arg)?;

            let subst = compose_subst(&s4, &compose_subst(&s3, &compose_subst(&s2, &s1)));
            Ok((expected_ret, subst))
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

        Expr::Let(name, value, body) => {
            let (value_ty, s1) = infer(value, env)?;

            let mut env1 = env.clone();
            apply_subst_env(&s1, &mut env1);

            // Generalize the type (let-polymorphism)
            let scheme = env1.generalize(&value_ty);
            env1.bind(name.clone(), scheme);

            let (body_ty, s2) = infer(body, &mut env1)?;

            let subst = compose_subst(&s2, &s1);
            Ok((body_ty, subst))
        }

        Expr::Fun(param, body) => {
            let param_ty = env.fresh_var();
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

        Expr::Rec(_, _) => {
            // For recursive functions, we need to handle the self-reference
            // This is a simplified version - full implementation would need fixpoint typing
            Err(TypeError::RecursionRequiresAnnotation)
        }

        Expr::Tuple(_elements) => {
            // For now, return a type variable for tuples
            // A full implementation would need tuple types
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
                Type::Var(_) => {
                    // Polymorphic record - create a fresh type variable for the field
                    // This is a simplified approach; full row polymorphism would be more complex
                    let field_ty = env.fresh_var();
                    
                    // Create a record type with at least this field
                    let mut fields = HashMap::new();
                    fields.insert(field_name.clone(), field_ty.clone());
                    let record_with_field = Type::Record(fields);
                    
                    // Unify with the record type
                    let s2 = unify(&record_ty, &record_with_field)?;
                    let subst = compose_subst(&s2, &s1);
                    
                    Ok((field_ty, subst))
                }
                _ => {
                    Err(TypeError::RecordExpected(format!("{record_ty}")))
                }
            }
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
    fn test_rec_not_supported() {
        // Recursive functions should return an error for now
        assert!(matches!(
            check("rec f -> fun n -> if n == 0 then 1 else n"),
            Err(TypeError::RecursionRequiresAnnotation)
        ));
    }
}
