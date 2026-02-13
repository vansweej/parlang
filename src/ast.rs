/// Abstract Syntax Tree definitions for the ParLang language
/// This defines the structure of programs in our ML-alike functional language

use std::fmt;

/// Expression types in the language
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal: 42
    Int(i64),
    
    /// Boolean literal: true, false
    Bool(bool),
    
    /// Variable reference: x, y, foo
    Var(String),
    
    /// Binary operation: e1 + e2, e1 * e2, etc.
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    
    /// If-then-else: if e1 then e2 else e3
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    
    /// Let binding: let x = e1 in e2
    Let(String, Box<Expr>, Box<Expr>),
    
    /// Function definition: fun x -> e
    Fun(String, Box<Expr>),
    
    /// Function application: f e
    App(Box<Expr>, Box<Expr>),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,  // +
    Sub,  // -
    Mul,  // *
    Div,  // /
    Eq,   // ==
    Neq,  // !=
    Lt,   // <
    Le,   // <=
    Gt,   // >
    Ge,   // >=
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Int(n) => write!(f, "{}", n),
            Expr::Bool(b) => write!(f, "{}", b),
            Expr::Var(name) => write!(f, "{}", name),
            Expr::BinOp(op, left, right) => write!(f, "({} {} {})", left, op, right),
            Expr::If(cond, then_branch, else_branch) => {
                write!(f, "(if {} then {} else {})", cond, then_branch, else_branch)
            }
            Expr::Let(name, value, body) => {
                write!(f, "(let {} = {} in {})", name, value, body)
            }
            Expr::Fun(param, body) => write!(f, "(fun {} -> {})", param, body),
            Expr::App(func, arg) => write!(f, "({} {})", func, arg),
        }
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Eq => "==",
            BinOp::Neq => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
        };
        write!(f, "{}", s)
    }
}
