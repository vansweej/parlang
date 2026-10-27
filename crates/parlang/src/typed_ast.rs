//! Typed AST produced by Hindley-Milner inference.
//!
//! Each [`TypedExpr`] carries the `Type` inferred for that node. The tree is
//! built *through* the inference helpers and then resolved per root under the
//! final substitution, so every `ty` is fully substituted. Some nodes are
//! always type-variable-free leaves by construction: `Load` bodies are never
//! inspected by inference, and unregistered constructors yield a fresh var.

use crate::ast::{Expr, Pattern, TypeAnnotation, TypeExpr};
use crate::types::{Type, TypeScheme};
use std::fmt::Write;

/// An expression annotated with its inferred type.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
}

/// Typed mirror of [`crate::ast::Expr`]. Children are typed; see module docs
/// for the `Load` exception.
#[derive(Debug, Clone, PartialEq)]
pub enum TypedExprKind {
    Int(i64),
    Bool(bool),
    Char(char),
    Float(f64),
    Var(String),
    BinOp(crate::ast::BinOp, Box<TypedExpr>, Box<TypedExpr>),
    If(Box<TypedExpr>, Box<TypedExpr>, Box<TypedExpr>),
    Let(
        String,
        Option<TypeAnnotation>,
        Box<TypedExpr>,
        Box<TypedExpr>,
    ),
    Fun(String, Option<TypeAnnotation>, Box<TypedExpr>),
    App(Box<TypedExpr>, Box<TypedExpr>),
    /// The loaded body is carried untyped: inference never inspects it, so the
    /// node's `ty` is a fresh variable.
    Load(String, Box<Expr>),
    Rec(String, Box<TypedExpr>),
    Match(Box<TypedExpr>, Vec<TypedMatchArm>),
    Tuple(Vec<TypedExpr>),
    TupleProj(Box<TypedExpr>, usize),
    TypeAlias(String, TypeExpr, Box<TypedExpr>),
    Record(Vec<(String, TypedExpr)>),
    FieldAccess(Box<TypedExpr>, String),
    TypeDef {
        name: String,
        type_params: Vec<String>,
        constructors: Vec<(String, Vec<TypeAnnotation>)>,
        body: Box<TypedExpr>,
    },
    Constructor(String, Vec<TypedExpr>),
}

/// A typed match arm: the (untyped) pattern, the variable bindings it
/// introduces with their types, and the typed body.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedMatchArm {
    pub pattern: Pattern,
    pub bindings: Vec<(String, Type)>,
    pub body: TypedExpr,
}

/// Typed mirror of [`crate::ast::Decl`].
#[derive(Debug, Clone, PartialEq)]
pub enum TypedDecl {
    Let {
        name: String,
        ty_ann: Option<TypeAnnotation>,
        scheme: TypeScheme,
        value: Box<TypedExpr>,
        doc: Option<String>,
    },
    Data {
        name: String,
        type_params: Vec<String>,
        constructors: Vec<(String, Vec<TypeAnnotation>)>,
        doc: Option<String>,
    },
    TypeAlias {
        name: String,
        ty_expr: TypeExpr,
        doc: Option<String>,
    },
}

/// Typed mirror of [`crate::ast::Program`]: typed declarations, an optional
/// typed trailing body, and the program's overall type.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    pub decls: Vec<TypedDecl>,
    pub body: Option<TypedExpr>,
    pub ty: Type,
}

/// Render a [`TypedProgram`] as an indented text tree (2 spaces per level).
///
/// The root line is `Program : <ty>`; every expression node prints as
/// `<role>: <Label> : <ty>` and declarations show their `TypeScheme`.
#[must_use]
pub fn typed_program_to_text(program: &TypedProgram) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Program : {}", program.ty);
    for (i, decl) in program.decls.iter().enumerate() {
        write_decl(&mut out, i, decl, 1);
    }
    if let Some(body) = &program.body {
        write_expr(&mut out, "body", body, 1);
    }
    out
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn opt_ann(ann: Option<&TypeAnnotation>) -> String {
    match ann {
        Some(a) => format!(" : {a}"),
        None => String::new(),
    }
}

fn render_params(params: &[String]) -> String {
    let mut s = String::new();
    for p in params {
        let _ = write!(s, " {p}");
    }
    s
}

fn render_anns(anns: &[TypeAnnotation]) -> String {
    let mut s = String::new();
    for a in anns {
        match a {
            TypeAnnotation::App(_, args) if !args.is_empty() => {
                let _ = write!(s, " ({a})");
            }
            _ => {
                let _ = write!(s, " {a}");
            }
        }
    }
    s
}

fn write_decl(out: &mut String, index: usize, decl: &TypedDecl, depth: usize) {
    indent(out, depth);
    match decl {
        TypedDecl::Let {
            name,
            scheme,
            value,
            ..
        } => {
            let _ = writeln!(out, "decl {index}: Let {name} : {scheme}");
            write_expr(out, "value", value, depth + 1);
        }
        TypedDecl::Data {
            name,
            type_params,
            constructors,
            ..
        } => {
            let _ = writeln!(
                out,
                "decl {index}: Data {name}{}",
                render_params(type_params)
            );
            for (ctor, anns) in constructors {
                indent(out, depth + 1);
                let _ = writeln!(out, "ctor: {ctor}{}", render_anns(anns));
            }
        }
        TypedDecl::TypeAlias { name, ty_expr, .. } => {
            let _ = writeln!(out, "decl {index}: TypeAlias {name} = {ty_expr}");
        }
    }
}

fn node_label(expr: &TypedExpr) -> String {
    match &expr.kind {
        TypedExprKind::Int(n) => format!("Int {n}"),
        TypedExprKind::Bool(b) => format!("Bool {b}"),
        TypedExprKind::Char(c) => format!("Char {}", crate::ast::Literal::Char(*c)),
        TypedExprKind::Float(fl) => format!("Float {fl}"),
        TypedExprKind::Var(name) => format!("Var {name}"),
        TypedExprKind::BinOp(op, _, _) => format!("BinOp {op}"),
        TypedExprKind::If(_, _, _) => "If".to_string(),
        TypedExprKind::Let(name, ann, _, _) => {
            format!("Let {name}{}", opt_ann(ann.as_ref()))
        }
        TypedExprKind::Fun(name, ann, _) => {
            format!("Fun {name}{}", opt_ann(ann.as_ref()))
        }
        TypedExprKind::App(_, _) => "App".to_string(),
        TypedExprKind::Load(path, _) => format!("Load \"{path}\""),
        TypedExprKind::Rec(name, _) => format!("Rec {name}"),
        TypedExprKind::Match(_, _) => "Match".to_string(),
        TypedExprKind::Tuple(_) => "Tuple".to_string(),
        TypedExprKind::TupleProj(_, i) => format!("TupleProj {i}"),
        TypedExprKind::TypeAlias(name, ty_expr, _) => format!("TypeAlias {name} = {ty_expr}"),
        TypedExprKind::Record(_) => "Record".to_string(),
        TypedExprKind::FieldAccess(_, field) => format!("FieldAccess {field}"),
        TypedExprKind::TypeDef {
            name, type_params, ..
        } => format!("TypeDef {name}{}", render_params(type_params)),
        TypedExprKind::Constructor(name, _) => format!("Constructor {name}"),
    }
}

fn write_bindings(bindings: &[(String, Type)]) -> String {
    let mut s = String::new();
    for (i, (name, ty)) in bindings.iter().enumerate() {
        if i == 0 {
            let _ = write!(s, "{name} : {ty}");
        } else {
            let _ = write!(s, ", {name} : {ty}");
        }
    }
    s
}

fn write_arm(out: &mut String, index: usize, arm: &TypedMatchArm, depth: usize) {
    indent(out, depth);
    if arm.bindings.is_empty() {
        let _ = writeln!(out, "arm {index}: {}", arm.pattern);
    } else {
        let _ = writeln!(
            out,
            "arm {index}: {} [{}]",
            arm.pattern,
            write_bindings(&arm.bindings)
        );
    }
    write_expr(out, "result", &arm.body, depth + 1);
}

fn write_children(out: &mut String, expr: &TypedExpr, depth: usize) {
    match &expr.kind {
        TypedExprKind::BinOp(_, l, r) => {
            write_expr(out, "left", l, depth);
            write_expr(out, "right", r, depth);
        }
        TypedExprKind::If(c, t, e) => {
            write_expr(out, "cond", c, depth);
            write_expr(out, "then", t, depth);
            write_expr(out, "else", e, depth);
        }
        TypedExprKind::Let(_, _, v, b) => {
            write_expr(out, "value", v, depth);
            write_expr(out, "body", b, depth);
        }
        TypedExprKind::Fun(_, _, b)
        | TypedExprKind::Rec(_, b)
        | TypedExprKind::TypeAlias(_, _, b)
        | TypedExprKind::TypeDef { body: b, .. } => write_expr(out, "body", b, depth),
        TypedExprKind::App(f, a) => {
            write_expr(out, "func", f, depth);
            write_expr(out, "arg", a, depth);
        }
        TypedExprKind::Load(_, body) => {
            indent(out, depth);
            let _ = writeln!(out, "body (untyped): {body}");
        }
        TypedExprKind::Match(s, arms) => {
            write_expr(out, "scrutinee", s, depth);
            for (i, arm) in arms.iter().enumerate() {
                write_arm(out, i, arm, depth);
            }
        }
        TypedExprKind::Tuple(elems) => {
            for (i, elem) in elems.iter().enumerate() {
                let role = format!("elem {i}");
                write_expr(out, &role, elem, depth);
            }
        }
        TypedExprKind::TupleProj(t, _) => write_expr(out, "tuple", t, depth),
        TypedExprKind::Record(fields) => {
            for (name, value) in fields {
                let role = format!("field {name}");
                write_expr(out, &role, value, depth);
            }
        }
        TypedExprKind::FieldAccess(r, _) => write_expr(out, "record", r, depth),
        TypedExprKind::Constructor(_, args) => {
            for (i, arg) in args.iter().enumerate() {
                let role = format!("arg{i}");
                write_expr(out, &role, arg, depth);
            }
        }
        TypedExprKind::Int(_)
        | TypedExprKind::Bool(_)
        | TypedExprKind::Char(_)
        | TypedExprKind::Float(_)
        | TypedExprKind::Var(_) => {}
    }
}

fn write_expr(out: &mut String, role: &str, expr: &TypedExpr, depth: usize) {
    indent(out, depth);
    let _ = writeln!(out, "{role}: {} : {}", node_label(expr), expr.ty);
    write_children(out, expr, depth + 1);
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_program;
    use crate::typechecker::typecheck_program_typed;

    fn text_of(src: &str) -> String {
        let program = parse_program(src).expect("parse should succeed");
        let typed = typecheck_program_typed(&program).expect("typecheck should succeed");
        super::typed_program_to_text(&typed)
    }

    #[test]
    fn identity_application_tree_is_byte_exact() {
        let out = text_of("(fun x -> x) 100");
        assert_eq!(
            out,
            "Program : Int\n  body: App : Int\n    func: Fun x : Int -> Int\n      body: Var x : Int\n    arg: Int 100 : Int\n"
        );
    }

    #[test]
    fn polymorphic_decl_shows_scheme() {
        let out = text_of("let id = fun x -> x;\nid 1");
        assert!(out.starts_with("Program : Int\n"));
        assert!(out.contains("decl 0: Let id : forall t"));
    }

    #[test]
    fn match_tree_shows_scrutinee_and_arm() {
        let out = text_of("match 1 with | x -> x");
        assert!(out.contains("scrutinee: Int 1 : Int"));
        assert!(out.contains("arm 0: x [x : Int]"));
        assert!(out.contains("result: Var x : Int"));
    }

    #[test]
    fn data_decl_shows_constructors() {
        let out = text_of("data Option a = Some a | None;\n0");
        assert!(out.contains("decl 0: Data Option a"));
        assert!(out.contains("    ctor: Some a"));
        assert!(out.contains("    ctor: None"));
    }

    #[test]
    fn data_decl_parenthesises_app_arg() {
        let out = text_of("data List a = Nil | Cons a (List a);\n0");
        assert!(out.contains("ctor: Cons a (List a)"));
    }
}
