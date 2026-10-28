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
use parlang_core::ty::Type as CoreType;
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

/// Translates a FULL surface `Type` (as inferred by the surface typechecker)
/// into a Core `CoreType`, hard-rejecting (via `None`) any construct outside
/// the System-F-shaped Core slice rather than silently falling back. This is
/// the slice-6b pull on the Core `Type` enum: it is NOT the real elaborator and
/// does NOT touch `Term::Lam` or `base_type_of` (that is slice 6c).
///
/// Mappings:
/// - `Int`/`Bool`/`Char`/`Float`/`Unit` -> the matching `CoreType::Base` leaf.
/// - `Fun(a, b)` -> `CoreType::Fun`.
/// - `Var(TypeVar(n))` -> a named `CoreType::Var` spelled `tN`, mirroring the
///   surface's own `tN` rendering. The `format!("t{}", v.0)` positional form is
///   deliberate: `v.0` is a tuple-struct field access on `TypeVar`.
/// - `SumType(name, args)` -> left-associative `CoreType::App` of the named
///   `CoreType::Con` over the translated args (so `List (Option a)` is NOT
///   lossy, unlike the surface Display).
/// - `Record(..)` and `Tuple(..)` -> hard-reject (`None`): Core has no row type
///   yet, and a tuple needs an arity-tagged constructor (deferred to slice 7),
///   which would also collide with user `data Tuple ...` declarations.
fn core_type_of(ty: &Type) -> Option<CoreType> {
    match ty {
        Type::Int => Some(CoreType::Base(BaseType::Int)),
        Type::Bool => Some(CoreType::Base(BaseType::Bool)),
        Type::Char => Some(CoreType::Base(BaseType::Char)),
        Type::Float => Some(CoreType::Base(BaseType::Float)),
        Type::Unit => Some(CoreType::Base(BaseType::Unit)),
        Type::Fun(dom, cod) => {
            let d = core_type_of(dom)?;
            let c = core_type_of(cod)?;
            Some(CoreType::Fun(Box::new(d), Box::new(c)))
        }
        // Positional field access: `v.0` is the inner `usize` of `TypeVar`.
        Type::Var(v) => Some(CoreType::Var(format!("t{}", v.0))),
        Type::SumType(name, args) => {
            let mut head = CoreType::Con(name.clone());
            for arg in args {
                let a = core_type_of(arg)?;
                head = CoreType::App(Box::new(head), Box::new(a));
            }
            Some(head)
        }
        Type::Record(_) | Type::Tuple(_) => None,
    }
}

#[test]
fn core_type_of_translates_base_and_arrow() {
    // Int -> Bool
    let ty = Type::Fun(Box::new(Type::Int), Box::new(Type::Bool));
    let core = core_type_of(&ty).expect("Int -> Bool should translate");
    assert_eq!(core.to_string(), "int -> bool", "rendering: {core}");
}

#[test]
fn core_type_of_translates_sum_type_without_loss() {
    // List (Option t0)  -- the surface Display would lose the nesting.
    let option_a = Type::SumType("Option".to_string(), vec![Type::Var(parlang::TypeVar(0))]);
    let list = Type::SumType("List".to_string(), vec![option_a]);
    let core = core_type_of(&list).expect("nested sum type should translate");
    assert_eq!(core.to_string(), "List (Option t0)", "rendering: {core}");
}

#[test]
fn core_type_of_rejects_record() {
    let mut fields = std::collections::HashMap::new();
    fields.insert("x".to_string(), Type::Int);
    assert!(
        core_type_of(&Type::Record(fields)).is_none(),
        "records must hard-reject"
    );
}

#[test]
fn core_type_of_rejects_tuple() {
    let tuple = Type::Tuple(vec![Type::Int, Type::Bool]);
    assert!(
        core_type_of(&tuple).is_none(),
        "tuples must hard-reject (arity-tagged ctor deferred to slice 7)"
    );
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

/// Infers the surface type of a whole program's trailing body, then translates
/// it to a Core `CoreType` and renders it. Returns `None` when the body is
/// absent or the inferred type falls outside the Core slice.
fn core_type_render(src: &str) -> Option<String> {
    let program = parse_program(src).expect("parse should succeed");
    let typed = typecheck_program_typed(&program).expect("typecheck should succeed");
    let body = typed.body.as_ref()?;
    core_type_of(&body.ty).map(|t| t.to_string())
}

#[test]
fn inferred_monomorphic_body_type_translates() {
    // A plain arithmetic body infers to int.
    let rendered = core_type_render("let x = 3 in x * 4").expect("body type should translate");
    assert_eq!(rendered, "int", "unexpected rendering: {rendered}");
}

#[test]
fn inferred_function_body_type_translates() {
    // An increment lambda body infers to a Core arrow over base leaves.
    let rendered = core_type_render("fun x -> x + 1").expect("body type should translate");
    assert_eq!(rendered, "int -> int", "unexpected rendering: {rendered}");
}

#[test]
fn inferred_sum_type_body_translates_without_loss() {
    // A List Int value body translates to a non-lossy Core application.
    let src = "data List a = Nil | Cons a (List a) in Cons 1 (Cons 2 Nil)";
    let rendered = core_type_render(src).expect("body type should translate");
    assert_eq!(rendered, "List int", "unexpected rendering: {rendered}");
}

/// Translates a generalized surface `TypeScheme` into a Core `CoreType`,
/// wrapping the translated body in nested `CoreType::Forall`, outermost binder
/// first. Each binder is spelled IDENTICALLY to its occurrence spelling
/// (`format!("t{}", v.0)`), so the quantifier binds the same name the body
/// mentions. Hard-rejects (`None`) if any typeclass constraints are present
/// (Core has no classes yet) or the body is untranslatable.
fn core_scheme_of(scheme: &parlang::TypeScheme) -> Option<CoreType> {
    if !scheme.constraints.is_empty() {
        return None;
    }
    let mut body = core_type_of(&scheme.ty)?;
    // Positional field access: `v.0` is the inner `usize` of `TypeVar`.
    for v in scheme.vars.iter().rev() {
        body = CoreType::Forall(format!("t{}", v.0), Box::new(body));
    }
    Some(body)
}

/// Finds a top-level `let` decl by name in a typechecked program and returns a
/// clone of its generalized scheme.
fn decl_scheme(src: &str, want: &str) -> parlang::TypeScheme {
    let program = parse_program(src).expect("parse should succeed");
    let typed = typecheck_program_typed(&program).expect("typecheck should succeed");
    for decl in &typed.decls {
        if let TypedDecl::Let { name, scheme, .. } = decl {
            if name == want {
                return scheme.clone();
            }
        }
    }
    panic!("no let decl named {want}");
}

#[test]
fn generalized_scheme_translates_to_forall() {
    // `id` generalizes to a single-variable scheme; its Core type is a single
    // Forall, closed, over an arrow whose domain and codomain are the same var.
    let scheme = decl_scheme("let id = fun x -> x;\nid 3", "id");
    assert_eq!(scheme.vars.len(), 1, "id should have one quantified var");
    let core = core_scheme_of(&scheme).expect("id scheme should translate");
    assert!(
        matches!(core, CoreType::Forall(_, _)),
        "expected a Forall, got: {core}"
    );
    assert!(
        core.free_vars().is_empty(),
        "a generalized scheme must be closed, got free vars in: {core}"
    );
    // Build the expected rendering FROM the scheme's own var number; the fresh
    // counter is not stable across runs.
    let v = scheme.vars[0].0;
    let expected = format!("forall t{v}. t{v} -> t{v}");
    assert_eq!(core.to_string(), expected, "unexpected rendering");
}

#[test]
fn open_scheme_body_has_one_free_var() {
    // The UN-generalized body type of `id` is the open `t{v} -> t{v}`, whose
    // free type variables are exactly {t{v}}.
    let scheme = decl_scheme("let id = fun x -> x;\nid 3", "id");
    let v = scheme.vars[0].0;
    let body = core_type_of(&scheme.ty).expect("id body type should translate");
    let fv: Vec<String> = body.free_vars().into_iter().collect();
    assert_eq!(fv, vec![format!("t{v}")], "unexpected free vars in: {body}");
}

#[test]
fn two_variable_scheme_translates_to_nested_foralls() {
    // `konst a b = a` generalizes to a two-variable scheme -> two nested
    // Foralls, closed. (`const` is a reserved-ish name to avoid; use `konst`.)
    let scheme = decl_scheme("let konst = fun a -> fun b -> a;\nkonst 1 2", "konst");
    assert_eq!(
        scheme.vars.len(),
        2,
        "konst should have two quantified vars"
    );
    let core = core_scheme_of(&scheme).expect("konst scheme should translate");
    assert!(
        core.free_vars().is_empty(),
        "a generalized scheme must be closed, got: {core}"
    );
    // Outermost binder first == scheme.vars[0]; body is `t0 -> t1 -> t0`.
    let v0 = scheme.vars[0].0;
    let v1 = scheme.vars[1].0;
    let expected = format!("forall t{v0}. forall t{v1}. t{v0} -> t{v1} -> t{v0}");
    assert_eq!(core.to_string(), expected, "unexpected rendering");
}

#[test]
fn scheme_with_constraints_hard_rejects() {
    // Build a scheme carrying a (reserved, currently-unused) constraint and
    // confirm the translation hard-rejects, since Core has no classes yet.
    let scheme = parlang::TypeScheme {
        vars: vec![parlang::TypeVar(0)],
        ty: Type::Var(parlang::TypeVar(0)),
        constraints: vec![parlang::Constraint {
            class: "Eq".to_string(),
            args: vec![Type::Var(parlang::TypeVar(0))],
        }],
    };
    assert!(
        core_scheme_of(&scheme).is_none(),
        "constrained schemes must hard-reject"
    );
}

#[test]
fn stdlib_decl_schemes_all_translate_closed() {
    // Corpus sweep: every top-level `let` decl scheme in stdlib.par (which
    // includes polymorphic `id` and `compose`) must translate to a closed Core
    // type. The file has no record or tuple types in any decl scheme.
    let src = example_src("examples/stdlib.par");
    let program = parse_program(&src).expect("parse stdlib");
    let typed = typecheck_program_typed(&program).expect("typecheck stdlib");
    let mut seen = 0_usize;
    for decl in &typed.decls {
        if let TypedDecl::Let { name, scheme, .. } = decl {
            let core = core_scheme_of(scheme)
                .unwrap_or_else(|| panic!("decl {name} scheme should translate"));
            assert!(
                core.free_vars().is_empty(),
                "decl {name} scheme must be closed, got: {core}"
            );
            seen += 1;
        }
    }
    assert!(seen >= 8, "expected stdlib.par let decls, saw {seen}");
}
