/// DOT format generator for ParLang AST visualization
/// 
/// This module provides functionality to convert ParLang Abstract Syntax Trees
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

/// Counter for generating unique node IDs
struct NodeIdGenerator {
    counter: usize,
}

impl NodeIdGenerator {
    fn new() -> Self {
        NodeIdGenerator { counter: 0 }
    }

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
/// Result indicating success or IO error
pub fn write_ast_to_dot_file(expr: &Expr, path: &str) -> io::Result<()> {
    let dot_content = ast_to_dot(expr);
    std::fs::write(path, dot_content)
}

fn expr_to_dot(expr: &Expr, output: &mut String, gen: &mut NodeIdGenerator) -> String {
    let node_id = gen.next();
    
    match expr {
        Expr::Int(n) => {
            output.push_str(&format!("  {} [label=\"Int\\n{}\"];\n", node_id, n));
        }
        Expr::Bool(b) => {
            output.push_str(&format!("  {} [label=\"Bool\\n{}\"];\n", node_id, b));
        }
        Expr::Var(name) => {
            output.push_str(&format!("  {} [label=\"Var\\n{}\"];\n", node_id, escape_label(name)));
        }
        Expr::BinOp(op, left, right) => {
            output.push_str(&format!("  {} [label=\"BinOp\\n{}\"];\n", node_id, binop_label(op)));
            let left_id = expr_to_dot(left, output, gen);
            let right_id = expr_to_dot(right, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"left\"];\n", node_id, left_id));
            output.push_str(&format!("  {} -> {} [label=\"right\"];\n", node_id, right_id));
        }
        Expr::If(cond, then_branch, else_branch) => {
            output.push_str(&format!("  {} [label=\"If\"];\n", node_id));
            let cond_id = expr_to_dot(cond, output, gen);
            let then_id = expr_to_dot(then_branch, output, gen);
            let else_id = expr_to_dot(else_branch, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"cond\"];\n", node_id, cond_id));
            output.push_str(&format!("  {} -> {} [label=\"then\"];\n", node_id, then_id));
            output.push_str(&format!("  {} -> {} [label=\"else\"];\n", node_id, else_id));
        }
        Expr::Let(name, value, body) => {
            output.push_str(&format!("  {} [label=\"Let\\n{}\"];\n", node_id, escape_label(name)));
            let value_id = expr_to_dot(value, output, gen);
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"value\"];\n", node_id, value_id));
            output.push_str(&format!("  {} -> {} [label=\"body\"];\n", node_id, body_id));
        }
        Expr::Fun(param, body) => {
            output.push_str(&format!("  {} [label=\"Fun\\n{}\"];\n", node_id, escape_label(param)));
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"body\"];\n", node_id, body_id));
        }
        Expr::App(func, arg) => {
            output.push_str(&format!("  {} [label=\"App\"];\n", node_id));
            let func_id = expr_to_dot(func, output, gen);
            let arg_id = expr_to_dot(arg, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"func\"];\n", node_id, func_id));
            output.push_str(&format!("  {} -> {} [label=\"arg\"];\n", node_id, arg_id));
        }
        Expr::Load(filepath, body) => {
            output.push_str(&format!("  {} [label=\"Load\\n{}\"];\n", node_id, escape_label(filepath)));
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"body\"];\n", node_id, body_id));
        }
        Expr::Seq(bindings, body) => {
            output.push_str(&format!("  {} [label=\"Seq\"];\n", node_id));
            for (i, (name, value)) in bindings.iter().enumerate() {
                let binding_id = gen.next();
                output.push_str(&format!("  {} [label=\"Binding\\n{}\"];\n", binding_id, escape_label(name)));
                let value_id = expr_to_dot(value, output, gen);
                output.push_str(&format!("  {} -> {} [label=\"binding {}\"];\n", node_id, binding_id, i));
                output.push_str(&format!("  {} -> {} [label=\"value\"];\n", binding_id, value_id));
            }
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"body\"];\n", node_id, body_id));
        }
        Expr::Rec(name, body) => {
            output.push_str(&format!("  {} [label=\"Rec\\n{}\"];\n", node_id, escape_label(name)));
            let body_id = expr_to_dot(body, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"body\"];\n", node_id, body_id));
        }
        Expr::Match(scrutinee, arms) => {
            output.push_str(&format!("  {} [label=\"Match\"];\n", node_id));
            let scrutinee_id = expr_to_dot(scrutinee, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"scrutinee\"];\n", node_id, scrutinee_id));
            
            for (i, (pattern, result)) in arms.iter().enumerate() {
                let arm_id = gen.next();
                output.push_str(&format!("  {} [label=\"Arm {}\"];\n", arm_id, i));
                let pattern_id = pattern_to_dot(pattern, output, gen);
                let result_id = expr_to_dot(result, output, gen);
                output.push_str(&format!("  {} -> {} [label=\"arm {}\"];\n", node_id, arm_id, i));
                output.push_str(&format!("  {} -> {} [label=\"pattern\"];\n", arm_id, pattern_id));
                output.push_str(&format!("  {} -> {} [label=\"result\"];\n", arm_id, result_id));
            }
        }
        Expr::Tuple(elements) => {
            output.push_str(&format!("  {} [label=\"Tuple\"];\n", node_id));
            for (i, elem) in elements.iter().enumerate() {
                let elem_id = expr_to_dot(elem, output, gen);
                output.push_str(&format!("  {} -> {} [label=\"elem {}\"];\n", node_id, elem_id, i));
            }
        }
        Expr::TupleProj(tuple, index) => {
            output.push_str(&format!("  {} [label=\"TupleProj\\n{}\"];\n", node_id, index));
            let tuple_id = expr_to_dot(tuple, output, gen);
            output.push_str(&format!("  {} -> {} [label=\"tuple\"];\n", node_id, tuple_id));
        }
    }
    
    node_id
}

fn pattern_to_dot(pattern: &Pattern, output: &mut String, gen: &mut NodeIdGenerator) -> String {
    let node_id = gen.next();
    
    match pattern {
        Pattern::Literal(lit) => {
            let label = match lit {
                Literal::Int(n) => format!("Literal\\nInt {}", n),
                Literal::Bool(b) => format!("Literal\\nBool {}", b),
            };
            output.push_str(&format!("  {} [label=\"{}\"];\n", node_id, label));
        }
        Pattern::Var(name) => {
            output.push_str(&format!("  {} [label=\"Var\\n{}\"];\n", node_id, escape_label(name)));
        }
        Pattern::Wildcard => {
            output.push_str(&format!("  {} [label=\"Wildcard\\n_\"];\n", node_id));
        }
        Pattern::Tuple(patterns) => {
            output.push_str(&format!("  {} [label=\"TuplePattern\"];\n", node_id));
            for (i, pat) in patterns.iter().enumerate() {
                let pat_id = pattern_to_dot(pat, output, gen);
                output.push_str(&format!("  {} -> {} [label=\"elem {}\"];\n", node_id, pat_id, i));
            }
        }
    }
    
    node_id
}

fn binop_label(op: &BinOp) -> &'static str {
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
        let expr = Expr::Fun("x".to_string(), Box::new(Expr::Var("x".to_string())));
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
            ("x".to_string(), Expr::Int(42)),
            ("y".to_string(), Expr::Int(10)),
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
            let label = binop_label(op);
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
