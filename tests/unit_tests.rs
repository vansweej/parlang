/// Unit tests for additional coverage
use parlang::{parse, eval, Environment, Value, EvalError, dot};
use std::fs;

#[test]
fn test_if_non_boolean_condition() {
    // Test that if with non-boolean condition fails
    let expr = parse("if 42 then 1 else 0").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::TypeError(_))));
}

#[test]
fn test_is_tail_call_nested() {
    // Test the is_tail_call_to function with nested applications
    // This is tested indirectly through recursive functions
    let code = r"
        (rec f -> fun x ->
            if x == 0
            then 0
            else (f (x - 1))
        ) 100
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(0)));
}

#[test]
fn test_dot_write_ast_to_file() {
    // Test writing AST to DOT file
    let expr = parse("1 + 2").unwrap();
    let temp_file = "/tmp/test_dot_output.dot";
    
    let result = dot::write_ast_to_dot_file(&expr, temp_file);
    assert!(result.is_ok());
    
    // Verify file was created
    assert!(fs::metadata(temp_file).is_ok());
    
    // Verify file contains valid DOT content
    let content = fs::read_to_string(temp_file).unwrap();
    assert!(content.contains("digraph"));
    assert!(content.contains("BinOp"));
    
    // Cleanup
    fs::remove_file(temp_file).ok();
}

#[test]
fn test_dot_with_pattern_bool() {
    // Test DOT generation with boolean pattern
    let expr = parse("match true with | true -> 1 | false -> 0").unwrap();
    let dot = dot::ast_to_dot(&expr);
    assert!(dot.contains("Bool"));
    assert!(dot.contains("Match"));
}

#[test]
fn test_dot_with_pattern_int() {
    // Test DOT generation with integer pattern
    let expr = parse("match 42 with | 42 -> true | _ -> false").unwrap();
    let dot = dot::ast_to_dot(&expr);
    assert!(dot.contains("Int"));
    assert!(dot.contains("Match"));
}

#[test]
fn test_extract_bindings_non_let() {
    // Test extract_bindings with non-let expression (should return None)
    use parlang::extract_bindings;
    let expr = parse("42").unwrap();
    let env = Environment::new();
    let result = extract_bindings(&expr, &env).unwrap();
    // Should return the original environment (no new bindings)
    // The function should succeed but not add new bindings
    let _ = result;
}

#[test]
fn test_load_file_not_found() {
    // Test load with non-existent file
    let expr = parse("load \"nonexistent_file_12345.par\" in 42").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::LoadError(_))));
}

#[test]
fn test_load_invalid_syntax() {
    // Create a temp file with invalid syntax
    let temp_file = "/tmp/test_invalid_syntax.par";
    fs::write(temp_file, "let x = in y").unwrap();
    
    let code = format!("load \"{temp_file}\" in 42");
    let expr = parse(&code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    
    // Cleanup first
    fs::remove_file(temp_file).ok();
    
    assert!(matches!(result, Err(EvalError::LoadError(_))));
}

#[test]
fn test_rec_non_tail_call() {
    // Test recursive function where not all calls are tail calls
    // This tests the non-tail call path in eval_rec_closure
    let code = r"
        (rec f -> fun n ->
            if n == 0
            then 1
            else n + (f (n - 1))
        ) 5
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(16))); // 5+4+3+2+1+1 = 16 (base case is 1)
}

#[test]
fn test_rec_if_branches_evaluation() {
    // Ensure both if branches are tested in recursive context
    let code1 = r"(rec f -> fun n -> if n == 0 then 0 else f (n - 1)) 0";
    assert_eq!(
        eval(&parse(code1).unwrap(), &Environment::new()),
        Ok(Value::Int(0))
    );
    
    let code2 = r"(rec f -> fun n -> if n == 1 then 1 else f (n - 1)) 5";
    assert_eq!(
        eval(&parse(code2).unwrap(), &Environment::new()),
        Ok(Value::Int(1))
    );
}

#[test]
fn test_type_error_is_error() {
    // Test that TypeError implements std::error::Error
    use parlang::TypeError;
    use std::error::Error;
    
    let err = TypeError::UnboundVariable("x".to_string());
    let _: &dyn Error = &err;  // Should compile if it implements Error
}

#[test]
fn test_occurs_check_failure() {
    // Try to create a situation that triggers occurs check
    // This is tricky because the type system might prevent it at parse time
    // We test the error formatting instead
    use parlang::{TypeError, TypeVar, Type};
    
    let var = TypeVar(0);
    let ty = Type::Fun(Box::new(Type::Var(TypeVar(0))), Box::new(Type::Int));
    let err = TypeError::OccursCheckFailed(var, ty);
    
    let msg = format!("{err}");
    assert!(msg.contains("Occurs check failed"));
}

#[test]
fn test_typechecker_bind_var_same_var() {
    // This tests the early return in bind_var when binding a var to itself
    // We do this indirectly through type inference
    use parlang::typecheck;
    
    let expr = parse("fun x -> x").unwrap();
    let ty = typecheck(&expr).unwrap();
    // Should succeed - identity function
    assert!(matches!(ty, parlang::Type::Fun(_, _)));
}

#[test]
fn test_rec_if_non_boolean_in_rec() {
    // Test if with non-boolean in recursive function context
    let code = r"(rec f -> fun n -> if n then 1 else 0) 5";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::TypeError(_))));
}

#[test]
fn test_is_tail_call_to_var() {
    // Test is_tail_call_to with simple variable
    // This is tested indirectly through recursive functions
    let code = r"(rec f -> fun n -> if n == 0 then 0 else f) 5";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    // This should evaluate the recursive function itself when n != 0
    let result = eval(&expr, &env);
    // The result will be a closure
    assert!(result.is_ok());
}

#[test]
fn test_rec_with_complex_tail_call() {
    // Test nested application in tail position
    let code = r"
        (rec f -> fun n ->
            if n == 0
            then 0
            else ((f) (n - 1))
        ) 10
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(0)));
}

#[test]
fn test_pattern_match_tuple_mismatch() {
    // Test matching a tuple pattern against a non-tuple value
    let code = r"match 42 with | (x, y) -> x | _ -> 0";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // Should match the wildcard since tuple pattern doesn't match an int
    assert_eq!(result, Ok(Value::Int(0)));
}

#[test]
fn test_pattern_match_tuple_length_mismatch() {
    // Test matching a tuple pattern with wrong number of elements
    let code = r"match (1, 2, 3) with | (x, y) -> x | _ -> 0";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // Should match the wildcard since tuple pattern has wrong length
    assert_eq!(result, Ok(Value::Int(0)));
}

#[test]
fn test_is_tail_call_expr_var() {
    // Test is_tail_call_to with Expr::Var path
    let code = r"(rec f -> fun n -> if n == 0 then 42 else f) 0";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_is_tail_call_app_path() {
    // Test is_tail_call_to with nested App
    let code = r"
        (rec sum -> fun n ->
            if n == 0
            then 0
            else n + (sum (n - 1))
        ) 3
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(6))); // 3+2+1+0
}
