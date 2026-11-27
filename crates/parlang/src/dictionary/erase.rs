//! Erasure of a dictionary-explicit `TypedProgram` to an untyped `Program`
//! for the surface tree-walker (M2.3b).
//!
//! Types are dropped; dictionary lambdas, applications, records and field
//! selections become ordinary `Expr` nodes, and `class` declarations are
//! dropped. An instance dictionary `let` named `$dict_<Class>_<Tycon>` is
//! bound recursively so its method bodies may reference it: a dictionary
//! function becomes `rec $dict_X -> fun $d0 -> ...`, and a plain
//! dictionary record becomes `(rec $dict_X -> fun $unit -> record) ()`
//! with every self-reference inside `record` rewritten to `$dict_X ()`.
//! The builtin `Eq` dictionaries (`$dict_Eq_Int`, `$dict_Eq_Bool`,
//! `$dict_Eq_Char`, `$dict_Eq_Float`) are defined before all other
//! declarations when the program references them.

use std::collections::BTreeSet;
use std::fmt;

use crate::ast::{BinOp, Decl, Expr, Pattern, Program};
use crate::typed_ast::{TypedDecl, TypedExpr, TypedExprKind, TypedMatchArm, TypedProgram};

/// Names of the builtin `Eq` dictionaries; no program declaration defines them.
const BUILTIN_EQ_DICTIONARIES: [&str; 4] = [
    "$dict_Eq_Int",
    "$dict_Eq_Bool",
    "$dict_Eq_Char",
    "$dict_Eq_Float",
];

/// Name prefix of instance dictionary `let`s.
const DICTIONARY_PREFIX: &str = "$dict_";

/// Parameter of the thunk that ties the knot of a context-free instance.
const UNIT_PARAM: &str = "$unit";

/// Why a dictionary-explicit program cannot be erased.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EraseError {
    /// An `instance` declaration of this class survived dictionary insertion.
    UnexpectedInstance(String),
    /// The program contains `load` of this path; the untyped loaded body
    /// never received dictionaries.
    Load(String),
}

impl fmt::Display for EraseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EraseError::UnexpectedInstance(class) => {
                write!(f, "instance {class} survived dictionary insertion")
            }
            EraseError::Load(path) => {
                write!(f, "dictionary passing cannot cross load \"{path}\"")
            }
        }
    }
}

impl std::error::Error for EraseError {}

/// Erase the dictionary-explicit `program` (the output of
/// `insert_dictionaries`) to an untyped `Program` the surface evaluator runs.
///
/// # Errors
///
/// `UnexpectedInstance` when an `instance` declaration is left, and `Load`
/// when the program contains a `load` expression.
pub fn erase_program(program: &TypedProgram) -> Result<Program, EraseError> {
    let mut eraser = Eraser::default();
    let mut decls = Vec::with_capacity(program.decls.len());
    for decl in &program.decls {
        if let Some(erased) = eraser.decl(decl)? {
            decls.push(erased);
        }
    }
    let body = program
        .body
        .as_ref()
        .map(|body| eraser.expr(body))
        .transpose()?;
    let mut all: Vec<Decl> = BUILTIN_EQ_DICTIONARIES
        .into_iter()
        .filter(|name| eraser.used.contains(*name))
        .map(builtin_eq_dictionary)
        .collect();
    all.extend(decls);
    Ok(Program { decls: all, body })
}

/// The builtin `Eq` dictionary `name`: a record whose `eq` is primitive `==`.
fn builtin_eq_dictionary(name: &str) -> Decl {
    let eq = Expr::Fun(
        "a".to_string(),
        None,
        Box::new(Expr::Fun(
            "b".to_string(),
            None,
            Box::new(Expr::BinOp(
                BinOp::Eq,
                Box::new(Expr::Var("a".to_string())),
                Box::new(Expr::Var("b".to_string())),
            )),
        )),
    );
    Decl::Let {
        name: name.to_string(),
        ty_ann: None,
        value: Expr::Record(vec![("eq".to_string(), eq)]),
        doc: None,
    }
}

/// Erasure state.
#[derive(Default)]
struct Eraser {
    /// Builtin `Eq` dictionaries referenced so far.
    used: BTreeSet<String>,
    /// The context-free instance dictionary being erased; its
    /// self-references become `name ()`.
    thunked: Option<String>,
}

impl Eraser {
    fn decl(&mut self, decl: &TypedDecl) -> Result<Option<Decl>, EraseError> {
        match decl {
            TypedDecl::Let {
                name,
                ty_ann,
                value,
                doc,
                ..
            } => {
                let value_out = if name.starts_with(DICTIONARY_PREFIX) {
                    self.dictionary(name, value)?
                } else {
                    self.expr(value)?
                };
                Ok(Some(Decl::Let {
                    name: name.clone(),
                    ty_ann: ty_ann.clone(),
                    value: value_out,
                    doc: doc.clone(),
                }))
            }
            TypedDecl::Data {
                name,
                type_params,
                constructors,
                doc,
                ..
            } => Ok(Some(Decl::Data {
                name: name.clone(),
                type_params: type_params.clone(),
                constructors: constructors.clone(),
                doc: doc.clone(),
            })),
            TypedDecl::TypeAlias { name, ty_expr, doc } => Ok(Some(Decl::TypeAlias {
                name: name.clone(),
                ty_expr: ty_expr.clone(),
                doc: doc.clone(),
            })),
            TypedDecl::Class { .. } => Ok(None),
            TypedDecl::Instance { class, .. } => Err(EraseError::UnexpectedInstance(class.clone())),
        }
    }

    /// The instance dictionary `name` bound recursively, so its method
    /// bodies may reference it.
    fn dictionary(&mut self, name: &str, value: &TypedExpr) -> Result<Expr, EraseError> {
        if matches!(value.kind, TypedExprKind::Fun(..)) {
            let function = self.expr(value)?;
            return Ok(Expr::Rec(name.to_string(), Box::new(function)));
        }
        self.thunked = Some(name.to_string());
        let record = self.expr(value);
        self.thunked = None;
        let thunk = Expr::Fun(UNIT_PARAM.to_string(), None, Box::new(record?));
        Ok(Expr::App(
            Box::new(Expr::Rec(name.to_string(), Box::new(thunk))),
            Box::new(Expr::Tuple(Vec::new())),
        ))
    }

    fn var(&mut self, name: &str) -> Expr {
        if BUILTIN_EQ_DICTIONARIES.contains(&name) {
            self.used.insert(name.to_string());
        }
        let var = Expr::Var(name.to_string());
        if self.thunked.as_deref() == Some(name) {
            return Expr::App(Box::new(var), Box::new(Expr::Tuple(Vec::new())));
        }
        var
    }

    fn boxed(&mut self, expr: &TypedExpr) -> Result<Box<Expr>, EraseError> {
        self.expr(expr).map(Box::new)
    }

    fn all(&mut self, items: &[TypedExpr]) -> Result<Vec<Expr>, EraseError> {
        items.iter().map(|item| self.expr(item)).collect()
    }

    fn fields(
        &mut self,
        fields: &[(String, TypedExpr)],
    ) -> Result<Vec<(String, Expr)>, EraseError> {
        fields
            .iter()
            .map(|(name, value)| Ok((name.clone(), self.expr(value)?)))
            .collect()
    }

    fn arms(&mut self, arms: &[TypedMatchArm]) -> Result<Vec<(Pattern, Expr)>, EraseError> {
        arms.iter()
            .map(|arm| Ok((arm.pattern.to_pattern(), self.expr(&arm.body)?)))
            .collect()
    }

    fn expr(&mut self, expr: &TypedExpr) -> Result<Expr, EraseError> {
        Ok(match &expr.kind {
            TypedExprKind::Int(value) => Expr::Int(*value),
            TypedExprKind::Bool(value) => Expr::Bool(*value),
            TypedExprKind::Char(value) => Expr::Char(*value),
            TypedExprKind::Float(value) => Expr::Float(*value),
            TypedExprKind::Var(name, _) => self.var(name),
            TypedExprKind::BinOp(op, left, right) => {
                Expr::BinOp(*op, self.boxed(left)?, self.boxed(right)?)
            }
            TypedExprKind::If(cond, then_branch, else_branch) => Expr::If(
                self.boxed(cond)?,
                self.boxed(then_branch)?,
                self.boxed(else_branch)?,
            ),
            TypedExprKind::Let(name, ann, _, value, body) => Expr::Let(
                name.clone(),
                ann.clone(),
                self.boxed(value)?,
                self.boxed(body)?,
            ),
            TypedExprKind::Fun(param, ann, body) => {
                Expr::Fun(param.clone(), ann.clone(), self.boxed(body)?)
            }
            TypedExprKind::App(function, arg) => Expr::App(self.boxed(function)?, self.boxed(arg)?),
            TypedExprKind::Load(path, _) => return Err(EraseError::Load(path.clone())),
            TypedExprKind::Rec(name, body) => Expr::Rec(name.clone(), self.boxed(body)?),
            TypedExprKind::Match(scrutinee, arms) => {
                Expr::Match(self.boxed(scrutinee)?, self.arms(arms)?)
            }
            TypedExprKind::Tuple(items) => Expr::Tuple(self.all(items)?),
            TypedExprKind::TupleProj(tuple, index) => Expr::TupleProj(self.boxed(tuple)?, *index),
            TypedExprKind::TypeAlias(name, ty_expr, body) => {
                Expr::TypeAlias(name.clone(), ty_expr.clone(), self.boxed(body)?)
            }
            TypedExprKind::Record(fields) => Expr::Record(self.fields(fields)?),
            TypedExprKind::FieldAccess(record, field) => {
                Expr::FieldAccess(self.boxed(record)?, field.clone())
            }
            TypedExprKind::TypeDef {
                name,
                type_params,
                constructors,
                body,
                ..
            } => Expr::TypeDef {
                name: name.clone(),
                type_params: type_params.clone(),
                constructors: constructors.clone(),
                body: self.boxed(body)?,
            },
            TypedExprKind::Constructor(name, args) => {
                Expr::Constructor(name.clone(), self.all(args)?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::insert_dictionaries;
    use crate::parser::parse_program;
    use crate::typechecker::{typecheck_program_typed_with_env, TypeEnv};
    use crate::{eval_program, Environment};

    const NAT_EQ: &str = "data Nat = Z | S Nat;\ninstance Eq Nat where { eq = fun x -> fun y -> (match x with | Z -> (match y with | Z -> true | S m -> false) | S n -> (match y with | Z -> false | S m -> eq n m)) };\n";

    const BOX_EQ: &str = "data Box a = MkBox a;\ninstance Eq a => Eq (Box a) where { eq = fun x -> fun y -> (match x with | MkBox p -> (match y with | MkBox q -> eq p q)) };\nlet same = fun x -> fun y -> x == y;\n";

    const PAIR: &str = "class Pair a where { first : a -> Int; second : a -> Int };\ninstance Pair Bool where { second = fun b -> 2; first = fun b -> 1 };\n";

    /// `source` type-checked, before dictionary insertion.
    fn checked(source: &str) -> (TypedProgram, TypeEnv) {
        let parsed = parse_program(source).expect("parses");
        let mut env = TypeEnv::new();
        let typed = typecheck_program_typed_with_env(&parsed, &mut env).expect("type checks");
        (typed, env)
    }

    /// `source` after dictionary insertion.
    fn translate(source: &str) -> TypedProgram {
        let (typed, env) = checked(source);
        insert_dictionaries(&typed, &env).expect("dictionaries inserted")
    }

    /// The displayed value of the erased translation of `source`.
    fn run(source: &str) -> String {
        let erased = erase_program(&translate(source)).expect("erases");
        match eval_program(&erased, &Environment::new()) {
            Ok(value) => format!("{value}"),
            Err(error) => format!("error: {error}"),
        }
    }

    #[test]
    fn instance_with_context_compares_through_dictionaries() {
        assert_eq!(run(&format!("{BOX_EQ}same (MkBox 1) (MkBox 2)")), "false");
        assert_eq!(run(&format!("{BOX_EQ}same (MkBox 1) (MkBox 1)")), "true");
    }

    #[test]
    fn context_free_instance_references_itself() {
        assert_eq!(run(&format!("{NAT_EQ}eq (S Z) (S Z)")), "true");
        assert_eq!(run(&format!("{NAT_EQ}eq (S Z) Z")), "false");
        assert_eq!(run(&format!("{NAT_EQ}eq (S (S Z)) (S (S Z))")), "true");
    }

    #[test]
    fn neq_on_a_user_type_negates_the_dictionary_call() {
        assert_eq!(run(&format!("{NAT_EQ}(S Z) != Z")), "true");
        assert_eq!(run(&format!("{NAT_EQ}(S Z) != (S Z)")), "false");
    }

    #[test]
    fn methods_are_selected_by_name_in_class_order() {
        assert_eq!(run(&format!("{PAIR}first true + second true")), "3");
    }

    #[test]
    fn body_residual_is_a_dictionary_function() {
        let shown = run("let same = fun x -> fun y -> x == y; same");
        assert!(shown.starts_with("<function $d"), "{shown}");
    }

    #[test]
    fn classes_are_dropped_and_used_builtins_prepended() {
        let erased = erase_program(&translate("let same = fun x -> fun y -> x == y; same 1 2"))
            .expect("erases");
        let names: Vec<&str> = erased
            .decls
            .iter()
            .filter_map(|decl| match decl {
                Decl::Let { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["$dict_Eq_Int", "same"]);
        let pair = erase_program(&translate(&format!("{PAIR}first true"))).expect("erases");
        assert!(!pair
            .decls
            .iter()
            .any(|decl| matches!(decl, Decl::Class { .. })));
    }

    #[test]
    fn surviving_instance_is_an_error() {
        let (typed, _) = checked(&format!("{NAT_EQ}eq Z Z"));
        assert_eq!(
            erase_program(&typed),
            Err(EraseError::UnexpectedInstance("Eq".to_string()))
        );
    }

    #[test]
    fn load_is_an_error() {
        let translated =
            translate("let same = fun x -> fun y -> x == y;\nload \"nofile.par\" in same 1 1");
        assert_eq!(
            erase_program(&translated),
            Err(EraseError::Load("nofile.par".to_string()))
        );
    }
}
