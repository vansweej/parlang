//! Expression rewriting for dictionary insertion. Constrained variable
//! occurrences receive their dictionaries, class methods select a field of
//! one, and `==` or `!=` away from a base type call the `Eq` dictionary.

use std::collections::{HashMap, HashSet};

use super::evidence::{apply_all, dictionary_type, evidence_expr, solve, DictError};
use crate::ast::BinOp;
use crate::typechecker::{apply_subst_constraint, TypeEnv};
use crate::typed_ast::{TypedExpr, TypedExprKind, TypedMatchArm};
use crate::types::{Constraint, Type, TypeScheme, TypeVar};

/// What a variable name refers to at one occurrence.
enum Binding {
    /// A monomorphic binder or an unknown name: no dictionaries.
    Mono,
    /// A let-bound name with its original, possibly constrained, scheme.
    Scheme(TypeScheme),
    /// A class method with its scheme, class parameter first.
    Method(TypeScheme),
}

/// State of the insertion pass over one program.
pub(super) struct Inserter<'a> {
    /// Class, instance and binding environment after type checking.
    pub(super) env: &'a TypeEnv,
    /// Every top-level `let` name of this program. After type checking,
    /// `env` already binds all of them, including later ones, so these
    /// names resolve only through `top`.
    program_names: HashSet<String>,
    /// Original schemes of this program's top-level `let`s seen so far.
    pub(super) top: HashMap<String, TypeScheme>,
    /// Dictionary parameters in scope with their constraints, innermost last.
    pub(super) givens: Vec<(Constraint, String)>,
    /// Local binders in scope, innermost last; `Some` holds a `let` scheme.
    scope: Vec<(String, Option<TypeScheme>)>,
    /// Number of dictionary parameters named so far.
    next_param: usize,
}

impl<'a> Inserter<'a> {
    /// A pass over a program checked in `env` whose top-level `let` names
    /// are `program_names`.
    pub(super) fn new(env: &'a TypeEnv, program_names: HashSet<String>) -> Self {
        Inserter {
            env,
            program_names,
            top: HashMap::new(),
            givens: Vec::new(),
            scope: Vec::new(),
            next_param: 0,
        }
    }

    /// A fresh dictionary parameter name `$d<n>`.
    pub(super) fn fresh_param(&mut self) -> String {
        let name = format!("$d{}", self.next_param);
        self.next_param += 1;
        name
    }

    /// `value` rewritten with one dictionary parameter per constraint in
    /// scope, wrapped in one `Fun` per parameter (first constraint
    /// outermost). With no constraints this is just the rewrite.
    pub(super) fn abstract_over(
        &mut self,
        constraints: &[Constraint],
        value: &TypedExpr,
    ) -> Result<TypedExpr, DictError> {
        let mut params = Vec::with_capacity(constraints.len());
        for constraint in constraints {
            params.push((self.fresh_param(), dictionary_type(self.env, constraint)?));
        }
        let mark = self.givens.len();
        for (constraint, (name, _)) in constraints.iter().zip(&params) {
            self.givens.push((constraint.clone(), name.clone()));
        }
        let body = self.expr(value);
        self.givens.truncate(mark);
        Ok(lambda_all(params, body?))
    }

    /// Rewrite one expression.
    pub(super) fn expr(&mut self, expr: &TypedExpr) -> Result<TypedExpr, DictError> {
        let kind = match &expr.kind {
            TypedExprKind::Var(name, args) => return self.var(name, args, &expr.ty),
            TypedExprKind::BinOp(op, left, right) => return self.binop(*op, left, right, &expr.ty),
            TypedExprKind::Let(..) => return self.local_let(expr),
            TypedExprKind::Fun(name, ann, body) => {
                let body_out = self.under(vec![name.clone()], body)?;
                TypedExprKind::Fun(name.clone(), ann.clone(), Box::new(body_out))
            }
            TypedExprKind::Rec(name, body) => {
                let body_out = self.under(vec![name.clone()], body)?;
                TypedExprKind::Rec(name.clone(), Box::new(body_out))
            }
            TypedExprKind::Match(scrutinee, arms) => {
                TypedExprKind::Match(self.boxed(scrutinee)?, self.arms(arms)?)
            }
            other => self.structural(other)?,
        };
        Ok(TypedExpr {
            kind,
            ty: expr.ty.clone(),
        })
    }

    fn boxed(&mut self, expr: &TypedExpr) -> Result<Box<TypedExpr>, DictError> {
        Ok(Box::new(self.expr(expr)?))
    }

    fn all(&mut self, exprs: &[TypedExpr]) -> Result<Vec<TypedExpr>, DictError> {
        exprs.iter().map(|expr| self.expr(expr)).collect()
    }

    /// Rewrite the children of a node that binds no name.
    fn structural(&mut self, kind: &TypedExprKind) -> Result<TypedExprKind, DictError> {
        Ok(match kind {
            TypedExprKind::If(cond, then_branch, else_branch) => TypedExprKind::If(
                self.boxed(cond)?,
                self.boxed(then_branch)?,
                self.boxed(else_branch)?,
            ),
            TypedExprKind::App(func, arg) => {
                TypedExprKind::App(self.boxed(func)?, self.boxed(arg)?)
            }
            TypedExprKind::Tuple(items) => TypedExprKind::Tuple(self.all(items)?),
            TypedExprKind::TupleProj(tuple, index) => {
                TypedExprKind::TupleProj(self.boxed(tuple)?, *index)
            }
            TypedExprKind::TypeAlias(name, ty_expr, body) => {
                TypedExprKind::TypeAlias(name.clone(), ty_expr.clone(), self.boxed(body)?)
            }
            TypedExprKind::Record(fields) => TypedExprKind::Record(self.record_fields(fields)?),
            TypedExprKind::FieldAccess(record, field) => {
                TypedExprKind::FieldAccess(self.boxed(record)?, field.clone())
            }
            TypedExprKind::TypeDef {
                name,
                type_params,
                param_kinds,
                constructors,
                body,
            } => TypedExprKind::TypeDef {
                name: name.clone(),
                type_params: type_params.clone(),
                param_kinds: param_kinds.clone(),
                constructors: constructors.clone(),
                body: self.boxed(body)?,
            },
            TypedExprKind::Constructor(name, args) => {
                TypedExprKind::Constructor(name.clone(), self.all(args)?)
            }
            other => other.clone(),
        })
    }

    fn record_fields(
        &mut self,
        fields: &[(String, TypedExpr)],
    ) -> Result<Vec<(String, TypedExpr)>, DictError> {
        fields
            .iter()
            .map(|(field, value)| Ok((field.clone(), self.expr(value)?)))
            .collect()
    }

    /// Rewrite `body` with the monomorphic binders `names` in scope.
    fn under(&mut self, names: Vec<String>, body: &TypedExpr) -> Result<TypedExpr, DictError> {
        let mark = self.scope.len();
        self.scope
            .extend(names.into_iter().map(|name| (name, None)));
        let result = self.expr(body);
        self.scope.truncate(mark);
        result
    }

    fn arms(&mut self, arms: &[TypedMatchArm]) -> Result<Vec<TypedMatchArm>, DictError> {
        arms.iter()
            .map(|arm| {
                let names = arm.bindings.iter().map(|(name, _)| name.clone()).collect();
                Ok(TypedMatchArm {
                    pattern: arm.pattern.clone(),
                    bindings: arm.bindings.clone(),
                    body: self.under(names, &arm.body)?,
                })
            })
            .collect()
    }

    /// A local `let`: a constrained value becomes a dictionary lambda and
    /// loses its annotation; the body sees the original scheme.
    fn local_let(&mut self, expr: &TypedExpr) -> Result<TypedExpr, DictError> {
        let TypedExprKind::Let(name, ann, scheme, value, body) = &expr.kind else {
            return Ok(expr.clone());
        };
        let value_out = self.abstract_over(&scheme.constraints, value)?;
        let (ann_out, scheme_out) = if scheme.constraints.is_empty() {
            (ann.clone(), scheme.clone())
        } else {
            (None, dictionary_scheme(scheme, &value_out))
        };
        self.scope.push((name.clone(), Some(scheme.clone())));
        let body_out = self.expr(body);
        self.scope.pop();
        Ok(TypedExpr {
            kind: TypedExprKind::Let(
                name.clone(),
                ann_out,
                scheme_out,
                Box::new(value_out),
                Box::new(body_out?),
            ),
            ty: expr.ty.clone(),
        })
    }

    /// Mirror the type checker's lookup at this point of the program:
    /// local binders, then this program's top-level `let`s seen so far,
    /// then earlier REPL bindings (names this program does not define),
    /// then class methods.
    fn lookup(&self, name: &str) -> Binding {
        if let Some((_, entry)) = self.scope.iter().rev().find(|(bound, _)| bound == name) {
            return entry.clone().map_or(Binding::Mono, Binding::Scheme);
        }
        if let Some(scheme) = self.top.get(name) {
            return Binding::Scheme(scheme.clone());
        }
        if let Some(scheme) = self
            .env
            .binding_scheme(name)
            .filter(|_| !self.program_names.contains(name))
        {
            return Binding::Scheme(scheme.clone());
        }
        match self.env.method_info(name) {
            Some(info) => Binding::Method(info.scheme.clone()),
            None => Binding::Mono,
        }
    }

    fn var(&mut self, name: &str, args: &[Type], ty: &Type) -> Result<TypedExpr, DictError> {
        let plain = TypedExpr {
            kind: TypedExprKind::Var(name.to_string(), args.to_vec()),
            ty: ty.clone(),
        };
        let (scheme, is_method) = match self.lookup(name) {
            Binding::Mono => return Ok(plain),
            Binding::Scheme(scheme) => (scheme, false),
            Binding::Method(scheme) => (scheme, true),
        };
        if scheme.constraints.is_empty() {
            return Ok(plain);
        }
        let at_args: HashMap<TypeVar, Type> = scheme
            .vars
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        let mut dicts = Vec::with_capacity(scheme.constraints.len());
        for constraint in &scheme.constraints {
            let wanted = apply_subst_constraint(&at_args, constraint);
            let evidence = solve(self.env, &self.givens, &wanted)?;
            dicts.push(evidence_expr(self.env, &evidence)?);
        }
        if is_method {
            let Some(dict) = dicts.pop() else {
                return Ok(plain);
            };
            return Ok(TypedExpr {
                kind: TypedExprKind::FieldAccess(Box::new(dict), name.to_string()),
                ty: ty.clone(),
            });
        }
        let fun_ty = dicts.iter().rev().fold(ty.clone(), |acc, dict| {
            Type::Fun(Box::new(dict.ty.clone()), Box::new(acc))
        });
        let head = TypedExpr {
            kind: TypedExprKind::Var(name.to_string(), args.to_vec()),
            ty: fun_ty,
        };
        Ok(apply_all(head, dicts))
    }

    /// `==` and `!=` at a base type stay primitive; otherwise they call the
    /// `eq` field of the `Eq` dictionary, and `!=` negates the result.
    fn binop(
        &mut self,
        op: BinOp,
        left: &TypedExpr,
        right: &TypedExpr,
        ty: &Type,
    ) -> Result<TypedExpr, DictError> {
        let left_out = self.expr(left)?;
        let right_out = self.expr(right)?;
        let base = matches!(left.ty, Type::Int | Type::Bool | Type::Char | Type::Float);
        if base || !matches!(op, BinOp::Eq | BinOp::Neq) {
            return Ok(TypedExpr {
                kind: TypedExprKind::BinOp(op, Box::new(left_out), Box::new(right_out)),
                ty: ty.clone(),
            });
        }
        let wanted = Constraint {
            class: "Eq".to_string(),
            args: vec![left.ty.clone()],
        };
        let dict = evidence_expr(self.env, &solve(self.env, &self.givens, &wanted)?)?;
        let operand = Box::new(left.ty.clone());
        let eq_ty = Type::Fun(
            operand.clone(),
            Box::new(Type::Fun(operand, Box::new(Type::Bool))),
        );
        let method = TypedExpr {
            kind: TypedExprKind::FieldAccess(Box::new(dict), "eq".to_string()),
            ty: eq_ty,
        };
        let call = apply_all(method, vec![left_out, right_out]);
        if op == BinOp::Neq {
            return Ok(negate(call));
        }
        Ok(call)
    }
}

/// `if call then false else true`, typed `Bool`.
fn negate(call: TypedExpr) -> TypedExpr {
    let literal = |value| TypedExpr {
        kind: TypedExprKind::Bool(value),
        ty: Type::Bool,
    };
    TypedExpr {
        kind: TypedExprKind::If(
            Box::new(call),
            Box::new(literal(false)),
            Box::new(literal(true)),
        ),
        ty: Type::Bool,
    }
}

/// Wrap `body` in one unannotated `Fun` per dictionary parameter, the
/// first parameter outermost.
pub(super) fn lambda_all(params: Vec<(String, Type)>, body: TypedExpr) -> TypedExpr {
    params.into_iter().rev().fold(body, |acc, (name, ty)| {
        let fun_ty = Type::Fun(Box::new(ty), Box::new(acc.ty.clone()));
        TypedExpr {
            kind: TypedExprKind::Fun(name, None, Box::new(acc)),
            ty: fun_ty,
        }
    })
}

/// The unconstrained scheme of a dictionary-abstracted binding: the same
/// quantified variables over the type of the dictionary lambda `value`.
pub(super) fn dictionary_scheme(scheme: &TypeScheme, value: &TypedExpr) -> TypeScheme {
    TypeScheme {
        vars: scheme.vars.clone(),
        ty: value.ty.clone(),
        constraints: Vec::new(),
    }
}
