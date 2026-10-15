use parlang::{
    eval_program, parse_program, run_on_evaluator_stack, typecheck_program, Environment,
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
    fn collect(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
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

    let rels: Vec<String> = files
        .iter()
        .map(|path| {
            let rel = path.strip_prefix(root).expect("strip manifest prefix");
            rel.to_string_lossy().replace('\\', "/")
        })
        .collect();

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
