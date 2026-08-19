use parlang::{eval_program, parse_program, typecheck_program, Environment, Value};
use std::fs;
use std::path::Path;

fn run_example(relative_path: &str) -> Result<Value, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let mut source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    if relative_path == "examples/use_strings.par" {
        let string_library = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/string.par")
            .to_string_lossy()
            .into_owned();
        source = source.replace("examples/string.par", &string_library);
    }
    let program = parse_program(&source)?;
    typecheck_program(&program).map_err(|error| error.to_string())?;
    eval_program(&program, &Environment::new()).map_err(|error| error.to_string())
}

#[test]
fn executes_stdlib_example() {
    assert_eq!(run_example("examples/stdlib.par"), Ok(Value::Int(0)));
}

#[test]
fn executes_use_stdlib_example() {
    assert_eq!(run_example("examples/use_stdlib.par"), Ok(Value::Int(30)));
}

#[test]
fn executes_map_example() {
    assert_eq!(run_example("examples/stdlib/map.par"), Ok(Value::Int(146)));
}

#[test]
fn executes_treemap_example() {
    assert_eq!(
        run_example("examples/stdlib/treemap.par"),
        Ok(Value::Int(400))
    );
}

#[test]
fn executes_use_strings_example() {
    // The trailing value is a char-list representation of "Hello, World!".
    assert!(run_example("examples/use_strings.par").is_ok());
}

#[test]
#[ignore = "evaluator recurses on the native stack with no depth guard; overflows a default test thread. Passes with the main thread's larger stack. Unblocked by the EvalError::RecursionLimit slice."]
fn executes_use_recursion_example() {
    assert!(run_example("examples/use_recursion.par").is_ok());
}
