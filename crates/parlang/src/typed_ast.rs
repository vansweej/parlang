//! Typed AST produced by Hindley-Milner inference.
//!
//! Each [`TypedExpr`] carries the `Type` inferred for that node. The tree is
//! built *through* the inference helpers and then resolved per root under the
//! final substitution, so every `ty` is fully substituted. Some nodes are
//! always type-variable-free leaves by construction: `Load` bodies are never
//! inspected by inference, and unregistered constructors yield a fresh var.
//!
//! [`TypedExprKind::Pending`] is temporary scaffolding used during the
//! incremental migration of the inference helpers; it is removed once every
//! helper threads a real typed node.

use crate::ast::{Expr, Pattern, TypeAnnotation, TypeExpr};
use crate::types::{Type, TypeScheme};

/// An expression annotated with its inferred type.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub kind: TypedExprKind,
    pub ty: Type,
}

/// Typed mirror of [`crate::ast::Expr`]. Children are typed; see module docs
/// for the `Load` and `Pending` exceptions.
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
    /// Temporary scaffold: an un-migrated subtree carried opaquely as its
    /// original untyped `Expr`. Removed once all helpers are migrated.
    Pending(Expr),
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
