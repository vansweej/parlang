/// Abstract Syntax Tree definitions for the `ParLang` language
/// 
/// This module defines the Abstract Syntax Tree (AST) structure for ParLang,
/// representing the parsed structure of programs before evaluation or type checking.
/// 
/// # Overview
/// 
/// The AST consists of several key types:
/// 
/// - **`Expr`**: Core expression type representing all language constructs (literals,
///   variables, functions, applications, conditionals, pattern matching, etc.)
/// - **`Pattern`**: Patterns used in pattern matching and destructuring
/// - **`Literal`**: Primitive literal values (integers, booleans)
/// - **`TypeExpr`**: Type expressions for type aliases
/// - **`TypeAnnotation`**: Type annotations for sum type definitions
/// - **`BinOp`**: Binary operators for arithmetic and comparison
/// 
/// # Expression Types
/// 
/// The language supports:
/// - Literals: `Lit(Literal)`
/// - Variables: `Var(String)`
/// - Functions: `Fun(param, body)`
/// - Applications: `App(func, arg)`
/// - Recursion: `Rec(name, param, body)`
/// - Let bindings: `Let(name, ty_ann, value, body)`
/// - Sequential bindings: `Seq(bindings, body)`
/// - Conditionals: `If(cond, then_expr, else_expr)`
/// - Binary operations: `BinOp(op, left, right)`
/// - Pattern matching: `Match(expr, arms)`
/// - Tuples: `Tuple(elements)`
/// - Tuple projection: `TupleProj(tuple, index)`
/// - Records: `Record(fields)`
/// - Record access: `RecordAccess(record, field)`
/// - Type aliases: `TypeAlias(name, type_expr, body)`
/// - Sum types: `SumType(name, params, constructors, body)`
/// - Constructors: `Constructor(name, args)`
/// - Library loading: `Load(filepath, body)`
/// 
/// # Pattern Matching
/// 
/// Patterns support:
/// - Literal patterns: `Pattern::Literal(lit)`
/// - Variable binding: `Pattern::Var(name)`
/// - Wildcards: `Pattern::Wildcard`
/// - Tuples: `Pattern::Tuple(patterns)`
/// - Records: `Pattern::Record(fields)` with partial matching
/// - Constructors: `Pattern::Constructor(name, args)`
/// 
/// # Example
/// 
/// ```text
/// let x = 42 in x + 1
/// ```
/// 
/// Is represented as:
/// 
/// ```text
/// Let("x", None, 
///     Lit(Int(42)),
///     BinOp(Add, Var("x"), Lit(Int(1))))
/// ```
use std::fmt;

/// Literal values for pattern matching
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Literal {
    /// Integer literal
    Int(i64),
    /// Boolean literal
    Bool(bool),
    /// Character literal
    Char(char),
    /// Byte literal
    Byte(u8),
}

/// Pattern for pattern matching
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Literal pattern: 0, 1, true, false
    Literal(Literal),
    /// Variable pattern: binds the value to a name (x, n, acc)
    Var(String),
    /// Wildcard pattern: _ (matches anything without binding)
    Wildcard,
    /// Tuple pattern: (p1, p2, p3)
    Tuple(Vec<Pattern>),
    /// Record pattern: { field1: pattern1, field2: pattern2, ... }
    /// Can be partial (only match some fields)
    Record(Vec<(String, Pattern)>),
    
    /// Constructor pattern: Some x, Cons head tail, Left value
    Constructor(String, Vec<Pattern>),
}

/// Type expressions for type aliases
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// Integer type: Int
    Int,
    /// Boolean type: Bool
    Bool,
    /// Function type: T1 -> T2
    Fun(Box<TypeExpr>, Box<TypeExpr>),
    /// Type alias reference: Name
    Alias(String),
}

/// Type annotations for sum type definitions
#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnotation {
    /// Concrete type: Int, Bool
    Concrete(String),
    /// Type variable: a, b, t
    Var(String),
    /// Function type: a -> b
    Fun(Box<TypeAnnotation>, Box<TypeAnnotation>),
    /// Applied type: Option Int, List a
    App(String, Vec<TypeAnnotation>),
}

/// Expression types in the language
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Integer literal: 42
    Int(i64),
    
    /// Boolean literal: true, false
    Bool(bool),
    
    /// Character literal: 'a', 'Z', '\n'
    Char(char),
    
    /// Floating point literal: 3.14, -2.5, 0.0
    Float(f64),
    
    /// Byte literal: 0b, 255b
    Byte(u8),
    
    /// Variable reference: x, y, foo
    Var(String),
    
    /// Binary operation: e1 + e2, e1 * e2, etc.
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    
    /// If-then-else: if e1 then e2 else e3
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    
    /// Let binding: let x = e1 in e2
    /// Optional type annotation for the variable
    Let(String, Option<TypeAnnotation>, Box<Expr>, Box<Expr>),
    
    /// Function definition: fun x -> e
    /// Optional type annotation for the parameter
    Fun(String, Option<TypeAnnotation>, Box<Expr>),
    
    /// Function application: f e
    App(Box<Expr>, Box<Expr>),
    
    /// Load expression: load "filepath" in e
    Load(String, Box<Expr>),
    
    /// Sequential let bindings: let x = e1; let y = e2; expr
    /// Vector of (name, optional type annotation, value) triples, followed by a body expression
    Seq(Vec<(String, Option<TypeAnnotation>, Expr)>, Box<Expr>),
    
    /// Recursive function definition: rec name -> body
    /// The function can reference itself by name within its body
    Rec(String, Box<Expr>),
    
    /// Pattern matching: match e with | p1 -> e1 | p2 -> e2 | ...
    /// (scrutinee expression, vector of (pattern, result expression) arms)
    Match(Box<Expr>, Vec<(Pattern, Expr)>),
    
    /// Tuple construction: (e1, e2, e3, ...)
    Tuple(Vec<Expr>),
    
    /// Tuple projection: e.0, e.1, e.2, ...
    TupleProj(Box<Expr>, usize),
    
    /// Type alias definition: `type Name = TypeExpr in body`
    /// Defines a type alias that can be used in the body expression
    TypeAlias(String, TypeExpr, Box<Expr>),
    
    /// Record construction: { field1: expr1, field2: expr2, ... }
    /// Vec maintains insertion order for display purposes
    Record(Vec<(String, Expr)>),
    
    /// Field access: expr.field
    /// Accesses a named field from a record
    FieldAccess(Box<Expr>, String),
    
    /// Type definition: type Name a b = Constructor1 T1 T2 | Constructor2 T3 | ...
    /// Introduces a new algebraic data type with constructors
    TypeDef {
        /// Type name (e.g., "Option", "Either", "List")
        name: String,
        /// Type parameters (e.g., `["a", "b"]` for polymorphic types)
        type_params: Vec<String>,
        /// Constructors: (name, payload types)
        /// e.g., `[("Some", vec![TypeAnnotation::Var("a")]), ("None", vec![])]`
        constructors: Vec<(String, Vec<TypeAnnotation>)>,
        /// Body expression where this type is in scope
        body: Box<Expr>,
    },
    
    /// Constructor application: Some 42, Cons 1 rest, Left x
    Constructor(String, Vec<Expr>),
    
    /// Fixed-size array construction: [|e1, e2, e3|]
    /// All elements must be of the same type
    Array(Vec<Expr>),
    
    /// Array indexing: arr[i]
    /// Accesses element at index i (zero-based)
    ArrayIndex(Box<Expr>, Box<Expr>),
    
    /// Reference creation: ref expr
    /// Creates a mutable reference to a value
    Ref(Box<Expr>),
    
    /// Reference dereference: !expr
    /// Reads the value from a reference
    Deref(Box<Expr>),
    
    /// Reference assignment: ref_expr := value_expr
    /// Mutates the value stored in a reference
    RefAssign(Box<Expr>, Box<Expr>),
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
            Expr::Int(n) => write!(f, "{n}"),
            Expr::Bool(b) => write!(f, "{b}"),
            Expr::Char(c) => {
                write!(f, "'")?;
                match c {
                    '\n' => write!(f, "\\n")?,
                    '\t' => write!(f, "\\t")?,
                    '\r' => write!(f, "\\r")?,
                    '\\' => write!(f, "\\\\")?,
                    '\'' => write!(f, "\\'")?,
                    _ => write!(f, "{c}")?,
                }
                write!(f, "'")
            }
            Expr::Float(fl) => write!(f, "{fl}"),
            Expr::Byte(b) => write!(f, "{}b", b),
            Expr::Var(name) => write!(f, "{name}"),
            Expr::BinOp(op, left, right) => write!(f, "({left} {op} {right})"),
            Expr::If(cond, then_branch, else_branch) => {
                write!(f, "(if {cond} then {then_branch} else {else_branch})")
            }
            Expr::Let(name, ty_ann, value, body) => {
                if let Some(ty) = ty_ann {
                    write!(f, "(let {name} : {ty} = {value} in {body})")
                } else {
                    write!(f, "(let {name} = {value} in {body})")
                }
            }
            Expr::Fun(param, ty_ann, body) => {
                if let Some(ty) = ty_ann {
                    write!(f, "(fun {param} : {ty} -> {body})")
                } else {
                    write!(f, "(fun {param} -> {body})")
                }
            }
            Expr::App(func, arg) => write!(f, "({func} {arg})"),
            Expr::Load(filepath, body) => write!(f, "(load \"{filepath}\" in {body})"),
            Expr::Seq(bindings, body) => {
                write!(f, "(")?;
                for (i, (name, ty_ann, value)) in bindings.iter().enumerate() {
                    if i > 0 {
                        write!(f, "; ")?;
                    }
                    if let Some(ty) = ty_ann {
                        write!(f, "let {name} : {ty} = {value}")?;
                    } else {
                        write!(f, "let {name} = {value}")?;
                    }
                }
                write!(f, "; {body})")
            }
            Expr::Rec(name, body) => write!(f, "(rec {name} -> {body})"),
            Expr::Match(scrutinee, arms) => {
                write!(f, "(match {scrutinee} with")?;
                for (pattern, result) in arms {
                    write!(f, " | {pattern} -> {result}")?;
                }
                write!(f, ")")
            }
            Expr::Tuple(elements) => {
                write!(f, "(")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, ")")
            }
            Expr::TupleProj(tuple, index) => write!(f, "{tuple}.{index}"),
            Expr::TypeAlias(name, ty_expr, body) => {
                write!(f, "(type {name} = {ty_expr} in {body})")
            }
            Expr::Record(fields) => {
                write!(f, "{{")?;
                for (i, (name, expr)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name}: {expr}")?;
                }
                write!(f, "}}")
            }
            Expr::FieldAccess(record, field) => {
                write!(f, "{record}.{field}")
            }
            Expr::TypeDef { name, type_params, constructors, body } => {
                write!(f, "(type {}", name)?;
                for param in type_params {
                    write!(f, " {}", param)?;
                }
                write!(f, " =")?;
                for (i, (ctor, types)) in constructors.iter().enumerate() {
                    if i > 0 { write!(f, " |")?; }
                    write!(f, " {}", ctor)?;
                    for ty in types {
                        write!(f, " {}", ty)?;
                    }
                }
                write!(f, " in {})", body)
            }
            Expr::Constructor(name, args) => {
                write!(f, "{}", name)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                Ok(())
            }
            Expr::Array(elements) => {
                write!(f, "[|")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "|]")
            }
            Expr::ArrayIndex(arr, index) => write!(f, "{arr}[{index}]"),
            Expr::Ref(expr) => write!(f, "(ref {expr})"),
            Expr::Deref(expr) => write!(f, "(!{expr})"),
            Expr::RefAssign(ref_expr, value) => write!(f, "({ref_expr} := {value})"),
        }
    }
}

impl fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeExpr::Int => write!(f, "Int"),
            TypeExpr::Bool => write!(f, "Bool"),
            TypeExpr::Fun(arg, ret) => {
                // Add parentheses around function arguments if they are also functions
                match arg.as_ref() {
                    TypeExpr::Fun(_, _) => write!(f, "({arg}) -> {ret}"),
                    _ => write!(f, "{arg} -> {ret}"),
                }
            }
            TypeExpr::Alias(name) => write!(f, "{name}"),
        }
    }
}

impl fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TypeAnnotation::Concrete(name) => write!(f, "{}", name),
            TypeAnnotation::Var(name) => write!(f, "{}", name),
            TypeAnnotation::Fun(arg, ret) => write!(f, "({} -> {})", arg, ret),
            TypeAnnotation::App(name, args) => {
                write!(f, "{}", name)?;
                for arg in args {
                    write!(f, " {}", arg)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Literal::Int(n) => write!(f, "{n}"),
            Literal::Bool(b) => write!(f, "{b}"),
            Literal::Char(c) => {
                write!(f, "'")?;
                match c {
                    '\n' => write!(f, "\\n")?,
                    '\t' => write!(f, "\\t")?,
                    '\r' => write!(f, "\\r")?,
                    '\\' => write!(f, "\\\\")?,
                    '\'' => write!(f, "\\'")?,
                    _ => write!(f, "{c}")?,
                }
                write!(f, "'")
            }
            Literal::Byte(b) => write!(f, "{}b", b),
        }
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pattern::Literal(lit) => write!(f, "{lit}"),
            Pattern::Var(name) => write!(f, "{name}"),
            Pattern::Wildcard => write!(f, "_"),
            Pattern::Tuple(patterns) => {
                write!(f, "(")?;
                for (i, pat) in patterns.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{pat}")?;
                }
                write!(f, ")")
            }
            Pattern::Record(fields) => {
                write!(f, "{{")?;
                for (i, (name, pattern)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name}: {pattern}")?;
                }
                write!(f, "}}")
            }
            Pattern::Constructor(name, patterns) => {
                write!(f, "{}", name)?;
                for pattern in patterns {
                    write!(f, " {}", pattern)?;
                }
                Ok(())
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
        write!(f, "{s}")
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
            None,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(
            expr,
            Expr::Let(
                "x".to_string(),
                None,
                Box::new(Expr::Int(42)),
                Box::new(Expr::Var("x".to_string())),
            )
        );
    }

    #[test]
    fn test_expr_fun() {
        let expr = Expr::Fun("x".to_string(), None, Box::new(Expr::Var("x".to_string())));
        assert_eq!(
            expr,
            Expr::Fun("x".to_string(), None, Box::new(Expr::Var("x".to_string())))
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
            ("x".to_string(), None, Expr::Int(42)),
            ("y".to_string(), None, Expr::Int(10)),
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
        assert_eq!(format!("{expr}"), "(1 + 2)");
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
        assert_eq!(format!("{expr}"), "((1 + 2) * 3)");
    }

    #[test]
    fn test_display_if() {
        let expr = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        assert_eq!(format!("{expr}"), "(if true then 1 else 2)");
    }

    #[test]
    fn test_display_let() {
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(format!("{expr}"), "(let x = 42 in x)");
    }

    #[test]
    fn test_display_fun() {
        let expr = Expr::Fun("x".to_string(), None, Box::new(Expr::Var("x".to_string())));
        assert_eq!(format!("{expr}"), "(fun x -> x)");
    }

    #[test]
    fn test_display_app() {
        let expr = Expr::App(
            Box::new(Expr::Var("f".to_string())),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(format!("{expr}"), "(f 42)");
    }

    #[test]
    fn test_display_load() {
        let expr = Expr::Load(
            "lib.par".to_string(),
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(format!("{expr}"), "(load \"lib.par\" in x)");
    }

    #[test]
    fn test_display_seq() {
        let bindings = vec![
            ("x".to_string(), None, Expr::Int(42)),
            ("y".to_string(), None, Expr::Int(10)),
        ];
        let expr = Expr::Seq(bindings, Box::new(Expr::Var("x".to_string())));
        assert_eq!(format!("{expr}"), "(let x = 42; let y = 10; x)");
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
            None,
            Box::new(Expr::Fun(
                "x".to_string(),
                None,
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
            format!("{expr}"),
            "(let f = (fun x -> (x + 1)) in (f 41))"
        );
    }

    #[test]
    fn test_expr_rec() {
        let expr = Expr::Rec("f".to_string(), Box::new(Expr::Var("f".to_string())));
        assert_eq!(
            expr,
            Expr::Rec("f".to_string(), Box::new(Expr::Var("f".to_string())))
        );
    }

    #[test]
    fn test_display_rec() {
        let expr = Expr::Rec(
            "factorial".to_string(),
            Box::new(Expr::Fun(
                "n".to_string(),
                None,
                Box::new(Expr::Var("n".to_string())),
            )),
        );
        assert_eq!(format!("{expr}"), "(rec factorial -> (fun n -> n))");
    }

    // Test Literal construction and equality
    #[test]
    fn test_literal_int() {
        let lit = Literal::Int(42);
        assert_eq!(lit, Literal::Int(42));
        assert_ne!(lit, Literal::Int(43));
    }

    #[test]
    fn test_literal_bool() {
        let lit_true = Literal::Bool(true);
        let lit_false = Literal::Bool(false);
        assert_eq!(lit_true, Literal::Bool(true));
        assert_eq!(lit_false, Literal::Bool(false));
        assert_ne!(lit_true, lit_false);
    }

    // Test Pattern construction and equality
    #[test]
    fn test_pattern_literal() {
        let pat = Pattern::Literal(Literal::Int(42));
        assert_eq!(pat, Pattern::Literal(Literal::Int(42)));
    }

    #[test]
    fn test_pattern_var() {
        let pat = Pattern::Var("x".to_string());
        assert_eq!(pat, Pattern::Var("x".to_string()));
        assert_ne!(pat, Pattern::Var("y".to_string()));
    }

    #[test]
    fn test_pattern_wildcard() {
        let pat = Pattern::Wildcard;
        assert_eq!(pat, Pattern::Wildcard);
    }

    // Test Display for Literal
    #[test]
    fn test_display_literal_int() {
        assert_eq!(format!("{}", Literal::Int(42)), "42");
        assert_eq!(format!("{}", Literal::Int(-10)), "-10");
    }

    #[test]
    fn test_display_literal_bool() {
        assert_eq!(format!("{}", Literal::Bool(true)), "true");
        assert_eq!(format!("{}", Literal::Bool(false)), "false");
    }

    // Test Display for Pattern
    #[test]
    fn test_display_pattern_literal() {
        let pat = Pattern::Literal(Literal::Int(42));
        assert_eq!(format!("{pat}"), "42");
    }

    #[test]
    fn test_display_pattern_var() {
        let pat = Pattern::Var("x".to_string());
        assert_eq!(format!("{pat}"), "x");
    }

    #[test]
    fn test_display_pattern_wildcard() {
        let pat = Pattern::Wildcard;
        assert_eq!(format!("{pat}"), "_");
    }

    // Test Tuple pattern
    #[test]
    fn test_pattern_tuple() {
        let pat = Pattern::Tuple(vec![
            Pattern::Literal(Literal::Int(1)),
            Pattern::Var("x".to_string()),
        ]);
        assert_eq!(
            pat,
            Pattern::Tuple(vec![
                Pattern::Literal(Literal::Int(1)),
                Pattern::Var("x".to_string()),
            ])
        );
    }

    #[test]
    fn test_display_pattern_tuple() {
        let pat = Pattern::Tuple(vec![
            Pattern::Literal(Literal::Int(1)),
            Pattern::Var("x".to_string()),
            Pattern::Wildcard,
        ]);
        assert_eq!(format!("{pat}"), "(1, x, _)");
    }

    #[test]
    fn test_display_pattern_tuple_empty() {
        let pat = Pattern::Tuple(vec![]);
        assert_eq!(format!("{pat}"), "()");
    }

    #[test]
    fn test_display_pattern_tuple_nested() {
        let pat = Pattern::Tuple(vec![
            Pattern::Tuple(vec![Pattern::Var("x".to_string()), Pattern::Var("y".to_string())]),
            Pattern::Var("z".to_string()),
        ]);
        assert_eq!(format!("{pat}"), "((x, y), z)");
    }

    // Test Match expression
    #[test]
    fn test_expr_match() {
        let arms = vec![
            (Pattern::Literal(Literal::Int(0)), Expr::Int(1)),
            (Pattern::Var("n".to_string()), Expr::Var("n".to_string())),
        ];
        let expr = Expr::Match(Box::new(Expr::Var("x".to_string())), arms.clone());
        assert_eq!(
            expr,
            Expr::Match(Box::new(Expr::Var("x".to_string())), arms)
        );
    }

    #[test]
    fn test_display_match() {
        let arms = vec![
            (Pattern::Literal(Literal::Int(0)), Expr::Int(1)),
            (Pattern::Var("n".to_string()), Expr::Var("n".to_string())),
            (Pattern::Wildcard, Expr::Int(42)),
        ];
        let expr = Expr::Match(Box::new(Expr::Var("x".to_string())), arms);
        assert_eq!(
            format!("{expr}"),
            "(match x with | 0 -> 1 | n -> n | _ -> 42)"
        );
    }

    // Test Tuple expression
    #[test]
    fn test_expr_tuple() {
        let expr = Expr::Tuple(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)]);
        assert_eq!(
            expr,
            Expr::Tuple(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)])
        );
    }

    #[test]
    fn test_expr_tuple_empty() {
        let expr = Expr::Tuple(vec![]);
        assert_eq!(expr, Expr::Tuple(vec![]));
    }

    #[test]
    fn test_expr_tuple_nested() {
        let expr = Expr::Tuple(vec![
            Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]),
            Expr::Int(3),
        ]);
        assert_eq!(
            expr,
            Expr::Tuple(vec![
                Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]),
                Expr::Int(3),
            ])
        );
    }

    #[test]
    fn test_display_tuple() {
        let expr = Expr::Tuple(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)]);
        assert_eq!(format!("{expr}"), "(1, 2, 3)");
    }

    #[test]
    fn test_display_tuple_empty() {
        let expr = Expr::Tuple(vec![]);
        assert_eq!(format!("{expr}"), "()");
    }

    #[test]
    fn test_display_tuple_single() {
        let expr = Expr::Tuple(vec![Expr::Int(42)]);
        assert_eq!(format!("{expr}"), "(42)");
    }

    #[test]
    fn test_display_tuple_nested() {
        let expr = Expr::Tuple(vec![
            Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]),
            Expr::Tuple(vec![Expr::Int(3), Expr::Int(4)]),
        ]);
        assert_eq!(format!("{expr}"), "((1, 2), (3, 4))");
    }

    #[test]
    fn test_display_tuple_mixed() {
        let expr = Expr::Tuple(vec![
            Expr::Int(42),
            Expr::Bool(true),
            Expr::Var("x".to_string()),
        ]);
        assert_eq!(format!("{expr}"), "(42, true, x)");
    }

    // Test TupleProj expression
    #[test]
    fn test_expr_tuple_proj() {
        let expr = Expr::TupleProj(Box::new(Expr::Var("t".to_string())), 0);
        assert_eq!(
            expr,
            Expr::TupleProj(Box::new(Expr::Var("t".to_string())), 0)
        );
    }

    #[test]
    fn test_expr_tuple_proj_nested() {
        // t.0.1
        let expr = Expr::TupleProj(
            Box::new(Expr::TupleProj(Box::new(Expr::Var("t".to_string())), 0)),
            1,
        );
        assert_eq!(
            expr,
            Expr::TupleProj(
                Box::new(Expr::TupleProj(Box::new(Expr::Var("t".to_string())), 0)),
                1,
            )
        );
    }

    #[test]
    fn test_display_tuple_proj() {
        let expr = Expr::TupleProj(Box::new(Expr::Var("t".to_string())), 0);
        assert_eq!(format!("{expr}"), "t.0");
    }

    #[test]
    fn test_display_tuple_proj_index() {
        let expr = Expr::TupleProj(Box::new(Expr::Var("pair".to_string())), 1);
        assert_eq!(format!("{expr}"), "pair.1");
    }

    #[test]
    fn test_display_tuple_proj_nested() {
        // ((1, 2), 3).0.1
        let expr = Expr::TupleProj(
            Box::new(Expr::TupleProj(
                Box::new(Expr::Tuple(vec![
                    Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]),
                    Expr::Int(3),
                ])),
                0,
            )),
            1,
        );
        assert_eq!(format!("{expr}"), "((1, 2), 3).0.1");
    }

    // Test TypeExpr construction and equality
    #[test]
    fn test_type_expr_int() {
        let ty = TypeExpr::Int;
        assert_eq!(ty, TypeExpr::Int);
    }

    #[test]
    fn test_type_expr_bool() {
        let ty = TypeExpr::Bool;
        assert_eq!(ty, TypeExpr::Bool);
    }

    #[test]
    fn test_type_expr_fun() {
        let ty = TypeExpr::Fun(Box::new(TypeExpr::Int), Box::new(TypeExpr::Bool));
        assert_eq!(
            ty,
            TypeExpr::Fun(Box::new(TypeExpr::Int), Box::new(TypeExpr::Bool))
        );
    }

    #[test]
    fn test_type_expr_alias() {
        let ty = TypeExpr::Alias("MyType".to_string());
        assert_eq!(ty, TypeExpr::Alias("MyType".to_string()));
    }

    // Test TypeExpr Display
    #[test]
    fn test_display_type_expr_int() {
        assert_eq!(format!("{}", TypeExpr::Int), "Int");
    }

    #[test]
    fn test_display_type_expr_bool() {
        assert_eq!(format!("{}", TypeExpr::Bool), "Bool");
    }

    #[test]
    fn test_display_type_expr_simple_fun() {
        let ty = TypeExpr::Fun(Box::new(TypeExpr::Int), Box::new(TypeExpr::Bool));
        assert_eq!(format!("{ty}"), "Int -> Bool");
    }

    #[test]
    fn test_display_type_expr_fun_with_fun_arg() {
        // (Int -> Bool) -> Bool
        let ty = TypeExpr::Fun(
            Box::new(TypeExpr::Fun(
                Box::new(TypeExpr::Int),
                Box::new(TypeExpr::Bool),
            )),
            Box::new(TypeExpr::Bool),
        );
        assert_eq!(format!("{ty}"), "(Int -> Bool) -> Bool");
    }

    #[test]
    fn test_display_type_expr_alias() {
        let ty = TypeExpr::Alias("MyFunc".to_string());
        assert_eq!(format!("{ty}"), "MyFunc");
    }

    // Test TypeAlias expression
    #[test]
    fn test_expr_type_alias() {
        let expr = Expr::TypeAlias(
            "MyInt".to_string(),
            TypeExpr::Int,
            Box::new(Expr::Var("x".to_string())),
        );
        assert_eq!(
            expr,
            Expr::TypeAlias(
                "MyInt".to_string(),
                TypeExpr::Int,
                Box::new(Expr::Var("x".to_string())),
            )
        );
    }

    #[test]
    fn test_display_type_alias() {
        let expr = Expr::TypeAlias(
            "MyFunc".to_string(),
            TypeExpr::Fun(Box::new(TypeExpr::Int), Box::new(TypeExpr::Int)),
            Box::new(Expr::Int(42)),
        );
        assert_eq!(format!("{expr}"), "(type MyFunc = Int -> Int in 42)");
    }

    #[test]
    fn test_type_alias_clone() {
        let expr = Expr::TypeAlias(
            "MyInt".to_string(),
            TypeExpr::Int,
            Box::new(Expr::Int(42)),
        );
        let cloned = expr.clone();
        assert_eq!(expr, cloned);
    }

    // Test Record expression
    #[test]
    fn test_expr_record_empty() {
        let expr = Expr::Record(vec![]);
        assert_eq!(expr, Expr::Record(vec![]));
    }

    #[test]
    fn test_expr_record_single_field() {
        let expr = Expr::Record(vec![("name".to_string(), Expr::Int(42))]);
        assert_eq!(
            expr,
            Expr::Record(vec![("name".to_string(), Expr::Int(42))])
        );
    }

    #[test]
    fn test_expr_record_multiple_fields() {
        let fields = vec![
            ("name".to_string(), Expr::Int(42)),
            ("age".to_string(), Expr::Int(30)),
        ];
        let expr = Expr::Record(fields.clone());
        assert_eq!(expr, Expr::Record(fields));
    }

    #[test]
    fn test_expr_record_nested() {
        let inner_record = Expr::Record(vec![("x".to_string(), Expr::Int(10))]);
        let outer_record = Expr::Record(vec![
            ("inner".to_string(), inner_record.clone()),
            ("y".to_string(), Expr::Int(20)),
        ]);
        assert_eq!(
            outer_record,
            Expr::Record(vec![
                ("inner".to_string(), inner_record),
                ("y".to_string(), Expr::Int(20)),
            ])
        );
    }

    #[test]
    fn test_display_record_empty() {
        let expr = Expr::Record(vec![]);
        assert_eq!(format!("{expr}"), "{}");
    }

    #[test]
    fn test_display_record_single_field() {
        let expr = Expr::Record(vec![("name".to_string(), Expr::Int(42))]);
        assert_eq!(format!("{expr}"), "{name: 42}");
    }

    #[test]
    fn test_display_record_multiple_fields() {
        let expr = Expr::Record(vec![
            ("name".to_string(), Expr::Int(42)),
            ("age".to_string(), Expr::Int(30)),
        ]);
        assert_eq!(format!("{expr}"), "{name: 42, age: 30}");
    }

    #[test]
    fn test_display_record_mixed_types() {
        let expr = Expr::Record(vec![
            ("name".to_string(), Expr::Int(42)),
            ("active".to_string(), Expr::Bool(true)),
            ("count".to_string(), Expr::Var("x".to_string())),
        ]);
        assert_eq!(format!("{expr}"), "{name: 42, active: true, count: x}");
    }

    // Test FieldAccess expression
    #[test]
    fn test_expr_field_access() {
        let expr = Expr::FieldAccess(
            Box::new(Expr::Var("person".to_string())),
            "name".to_string(),
        );
        assert_eq!(
            expr,
            Expr::FieldAccess(
                Box::new(Expr::Var("person".to_string())),
                "name".to_string(),
            )
        );
    }

    #[test]
    fn test_expr_field_access_nested() {
        let expr = Expr::FieldAccess(
            Box::new(Expr::FieldAccess(
                Box::new(Expr::Var("person".to_string())),
                "address".to_string(),
            )),
            "city".to_string(),
        );
        assert_eq!(
            expr,
            Expr::FieldAccess(
                Box::new(Expr::FieldAccess(
                    Box::new(Expr::Var("person".to_string())),
                    "address".to_string(),
                )),
                "city".to_string(),
            )
        );
    }

    #[test]
    fn test_display_field_access() {
        let expr = Expr::FieldAccess(
            Box::new(Expr::Var("person".to_string())),
            "name".to_string(),
        );
        assert_eq!(format!("{expr}"), "person.name");
    }

    #[test]
    fn test_display_field_access_nested() {
        let expr = Expr::FieldAccess(
            Box::new(Expr::FieldAccess(
                Box::new(Expr::Var("person".to_string())),
                "address".to_string(),
            )),
            "city".to_string(),
        );
        assert_eq!(format!("{expr}"), "person.address.city");
    }

    #[test]
    fn test_display_field_access_on_record() {
        let record = Expr::Record(vec![("name".to_string(), Expr::Int(42))]);
        let expr = Expr::FieldAccess(Box::new(record), "name".to_string());
        assert_eq!(format!("{expr}"), "{name: 42}.name");
    }

    // Test Pattern::Record
    #[test]
    fn test_pattern_record_empty() {
        let pat = Pattern::Record(vec![]);
        assert_eq!(pat, Pattern::Record(vec![]));
    }

    #[test]
    fn test_pattern_record_single_field() {
        let pat = Pattern::Record(vec![("name".to_string(), Pattern::Var("n".to_string()))]);
        assert_eq!(
            pat,
            Pattern::Record(vec![("name".to_string(), Pattern::Var("n".to_string()))])
        );
    }

    #[test]
    fn test_pattern_record_multiple_fields() {
        let pat = Pattern::Record(vec![
            ("name".to_string(), Pattern::Var("n".to_string())),
            ("age".to_string(), Pattern::Var("a".to_string())),
        ]);
        assert_eq!(
            pat,
            Pattern::Record(vec![
                ("name".to_string(), Pattern::Var("n".to_string())),
                ("age".to_string(), Pattern::Var("a".to_string())),
            ])
        );
    }

    #[test]
    fn test_pattern_record_with_wildcard() {
        let pat = Pattern::Record(vec![
            ("name".to_string(), Pattern::Var("n".to_string())),
            ("age".to_string(), Pattern::Wildcard),
        ]);
        assert_eq!(
            pat,
            Pattern::Record(vec![
                ("name".to_string(), Pattern::Var("n".to_string())),
                ("age".to_string(), Pattern::Wildcard),
            ])
        );
    }

    #[test]
    fn test_pattern_record_nested() {
        let inner_pat = Pattern::Record(vec![("x".to_string(), Pattern::Var("n".to_string()))]);
        let outer_pat = Pattern::Record(vec![("inner".to_string(), inner_pat.clone())]);
        assert_eq!(
            outer_pat,
            Pattern::Record(vec![("inner".to_string(), inner_pat)])
        );
    }

    #[test]
    fn test_display_pattern_record_empty() {
        let pat = Pattern::Record(vec![]);
        assert_eq!(format!("{pat}"), "{}");
    }

    #[test]
    fn test_display_pattern_record_single_field() {
        let pat = Pattern::Record(vec![("name".to_string(), Pattern::Var("n".to_string()))]);
        assert_eq!(format!("{pat}"), "{name: n}");
    }

    #[test]
    fn test_display_pattern_record_multiple_fields() {
        let pat = Pattern::Record(vec![
            ("name".to_string(), Pattern::Var("n".to_string())),
            ("age".to_string(), Pattern::Var("a".to_string())),
        ]);
        assert_eq!(format!("{pat}"), "{name: n, age: a}");
    }

    #[test]
    fn test_display_pattern_record_with_wildcard() {
        let pat = Pattern::Record(vec![
            ("name".to_string(), Pattern::Var("n".to_string())),
            ("age".to_string(), Pattern::Wildcard),
        ]);
        assert_eq!(format!("{pat}"), "{name: n, age: _}");
    }

    #[test]
    fn test_display_pattern_record_with_literal() {
        let pat = Pattern::Record(vec![
            ("status".to_string(), Pattern::Literal(Literal::Int(1))),
            ("name".to_string(), Pattern::Var("n".to_string())),
        ]);
        assert_eq!(format!("{pat}"), "{status: 1, name: n}");
    }

    #[test]
    fn test_record_clone() {
        let expr = Expr::Record(vec![("name".to_string(), Expr::Int(42))]);
        let cloned = expr.clone();
        assert_eq!(expr, cloned);
    }

    #[test]
    fn test_field_access_clone() {
        let expr = Expr::FieldAccess(
            Box::new(Expr::Var("person".to_string())),
            "name".to_string(),
        );
        let cloned = expr.clone();
        assert_eq!(expr, cloned);
    }

    #[test]
    fn test_pattern_record_clone() {
        let pat = Pattern::Record(vec![("name".to_string(), Pattern::Var("n".to_string()))]);
        let cloned = pat.clone();
        assert_eq!(pat, cloned);
    }
}
