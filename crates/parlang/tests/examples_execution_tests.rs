use parlang::{
    eval_program, parse_program, run_on_evaluator_stack, typecheck_program, Environment,
};
use std::fs;
use std::path::Path;

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
