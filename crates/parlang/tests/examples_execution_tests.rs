use parlang::{
    eval_program, parse_program, run_on_evaluator_stack, typecheck_program,
    typecheck_program_typed, BinOp, Environment, Type, TypedDecl, TypedExpr, TypedExprKind,
};
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
        K::Int(_) | K::Bool(_) | K::Char(_) | K::Float(_) | K::Var(_) | K::Load(_, _) => vec![],
        K::BinOp(_, l, r) | K::App(l, r) | K::Let(_, _, l, r) => vec![&**l, &**r],
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
        K::Let(_, _, _, b) | K::Rec(_, b) => node.ty == b.ty,
        K::Fun(_, _, b) => matches!(&node.ty, Type::Fun(_, bt) if **bt == b.ty),
        K::App(f, a) => f.ty == Type::Fun(Box::new(a.ty.clone()), Box::new(node.ty.clone())),
        K::Match(_, arms) => arms.iter().all(|arm| arm.body.ty == node.ty),
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
        | K::Var(_)
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
