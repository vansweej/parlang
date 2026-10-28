//! THROWAWAY SPIKE — roadmap slice 5 (elaborator probe).
//!
//! This file is a deliberately disposable, monomorphic elaborator from the
//! surface typed AST to `parlang-core`'s Core IR over the narrowest slice
//! (`Int`, `Bool`, `Lam`, `App`, `Let`, `rec`, `if`, arithmetic, one ADT,
//! `match`). It answers ONE question: does the Core `Case` shape survive
//! contact with real elaboration of `match`? It is NOT a draft of the real
//! elaborator (slice 7); it will be deleted by an operator once its findings
//! are recorded. Nothing here is added to the shipped `parlang` library API.
//!
//! Hard-reject invariant: any construct the probe does not support returns
//! `ElabError::Unsupported`; the match compiler reaching an empty matrix
//! returns `ElabError::NonExhaustive`. The probe never silently falls back and
//! never emits a hole into a `Case` table.

use std::collections::HashMap;

use parlang::ast::Pattern;
use parlang::{
    eval_program, parse_program, run_on_evaluator_stack, typecheck_program,
    typecheck_program_typed, BinOp, Environment as SurfaceEnv, Type, TypedDecl, TypedExpr,
    TypedExprKind, TypedMatchArm, TypedProgram,
};
use parlang_core::term::{Alt, Lit, Term};
use parlang_core::{eval, BaseType, Environment as CoreEnv, Value as CoreValue};

/// An error raised when the probe meets a construct it does not support, or
/// when match compilation reaches a non-exhaustive position.
#[derive(Debug, Clone, PartialEq)]
enum ElabError {
    /// A surface construct outside the probe's deliberately narrow slice.
    Unsupported(String),
    /// Match compilation reached an empty (unmatched) matrix position.
    NonExhaustive(String),
}

/// Constructor metadata: the owning type's full constructor family (names in
/// declaration order) and this constructor's arity.
#[derive(Debug, Clone)]
struct CtorInfo {
    family: Vec<String>,
    arity: usize,
}

/// An elaboration-time table mapping constructor name -> its `CtorInfo`.
type CtorTable = HashMap<String, CtorInfo>;

/// The elaboration context: the constructor table plus a monotonic fresh-name
/// counter threaded through the WHOLE elaboration so every `%u{n}` name is
/// globally unique, including across nested matches and matches inside arm
/// bodies.
struct Elab {
    table: CtorTable,
    fresh: usize,
}

impl Elab {
    fn new() -> Self {
        Self {
            table: CtorTable::new(),
            fresh: 0,
        }
    }

    /// Produces a globally fresh scrutinee variable name. The `%u` prefix is
    /// lexically impossible in surface source (identifiers start with a
    /// letter), so these names never collide with user variables.
    fn fresh_var(&mut self) -> String {
        let n = self.fresh;
        self.fresh += 1;
        format!("%u{n}")
    }

    /// Registers a `data`/`type` constructor family into the shared table.
    /// The probe programs never shadow constructor names, so mutating the one
    /// shared table (rather than a scoped clone) is safe here; a real
    /// elaborator would scope `TypeDef` ctor registration lexically.
    fn register_ctors(&mut self, constructors: &[(String, Vec<parlang::ast::TypeAnnotation>)]) {
        let family: Vec<String> = constructors.iter().map(|(name, _)| name.clone()).collect();
        for (name, anns) in constructors {
            self.table.insert(
                name.clone(),
                CtorInfo {
                    family: family.clone(),
                    arity: anns.len(),
                },
            );
        }
    }
}

/// Maps a surface param `Type` to a Core `BaseType`.
///
/// FINDING F1: Core's `Lam` requires a `BaseType`, which cannot express ADT-
/// or function-typed binders. The probe maps `Int`/`Bool` faithfully and uses
/// `BaseType::Unit` as a documented placeholder for everything else. The
/// annotation is inert at runtime, so results are unaffected. Slice 6's
/// System-F binder types replace `BaseType` on `Lam` and retire this
/// placeholder.
fn base_type_of(ty: &Type) -> BaseType {
    match ty {
        Type::Int => BaseType::Int,
        Type::Bool => BaseType::Bool,
        _ => BaseType::Unit,
    }
}

/// Extracts the Core binder type for a `Fun` node from its own `Type::Fun`
/// domain, falling back to the F1 placeholder when the type is not a function
/// (which should not happen for a well-typed `Fun`).
fn fun_param_base_type(fun_ty: &Type) -> BaseType {
    match fun_ty {
        Type::Fun(param, _) => base_type_of(param),
        _ => BaseType::Unit,
    }
}

/// Recursively validates that a pattern is within the probe's supported slice
/// (`Var`, `Wildcard`, `Constructor` whose sub-patterns are themselves
/// supported). Literal, tuple, and record patterns are rejected here, BEFORE
/// compilation, so an unsupported pattern yields `Unsupported` rather than a
/// spurious `NonExhaustive`.
fn validate_pattern(pat: &Pattern) -> Result<(), ElabError> {
    match pat {
        Pattern::Var(_) | Pattern::Wildcard => Ok(()),
        Pattern::Constructor(_, subs) => {
            for sub in subs {
                validate_pattern(sub)?;
            }
            Ok(())
        }
        Pattern::Literal(_) => Err(ElabError::Unsupported("literal pattern".to_string())),
        Pattern::Tuple(_) => Err(ElabError::Unsupported("tuple pattern".to_string())),
        Pattern::Record(_) => Err(ElabError::Unsupported("record pattern".to_string())),
    }
}

/// Elaborates a whole typed program to a single Core `Term`.
fn elaborate_program(program: &TypedProgram) -> Result<Term, ElabError> {
    let mut ctx = Elab::new();
    for decl in &program.decls {
        match decl {
            TypedDecl::Data { constructors, .. } => ctx.register_ctors(constructors),
            TypedDecl::Let { .. } => {}
            TypedDecl::TypeAlias { .. } => {
                return Err(ElabError::Unsupported("type alias decl".to_string()));
            }
        }
    }
    let body = program
        .body
        .as_ref()
        .ok_or_else(|| ElabError::Unsupported("program without trailing body".to_string()))?;
    let mut term = elaborate(body, &mut ctx)?;
    // Wrap the body in the program's `let` decls (reverse order) so bound
    // names resolve.
    for decl in program.decls.iter().rev() {
        if let TypedDecl::Let { name, value, .. } = decl {
            let value_term = elaborate(value, &mut ctx)?;
            term = Term::Let(name.clone(), Box::new(value_term), Box::new(term));
        }
    }
    Ok(term)
}

/// Elaborates a single typed expression to a Core `Term`.
fn elaborate(expr: &TypedExpr, ctx: &mut Elab) -> Result<Term, ElabError> {
    match &expr.kind {
        TypedExprKind::Int(n) => Ok(Term::Lit(Lit::Int(*n))),
        TypedExprKind::Bool(b) => Ok(Term::Lit(Lit::Bool(*b))),
        TypedExprKind::Var(name) => Ok(Term::Var(name.clone())),
        TypedExprKind::Fun(param, _ann, body) => {
            let bt = fun_param_base_type(&expr.ty);
            let body_term = elaborate(body, ctx)?;
            Ok(Term::Lam(param.clone(), bt, Box::new(body_term)))
        }
        TypedExprKind::App(f, a) => {
            let f_term = elaborate(f, ctx)?;
            let a_term = elaborate(a, ctx)?;
            Ok(Term::App(Box::new(f_term), Box::new(a_term)))
        }
        TypedExprKind::Let(name, _ann, value, body) => {
            let value_term = elaborate(value, ctx)?;
            let body_term = elaborate(body, ctx)?;
            Ok(Term::Let(
                name.clone(),
                Box::new(value_term),
                Box::new(body_term),
            ))
        }
        TypedExprKind::Rec(name, body) => elaborate_rec(name, body, ctx),
        TypedExprKind::If(c, t, e) => {
            let c_term = elaborate(c, ctx)?;
            let t_term = elaborate(t, ctx)?;
            let e_term = elaborate(e, ctx)?;
            Ok(Term::Con("if".to_string(), vec![c_term, t_term, e_term]))
        }
        TypedExprKind::BinOp(op, l, r) => elaborate_binop(*op, l, r, ctx),
        TypedExprKind::TypeDef {
            constructors, body, ..
        } => {
            ctx.register_ctors(constructors);
            elaborate(body, ctx)
        }
        TypedExprKind::Constructor(name, args) => {
            let mut arg_terms = Vec::with_capacity(args.len());
            for arg in args {
                arg_terms.push(elaborate(arg, ctx)?);
            }
            Ok(Term::Con(name.clone(), arg_terms))
        }
        TypedExprKind::Match(scrutinee, arms) => elaborate_match(scrutinee, arms, ctx),
        other => Err(ElabError::Unsupported(format!("expr kind: {other:?}"))),
    }
}

/// Elaborates `rec f -> fun x -> ..` to `LetRec(f, Lam(x, ..), Var(f))`.
fn elaborate_rec(name: &str, body: &TypedExpr, ctx: &mut Elab) -> Result<Term, ElabError> {
    let lam = elaborate(body, ctx)?;
    if !matches!(lam, Term::Lam(..)) {
        return Err(ElabError::Unsupported(
            "rec whose body is not a lambda".to_string(),
        ));
    }
    Ok(Term::LetRec(
        name.to_string(),
        Box::new(lam),
        Box::new(Term::Var(name.to_string())),
    ))
}

/// Elaborates a supported surface binary operator to a reserved Core `Con`.
fn elaborate_binop(
    op: BinOp,
    l: &TypedExpr,
    r: &TypedExpr,
    ctx: &mut Elab,
) -> Result<Term, ElabError> {
    let name = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Lt => "<",
        BinOp::Eq => "eq",
        BinOp::Neq | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            return Err(ElabError::Unsupported(format!("binop: {op:?}")));
        }
    };
    let l_term = elaborate(l, ctx)?;
    let r_term = elaborate(r, ctx)?;
    Ok(Term::Con(name.to_string(), vec![l_term, r_term]))
}

/// Elaborates a surface `match` into a Core `Term` via backtracking-free,
/// column-wise specialisation (Wadler ch. 5 with default handling).
///
/// Every arm's pattern is validated up front so that unsupported patterns
/// (literal/tuple/record) yield `Unsupported` rather than a spurious
/// `NonExhaustive` during compilation.
fn elaborate_match(
    scrutinee: &TypedExpr,
    arms: &[TypedMatchArm],
    ctx: &mut Elab,
) -> Result<Term, ElabError> {
    for arm in arms {
        validate_pattern(&arm.pattern)?;
    }
    let u = ctx.fresh_var();
    let scrutinee_term = elaborate(scrutinee, ctx)?;
    let rows: Vec<Row> = arms
        .iter()
        .map(|arm| Row {
            cols: vec![(arm.pattern.clone(), u.clone())],
            binds: Vec::new(),
            body: arm.body.clone(),
        })
        .collect();
    let compiled = compile_matrix(rows, ctx)?;
    Ok(Term::Let(u, Box::new(scrutinee_term), Box::new(compiled)))
}

// Match compiler: backtracking-free, column-wise specialisation (Wadler 1987
// ch. 5 with default handling; cf. Maranget 2008). A row carries its
// outstanding (pattern, scrutinee-var) columns, pending variable bindings to
// emit as Core `Let`s around its body, and the typed body itself. Fresh
// scrutinee vars come from `Elab::fresh_var` and are globally unique.

/// One row of the pattern matrix.
#[derive(Clone)]
struct Row {
    /// Outstanding (pattern, scrutinee-var) obligations, left to right.
    cols: Vec<(Pattern, String)>,
    /// Pending `(user_var, core_var)` renames, emitted as Core `Let`s when the
    /// body is finally elaborated.
    binds: Vec<(String, String)>,
    /// The typed arm body.
    body: TypedExpr,
}

/// True if the first column of `row` is a variable or wildcard pattern.
fn first_is_var(row: &Row) -> bool {
    matches!(
        row.cols.first().map(|(p, _)| p),
        Some(Pattern::Var(_) | Pattern::Wildcard)
    )
}

/// Elaborates a matched row's body, wrapping it in the Core `Let`s that bind
/// each pending user variable to its scrutinee variable.
fn finish_row(row: &Row, ctx: &mut Elab) -> Result<Term, ElabError> {
    let mut term = elaborate(&row.body, ctx)?;
    // Innermost binding first: fold in reverse so earlier binds are outermost.
    for (user, core) in row.binds.iter().rev() {
        term = Term::Let(
            user.clone(),
            Box::new(Term::Var(core.clone())),
            Box::new(term),
        );
    }
    Ok(term)
}

/// Compiles a pattern matrix to a Core `Term`.
///
/// - Empty matrix -> `NonExhaustive`.
/// - If the FIRST row's columns are all var/wildcard (including zero columns),
///   that row matches (first-match-wins); later rows are dropped.
/// - Otherwise column 0 drives the step: the variable rule when every row has
///   a var/wildcard there, else the constructor rule over the full family.
fn compile_matrix(rows: Vec<Row>, ctx: &mut Elab) -> Result<Term, ElabError> {
    let Some(first) = rows.first() else {
        return Err(ElabError::NonExhaustive(
            "match compiler reached an empty matrix".to_string(),
        ));
    };
    if first
        .cols
        .iter()
        .all(|(p, _)| matches!(p, Pattern::Var(_) | Pattern::Wildcard))
    {
        // The first row matches. Record its column variable binds, then finish.
        let mut row = first.clone();
        drain_var_cols(&mut row);
        return finish_row(&row, ctx);
    }
    if rows.iter().all(first_is_var) {
        return compile_var_rule(rows, ctx);
    }
    compile_con_rule(&rows, ctx)
}

/// Peels every leading var/wildcard column off `row`, recording `Var` binds.
fn drain_var_cols(row: &mut Row) {
    while let Some((Pattern::Var(_) | Pattern::Wildcard, _)) = row.cols.first() {
        let (pat, u) = row.cols.remove(0);
        if let Pattern::Var(name) = pat {
            row.binds.push((name, u));
        }
    }
}

/// Wadler variable rule: every row has a var/wildcard in column 0. Drop that
/// column from each row (recording `Var` binds) and recurse.
fn compile_var_rule(rows: Vec<Row>, ctx: &mut Elab) -> Result<Term, ElabError> {
    let mut next = Vec::with_capacity(rows.len());
    for row in rows {
        let mut row = row;
        let (pat, u) = row.cols.remove(0);
        if let Pattern::Var(name) = pat {
            row.binds.push((name, u));
        }
        next.push(row);
    }
    compile_matrix(next, ctx)
}

/// Wadler constructor rule: build a `Case` over the WHOLE constructor family of
/// column 0's type. For each constructor `c` of arity `k` with fresh binders
/// `us`: a row with `Constructor(c, subs)` contributes `subs ++ rest`; a row
/// with a different constructor is dropped; a row with a var/wildcard in column
/// 0 is KEPT, with `k` wildcards prepended to `rest` and (for `Var`) its bind
/// recorded. Because default rows are kept in every alt, arm bodies are
/// duplicated across alts (FINDING F2: the real elaborator needs join points).
fn compile_con_rule(rows: &[Row], ctx: &mut Elab) -> Result<Term, ElabError> {
    let u = rows
        .iter()
        .find_map(|r| r.cols.first().map(|(_, u)| u.clone()))
        .expect("non-empty matrix with a column");
    let family = con_family(rows, &ctx.table)?;
    let mut alts = Vec::with_capacity(family.len());
    for con_name in &family {
        let arity = ctx
            .table
            .get(con_name)
            .map(|info| info.arity)
            .ok_or_else(|| ElabError::Unsupported(format!("unknown ctor: {con_name}")))?;
        let fresh_vars: Vec<String> = (0..arity).map(|_| ctx.fresh_var()).collect();
        let specialised = specialise(rows, con_name, &fresh_vars);
        let body = compile_matrix(specialised, ctx)?;
        alts.push(Alt {
            con: con_name.clone(),
            binders: fresh_vars,
            body,
        });
    }
    Ok(Term::Case(Box::new(Term::Var(u)), alts))
}

/// Finds the full constructor family named by the first constructor pattern in
/// column 0. A constructor not in the table (unregistered) is `Unsupported`.
fn con_family(rows: &[Row], table: &CtorTable) -> Result<Vec<String>, ElabError> {
    for row in rows {
        if let Some((Pattern::Constructor(name, _), _)) = row.cols.first() {
            return table
                .get(name)
                .map(|info| info.family.clone())
                .ok_or_else(|| ElabError::Unsupported(format!("unknown ctor: {name}")));
        }
    }
    Err(ElabError::NonExhaustive(
        "no constructor column in matrix".to_string(),
    ))
}

/// Specialises the matrix to constructor `con_name`. Rows with `con_name` in
/// column 0 expand their sub-patterns into new leading columns bound to
/// `fresh_vars`; rows with a different constructor are dropped; rows with a
/// var/wildcard in column 0 are kept with `fresh_vars.len()` wildcard columns
/// prepended (and, for `Var`, the bind recorded).
fn specialise(rows: &[Row], con_name: &str, fresh_vars: &[String]) -> Vec<Row> {
    let mut out = Vec::new();
    for row in rows {
        let Some((head, u)) = row.cols.first() else {
            continue;
        };
        match head {
            Pattern::Constructor(name, subs) if name == con_name => {
                let mut cols: Vec<(Pattern, String)> = subs
                    .iter()
                    .cloned()
                    .zip(fresh_vars.iter().cloned())
                    .collect();
                cols.extend_from_slice(&row.cols[1..]);
                out.push(Row {
                    cols,
                    binds: row.binds.clone(),
                    body: row.body.clone(),
                });
            }
            Pattern::Var(_) | Pattern::Wildcard => {
                let mut binds = row.binds.clone();
                if let Pattern::Var(name) = head {
                    binds.push((name.clone(), u.clone()));
                }
                let mut cols: Vec<(Pattern, String)> = fresh_vars
                    .iter()
                    .map(|fv| (Pattern::Wildcard, fv.clone()))
                    .collect();
                cols.extend_from_slice(&row.cols[1..]);
                out.push(Row {
                    cols,
                    binds,
                    body: row.body.clone(),
                });
            }
            // A different constructor, or a pattern excluded by up-front
            // validation (literal/tuple/record): the row is dropped.
            Pattern::Constructor(_, _)
            | Pattern::Literal(_)
            | Pattern::Tuple(_)
            | Pattern::Record(_) => {}
        }
    }
    out
}

// ------------------------------------------------------------------------
// Tests: helpers, hard-reject invariants, and a match-free differential.
// ------------------------------------------------------------------------

/// Parses, typechecks, and elaborates a source program.
fn elaborate_source(src: &str) -> Result<Term, ElabError> {
    let program = parse_program(src).expect("parse should succeed");
    let typed = typecheck_program_typed(&program).expect("typecheck should succeed");
    elaborate_program(&typed)
}

/// Surface-evaluates a source program to its integer result. The worker
/// closure returns a `Send` `i64` payload (never a non-`Send` `Value`), as
/// `run_on_evaluator_stack` requires.
fn surface_int(src: &str) -> i64 {
    let src = src.to_string();
    run_on_evaluator_stack(move || -> Result<i64, String> {
        let program = parse_program(&src)?;
        typecheck_program(&program).map_err(|e| e.to_string())?;
        let value = eval_program(&program, &SurfaceEnv::new()).map_err(|e| e.to_string())?;
        match value {
            parlang::Value::Int(n) => Ok(n),
            other => Err(format!("expected surface Int, got {other:?}")),
        }
    })
    .expect("evaluator stack")
    .expect("surface eval")
}

/// Core-evaluates an elaborated term to its integer result.
fn core_int(term: &Term) -> i64 {
    match eval(term, &CoreEnv::new()).expect("core eval") {
        CoreValue::Int(n) => n,
        other => panic!("expected core Int, got {other:?}"),
    }
}

#[test]
fn match_free_program_differential() {
    let src = "let x = 3 in let y = 4 in x * y + 1";
    let term = elaborate_source(src).expect("elaborate");
    assert_eq!(core_int(&term), surface_int(src));
    assert_eq!(core_int(&term), 13);
}

/// Asserts that `err` is `ElabError::Unsupported` and returns its message.
/// Reading the message keeps both variants' payloads live for `dead_code`
/// and lets each test check WHY the probe rejected.
fn unsupported_msg(err: ElabError) -> String {
    match err {
        ElabError::Unsupported(msg) => msg,
        ElabError::NonExhaustive(msg) => panic!("expected Unsupported, got NonExhaustive: {msg}"),
    }
}

#[test]
fn rejects_float_literal() {
    let msg = unsupported_msg(elaborate_source("1.5").unwrap_err());
    assert!(msg.contains("Float"), "unexpected reason: {msg}");
}

#[test]
fn rejects_unsupported_binop() {
    let msg = unsupported_msg(elaborate_source("1 <= 2").unwrap_err());
    assert!(msg.contains("Le"), "unexpected reason: {msg}");
}

#[test]
fn rejects_tuple() {
    let msg = unsupported_msg(elaborate_source("(1, 2)").unwrap_err());
    assert!(msg.contains("Tuple"), "unexpected reason: {msg}");
}

// ------------------------------------------------------------------------
// Tests: match-compilation differentials, shape, stress, and hard-rejects.
// ------------------------------------------------------------------------

/// Reads an example file relative to this crate's manifest.
fn example_src(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(path).expect("read example")
}

/// Asserts the elaborated Core result equals both the surface result and the
/// expected literal for an example file.
fn assert_example(rel: &str, expected: i64) {
    let src = example_src(rel);
    let term = elaborate_source(&src).expect("elaborate example");
    assert_eq!(core_int(&term), surface_int(&src), "core vs surface: {rel}");
    assert_eq!(core_int(&term), expected, "literal: {rel}");
}

#[test]
fn example_exhaustiveness_differential() {
    assert_example("examples/exhaustiveness.par", 42);
}

#[test]
fn example_sum_types_differential() {
    assert_example("examples/sum_types.par", 6);
}

#[test]
fn example_generic_tree_differential() {
    assert_example("examples/generic_tree.par", 60);
}

#[test]
fn example_generic_result_differential() {
    assert_example("examples/generic_result.par", 20);
}

#[test]
fn case_shape_is_positional_tag_switch() {
    // The probe's question is about SHAPE, not just value: `match` must
    // elaborate to a Core `Case` over the full family with positional
    // binders (here `Cons` binds two fresh `%u` vars).
    let src = example_src("examples/sum_types.par");
    let text = elaborate_source(&src).expect("elaborate").to_string();
    assert!(
        text.contains("(case %u"),
        "no Case over a fresh var: {text}"
    );
    assert!(text.contains("Nil -> "), "no Nil alt: {text}");
    assert!(
        text.contains("| Cons(%u"),
        "no positional Cons binders: {text}"
    );
}

#[test]
fn nested_ctor_patterns_differential() {
    let src = "data List a = Nil | Cons a (List a) in\n\
               let xs = Cons 1 (Cons 2 (Cons 3 Nil)) in\n\
               let f = rec f -> fun l ->\n\
                 match l with\n\
                 | Cons x (Cons y rest) -> x + y + f rest\n\
                 | Cons x Nil -> x\n\
                 | Nil -> 0\n\
               in f xs";
    let term = elaborate_source(src).expect("elaborate nested");
    assert_eq!(core_int(&term), surface_int(src));
}

#[test]
fn overlapping_arms_first_match_wins() {
    let src = "data Option a = Some a | None in\n\
               let x = Some 7 in\n\
               match x with\n\
               | Some n -> n\n\
               | _ -> 0";
    let term = elaborate_source(src).expect("elaborate overlap");
    assert_eq!(core_int(&term), surface_int(src));
}

#[test]
fn non_variable_scrutinee_differential() {
    let src = "data Option a = Some a | None in\n\
               match Some (1 + 2) with\n\
               | Some n -> n\n\
               | None -> 0";
    let term = elaborate_source(src).expect("elaborate non-var scrutinee");
    assert_eq!(core_int(&term), surface_int(src));
}

#[test]
fn nested_match_in_arm_body_differential() {
    let src = "data Option a = Some a | None in\n\
               let outer = Some 10 in\n\
               let inner = Some 5 in\n\
               match outer with\n\
               | Some a ->\n\
                 (match inner with\n\
                  | Some b -> a + b\n\
                  | None -> a)\n\
               | None -> 0";
    let term = elaborate_source(src).expect("elaborate nested-match-in-body");
    assert_eq!(core_int(&term), surface_int(src));
}

#[test]
fn non_exhaustive_match_rejects() {
    // The surface typechecker does NOT enforce exhaustiveness (it is an
    // eval-time warning only; blocking is deferred to slice 7). So typecheck
    // succeeds and the probe must reject at compile time with NonExhaustive.
    let src = "data Option a = Some a | None in\n\
               match None with\n\
               | Some n -> n";
    let program = parse_program(src).expect("parse");
    let typed = typecheck_program_typed(&program).expect("typecheck");
    let err = elaborate_program(&typed).unwrap_err();
    assert!(matches!(err, ElabError::NonExhaustive(_)));
}

#[test]
fn literal_patterns_rejected() {
    let src = example_src("examples/pattern_matching.par");
    let msg = unsupported_msg(elaborate_source(&src).unwrap_err());
    assert!(msg.contains("literal pattern"), "unexpected reason: {msg}");
}
