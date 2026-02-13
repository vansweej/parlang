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
    
    /// Load expression: load "filepath" in e
    Load(String, Box<Expr>),
    
    /// Sequential let bindings: let x = e1; let y = e2; expr
    /// Vector of (name, value) pairs, followed by a body expression
    Seq(Vec<(String, Expr)>, Box<Expr>),
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
            Expr::Load(filepath, body) => write!(f, "(load \"{}\" in {})", filepath, body),
            Expr::Seq(bindings, body) => {
                write!(f, "(")?;
                for (i, (name, value)) in bindings.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    write!(f, "let {} = {}", name, value)?;
                }
                write!(f, "; {})", body)
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    // Test Expr variants construction and equality
    #[test]
    fn test_expr_int() {
        let expr = Expr::Int(42);
        assert_eq!(expr, Expr::Int(42));
        assert_ne!(expr, Expr::Int(43));
    }

    #[test]
    fn test_expr_bool() {
        let expr_true = Expr::Bool(true);
        let expr_false = Expr::Bool(false);
        assert_eq!(expr_true, Expr::Bool(true));
        assert_eq!(expr_false, Expr::Bool(false));
        assert_ne!(expr_true, expr_false);
    }

    #[test]
    fn test_expr_var() {
        let expr = Expr::Var("x".to_string());
        assert_eq!(expr, Expr::Var("x".to_string()));
        assert_ne!(expr, Expr::Var("y".to_string()));
    }

    #[test]
    fn test_expr_binop() {
        let expr = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        let expr2 = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(expr, expr2);
    }

    #[test]
    fn test_expr_if() {
        let expr = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(
            expr,
            Expr::If(
                Box::new(Expr::Bool(true)),
                Box::new(Expr::Int(1)),
                Box::new(Expr::Int(2)),
            )
        );
    }

    #[test]
    fn test_expr_let() {
        let expr = Expr::Let(
            "x".to_string(),
            Box::new(Expr::Int(42)),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(
            expr,
            Expr::Let(
                "x".to_string(),
                Box::new(Expr::Int(42)),
                Box::new(Expr::Var("x".to_string())),
            )
        );
    }

    #[test]
    fn test_expr_fun() {
        let expr = Expr::Fun("x".to_string(), Box::new(Expr::Var("x".to_string())));
        assert_eq!(
            expr,
            Expr::Fun("x".to_string(), Box::new(Expr::Var("x".to_string())))
        );
    }

    #[test]
    fn test_expr_app() {
        let expr = Expr::App(
            Box::new(Expr::Var("f".to_string())),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(
            expr,
            Expr::App(
                Box::new(Expr::Var("f".to_string())),
                Box::new(Expr::Int(42)),
            )
        );
    }

    #[test]
    fn test_expr_load() {
        let expr = Expr::Load(
            "lib.par".to_string(),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(
            expr,
            Expr::Load(
                "lib.par".to_string(),
                Box::new(Expr::Var("x".to_string())),
            )
        );
    }

    #[test]
    fn test_expr_seq() {
        let bindings = vec![
            ("x".to_string(), Expr::Int(42)),
            ("y".to_string(), Expr::Int(10)),
        ];
        let expr = Expr::Seq(bindings.clone(), Box::new(Expr::Var("x".to_string())));
        assert_eq!(
            expr,
            Expr::Seq(bindings, Box::new(Expr::Var("x".to_string())))
        );
    }

    // Test Clone trait
    #[test]
    fn test_expr_clone() {
        let expr = Expr::Int(42);
        let cloned = expr.clone();
        assert_eq!(expr, cloned);
    }

    #[test]
    fn test_binop_clone() {
        let op = BinOp::Add;
        let cloned = op;
        assert_eq!(op, cloned);
    }

    // Test Display implementation for Expr
    #[test]
    fn test_display_int() {
        assert_eq!(format!("{}", Expr::Int(42)), "42");
        assert_eq!(format!("{}", Expr::Int(-10)), "-10");
    }

    #[test]
    fn test_display_bool() {
        assert_eq!(format!("{}", Expr::Bool(true)), "true");
        assert_eq!(format!("{}", Expr::Bool(false)), "false");
    }

    #[test]
    fn test_display_var() {
        assert_eq!(format!("{}", Expr::Var("x".to_string())), "x");
        assert_eq!(format!("{}", Expr::Var("foo_bar".to_string())), "foo_bar");
    }

    #[test]
    fn test_display_binop() {
        let expr = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(format!("{}", expr), "(1 + 2)");
    }

    #[test]
    fn test_display_binop_nested() {
        let expr = Expr::BinOp(
            BinOp::Mul,
            Box::new(Expr::BinOp(
                BinOp::Add,
                Box::new(Expr::Int(1)),
                Box::new(Expr::Int(2)),
            )),
            Box::new(Expr::Int(3)),
        );
        assert_eq!(format!("{}", expr), "((1 + 2) * 3)");
    }

    #[test]
    fn test_display_if() {
        let expr = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(format!("{}", expr), "(if true then 1 else 2)");
    }

    #[test]
    fn test_display_let() {
        let expr = Expr::Let(
            "x".to_string(),
            Box::new(Expr::Int(42)),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(format!("{}", expr), "(let x = 42 in x)");
    }

    #[test]
    fn test_display_fun() {
        let expr = Expr::Fun("x".to_string(), Box::new(Expr::Var("x".to_string())));
        assert_eq!(format!("{}", expr), "(fun x -> x)");
    }

    #[test]
    fn test_display_app() {
        let expr = Expr::App(
            Box::new(Expr::Var("f".to_string())),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(format!("{}", expr), "(f 42)");
    }

    #[test]
    fn test_display_load() {
        let expr = Expr::Load(
            "lib.par".to_string(),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(format!("{}", expr), "(load \"lib.par\" in x)");
    }

    #[test]
    fn test_display_seq() {
        let bindings = vec![
            ("x".to_string(), Expr::Int(42)),
            ("y".to_string(), Expr::Int(10)),
        ];
        let expr = Expr::Seq(bindings, Box::new(Expr::Var("x".to_string())));
        assert_eq!(format!("{}", expr), "(let x = 42; let y = 10; x)");
    }

    // Test Display implementation for BinOp
    #[test]
    fn test_binop_display_add() {
        assert_eq!(format!("{}", BinOp::Add), "+");
    }

    #[test]
    fn test_binop_display_sub() {
        assert_eq!(format!("{}", BinOp::Sub), "-");
    }

    #[test]
    fn test_binop_display_mul() {
        assert_eq!(format!("{}", BinOp::Mul), "*");
    }

    #[test]
    fn test_binop_display_div() {
        assert_eq!(format!("{}", BinOp::Div), "/");
    }

    #[test]
    fn test_binop_display_eq() {
        assert_eq!(format!("{}", BinOp::Eq), "==");
    }

    #[test]
    fn test_binop_display_neq() {
        assert_eq!(format!("{}", BinOp::Neq), "!=");
    }

    #[test]
    fn test_binop_display_lt() {
        assert_eq!(format!("{}", BinOp::Lt), "<");
    }

    #[test]
    fn test_binop_display_le() {
        assert_eq!(format!("{}", BinOp::Le), "<=");
    }

    #[test]
    fn test_binop_display_gt() {
        assert_eq!(format!("{}", BinOp::Gt), ">");
    }

    #[test]
    fn test_binop_display_ge() {
        assert_eq!(format!("{}", BinOp::Ge), ">=");
    }

    // Test BinOp equality
    #[test]
    fn test_binop_equality() {
        assert_eq!(BinOp::Add, BinOp::Add);
        assert_ne!(BinOp::Add, BinOp::Sub);
        assert_eq!(BinOp::Eq, BinOp::Eq);
        assert_ne!(BinOp::Lt, BinOp::Gt);
    }

    // Test complex nested expressions
    #[test]
    fn test_complex_nested_expr() {
        // let f = fun x -> x + 1 in f 41
        let expr = Expr::Let(
            "f".to_string(),
            Box::new(Expr::Fun(
                "x".to_string(),
                Box::new(Expr::BinOp(
                    BinOp::Add,
                    Box::new(Expr::Var("x".to_string())),
                    Box::new(Expr::Int(1)),
                )),
            )),
            Box::new(Expr::App(
                Box::new(Expr::Var("f".to_string())),
                Box::new(Expr::Int(41)),
            )),
        );
        assert_eq!(
            format!("{}", expr),
            "(let f = (fun x -> (x + 1)) in (f 41))"
        );
    }
}
