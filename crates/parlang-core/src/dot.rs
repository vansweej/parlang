//! Graphviz DOT dumper for Core terms (Arc B, B3).
//!
//! This mirrors the surface-language DOT style so Core and surface graphs are
//! visually comparable. Node ids are generated in a stable pre-order. The
//! header is `digraph Core` (not `digraph AST`) so this dumper's output never
//! collides with the surface golden tests.

use std::fmt::Write as _;

use crate::term::{Alt, Lit, Term};

/// Renders a Core term as a Graphviz DOT document.
#[must_use]
pub fn core_to_dot(term: &Term) -> String {
    let mut out = String::from("digraph Core {\n");
    let mut ids = NodeIdGenerator { counter: 0 };
    emit(term, &mut ids, &mut out);
    out.push_str("}\n");
    out
}

/// Generates unique, stable node identifiers for the DOT output.
struct NodeIdGenerator {
    counter: usize,
}

impl NodeIdGenerator {
    fn next(&mut self) -> String {
        let n = self.counter;
        self.counter += 1;
        format!("node{n}")
    }
}

/// Escapes a raw label string for inclusion inside a DOT quoted label.
fn escape_label(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Emits a node for `term` and edges to its children, returning the node's id.
fn emit(term: &Term, ids: &mut NodeIdGenerator, out: &mut String) -> String {
    let id = ids.next();
    let label = escape_label(&node_label(term));
    let _ = writeln!(out, "  {id} [label=\"{label}\"];");
    for child in children(term) {
        let child_id = emit(child, ids, out);
        let _ = writeln!(out, "  {id} -> {child_id};");
    }
    if let Term::Case(_, alts) = term {
        for alt in alts {
            emit_alt(alt, id.as_str(), ids, out);
        }
    }
    id
}

/// Emits an intermediate alt node (labelled `Con binders`) under a `Case`
/// node, then the alt body as the alt node's child.
fn emit_alt(alt: &Alt, case_id: &str, ids: &mut NodeIdGenerator, out: &mut String) {
    let alt_id = ids.next();
    let mut label = alt.con.clone();
    if !alt.binders.is_empty() {
        label.push(' ');
        label.push_str(&alt.binders.join(" "));
    }
    let escaped = escape_label(&label);
    let _ = writeln!(out, "  {alt_id} [label=\"{escaped}\"];");
    let _ = writeln!(out, "  {case_id} -> {alt_id};");
    let body_id = emit(&alt.body, ids, out);
    let _ = writeln!(out, "  {alt_id} -> {body_id};");
}

/// Returns the DOT node label for a term.
fn node_label(term: &Term) -> String {
    match term {
        Term::Var(name) => format!("Var {name}"),
        Term::Lam(name, ty, _) => format!("Lam {name}: {ty}"),
        Term::App(_, _) => "App".to_string(),
        Term::Let(name, _, _) => format!("Let {name}"),
        Term::LetRec(name, _, _) => format!("LetRec {name}"),
        Term::Lit(lit) => lit_label(lit),
        Term::Con(name, _) => format!("Con {name}"),
        Term::Case(_, _) => "Case".to_string(),
    }
}

/// Returns the DOT node label for a literal.
fn lit_label(lit: &Lit) -> String {
    match lit {
        Lit::Int(v) => format!("Lit {v}"),
        Lit::Bool(v) => format!("Lit {v}"),
        Lit::Float(v) => format!("Lit {v}"),
        Lit::Unit => "Lit ()".to_string(),
        Lit::Char(c) => format!("Lit {c:?}"),
    }
}

/// Returns the ordered child term references of a term.
fn children(term: &Term) -> Vec<&Term> {
    match term {
        Term::Var(_) | Term::Lit(_) => Vec::new(),
        Term::Lam(_, _, body) => vec![body.as_ref()],
        Term::App(func, arg) => vec![func.as_ref(), arg.as_ref()],
        Term::Let(_, value, body) | Term::LetRec(_, value, body) => {
            vec![value.as_ref(), body.as_ref()]
        }
        Term::Con(_, args) => args.iter().collect(),
        Term::Case(scrutinee, _) => vec![scrutinee.as_ref()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_type::BaseType;

    #[test]
    fn identity_lambda_dumps_expected_nodes() {
        let term = Term::Lam(
            "x".to_string(),
            BaseType::Int,
            Box::new(Term::Var("x".to_string())),
        );
        let dot = core_to_dot(&term);
        assert!(dot.starts_with("digraph Core {\n"), "header missing: {dot}");
        assert!(
            dot.contains("node0 [label=\"Lam x: int\"];"),
            "lam node missing: {dot}"
        );
        assert!(
            dot.contains("node1 [label=\"Var x\"];"),
            "var node missing: {dot}"
        );
        assert!(dot.contains("node0 -> node1;"), "edge missing: {dot}");
    }

    #[test]
    fn escaping_quotes_in_char_literal() {
        let term = Term::Lit(Lit::Char('"'));
        let dot = core_to_dot(&term);
        assert!(dot.contains("\\\""), "quote not escaped: {dot}");
    }

    #[test]
    fn case_dumps_alt_nodes() {
        let term = Term::Case(
            Box::new(Term::Var("s".to_string())),
            vec![Alt {
                con: "Nil".to_string(),
                binders: vec![],
                body: Term::Lit(Lit::Int(0)),
            }],
        );
        let dot = core_to_dot(&term);
        assert!(
            dot.contains("[label=\"Case\"];"),
            "case node missing: {dot}"
        );
        assert!(dot.contains("[label=\"Nil\"];"), "alt node missing: {dot}");
    }
}
