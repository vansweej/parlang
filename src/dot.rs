/// DOT format generator for `ParLang` AST visualization
/// 
/// This module provides functionality to convert `ParLang` Abstract Syntax Trees
/// into Graphviz DOT format for visualization. The generated DOT files can be
/// rendered using tools like `dot` or `graphviz`.
/// 
/// # Example
/// 
/// ```
/// use parlang::{parse, dot::ast_to_dot};
/// use std::fs;
/// 
/// let source = "let x = 42 in x + 1";
/// let expr = parse(source).unwrap();
/// let dot_output = ast_to_dot(&expr);
/// fs::write("ast.dot", dot_output).unwrap();
/// ```
use crate::ast::{Expr, BinOp, Pattern, Literal};
use std::io;

/// Counter for generating unique node IDs in the DOT graph
/// 
/// This helper struct ensures each AST node gets a unique identifier
/// when converting to DOT format. Node IDs are sequential strings
/// in the format "node0", "node1", "node2", etc.
struct NodeIdGenerator {
    counter: usize,
}

impl NodeIdGenerator {
    /// Create a new generator starting from node0
    fn new() -> Self {
        NodeIdGenerator { counter: 0 }
    }

    /// Generate the next unique node ID
    /// 
    /// Returns a string like "node0", "node1", etc. and increments
    /// the internal counter for the next call.
    fn next(&mut self) -> String {
        let id = format!("node{}", self.counter);
        self.counter += 1;
        id
    }
}

/// Convert an expression to DOT format
/// 
/// # Arguments
/// 
/// * `expr` - The expression to convert to DOT format
/// 
/// # Returns
/// 
/// A String containing the DOT representation of the AST
pub fn ast_to_dot(expr: &Expr) -> String {
    let mut output = String::new();
    output.push_str("digraph AST {\n");
    output.push_str("  node [shape=box, style=rounded];\n");
    output.push_str("  edge [fontsize=10];\n\n");
    
    let mut gen = NodeIdGenerator::new();
    expr_to_dot(expr, &mut output, &mut gen);
    
    output.push_str("}\n");
    output
}

/// Write DOT representation of an expression to a file
/// 
/// # Arguments
/// 
/// * `expr` - The expression to convert
/// * `path` - The file path to write to
/// 
/// # Returns
/// 
/// # Errors
/// 
/// Result indicating success or IO error when writing to file fails
pub fn write_ast_to_dot_file(expr: &Expr, path: &str) -> io::Result<()> {
    let dot_content = ast_to_dot(expr);
    std::fs::write(path, dot_content)
}

fn expr_to_dot(expr: &Expr, output: &mut String, gen: &mut NodeIdGenerator) -> String {
    let node_id = gen.next();
    
    match expr {
        Expr::Int(n) => {
            output.push_str(&format!("  {node_id} [label=\"Int\\n{n}\"];\n"));
        }
        Expr::Bool(b) => {
            output.push_str(&format!("  {node_id} [label=\"Bool\\n{b}\"];\n"));
        }
        Expr::Char(c) => {
            let label = match c {
                '\n' => "\\\\n".to_string(),
                '\t' => "\\\\t".to_string(),
                '\r' => "\\\\r".to_string(),
                '\\' => "\\\\\\\\".to_string(),
                '\'' => "\\\\'".to_string(),
                _ => c.to_string(),
            };
            output.push_str(&format!("  {node_id} [label=\"Char\\n'{label}'\"];\n"));
        }
        Expr::Float(fl) => {
            output.push_str(&format!("  {node_id} [label=\"Float\\n{fl}\"];\n"));
        }
        Expr::Byte(b) => {
            output.push_str(&format!("  {node_id} [label=\"Byte\\n{b}b\"];\n"));
        }
        Expr::Var(name) => {
            output.push_str(&format!("  {} [label=\"Var\\n{}\"];\n", node_id, escape_label(name)));
        }
        Expr::BinOp(op, left, right) => {
            output.push_str(&format!("  {} [label=\"BinOp\\n{}\"];\n", node_id, binop_label(*op)));
            let left_id = expr_to_dot(left, output, gen);
            let right_id = expr_to_dot(right, output, gen);
            output.push_str(&format!("  {node_id} -> {left_id} [label=\"left\"];\n"));
            output.push_str(&format!("  {node_id} -> {right_id} [label=\"right\"];\n"));
        }
        Expr::If(cond, then_branch, else_branch) => {
            output.push_str(&format!("  {node_id} [label=\"If\"];\n"));
            let cond_id = expr_to_dot(cond, output, gen);
            let then_id = expr_to_dot(then_branch, output, gen);
            let else_id = expr_to_dot(else_branch, output, gen);
            output.push_str(&format!("  {node_id} -> {cond_id} [label=\"cond\"];\n"));
            output.push_str(&format!("  {node_id} -> {then_id} [label=\"then\"];\n"));
            output.push_str(&format!("  {node_id} -> {else_id} [label=\"else\"];\n"));
        }
        Expr::Let(name, ty_ann, value, body) => {
            let label = if let Some(ty) = ty_ann {
                format!("Let\\n{} : {}", escape_label(name), ty)
            } else {
                format!("Let\\n{}", escape_label(name))
            };
            output.push_str(&format!("  {} [label=\"{}\"];\n", node_id, label));
            let value_id = expr_to_dot(value, output, gen);
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {node_id} -> {value_id} [label=\"value\"];\n"));
            output.push_str(&format!("  {node_id} -> {body_id} [label=\"body\"];\n"));
        }
        Expr::Fun(param, ty_ann, body) => {
            let label = if let Some(ty) = ty_ann {
                format!("Fun\\n{} : {}", escape_label(param), ty)
            } else {
                format!("Fun\\n{}", escape_label(param))
            };
            output.push_str(&format!("  {} [label=\"{}\"];\n", node_id, label));
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {node_id} -> {body_id} [label=\"body\"];\n"));
        }
        Expr::App(func, arg) => {
            output.push_str(&format!("  {node_id} [label=\"App\"];\n"));
            let func_id = expr_to_dot(func, output, gen);
            let arg_id = expr_to_dot(arg, output, gen);
            output.push_str(&format!("  {node_id} -> {func_id} [label=\"func\"];\n"));
            output.push_str(&format!("  {node_id} -> {arg_id} [label=\"arg\"];\n"));
        }
        Expr::Load(filepath, body) => {
            output.push_str(&format!("  {} [label=\"Load\\n{}\"];\n", node_id, escape_label(filepath)));
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {node_id} -> {body_id} [label=\"body\"];\n"));
        }
        Expr::Seq(bindings, body) => {
            output.push_str(&format!("  {node_id} [label=\"Seq\"];\n"));
            for (i, (name, ty_ann, value)) in bindings.iter().enumerate() {
                let binding_id = gen.next();
                let label = if let Some(ty) = ty_ann {
                    format!("Binding\\n{} : {}", escape_label(name), ty)
                } else {
                    format!("Binding\\n{}", escape_label(name))
                };
                output.push_str(&format!("  {} [label=\"{}\"];\n", binding_id, label));
                let value_id = expr_to_dot(value, output, gen);
                output.push_str(&format!("  {node_id} -> {binding_id} [label=\"binding {i}\"];\n"));
                output.push_str(&format!("  {binding_id} -> {value_id} [label=\"value\"];\n"));
            }
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {node_id} -> {body_id} [label=\"body\"];\n"));
        }
        Expr::Rec(name, body) => {
            output.push_str(&format!("  {} [label=\"Rec\\n{}\"];\n", node_id, escape_label(name)));
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {node_id} -> {body_id} [label=\"body\"];\n"));
        }
        Expr::Match(scrutinee, arms) => {
            output.push_str(&format!("  {node_id} [label=\"Match\"];\n"));
            let scrutinee_id = expr_to_dot(scrutinee, output, gen);
            output.push_str(&format!("  {node_id} -> {scrutinee_id} [label=\"scrutinee\"];\n"));
            
            for (i, (pattern, result)) in arms.iter().enumerate() {
                let arm_id = gen.next();
                output.push_str(&format!("  {arm_id} [label=\"Arm {i}\"];\n"));
                let pattern_id = pattern_to_dot(pattern, output, gen);
                let result_id = expr_to_dot(result, output, gen);
                output.push_str(&format!("  {node_id} -> {arm_id} [label=\"arm {i}\"];\n"));
                output.push_str(&format!("  {arm_id} -> {pattern_id} [label=\"pattern\"];\n"));
                output.push_str(&format!("  {arm_id} -> {result_id} [label=\"result\"];\n"));
            }
        }
        Expr::Tuple(elements) => {
            output.push_str(&format!("  {node_id} [label=\"Tuple\"];\n"));
            for (i, elem) in elements.iter().enumerate() {
                let elem_id = expr_to_dot(elem, output, gen);
                output.push_str(&format!("  {node_id} -> {elem_id} [label=\"elem {i}\"];\n"));
            }
        }
        Expr::TupleProj(tuple, index) => {
            output.push_str(&format!("  {node_id} [label=\"TupleProj\\n{index}\"];\n"));
            let tuple_id = expr_to_dot(tuple, output, gen);
            output.push_str(&format!("  {node_id} -> {tuple_id} [label=\"tuple\"];\n"));
        }
        Expr::TypeAlias(name, ty_expr, body) => {
            output.push_str(&format!("  {} [label=\"TypeAlias\\n{}\"];\n", node_id, escape_label(name)));
            let type_id = type_expr_to_dot(ty_expr, output, gen);
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {node_id} -> {type_id} [label=\"type\"];\n"));
            output.push_str(&format!("  {node_id} -> {body_id} [label=\"body\"];\n"));
        }
        Expr::Record(fields) => {
            output.push_str(&format!("  {node_id} [label=\"Record\"];\n"));
            for (i, (name, expr)) in fields.iter().enumerate() {
                let field_id = gen.next();
                output.push_str(&format!("  {} [label=\"Field\\n{}\"];\n", field_id, escape_label(name)));
                let expr_id = expr_to_dot(expr, output, gen);
                output.push_str(&format!("  {node_id} -> {field_id} [label=\"field {i}\"];\n"));
                output.push_str(&format!("  {field_id} -> {expr_id} [label=\"value\"];\n"));
            }
        }
        Expr::FieldAccess(record, field) => {
            output.push_str(&format!("  {} [label=\"FieldAccess\\n{}\"];\n", node_id, escape_label(field)));
            let record_id = expr_to_dot(record, output, gen);
            output.push_str(&format!("  {node_id} -> {record_id} [label=\"record\"];\n"));
        }
        Expr::TypeDef { name, type_params, constructors, body } => {
            let params_str = type_params.join(" ");
            output.push_str(&format!("  {} [label=\"TypeDef\\n{}\\n{}\"];\n", node_id, escape_label(name), escape_label(&params_str)));
            
            // Add constructor nodes
            for (ctor_name, _ctor_types) in constructors {
                let ctor_id = gen.next();
                output.push_str(&format!("  {} [label=\"Constructor\\n{}\"];\n", ctor_id, escape_label(ctor_name)));
                output.push_str(&format!("  {node_id} -> {ctor_id} [label=\"ctor\"];\n"));
            }
            
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {node_id} -> {body_id} [label=\"body\"];\n"));
        }
        Expr::Constructor(name, args) => {
            output.push_str(&format!("  {} [label=\"Constructor\\n{}\"];\n", node_id, escape_label(name)));
            for (i, arg) in args.iter().enumerate() {
                let arg_id = expr_to_dot(arg, output, gen);
                output.push_str(&format!("  {node_id} -> {arg_id} [label=\"arg{}\"];\n", i));
            }
        }
        Expr::Array(elements) => {
            output.push_str(&format!("  {node_id} [label=\"Array\"];\n"));
            for (i, elem) in elements.iter().enumerate() {
                let elem_id = expr_to_dot(elem, output, gen);
                output.push_str(&format!("  {node_id} -> {elem_id} [label=\"elem{}\"];\n", i));
            }
        }
        Expr::ArrayIndex(arr, index) => {
            output.push_str(&format!("  {node_id} [label=\"ArrayIndex\"];\n"));
            let arr_id = expr_to_dot(arr, output, gen);
            let index_id = expr_to_dot(index, output, gen);
            output.push_str(&format!("  {node_id} -> {arr_id} [label=\"array\"];\n"));
            output.push_str(&format!("  {node_id} -> {index_id} [label=\"index\"];\n"));
        }
        Expr::Ref(expr) => {
            output.push_str(&format!("  {node_id} [label=\"Ref\"];\n"));
            let expr_id = expr_to_dot(expr, output, gen);
            output.push_str(&format!("  {node_id} -> {expr_id} [label=\"value\"];\n"));
        }
        Expr::Deref(expr) => {
            output.push_str(&format!("  {node_id} [label=\"Deref\"];\n"));
            let expr_id = expr_to_dot(expr, output, gen);
            output.push_str(&format!("  {node_id} -> {expr_id} [label=\"ref\"];\n"));
        }
        Expr::RefAssign(ref_expr, value) => {
            output.push_str(&format!("  {node_id} [label=\"RefAssign\"];\n"));
            let ref_id = expr_to_dot(ref_expr, output, gen);
            let value_id = expr_to_dot(value, output, gen);
            output.push_str(&format!("  {node_id} -> {ref_id} [label=\"ref\"];\n"));
            output.push_str(&format!("  {node_id} -> {value_id} [label=\"value\"];\n"));
        }
    }
    
    node_id
}

fn type_expr_to_dot(ty_expr: &crate::ast::TypeExpr, output: &mut String, gen: &mut NodeIdGenerator) -> String {
    let node_id = gen.next();
    
    match ty_expr {
        crate::ast::TypeExpr::Int => {
            output.push_str(&format!("  {node_id} [label=\"Type\\nInt\"];\n"));
        }
        crate::ast::TypeExpr::Bool => {
            output.push_str(&format!("  {node_id} [label=\"Type\\nBool\"];\n"));
        }
        crate::ast::TypeExpr::Fun(arg, ret) => {
            output.push_str(&format!("  {node_id} [label=\"Type\\nFun\"];\n"));
            let arg_id = type_expr_to_dot(arg, output, gen);
            let ret_id = type_expr_to_dot(ret, output, gen);
            output.push_str(&format!("  {node_id} -> {arg_id} [label=\"arg\"];\n"));
            output.push_str(&format!("  {node_id} -> {ret_id} [label=\"ret\"];\n"));
        }
        crate::ast::TypeExpr::Alias(name) => {
            output.push_str(&format!("  {} [label=\"TypeAlias\\n{}\"];\n", node_id, escape_label(name)));
        }
    }
    
    node_id
}

fn pattern_to_dot(pattern: &Pattern, output: &mut String, gen: &mut NodeIdGenerator) -> String {
    let node_id = gen.next();
    
    match pattern {
        Pattern::Literal(lit) => {
            let label = match lit {
                Literal::Int(n) => format!("Literal\\nInt {n}"),
                Literal::Bool(b) => format!("Literal\\nBool {b}"),
                Literal::Char(c) => {
                    let char_label = match c {
                        '\n' => "\\\\n".to_string(),
                        '\t' => "\\\\t".to_string(),
                        '\r' => "\\\\r".to_string(),
                        '\\' => "\\\\\\\\".to_string(),
                        '\'' => "\\\\'".to_string(),
                        _ => c.to_string(),
                    };
                    format!("Literal\\nChar '{char_label}'")
                }
                Literal::Byte(b) => format!("Literal\\nByte {b}b"),
            };
            output.push_str(&format!("  {node_id} [label=\"{label}\"];\n"));
        }
        Pattern::Var(name) => {
            output.push_str(&format!("  {} [label=\"Var\\n{}\"];\n", node_id, escape_label(name)));
        }
        Pattern::Wildcard => {
            output.push_str(&format!("  {node_id} [label=\"Wildcard\\n_\"];\n"));
        }
        Pattern::Tuple(patterns) => {
            output.push_str(&format!("  {node_id} [label=\"TuplePattern\"];\n"));
            for (i, pat) in patterns.iter().enumerate() {
                let pat_id = pattern_to_dot(pat, output, gen);
                output.push_str(&format!("  {node_id} -> {pat_id} [label=\"elem {i}\"];\n"));
            }
        }
        Pattern::Record(fields) => {
            output.push_str(&format!("  {node_id} [label=\"RecordPattern\"];\n"));
            for (i, (name, pat)) in fields.iter().enumerate() {
                let field_id = gen.next();
                output.push_str(&format!("  {} [label=\"Field\\n{}\"];\n", field_id, escape_label(name)));
                let pat_id = pattern_to_dot(pat, output, gen);
                output.push_str(&format!("  {node_id} -> {field_id} [label=\"field {i}\"];\n"));
                output.push_str(&format!("  {field_id} -> {pat_id} [label=\"pattern\"];\n"));
            }
        }
        Pattern::Constructor(name, patterns) => {
            output.push_str(&format!("  {} [label=\"ConstructorPattern\\n{}\"];\n", node_id, escape_label(name)));
            for (i, pat) in patterns.iter().enumerate() {
                let pat_id = pattern_to_dot(pat, output, gen);
                output.push_str(&format!("  {node_id} -> {pat_id} [label=\"arg {i}\"];\n"));
            }
        }
    }
    
    node_id
}

fn binop_label(op: BinOp) -> &'static str {
    match op {
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
    }
}

fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn test_int_literal() {
        let expr = Expr::Int(42);
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("digraph AST"));
        assert!(dot.contains("[label=\"Int\\n42\"]"));
    }

    #[test]
    fn test_bool_literal() {
        let expr = Expr::Bool(true);
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Bool\\ntrue\"]"));
    }

    #[test]
    fn test_var() {
        let expr = Expr::Var("x".to_string());
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Var\\nx\"]"));
    }

    #[test]
    fn test_binop() {
        let expr = Expr::BinOp(
            BinOp::Add,
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"BinOp\\n+\"]"));
        assert!(dot.contains("[label=\"Int\\n1\"]"));
        assert!(dot.contains("[label=\"Int\\n2\"]"));
        assert!(dot.contains("-> node1 [label=\"left\"]"));
        assert!(dot.contains("-> node2 [label=\"right\"]"));
    }

    #[test]
    fn test_if_expr() {
        let expr = Expr::If(
            Box::new(Expr::Bool(true)),
            Box::new(Expr::Int(1)),
            Box::new(Expr::Int(2)),
        );
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"If\"]"));
        assert!(dot.contains("[label=\"Bool\\ntrue\"]"));
        assert!(dot.contains("[label=\"then\"]"));
        assert!(dot.contains("[label=\"else\"]"));
    }

    #[test]
    fn test_let_expr() {
        let expr = Expr::Let(
            "x".to_string(),
            None,
            Box::new(Expr::Int(42)),
            Box::new(Expr::Var("x".to_string())),
        );
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Let\\nx\"]"));
        assert!(dot.contains("[label=\"Int\\n42\"]"));
        assert!(dot.contains("[label=\"value\"]"));
        assert!(dot.contains("[label=\"body\"]"));
    }

    #[test]
    fn test_fun_expr() {
        let expr = Expr::Fun("x".to_string(), None, Box::new(Expr::Var("x".to_string())));
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Fun\\nx\"]"));
        assert!(dot.contains("[label=\"Var\\nx\"]"));
    }

    #[test]
    fn test_app_expr() {
        let expr = Expr::App(
            Box::new(Expr::Var("f".to_string())),
            Box::new(Expr::Int(42)),
        );
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"App\"]"));
        assert!(dot.contains("[label=\"Var\\nf\"]"));
        assert!(dot.contains("[label=\"Int\\n42\"]"));
        assert!(dot.contains("[label=\"func\"]"));
        assert!(dot.contains("[label=\"arg\"]"));
    }

    #[test]
    fn test_tuple() {
        let expr = Expr::Tuple(vec![Expr::Int(1), Expr::Int(2)]);
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Tuple\"]"));
        assert!(dot.contains("[label=\"Int\\n1\"]"));
        assert!(dot.contains("[label=\"Int\\n2\"]"));
    }

    #[test]
    fn test_tuple_proj() {
        let expr = Expr::TupleProj(Box::new(Expr::Var("t".to_string())), 0);
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"TupleProj\\n0\"]"));
        assert!(dot.contains("[label=\"Var\\nt\"]"));
    }

    #[test]
    fn test_match_expr() {
        let arms = vec![
            (Pattern::Literal(Literal::Int(0)), Expr::Int(1)),
            (Pattern::Var("n".to_string()), Expr::Var("n".to_string())),
        ];
        let expr = Expr::Match(Box::new(Expr::Var("x".to_string())), arms);
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Match\"]"));
        assert!(dot.contains("[label=\"scrutinee\"]"));
        assert!(dot.contains("[label=\"Arm 0\"]"));
        assert!(dot.contains("[label=\"Arm 1\"]"));
    }

    #[test]
    fn test_rec_expr() {
        let expr = Expr::Rec("f".to_string(), Box::new(Expr::Var("f".to_string())));
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Rec\\nf\"]"));
    }

    #[test]
    fn test_seq_expr() {
        let bindings = vec![
            ("x".to_string(), None, Expr::Int(42)),
            ("y".to_string(), None, Expr::Int(10)),
        ];
        let expr = Expr::Seq(bindings, Box::new(Expr::Var("x".to_string())));
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Seq\"]"));
        assert!(dot.contains("[label=\"Binding\\nx\"]"));
        assert!(dot.contains("[label=\"Binding\\ny\"]"));
    }

    #[test]
    fn test_load_expr() {
        let expr = Expr::Load(
            "lib.par".to_string(),
            Box::new(Expr::Var("x".to_string())),
        );
        let dot = ast_to_dot(&expr);
        assert!(dot.contains("[label=\"Load\\nlib.par\"]"));
    }

    #[test]
    fn test_escape_label() {
        assert_eq!(escape_label("hello"), "hello");
        assert_eq!(escape_label("hello\"world"), "hello\\\"world");
        assert_eq!(escape_label("hello\\world"), "hello\\\\world");
        assert_eq!(escape_label("hello\nworld"), "hello\\nworld");
    }

    #[test]
    fn test_complex_expr() {
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
        let dot = ast_to_dot(&expr);
        
        // Verify structure
        assert!(dot.contains("digraph AST"));
        assert!(dot.contains("[label=\"Let\\nf\"]"));
        assert!(dot.contains("[label=\"Fun\\nx\"]"));
        assert!(dot.contains("[label=\"BinOp\\n+\"]"));
        assert!(dot.contains("[label=\"App\"]"));
        assert!(dot.contains("[label=\"Int\\n41\"]"));
    }

    #[test]
    fn test_all_binops() {
        let ops = vec![
            BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div,
            BinOp::Eq, BinOp::Neq, BinOp::Lt, BinOp::Le,
            BinOp::Gt, BinOp::Ge,
        ];
        let expected = vec!["+", "-", "*", "/", "==", "!=", "<", "<=", ">", ">="];
        
        for (op, expected_label) in ops.iter().zip(expected.iter()) {
            let label = binop_label(*op);
            assert_eq!(label, *expected_label);
        }
    }

    #[test]
    fn test_pattern_literal() {
        let pattern = Pattern::Literal(Literal::Int(42));
        let mut output = String::new();
        let mut gen = NodeIdGenerator::new();
        let node_id = pattern_to_dot(&pattern, &mut output, &mut gen);
        assert_eq!(node_id, "node0");
        assert!(output.contains("[label=\"Literal\\nInt 42\"]"));
    }

    #[test]
    fn test_pattern_var() {
        let pattern = Pattern::Var("x".to_string());
        let mut output = String::new();
        let mut gen = NodeIdGenerator::new();
        pattern_to_dot(&pattern, &mut output, &mut gen);
        assert!(output.contains("[label=\"Var\\nx\"]"));
    }

    #[test]
    fn test_pattern_wildcard() {
        let pattern = Pattern::Wildcard;
        let mut output = String::new();
        let mut gen = NodeIdGenerator::new();
        pattern_to_dot(&pattern, &mut output, &mut gen);
        assert!(output.contains("[label=\"Wildcard\\n_\"]"));
    }

    #[test]
    fn test_pattern_tuple() {
        let pattern = Pattern::Tuple(vec![
            Pattern::Literal(Literal::Int(1)),
            Pattern::Var("x".to_string()),
        ]);
        let mut output = String::new();
        let mut gen = NodeIdGenerator::new();
        pattern_to_dot(&pattern, &mut output, &mut gen);
        assert!(output.contains("[label=\"TuplePattern\"]"));
        assert!(output.contains("[label=\"Literal\\nInt 1\"]"));
        assert!(output.contains("[label=\"Var\\nx\"]"));
    }
}
