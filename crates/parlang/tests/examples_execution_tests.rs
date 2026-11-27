use parlang::{
    eval_program, parse_program, run_on_evaluator_stack, typecheck_program,
    typecheck_program_typed, BinOp, Environment, Type, TypedDecl, TypedExpr, TypedExprKind, Value,
};
use parlang_core::Value as CoreValue;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

fn example_source(relative_path: &str) -> Result<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let mut source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    if relative_path == "examples/use_strings.par" {
        let string_library = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/string.par")
            .to_string_lossy()
            .into_owned();
        source = source.replace("examples/string.par", &string_library);
    }
    Ok(source)
}

fn run_example(relative_path: &str) -> Result<String, String> {
    let source = example_source(relative_path)?;
    run_on_evaluator_stack(move || -> Result<String, String> {
        let program = parse_program(&source)?;
        typecheck_program(&program).map_err(|error| error.to_string())?;
        eval_program(&program, &Environment::new())
            .map(|value| format!("{value:?}"))
            .map_err(|error| error.to_string())
    })
    .map_err(|error| error.to_string())?
}

/// Recursively collect every `.par` example as a manifest-relative path,
/// sorted for determinism. Shared by the execution and typed-AST harnesses.
fn example_rels() -> Vec<String> {
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read_dir examples") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                collect(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("par") {
                out.push(path);
            }
        }
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let examples_dir = root.join("examples");
    let mut files = Vec::new();
    collect(&examples_dir, &mut files);
    files.sort();

    files
        .iter()
        .map(|path| {
            let rel = path.strip_prefix(root).expect("strip manifest prefix");
            rel.to_string_lossy().replace('\\', "/")
        })
        .collect()
}

/// Every typed child of `node`, in source order.
fn typed_children(node: &TypedExpr) -> Vec<&TypedExpr> {
    use TypedExprKind as K;
    match &node.kind {
        K::Int(_) | K::Bool(_) | K::Char(_) | K::Float(_) | K::Var(..) | K::Load(_, _) => vec![],
        K::BinOp(_, l, r) | K::App(l, r) | K::Let(_, _, _, l, r) => vec![&**l, &**r],
        K::If(c, t, e) => vec![&**c, &**t, &**e],
        K::Fun(_, _, b)
        | K::Rec(_, b)
        | K::TupleProj(b, _)
        | K::TypeAlias(_, _, b)
        | K::FieldAccess(b, _)
        | K::TypeDef { body: b, .. } => vec![&**b],
        K::Match(s, arms) => std::iter::once(&**s)
            .chain(arms.iter().map(|arm| &arm.body))
            .collect(),
        K::Tuple(elems) | K::Constructor(_, elems) => elems.iter().collect(),
        K::Record(fields) => fields.iter().map(|(_, e)| e).collect(),
    }
}

/// Check the local typing rule of a single fully-resolved node (children are
/// checked separately by `typed_rule_ok`). Only rules that genuinely hold on
/// resolved types are asserted.
fn node_rule_ok(node: &TypedExpr) -> Result<(), String> {
    use TypedExprKind as K;
    let ok = match &node.kind {
        K::BinOp(BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div, l, r) => {
            node.ty == l.ty && node.ty == r.ty
        }
        K::BinOp(..) => node.ty == Type::Bool,
        K::If(c, t, e) => c.ty == Type::Bool && t.ty == node.ty && e.ty == node.ty,
        K::Let(.., b) | K::Rec(_, b) => node.ty == b.ty,
        K::Fun(_, _, b) => matches!(&node.ty, Type::Fun(_, bt) if **bt == b.ty),
        K::App(f, a) => f.ty == Type::Fun(Box::new(a.ty.clone()), Box::new(node.ty.clone())),
        K::Match(s, arms) => arms
            .iter()
            .all(|arm| arm.body.ty == node.ty && arm.pattern.ty == s.ty),
        K::Tuple(elems) => node.ty == Type::Tuple(elems.iter().map(|e| e.ty.clone()).collect()),
        K::TupleProj(t, i) => {
            !matches!(&t.ty, Type::Tuple(elems) if elems.get(*i).is_some_and(|e| *e != node.ty))
        }
        K::Record(fields) => {
            node.ty
                == Type::Record(
                    fields
                        .iter()
                        .map(|(n, e)| (n.clone(), e.ty.clone()))
                        .collect(),
                )
        }
        K::Int(_)
        | K::Bool(_)
        | K::Char(_)
        | K::Float(_)
        | K::Var(..)
        | K::Load(_, _)
        | K::TypeAlias(..)
        | K::FieldAccess(..)
        | K::TypeDef { .. }
        | K::Constructor(..) => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "typed rule violated: ty={:?} kind={:?}",
            node.ty, node.kind
        ))
    }
}

/// Recursively verify a fully-resolved typed node against the inference rules
/// its type must satisfy. Recurses into every typed child before checking the
/// node itself. Returns `Err(description)` on the first violation.
fn typed_rule_ok(node: &TypedExpr) -> Result<(), String> {
    for child in typed_children(node) {
        typed_rule_ok(child)?;
    }
    node_rule_ok(node)
}

#[test]
fn every_example_typed_ast_is_consistent() {
    let rels = example_rels();

    let mut failures = Vec::new();
    for rel in &rels {
        let source = match example_source(rel) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{rel}: {e}"));
                continue;
            }
        };
        let outcome = run_on_evaluator_stack(move || -> Result<(), String> {
            let program = parse_program(&source)?;
            let plain = typecheck_program(&program).map_err(|e| e.to_string())?;
            let typed = typecheck_program_typed(&program).map_err(|e| e.to_string())?;
            if typed.ty != plain {
                return Err(format!(
                    "program ty mismatch: {:?} vs {:?}",
                    typed.ty, plain
                ));
            }
            for decl in &typed.decls {
                if let TypedDecl::Let { value, .. } = decl {
                    typed_rule_ok(value)?;
                }
            }
            if let Some(body) = &typed.body {
                typed_rule_ok(body)?;
            }
            Ok(())
        })
        .map_err(|e| e.to_string());
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(e)) | Err(e) => failures.push(format!("{rel}: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "typed AST inconsistencies:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_example_typed_dump_renders() {
    let rels = example_rels();

    let mut failures = Vec::new();
    for rel in &rels {
        let source = match example_source(rel) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{rel}: {e}"));
                continue;
            }
        };
        let outcome = run_on_evaluator_stack(move || -> Result<(), String> {
            let program = parse_program(&source)?;
            let typed = parlang::typecheck_program_typed(&program).map_err(|e| e.to_string())?;
            let text = parlang::typed_program_to_text(&typed);
            if !text.starts_with(&format!("Program : {}\n", typed.ty)) {
                return Err(format!("text missing header: {text}"));
            }
            if !text.ends_with('\n') {
                return Err("text missing trailing newline".to_string());
            }
            let dot = parlang::typed_program_to_dot(&typed);
            if !dot.starts_with("digraph TypedAST {\n") {
                return Err("dot missing header".to_string());
            }
            if !dot.ends_with("}\n") {
                return Err("dot missing footer".to_string());
            }
            Ok(())
        })
        .map_err(|e| e.to_string());
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(e)) | Err(e) => failures.push(format!("{rel}: {e}")),
        }
    }

    assert!(
        failures.is_empty(),
        "typed dump failed for examples:\n{}",
        failures.join("\n")
    );
}

#[test]
fn executes_stdlib_example() {
    assert_eq!(run_example("examples/stdlib.par"), Ok("Int(0)".to_string()));
}

#[test]
fn executes_use_stdlib_example() {
    assert_eq!(
        run_example("examples/use_stdlib.par"),
        Ok("Int(30)".to_string())
    );
}

#[test]
fn executes_map_example() {
    assert_eq!(
        run_example("examples/stdlib/map.par"),
        Ok("Int(146)".to_string())
    );
}

#[test]
fn executes_treemap_example() {
    assert_eq!(
        run_example("examples/stdlib/treemap.par"),
        Ok("Int(400)".to_string())
    );
}

#[test]
fn executes_use_strings_example() {
    // The trailing value is a char-list representation of "Hello, World!".
    assert!(run_example("examples/use_strings.par").is_ok());
}

#[test]
fn executes_use_recursion_example() {
    assert!(run_example("examples/use_recursion.par").is_ok());
}

/// The surface value of a string literal: a `Cons`/`Nil` list of chars.
fn surface_string(text: &str) -> Value {
    text.chars()
        .rev()
        .fold(Value::Variant("Nil".to_string(), vec![]), |tail, ch| {
            Value::Variant("Cons".to_string(), vec![Value::Char(ch), tail])
        })
}

/// Expected surface value of `examples/records.par`.
fn expected_records() -> Value {
    Value::Record(HashMap::from([
        ("dist".to_string(), Value::Int(7)),
        ("first".to_string(), Value::Int(4)),
        ("label".to_string(), surface_string("ada")),
        ("packed".to_string(), Value::Int(43)),
        ("size".to_string(), Value::Int(39)),
    ]))
}

/// Expected surface value of `examples/mvp.par`.
fn expected_mvp() -> Value {
    surface_string("aglnpr")
}

/// Run an example on the surface evaluator and compare its final value with
/// `expected()`. The surface `Value` is not `Send`, so the expected value is
/// built and compared inside the worker and only `String`s cross the thread
/// boundary.
fn example_value_matches(relative_path: &str, expected: fn() -> Value) -> Result<(), String> {
    let source = example_source(relative_path)?;
    run_on_evaluator_stack(move || -> Result<(), String> {
        let program = parse_program(&source)?;
        typecheck_program(&program).map_err(|error| error.to_string())?;
        let actual =
            eval_program(&program, &Environment::new()).map_err(|error| error.to_string())?;
        let expected = expected();
        if actual == expected {
            Ok(())
        } else {
            Err(format!("{actual:?} != {expected:?}"))
        }
    })
    .map_err(|error| error.to_string())?
}

/// Run an example through the Core pipeline and return the rendered result.
fn example_core_output(relative_path: &str) -> Result<String, String> {
    let source = example_source(relative_path)?;
    run_on_evaluator_stack(move || -> Result<String, String> {
        let program = parse_program(&source)?;
        parlang::pipeline::run_core(&program)
    })
    .map_err(|error| error.to_string())?
}

#[test]
fn executes_records_example() {
    assert_eq!(
        example_value_matches("examples/records.par", expected_records),
        Ok(())
    );
}

#[test]
fn records_example_runs_on_core() {
    assert_eq!(
        example_core_output("examples/records.par"),
        Ok("{dist: 7, first: 4, label: \"ada\", packed: 43, size: 39}\n".to_string())
    );
}

#[test]
fn executes_mvp_example() {
    assert_eq!(
        example_value_matches("examples/mvp.par", expected_mvp),
        Ok(())
    );
}

#[test]
fn mvp_example_runs_on_core() {
    assert_eq!(
        example_core_output("examples/mvp.par"),
        Ok("\"aglnpr\"\n".to_string())
    );
}

#[test]
fn hkt_example_runs_on_core() {
    assert_eq!(
        example_core_output("examples/hkt.par"),
        Ok("Some(1)\n".to_string())
    );
}

#[test]
fn every_example_typechecks_and_runs() {
    let rels = example_rels();

    assert!(
        rels.iter().any(|r| r == "examples/tuples.par"),
        "walk did not find examples/tuples.par"
    );
    assert!(
        rels.iter().any(|r| r == "examples/pattern_matching.par"),
        "walk did not find examples/pattern_matching.par"
    );

    let mut failures = Vec::new();
    for rel in &rels {
        if let Err(error) = run_example(rel) {
            failures.push(format!("{rel}: {error}"));
        }
    }

    assert!(
        failures.is_empty(),
        "examples failed to typecheck/run:\n{}",
        failures.join("\n")
    );
}

/// Examples that Core elaboration rejects on purpose, paired with a substring
/// the elaboration error must contain. Every other example must elaborate,
/// run on Core, and agree with the surface evaluator.
const EXPECTED_CORE_REJECTIONS: &[(&str, &str)] = &[
    ("examples/stdlib/map.par", "dictionary passing"),
    ("examples/string.par", "dictionary passing"),
    ("examples/use_recursion.par", "load"),
    ("examples/use_stdlib.par", "load"),
    ("examples/use_strings.par", "load"),
];

/// The Core data-type and constructor name of a tuple with `arity` elements:
/// an opening parenthesis, `arity - 1` commas, and a closing parenthesis.
fn tuple_con_name(arity: usize) -> String {
    format!("({})", ",".repeat(arity.saturating_sub(1)))
}

/// The Core data-type and constructor name of a closed record whose field
/// names are given in `sorted_keys` (already sorted).
fn record_con_name(sorted_keys: &[&str]) -> String {
    format!("{{{}}}", sorted_keys.join(","))
}

/// Pairwise structural agreement of two value sequences of equal length.
fn all_agree(surface: &[&Value], core: &[CoreValue]) -> bool {
    surface.len() == core.len()
        && surface
            .iter()
            .zip(core)
            .all(|(left, right)| values_agree(left, right))
}

/// Structural agreement between a surface `Value` and a Core `CoreValue`
/// under the slice-7 encodings: the empty tuple is Core `Unit`, tuples are
/// constructors named by `tuple_con_name`, closed records are constructors
/// named by `record_con_name` with fields in sorted-name order, and user
/// constructors keep their names. Closures agree with closures without being
/// inspected.
fn values_agree(surface: &Value, core: &CoreValue) -> bool {
    match (surface, core) {
        (Value::Int(left), CoreValue::Int(right)) => left == right,
        (Value::Bool(left), CoreValue::Bool(right)) => left == right,
        (Value::Char(left), CoreValue::Char(right)) => left == right,
        (Value::Float(left), CoreValue::Float(right)) => {
            left.total_cmp(right).is_eq() || (left.is_nan() && right.is_nan())
        }
        (Value::Tuple(items), CoreValue::Unit) => items.is_empty(),
        (Value::Tuple(items), CoreValue::Con(name, fields)) => {
            let refs: Vec<&Value> = items.iter().collect();
            items.len() >= 2 && *name == tuple_con_name(items.len()) && all_agree(&refs, fields)
        }
        (Value::Record(map), CoreValue::Con(name, fields)) => {
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_unstable();
            let refs: Vec<&Value> = keys.iter().map(|key| &map[*key]).collect();
            *name == record_con_name(&keys) && all_agree(&refs, fields)
        }
        (Value::Variant(tag, items), CoreValue::Con(name, fields)) => {
            let refs: Vec<&Value> = items.iter().collect();
            tag == name && all_agree(&refs, fields)
        }
        (
            Value::Closure(..) | Value::RecClosure(..),
            CoreValue::Closure(_) | CoreValue::RecClosure(_),
        ) => true,
        _ => false,
    }
}

/// How one example fared on the surface evaluator versus Core. Carries only
/// `String` payloads so it can cross the evaluator-stack thread boundary.
enum CoreOutcome {
    /// Elaborated, ran on Core, and agreed with the surface value.
    Agrees,
    /// Elaborated and ran on Core, but the values disagree.
    Disagrees(String),
    /// Elaboration rejected the program with this error.
    Rejected(String),
}

/// Parse, typecheck and evaluate `source` on the surface, then elaborate it
/// to Core and evaluate the Core term in the Core prelude environment. Both
/// values are compared and dropped inside the worker; only a `CoreOutcome`
/// or a `String` error is returned.
fn core_outcome(source: String) -> Result<CoreOutcome, String> {
    run_on_evaluator_stack(move || -> Result<CoreOutcome, String> {
        let program = parse_program(&source)?;
        typecheck_program(&program).map_err(|error| error.to_string())?;
        let surface =
            eval_program(&program, &Environment::new()).map_err(|error| error.to_string())?;
        let elaborated = match parlang::pipeline::core_of_program(&program) {
            Ok(elaborated) => elaborated,
            Err(error) => return Ok(CoreOutcome::Rejected(error)),
        };
        let env = parlang_core::prelude().map_err(|error| error.to_string())?;
        let core = parlang_core::eval(&elaborated.term, &env)
            .map_err(|error| format!("Core evaluation failed: {error:?}"))?;
        if values_agree(&surface, &core) {
            Ok(CoreOutcome::Agrees)
        } else {
            Ok(CoreOutcome::Disagrees(format!(
                "surface {surface:?} vs Core {core:?}"
            )))
        }
    })
    .map_err(|error| error.to_string())?
}

/// Classify one example's outcome against `EXPECTED_CORE_REJECTIONS`,
/// returning a failure description when it violates the oracle.
fn core_failure(rel: &str, outcome: Result<CoreOutcome, String>) -> Option<String> {
    let expected = EXPECTED_CORE_REJECTIONS
        .iter()
        .find(|(path, _)| *path == rel)
        .map(|(_, needle)| *needle);
    match (outcome, expected) {
        (Err(error), _) => Some(format!("{rel}: {error}")),
        (Ok(CoreOutcome::Agrees), None) => None,
        (Ok(CoreOutcome::Agrees), Some(_)) => Some(format!(
            "{rel}: listed in EXPECTED_CORE_REJECTIONS but elaborated and agreed (stale entry)"
        )),
        (Ok(CoreOutcome::Disagrees(detail)), _) => {
            Some(format!("{rel}: surface and Core disagree: {detail}"))
        }
        (Ok(CoreOutcome::Rejected(error)), None) => {
            Some(format!("{rel}: unexpected elaboration error: {error}"))
        }
        (Ok(CoreOutcome::Rejected(error)), Some(needle)) => (!error.contains(needle))
            .then(|| format!("{rel}: elaboration error lacks {needle:?}: {error}")),
    }
}

#[test]
fn every_example_agrees_on_core() {
    let rels = example_rels();

    let mut failures: Vec<String> = EXPECTED_CORE_REJECTIONS
        .iter()
        .filter(|(path, _)| !rels.iter().any(|rel| rel.as_str() == *path))
        .map(|(path, _)| format!("{path}: listed in EXPECTED_CORE_REJECTIONS but not found"))
        .collect();
    for rel in &rels {
        let outcome = example_source(rel).and_then(core_outcome);
        if let Some(failure) = core_failure(rel, outcome) {
            failures.push(failure);
        }
    }

    assert!(
        failures.is_empty(),
        "surface/Core differential failures:\n{}",
        failures.join("\n")
    );
}
