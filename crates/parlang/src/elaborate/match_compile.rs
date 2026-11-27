//! Compiles typed `match` expressions into Core `Case`/`if` decision trees.
//!
//! The algorithm is a classical column-based pattern-match compiler
//! (Maranget-style): patterns are lowered into a simplified `Pat` form that
//! tracks each variable's source position (`seq`), then `build` repeatedly
//! picks the leftmost refutable column of the first row and specialises the
//! row matrix on it, producing a `Tree` of `Switch` (constructor dispatch via
//! `Case`), `IfBool`/`IfLit` (literal dispatch via `if`/`eq`), and `Leaf`
//! nodes. Because column-wise specialisation can duplicate a single arm's
//! body across multiple leaves, any arm reached by more than one leaf is
//! compiled once into a join point (a Core `let`-bound closure) and every
//! leaf for that arm becomes a call to the join point instead of duplicating
//! the arm body.

use std::collections::BTreeMap;

use parlang_core::{builder, BaseType, Term};

use super::{unsupported, Elab, ElabError};
use crate::ast::Literal;
use crate::typed_ast::{TypedExpr, TypedMatchArm, TypedPattern, TypedPatternKind};
use crate::types::Type;

/// A surface variable bound to an occurrence, with its source pre-order position.
#[derive(Clone)]
struct Bind {
    name: String,
    seq: usize,
    occ: String,
}

/// A simplified pattern used by the match compiler.
#[derive(Clone)]
enum Pat {
    /// A variable (with its name and source position) or a wildcard (`None`).
    Any(Option<(String, usize)>),
    /// A constructor pattern: constructor name and sub-patterns.
    Ctor(String, Vec<Pat>),
    /// A literal pattern.
    Lit(Literal),
}

/// A single row of the pattern matrix: one pattern per occurrence, the
/// bindings already resolved by prior specialisation, and the originating
/// arm index.
#[derive(Clone)]
struct Row {
    pats: Vec<Pat>,
    binds: Vec<Bind>,
    arm: usize,
}

/// The compiled decision tree.
enum Tree {
    /// A successful match: the arm index and its accumulated bindings.
    Leaf { arm: usize, binds: Vec<Bind> },
    /// Constructor dispatch via `Case`.
    Switch {
        occ: String,
        alts: Vec<(String, Vec<String>, Tree)>,
    },
    /// Boolean dispatch via `if`.
    IfBool {
        occ: String,
        then_t: Box<Tree>,
        else_t: Box<Tree>,
    },
    /// Int/Char literal dispatch via `if (eq occ lit) ...`.
    IfLit {
        occ: String,
        lit: Literal,
        then_t: Box<Tree>,
        else_t: Box<Tree>,
    },
}

/// A let-bound join point: a closure shared by every leaf of one arm.
struct Join {
    name: String,
    params: Vec<String>,
}

/// Per-arm join points, paired with their `let`-bindings in ascending arm order.
type JoinsAndLets = (Vec<Option<Join>>, Vec<(String, Term)>);

impl Elab {
    /// Compiles a `match` into a Core decision tree with join points; see the module docs.
    pub(super) fn compile_match(
        &mut self,
        scrut: &TypedExpr,
        arms: &[TypedMatchArm],
    ) -> Result<Term, ElabError> {
        let s_term = self.elab_expr(scrut)?;
        let s = self.fresh_name("s");

        let mut rows = Vec::with_capacity(arms.len());
        for (i, arm) in arms.iter().enumerate() {
            let mut seq = 0usize;
            let p = self.lower_pat(&arm.pattern, Some(&scrut.ty), &mut seq)?;
            rows.push(Row {
                pats: vec![p],
                binds: Vec::new(),
                arm: i,
            });
        }

        let tree = self.build(std::slice::from_ref(&s), &rows)?;

        let mut counts = vec![0usize; arms.len()];
        count_leaves(&tree, &mut counts);

        let bodies: Vec<Term> = arms
            .iter()
            .map(|a| self.elab_expr(&a.body))
            .collect::<Result<_, _>>()?;

        let (joins, join_lets) = self.make_joins(arms, &counts, &bodies)?;

        let mut out = emit(&tree, &bodies, &joins)?;
        for (name, value) in join_lets.into_iter().rev() {
            out = builder::let_(&name, value, out)?;
        }
        Ok(builder::let_(&s, s_term, out)?)
    }

    /// Lowers a typed pattern into the simplified `Pat` form, assigning each
    /// variable its source pre-order position via `seq`.
    fn lower_pat(
        &mut self,
        p: &TypedPattern,
        hint: Option<&Type>,
        seq: &mut usize,
    ) -> Result<Pat, ElabError> {
        match &p.kind {
            TypedPatternKind::Var(x) => {
                let n = *seq;
                *seq += 1;
                Ok(Pat::Any(Some((x.clone(), n))))
            }
            TypedPatternKind::Wildcard => Ok(Pat::Any(None)),
            TypedPatternKind::Literal(l) => Ok(Pat::Lit(l.clone())),
            TypedPatternKind::Constructor(c, ps) => {
                let mut lowered = Vec::with_capacity(ps.len());
                for sub in ps {
                    lowered.push(self.lower_pat(sub, None, seq)?);
                }
                Ok(Pat::Ctor(c.clone(), lowered))
            }
            TypedPatternKind::Tuple(ps) => {
                if ps.is_empty() {
                    return Ok(Pat::Any(None));
                }
                let mut lowered = Vec::with_capacity(ps.len());
                for (idx, sub) in ps.iter().enumerate() {
                    let sub_hint = match hint {
                        Some(Type::Tuple(ts)) => ts.get(idx),
                        _ => None,
                    };
                    lowered.push(self.lower_pat(sub, sub_hint, seq)?);
                }
                let name = self.ensure_tuple(ps.len())?;
                Ok(Pat::Ctor(name, lowered))
            }
            TypedPatternKind::Record(fs) => {
                let ty = match hint {
                    Some(h @ Type::Record(_)) => h,
                    _ => &p.ty,
                };
                self.lower_record(fs, ty, seq)
            }
        }
    }

    /// Lowers a record pattern against its (already-known) record type,
    /// filling absent fields with wildcards.
    fn lower_record(
        &mut self,
        fs: &[(String, TypedPattern)],
        ty: &Type,
        seq: &mut usize,
    ) -> Result<Pat, ElabError> {
        let Type::Record(m) = ty else {
            return Err(unsupported("match", "record pattern without record type"));
        };
        let mut map: BTreeMap<String, Pat> = BTreeMap::new();
        for (name, sub) in fs {
            let lowered = self.lower_pat(sub, None, seq)?;
            map.insert(name.clone(), lowered);
        }
        let mut sorted: Vec<String> = m.keys().cloned().collect();
        sorted.sort();
        let name = self.ensure_record(&sorted)?;
        let subs: Vec<Pat> = sorted
            .iter()
            .map(|k| map.remove(k).unwrap_or(Pat::Any(None)))
            .collect();
        Ok(Pat::Ctor(name, subs))
    }

    /// Builds a decision tree for `rows` over the given occurrences.
    fn build(&mut self, occs: &[String], rows: &[Row]) -> Result<Tree, ElabError> {
        let Some(first) = rows.first() else {
            return Err(unsupported(
                "match",
                "non-exhaustive match reached elaboration",
            ));
        };
        let col = first.pats.iter().position(|p| !matches!(p, Pat::Any(_)));
        let Some(j) = col else {
            return Ok(Tree::Leaf {
                arm: first.arm,
                binds: leaf_binds(first, occs),
            });
        };
        match first.pats.get(j) {
            Some(Pat::Ctor(c, _)) => {
                let head = c.clone();
                self.build_ctor(occs, rows, j, &head)
            }
            Some(Pat::Lit(_)) => self.build_lit(occs, rows, j),
            _ => Err(unsupported(
                "match",
                "non-exhaustive match reached elaboration",
            )),
        }
    }

    /// Builds the `Switch` subtree for the column headed by a constructor.
    fn build_ctor(
        &mut self,
        occs: &[String],
        rows: &[Row],
        j: usize,
        head: &str,
    ) -> Result<Tree, ElabError> {
        let ty_name = self
            .ctor_owner
            .get(head)
            .cloned()
            .ok_or_else(|| unsupported("match", "unknown constructor"))?;
        let ctors = self
            .data_ctors
            .get(&ty_name)
            .cloned()
            .ok_or_else(|| unsupported("match", "unknown constructor"))?;
        let occ = occs.get(j).cloned().unwrap_or_default();

        let mut alts = Vec::with_capacity(ctors.len());
        for (cname, k) in ctors {
            let fields: Vec<String> = (0..k).map(|_| self.fresh_name("f")).collect();
            let new_occs: Vec<String> = occs
                .iter()
                .take(j)
                .cloned()
                .chain(fields.iter().cloned())
                .chain(occs.iter().skip(j + 1).cloned())
                .collect();
            let sub_rows = specialise_ctor(rows, j, &cname, k, &occ);
            let sub = self.build(&new_occs, &sub_rows)?;
            alts.push((cname, fields, sub));
        }
        Ok(Tree::Switch { occ, alts })
    }

    /// Builds the `IfBool`/`IfLit` subtree for a column headed by a literal.
    fn build_lit(&mut self, occs: &[String], rows: &[Row], j: usize) -> Result<Tree, ElabError> {
        let occ = occs.get(j).cloned().unwrap_or_default();
        let rest_occs: Vec<String> = occs
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != j)
            .map(|(_, o)| o.clone())
            .collect();

        if let Some(first) = rows.first() {
            if matches!(first.pats.get(j), Some(Pat::Lit(Literal::Bool(_)))) {
                let then_t = self.build(
                    &rest_occs,
                    &specialise_lit(rows, j, Some(&Literal::Bool(true)), &occ),
                )?;
                let else_t = self.build(
                    &rest_occs,
                    &specialise_lit(rows, j, Some(&Literal::Bool(false)), &occ),
                )?;
                return Ok(Tree::IfBool {
                    occ,
                    then_t: Box::new(then_t),
                    else_t: Box::new(else_t),
                });
            }
        }

        let mut lits: Vec<Literal> = Vec::new();
        for row in rows {
            if let Some(Pat::Lit(l)) = row.pats.get(j) {
                if !lits.iter().any(|existing| lit_eq(existing, l)) {
                    lits.push(l.clone());
                }
            }
        }

        let mut tail = self.build(&rest_occs, &specialise_lit(rows, j, None, &occ))?;
        for lit in lits.iter().rev() {
            let then_t = self.build(&rest_occs, &specialise_lit(rows, j, Some(lit), &occ))?;
            tail = Tree::IfLit {
                occ: occ.clone(),
                lit: lit.clone(),
                then_t: Box::new(then_t),
                else_t: Box::new(tail),
            };
        }
        Ok(tail)
    }

    /// Builds a join point for every arm reached by more than one leaf.
    fn make_joins(
        &mut self,
        arms: &[TypedMatchArm],
        counts: &[usize],
        bodies: &[Term],
    ) -> Result<JoinsAndLets, ElabError> {
        let mut joins = Vec::with_capacity(arms.len());
        let mut join_lets = Vec::new();

        for (i, arm) in arms.iter().enumerate() {
            if counts.get(i).copied().unwrap_or(0) <= 1 {
                joins.push(None);
                continue;
            }

            let name = self.fresh_name(parlang_core::eligibility::JOIN_POINT_TAG);
            let mut params: Vec<(String, Type)> = Vec::new();
            for (pname, pty) in &arm.bindings {
                if let Some(existing) = params.iter_mut().find(|(n, _)| n == pname) {
                    existing.1 = pty.clone();
                } else {
                    params.push((pname.clone(), pty.clone()));
                }
            }

            let body = bodies
                .get(i)
                .cloned()
                .ok_or_else(|| unsupported("match", "arm body missing"))?;

            let value = if params.is_empty() {
                let u = self.fresh_name("u");
                builder::lam(&u, BaseType::Unit, body)?
            } else {
                let mut value = body;
                for (p, ty) in params.iter().rev() {
                    let core_ty = self.conv_type(ty)?;
                    value = builder::lam(p, core_ty, value)?;
                }
                value
            };

            let param_names: Vec<String> = params.into_iter().map(|(n, _)| n).collect();
            joins.push(Some(Join {
                name: name.clone(),
                params: param_names,
            }));
            join_lets.push((name, value));
        }

        Ok((joins, join_lets))
    }
}

/// Compares two literals for equality (used to deduplicate a literal column).
fn lit_eq(a: &Literal, b: &Literal) -> bool {
    match (a, b) {
        (Literal::Int(x), Literal::Int(y)) => x == y,
        (Literal::Bool(x), Literal::Bool(y)) => x == y,
        (Literal::Char(x), Literal::Char(y)) => x == y,
        _ => false,
    }
}

/// Specialises `rows` on column `j` for constructor `ctor` of arity `arity`.
fn specialise_ctor(rows: &[Row], j: usize, ctor: &str, arity: usize, occ: &str) -> Vec<Row> {
    let mut out = Vec::new();
    for row in rows {
        match row.pats.get(j) {
            Some(Pat::Ctor(c, ps)) if c == ctor => {
                let mut new_pats: Vec<Pat> = row.pats.iter().take(j).cloned().collect();
                new_pats.extend(ps.iter().cloned());
                new_pats.extend(row.pats.iter().skip(j + 1).cloned());
                out.push(Row {
                    pats: new_pats,
                    binds: row.binds.clone(),
                    arm: row.arm,
                });
            }
            Some(Pat::Ctor(..) | Pat::Lit(_)) | None => {}
            Some(Pat::Any(v)) => {
                let mut new_pats: Vec<Pat> = row.pats.iter().take(j).cloned().collect();
                for _ in 0..arity {
                    new_pats.push(Pat::Any(None));
                }
                new_pats.extend(row.pats.iter().skip(j + 1).cloned());
                let mut binds = row.binds.clone();
                if let Some((name, seq)) = v {
                    binds.push(Bind {
                        name: name.clone(),
                        seq: *seq,
                        occ: occ.to_string(),
                    });
                }
                out.push(Row {
                    pats: new_pats,
                    binds,
                    arm: row.arm,
                });
            }
        }
    }
    out
}

/// Specialises `rows` on column `j` for the literal `lit` (or the default
/// matrix, when `lit` is `None`).
fn specialise_lit(rows: &[Row], j: usize, lit: Option<&Literal>, occ: &str) -> Vec<Row> {
    let mut out = Vec::new();
    for row in rows {
        let keep = match (row.pats.get(j), lit) {
            (Some(Pat::Lit(x)), Some(l)) => lit_eq(x, l),
            (Some(Pat::Any(_)), _) => true,
            _ => false,
        };
        if !keep {
            continue;
        }
        let new_pats: Vec<Pat> = row
            .pats
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != j)
            .map(|(_, p)| p.clone())
            .collect();
        let mut binds = row.binds.clone();
        if let Some(Pat::Any(Some((name, seq)))) = row.pats.get(j) {
            binds.push(Bind {
                name: name.clone(),
                seq: *seq,
                occ: occ.to_string(),
            });
        }
        out.push(Row {
            pats: new_pats,
            binds,
            arm: row.arm,
        });
    }
    out
}

/// Collects the bindings for a leaf row: its already-accumulated bindings
/// plus a `Bind` for every remaining `Any(Some(..))` column.
fn leaf_binds(row: &Row, occs: &[String]) -> Vec<Bind> {
    let mut binds = row.binds.clone();
    for (i, p) in row.pats.iter().enumerate() {
        if let Pat::Any(Some((name, seq))) = p {
            if let Some(occ) = occs.get(i) {
                binds.push(Bind {
                    name: name.clone(),
                    seq: *seq,
                    occ: occ.clone(),
                });
            }
        }
    }
    binds
}

/// Selects the "last in source wins" binding for each distinct name,
/// returned sorted by source position.
fn winners(binds: &[Bind]) -> Vec<Bind> {
    let mut best: BTreeMap<String, Bind> = BTreeMap::new();
    for b in binds {
        let replace = match best.get(&b.name) {
            Some(existing) => b.seq >= existing.seq,
            None => true,
        };
        if replace {
            best.insert(b.name.clone(), b.clone());
        }
    }
    let mut out: Vec<Bind> = best.into_values().collect();
    out.sort_by_key(|b| b.seq);
    out
}

/// Counts how many leaves target each arm.
fn count_leaves(t: &Tree, counts: &mut [usize]) {
    match t {
        Tree::Leaf { arm, .. } => {
            if let Some(c) = counts.get_mut(*arm) {
                *c += 1;
            }
        }
        Tree::Switch { alts, .. } => {
            for (_, _, sub) in alts {
                count_leaves(sub, counts);
            }
        }
        Tree::IfBool { then_t, else_t, .. } | Tree::IfLit { then_t, else_t, .. } => {
            count_leaves(then_t, counts);
            count_leaves(else_t, counts);
        }
    }
}

/// Emits Core terms for a decision tree.
fn emit(t: &Tree, bodies: &[Term], joins: &[Option<Join>]) -> Result<Term, ElabError> {
    match t {
        Tree::Leaf { arm, binds } => emit_leaf(*arm, binds, bodies, joins),
        Tree::Switch { occ, alts } => {
            let mut built = Vec::with_capacity(alts.len());
            for (cname, fields, sub) in alts {
                let refs: Vec<&str> = fields.iter().map(String::as_str).collect();
                built.push(builder::alt(cname, &refs, emit(sub, bodies, joins)?)?);
            }
            Ok(builder::case_(builder::var(occ)?, built))
        }
        Tree::IfBool {
            occ,
            then_t,
            else_t,
        } => Ok(builder::con(
            "if",
            vec![
                builder::var(occ)?,
                emit(then_t, bodies, joins)?,
                emit(else_t, bodies, joins)?,
            ],
        )?),
        Tree::IfLit {
            occ,
            lit,
            then_t,
            else_t,
        } => {
            let lit_term = match lit {
                Literal::Int(n) => builder::int(*n),
                Literal::Char(c) => builder::char_(*c),
                Literal::Bool(b) => builder::bool_(*b),
            };
            Ok(builder::con(
                "if",
                vec![
                    builder::con("eq", vec![builder::var(occ)?, lit_term])?,
                    emit(then_t, bodies, joins)?,
                    emit(else_t, bodies, joins)?,
                ],
            )?)
        }
    }
}

/// Emits the Core term for a single leaf, either a call to its join point
/// (when the arm is shared across multiple leaves) or the arm body wrapped
/// in `let`-bindings for its pattern variables.
fn emit_leaf(
    arm: usize,
    binds: &[Bind],
    bodies: &[Term],
    joins: &[Option<Join>],
) -> Result<Term, ElabError> {
    let won = winners(binds);

    if let Some(Some(j)) = joins.get(arm) {
        if j.params.is_empty() {
            return Ok(builder::app(builder::var(&j.name)?, builder::unit()));
        }
        let mut call = builder::var(&j.name)?;
        for p in &j.params {
            let occ = won
                .iter()
                .find(|b| &b.name == p)
                .map(|b| b.occ.clone())
                .ok_or_else(|| unsupported("match", "join point binding missing"))?;
            call = builder::app(call, builder::var(&occ)?);
        }
        return Ok(call);
    }

    let mut body = bodies
        .get(arm)
        .cloned()
        .ok_or_else(|| unsupported("match", "arm body missing"))?;
    for b in won.iter().rev() {
        body = builder::let_(&b.name, builder::var(&b.occ)?, body)?;
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use parlang_core::{Term, Value};

    /// Parses, typechecks, elaborates, Lint-checks, and evaluates `src`,
    /// returning the elaborated `Term` and its runtime `Value`.
    fn run(src: &str) -> (Term, Value) {
        let program = crate::parser::parse_program(src).expect("parse");
        let typed = crate::typechecker::typecheck_program_typed(&program).expect("typecheck");
        let elab = super::super::elaborate_program(&typed).expect("elaborate");
        let checked = super::super::check_elaborated(&elab);
        assert!(checked.is_ok(), "check_elaborated: {checked:?}");
        let value =
            parlang_core::eval(&elab.term, &parlang_core::Environment::new()).expect("eval");
        (elab.term, value)
    }

    /// A constructor pattern match with a wildcard catch-all evaluates correctly.
    #[test]
    fn ctor_match_with_catch_all() {
        let (_, v) = run("data Color = Red | Green | Blue; match Green with | Red -> 1 | _ -> 2");
        assert_eq!(v, Value::Int(2));
    }

    /// A constructor pattern with a binder evaluates correctly.
    #[test]
    fn option_with_binders() {
        let (_, v) =
            run("data Option a = None | Some a; match Some 41 with | None -> 0 | Some x -> x + 1");
        assert_eq!(v, Value::Int(42));
    }

    /// A polymorphic recursive list-length function compiles and evaluates correctly.
    #[test]
    fn polymorphic_recursive_list_length() {
        let (_, v) = run(
            "data List a = Nil | Cons a (List a); let len = rec len -> fun xs -> match xs with | Nil -> 0 | Cons h t -> 1 + len t; len (Cons 1 (Cons 2 Nil)) + len (Cons true Nil)",
        );
        assert_eq!(v, Value::Int(3));
    }

    /// Nested constructor patterns compile and evaluate correctly.
    #[test]
    fn nested_ctor_patterns() {
        let (_, v) = run(
            "data List a = Nil | Cons a (List a); let f = fun xs -> match xs with | Cons x Nil -> x | Cons x (Cons y t) -> x + y | Nil -> 0; f (Cons 1 Nil) + f (Cons 2 (Cons 3 Nil)) + f Nil",
        );
        assert_eq!(v, Value::Int(6));
    }

    /// Integer literal patterns with a default arm compile and evaluate correctly.
    #[test]
    fn int_literal_with_default() {
        let (_, v) =
            run("let f = fun n -> match n with | 0 -> 10 | 1 -> 20 | _ -> 30; f 0 + f 1 + f 7");
        assert_eq!(v, Value::Int(60));
    }

    /// Char literal patterns compile and evaluate correctly.
    #[test]
    fn char_literal_match() {
        let (_, v) = run(
            "let f = fun c -> match c with | 'a' -> 1 | 'b' -> 2 | _ -> 3; f 'a' + f 'b' + f 'z'",
        );
        assert_eq!(v, Value::Int(6));
    }

    /// A bool literal match compiles to `if` and evaluates correctly.
    #[test]
    fn bool_literal_match() {
        let (_, v) = run("match true with | true -> 'y' | false -> 'n'");
        assert_eq!(v, Value::Char('y'));
    }

    /// A bool pattern nested inside a tuple compiles and evaluates correctly.
    #[test]
    fn bool_nested_in_tuple() {
        let (_, v) = run(
            "let f = fun p -> match p with | (true, x) -> x | (false, _) -> 0; f (true, 5) + f (false, 9)",
        );
        assert_eq!(v, Value::Int(5));
    }

    /// A tuple of constructor patterns compiles and evaluates correctly.
    #[test]
    fn tuple_of_constructors() {
        let (_, v) = run(
            "data List a = Nil | Cons a (List a); let g = fun p -> match p with | (Nil, Nil) -> 0 | (Nil, _) -> 1 | (_, Nil) -> 2 | (Cons a _, Cons b _) -> a + b; g (Nil, Nil) + g (Nil, Cons 1 Nil) + g (Cons 1 Nil, Nil) + g (Cons 3 Nil, Cons 4 Nil)",
        );
        assert_eq!(v, Value::Int(10));
    }

    /// A partial record pattern compiles and evaluates correctly.
    #[test]
    fn partial_record_pattern() {
        let (_, v) = run("match {x: 1, y: 2} with | {x: 1} -> 10 | {y: n} -> n");
        assert_eq!(v, Value::Int(10));
    }

    /// The unit pattern compiles and evaluates correctly.
    #[test]
    fn unit_pattern() {
        let (_, v) = run("match () with | () -> 7");
        assert_eq!(v, Value::Int(7));
    }

    /// A variable catch-all pattern binds the scrutinee.
    #[test]
    fn var_catch_all_binds_scrutinee() {
        let (_, v) = run("match 5 with | n -> n * 2");
        assert_eq!(v, Value::Int(10));
    }

    /// A strcmp-shaped recursive match over a custom char-list type compiles and evaluates correctly.
    #[test]
    fn strcmp_over_char_lists() {
        let (_, v) = run(
            "data Str = SNil | SCons (Char) (Str); let cmp = rec cmp -> fun a -> fun b -> match (a, b) with | (SNil, SNil) -> 0 | (SNil, _) -> 0 - 1 | (_, SNil) -> 1 | (SCons x xs, SCons y ys) -> if x == y then cmp xs ys else if x < y then 0 - 1 else 1; cmp (SCons 'a' (SCons 'b' SNil)) (SCons 'a' (SCons 'c' SNil))",
        );
        assert_eq!(v, Value::Int(-1));
    }

    /// A match arm whose body constructs a value evaluates correctly.
    #[test]
    fn constructor_value_result() {
        let (_, v) =
            run("data Option a = None | Some a; match 3 with | 3 -> Some true | _ -> None");
        assert_eq!(v, Value::Con("Some".to_string(), vec![Value::Bool(true)]));
    }

    /// An arm reached by more than one leaf is compiled as a shared join point.
    #[test]
    fn join_point_shares_arm_body() {
        let (term, v) = run(
            "data List a = Nil | Cons a (List a); let f = fun p -> match p with | (Nil, Nil) -> 0 | (x, y) -> 42; f (Nil, Cons 1 Nil) + f (Cons 1 Nil, Nil)",
        );
        assert!(term.to_text().contains("$j"));
        assert_eq!(v, Value::Int(84));
    }

    /// A join point for an arm with no pattern bindings still compiles and stays lazy.
    #[test]
    fn join_point_without_bindings_is_lazy() {
        let (term, v) = run(
            "data AB = A | B; let f = fun p -> match p with | (A, A) -> 1 | _ -> 2; f (A, B) + f (B, B) + f (A, A)",
        );
        assert!(term.to_text().contains("$j"));
        assert_eq!(v, Value::Int(5));
    }

    /// A pattern with a duplicate binder mirrors the surface evaluator's
    /// last-binding-wins semantics.
    #[test]
    fn duplicate_binder_mirrors_surface() {
        let (_, v) = run("match (1, 2) with | (x, x) -> x");
        assert_eq!(v, Value::Int(2));
    }
}
