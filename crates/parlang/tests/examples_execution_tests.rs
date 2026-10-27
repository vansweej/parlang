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

/// Recursively verify a fully-resolved typed node against the inference rules
/// its type must satisfy. Recurses into every typed child before checking the
/// node itself. Returns `Err(description)` on the first violation.
fn typed_rule_ok(node: &TypedExpr) -> Result<(), String> {
    use TypedExprKind as K;
    match &node.kind {
        K::Int(_)
        | K::Bool(_)
        | K::Char(_)
        | K::Float(_)
        | K::Var(_)
        | K::Pending(_)
        | K::Load(_, _) => Ok(()),
        K::BinOp(op, l, r) => {
            typed_rule_ok(l)?;
            typed_rule_ok(r)?;
            match op {
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                    if node.ty != l.ty || node.ty != r.ty {
                        return Err(format!(
                            "arith binop ty mismatch: node={:?} l={:?} r={:?}",
                            node.ty, l.ty, r.ty
                        ));
                    }
                    Ok(())
                }
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Neq => {
                    if node.ty != Type::Bool {
                        return Err(format!("comparison binop ty not Bool: {:?}", node.ty));
                    }
                    Ok(())
                }
            }
        }
        K::If(c, t, e) => {
            typed_rule_ok(c)?;
            typed_rule_ok(t)?;
            typed_rule_ok(e)?;
            if c.ty != Type::Bool {
                return Err(format!("if cond not Bool: {:?}", c.ty));
            }
            if t.ty != node.ty || e.ty != node.ty {
                return Err(format!(
                    "if branch ty mismatch: node={:?} t={:?} e={:?}",
                    node.ty, t.ty, e.ty
                ));
            }
            Ok(())
        }
        K::Let(_, _, v, b) => {
            typed_rule_ok(v)?;
            typed_rule_ok(b)?;
            if node.ty != b.ty {
                return Err(format!("let ty != body ty: {:?} vs {:?}", node.ty, b.ty));
            }
            Ok(())
        }
        K::Fun(_, _, b) => {
            typed_rule_ok(b)?;
            match &node.ty {
                Type::Fun(_, bt) if **bt == b.ty => Ok(()),
                other => Err(format!("fun ty not Fun(_, body ty): {other:?}")),
            }
        }
        K::App(f, a) => {
            typed_rule_ok(f)?;
            typed_rule_ok(a)?;
            let expected = Type::Fun(Box::new(a.ty.clone()), Box::new(node.ty.clone()));
            if f.ty != expected {
                return Err(format!(
                    "app func ty mismatch: func={:?} expected={expected:?}",
                    f.ty
                ));
            }
            Ok(())
        }
        K::Rec(_, b) => {
            typed_rule_ok(b)?;
            if node.ty != b.ty {
                return Err(format!("rec ty != body ty: {:?} vs {:?}", node.ty, b.ty));
            }
            Ok(())
        }
        K::Match(s, arms) => {
            typed_rule_ok(s)?;
            for arm in arms {
                typed_rule_ok(&arm.body)?;
                if arm.body.ty != node.ty {
                    return Err(format!(
                        "match arm ty mismatch: node={:?} arm={:?}",
                        node.ty, arm.body.ty
                    ));
                }
            }
            Ok(())
        }
        K::Tuple(elems) => {
            for e in elems {
                typed_rule_ok(e)?;
            }
            let expected = Type::Tuple(elems.iter().map(|e| e.ty.clone()).collect());
            if node.ty != expected {
                return Err(format!(
                    "tuple ty mismatch: node={:?} expected={expected:?}",
                    node.ty
                ));
            }
            Ok(())
        }
        K::TupleProj(t, i) => {
            typed_rule_ok(t)?;
            if let Type::Tuple(elems) = &t.ty {
                if let Some(elem_ty) = elems.get(*i) {
                    if *elem_ty != node.ty {
                        return Err(format!(
                            "tuple-proj ty mismatch: elem={elem_ty:?} node={:?}",
                            node.ty
                        ));
                    }
                }
            }
            Ok(())
        }
        K::TypeAlias(_, _, b) | K::FieldAccess(b, _) | K::TypeDef { body: b, .. } => {
            typed_rule_ok(b)
        }
        K::Record(fields) => {
            for (_, e) in fields {
                typed_rule_ok(e)?;
            }
            let expected = Type::Record(
                fields
                    .iter()
                    .map(|(n, e)| (n.clone(), e.ty.clone()))
                    .collect(),
            );
            if node.ty != expected {
                return Err(format!("record ty mismatch: node={:?}", node.ty));
            }
            Ok(())
        }
        K::Constructor(_, args) => {
            for a in args {
                typed_rule_ok(a)?;
            }
            Ok(())
        }
    }
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
