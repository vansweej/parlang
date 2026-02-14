/// Integration tests combining parser and evaluator
/// These tests verify the full pipeline from source code to evaluation

use parlang::{parse, eval, extract_bindings, Environment, Value};

fn parse_and_eval(input: &str) -> Result<Value, String> {
    let expr = parse(input)?;
    eval(&expr, &Environment::new()).map_err(|e| e.to_string())
}

/// Simulate REPL-style execution with persistent environment
/// This helper function parses and evaluates an expression while
/// extracting bindings to persist in the environment
fn parse_eval_and_extract(input: &str, env: &Environment) -> Result<(Value, Environment), String> {
    let expr = parse(input)?;
    let value = eval(&expr, env).map_err(|e| e.to_string())?;
    
    // Extract bindings from the expression to persist them
    let new_env = extract_bindings(&expr, env).map_err(|e| e.to_string())?;
    
    Ok((value, new_env))
}


#[test]
fn test_int_literal() {
    assert_eq!(parse_and_eval("42"), Ok(Value::Int(42)));
}

#[test]
fn test_bool_literal() {
    assert_eq!(parse_and_eval("true"), Ok(Value::Bool(true)));
}

#[test]
fn test_negative_int() {
    assert_eq!(parse_and_eval("-10"), Ok(Value::Int(-10)));
}

#[test]
fn test_addition() {
    assert_eq!(parse_and_eval("1 + 2"), Ok(Value::Int(3)));
}

#[test]
fn test_subtraction() {
    assert_eq!(parse_and_eval("10 - 3"), Ok(Value::Int(7)));
}

#[test]
fn test_multiplication() {
    assert_eq!(parse_and_eval("6 * 7"), Ok(Value::Int(42)));
}

#[test]
fn test_division() {
    assert_eq!(parse_and_eval("84 / 2"), Ok(Value::Int(42)));
}

#[test]
fn test_operator_precedence() {
    assert_eq!(parse_and_eval("1 + 2 * 3"), Ok(Value::Int(7))); // 1 + (2 * 3)
}

#[test]
fn test_parentheses() {
    assert_eq!(parse_and_eval("(1 + 2) * 3"), Ok(Value::Int(9))); // (1 + 2) * 3
}

#[test]
fn test_complex_arithmetic() {
    assert_eq!(parse_and_eval("10 - 6 / 2"), Ok(Value::Int(7))); // 10 - (6 / 2)
}

#[test]
fn test_equality() {
    assert_eq!(parse_and_eval("5 == 5"), Ok(Value::Bool(true)));
}

#[test]
fn test_inequality() {
    assert_eq!(parse_and_eval("5 != 3"), Ok(Value::Bool(true)));
}

#[test]
fn test_less_than() {
    assert_eq!(parse_and_eval("3 < 5"), Ok(Value::Bool(true)));
}

#[test]
fn test_greater_than() {
    assert_eq!(parse_and_eval("5 > 3"), Ok(Value::Bool(true)));
}

#[test]
fn test_less_equal() {
    assert_eq!(parse_and_eval("3 <= 3"), Ok(Value::Bool(true)));
}

#[test]
fn test_greater_equal() {
    assert_eq!(parse_and_eval("5 >= 5"), Ok(Value::Bool(true)));
}

#[test]
fn test_comparison_with_arithmetic() {
    assert_eq!(parse_and_eval("1 + 2 == 3"), Ok(Value::Bool(true)));
}

#[test]
fn test_if_true() {
    assert_eq!(parse_and_eval("if true then 1 else 2"), Ok(Value::Int(1)));
}

#[test]
fn test_if_false() {
    assert_eq!(parse_and_eval("if false then 1 else 2"), Ok(Value::Int(2)));
}

#[test]
fn test_if_with_comparison() {
    assert_eq!(parse_and_eval("if 5 > 3 then 100 else 0"), Ok(Value::Int(100)));
}

#[test]
fn test_nested_if() {
    assert_eq!(parse_and_eval("if true then if false then 1 else 2 else 3"), Ok(Value::Int(2)));
}

#[test]
fn test_simple_let() {
    assert_eq!(parse_and_eval("let x = 42 in x"), Ok(Value::Int(42)));
}

#[test]
fn test_let_with_operation() {
    assert_eq!(parse_and_eval("let x = 10 in x + 32"), Ok(Value::Int(42)));
}

#[test]
fn test_nested_let() {
    assert_eq!(parse_and_eval("let x = 1 in let y = 2 in x + y"), Ok(Value::Int(3)));
}

#[test]
fn test_let_shadowing() {
    assert_eq!(parse_and_eval("let x = 1 in let x = 2 in x"), Ok(Value::Int(2)));
}

#[test]
fn test_identity_function() {
    assert_eq!(parse_and_eval("(fun x -> x) 42"), Ok(Value::Int(42)));
}

#[test]
fn test_increment_function() {
    assert_eq!(parse_and_eval("(fun x -> x + 1) 41"), Ok(Value::Int(42)));
}

#[test]
fn test_function_with_let() {
    assert_eq!(parse_and_eval("let inc = fun x -> x + 1 in inc 41"), Ok(Value::Int(42)));
}

#[test]
fn test_double_function() {
    assert_eq!(parse_and_eval("let double = fun x -> x + x in double 21"), Ok(Value::Int(42)));
}

#[test]
fn test_curried_addition() {
    assert_eq!(parse_and_eval("(fun x -> fun y -> x + y) 40 2"), Ok(Value::Int(42)));
}

#[test]
fn test_partial_application() {
    assert_eq!(parse_and_eval("let add = fun x -> fun y -> x + y in let add5 = add 5 in add5 10"), Ok(Value::Int(15)));
}

#[test]
fn test_closure_captures_environment() {
    assert_eq!(parse_and_eval("let x = 10 in (fun y -> x + y) 32"), Ok(Value::Int(42)));
}

#[test]
fn test_higher_order_function() {
    assert_eq!(parse_and_eval("let f = fun g -> g 5 in let inc = fun x -> x + 1 in f inc"), Ok(Value::Int(6)));
}

#[test]
fn test_complex_nested_expression() {
    assert_eq!(parse_and_eval("let a = 1 in let b = 2 in let c = 3 in a + b * c"), Ok(Value::Int(7)));
}

#[test]
fn test_function_in_if() {
    assert_eq!(parse_and_eval("if true then (fun x -> x + 1) 5 else 0"), Ok(Value::Int(6)));
}

#[test]
fn test_comparison_result_in_let() {
    assert_eq!(parse_and_eval("let b = 5 > 3 in if b then 1 else 0"), Ok(Value::Int(1)));
}

#[test]
fn test_boolean_operations() {
    assert_eq!(parse_and_eval("true == true"), Ok(Value::Bool(true)));
}

#[test]
fn test_boolean_inequality() {
    assert_eq!(parse_and_eval("true != false"), Ok(Value::Bool(true)));
}

#[test]
fn test_multiple_function_calls() {
    assert_eq!(parse_and_eval("let f = fun x -> x * 2 in f (f 5)"), Ok(Value::Int(20)));
}

#[test]
fn test_nested_closures() {
    // f captures x = 1, not x = 10
    assert_eq!(parse_and_eval("let x = 1 in let f = fun y -> x + y in let x = 10 in f 5"), Ok(Value::Int(6)));
}

#[test]
fn test_complex_currying() {
    // (2*3) + (4*5) = 6 + 20 = 26
    assert_eq!(
        parse_and_eval("let add = fun x -> fun y -> x + y in let mul = fun x -> fun y -> x * y in add (mul 2 3) (mul 4 5)"),
        Ok(Value::Int(26))
    );
}

#[test]
fn test_whitespace_handling() {
    assert_eq!(parse_and_eval("1+2"), parse_and_eval("  1  +  2  "));
    assert_eq!(parse_and_eval("1+2"), Ok(Value::Int(3)));
}

#[test]
fn test_negative_numbers_in_expressions() {
    assert_eq!(parse_and_eval("-5 + 10"), Ok(Value::Int(5)));
}

#[test]
fn test_zero_division() {
    assert!(parse_and_eval("10 / 0").is_err());
}

#[test]
fn test_realistic_example_1() {
    // Absolute value function
    assert_eq!(parse_and_eval("let abs = fun x -> if x < 0 then 0 - x else x in abs (-5)"), Ok(Value::Int(5)));
}

#[test]
fn test_realistic_example_2() {
    // Max function using currying
    assert_eq!(parse_and_eval("let max = fun a -> fun b -> if a > b then a else b in max 10 20"), Ok(Value::Int(20)));
}

#[test]
fn test_realistic_example_3() {
    // Function composition: inc(double(5)) = inc(10) = 11
    assert_eq!(
        parse_and_eval("let compose = fun f -> fun g -> fun x -> f (g x) in let inc = fun x -> x + 1 in let double = fun x -> x + x in compose inc double 5"),
        Ok(Value::Int(11))
    );
}

#[test]
fn test_all_operators_together() {
    assert_eq!(parse_and_eval("if 10 + 5 * 2 == 20 then 1 else 0"), Ok(Value::Int(1)));
}

// ========================================
// REPL Persistence Tests
// ========================================
// These tests verify that bindings persist across evaluations in the REPL

#[test]
fn test_repl_persistence_simple_binding() {
    // Define a variable with semicolon syntax
    let env = Environment::new();
    let (value1, env) = parse_eval_and_extract("let x = 42; 0", &env).unwrap();
    assert_eq!(value1, Value::Int(0));
    
    // Use the variable in the next evaluation
    let (value2, _) = parse_eval_and_extract("x", &env).unwrap();
    assert_eq!(value2, Value::Int(42));
}

#[test]
fn test_repl_persistence_function_definition() {
    // Define a function
    let env = Environment::new();
    let (value1, env) = parse_eval_and_extract("let double = fun x -> x + x; 0", &env).unwrap();
    assert_eq!(value1, Value::Int(0));
    
    // Use the function in the next evaluation
    let (value2, _) = parse_eval_and_extract("double 21", &env).unwrap();
    assert_eq!(value2, Value::Int(42));
}

#[test]
fn test_repl_persistence_multiple_functions() {
    // Define multiple functions across different evaluations
    let env = Environment::new();
    
    let (_, env) = parse_eval_and_extract("let double = fun x -> x + x; 0", &env).unwrap();
    let (_, env) = parse_eval_and_extract("let triple = fun x -> x + x + x; 0", &env).unwrap();
    
    // Use both functions
    let (value1, env) = parse_eval_and_extract("double 5", &env).unwrap();
    assert_eq!(value1, Value::Int(10));
    
    let (value2, _) = parse_eval_and_extract("triple 10", &env).unwrap();
    assert_eq!(value2, Value::Int(30));
}

#[test]
fn test_repl_persistence_chained_definitions() {
    // Define a function, then use it in another function
    let env = Environment::new();
    
    let (_, env) = parse_eval_and_extract("let double = fun x -> x + x; 0", &env).unwrap();
    let (_, env) = parse_eval_and_extract("let quadruple = fun x -> double (double x); 0", &env).unwrap();
    
    // Use the second function
    let (value, _) = parse_eval_and_extract("quadruple 5", &env).unwrap();
    assert_eq!(value, Value::Int(20));
}

#[test]
fn test_repl_persistence_load_library() {
    // Load a library and verify functions persist
    let env = Environment::new();
    
    let (_, env) = parse_eval_and_extract("load \"examples/stdlib.par\" in 0", &env).unwrap();
    
    // Use functions from the loaded library
    let (value1, env) = parse_eval_and_extract("double 21", &env).unwrap();
    assert_eq!(value1, Value::Int(42));
    
    let (value2, env) = parse_eval_and_extract("triple 14", &env).unwrap();
    assert_eq!(value2, Value::Int(42));
    
    // Use max function with currying
    let (value3, _) = parse_eval_and_extract("max 10 20", &env).unwrap();
    assert_eq!(value3, Value::Int(20));
}

#[test]
fn test_repl_persistence_shadowing() {
    // Define a variable, then shadow it
    let env = Environment::new();
    
    let (_, env) = parse_eval_and_extract("let x = 10; 0", &env).unwrap();
    let (value1, env) = parse_eval_and_extract("x", &env).unwrap();
    assert_eq!(value1, Value::Int(10));
    
    // Shadow the variable
    let (_, env) = parse_eval_and_extract("let x = 20; 0", &env).unwrap();
    let (value2, _) = parse_eval_and_extract("x", &env).unwrap();
    assert_eq!(value2, Value::Int(20));
}

#[test]
fn test_repl_persistence_seq_multiple_bindings() {
    // Define multiple bindings in a single expression
    let env = Environment::new();
    
    let (_, env) = parse_eval_and_extract("let x = 1; let y = 2; let z = 3; 0", &env).unwrap();
    
    // Use all three variables
    let (value, _) = parse_eval_and_extract("x + y + z", &env).unwrap();
    assert_eq!(value, Value::Int(6));
}

#[test]
fn test_repl_persistence_function_using_persistent_var() {
    // Define a variable, then a function that uses it
    let env = Environment::new();
    
    let (_, env) = parse_eval_and_extract("let base = 10; 0", &env).unwrap();
    let (_, env) = parse_eval_and_extract("let add_base = fun x -> x + base; 0", &env).unwrap();
    
    // Use the function
    let (value, _) = parse_eval_and_extract("add_base 5", &env).unwrap();
    assert_eq!(value, Value::Int(15));
}

// ========================================
// Tests for Optional Body Expression
// ========================================

#[test]
fn test_optional_body_simple_binding() {
    // Define a binding without trailing 0
    let env = Environment::new();
    
    let (value, env) = parse_eval_and_extract("let x = 42;", &env).unwrap();
    assert_eq!(value, Value::Int(0)); // Should default to 0
    
    // Use the variable
    let (value, _) = parse_eval_and_extract("x", &env).unwrap();
    assert_eq!(value, Value::Int(42));
}

#[test]
fn test_optional_body_function_definition() {
    // Define a function without trailing 0
    let env = Environment::new();
    
    let (value, env) = parse_eval_and_extract("let double = fun x -> x + x;", &env).unwrap();
    assert_eq!(value, Value::Int(0)); // Should default to 0
    
    // Use the function
    let (value, _) = parse_eval_and_extract("double 21", &env).unwrap();
    assert_eq!(value, Value::Int(42));
}

#[test]
fn test_optional_body_multiple_bindings() {
    // Define multiple bindings without trailing expression
    let env = Environment::new();
    
    let (value, env) = parse_eval_and_extract("let x = 1; let y = 2; let z = 3;", &env).unwrap();
    assert_eq!(value, Value::Int(0)); // Should default to 0
    
    // Use the variables
    let (value, _) = parse_eval_and_extract("x + y + z", &env).unwrap();
    assert_eq!(value, Value::Int(6));
}

#[test]
fn test_optional_body_load_library() {
    // Load a library without "in 0"
    let env = Environment::new();
    
    let (value, env) = parse_eval_and_extract("load \"examples/stdlib.par\"", &env).unwrap();
    assert_eq!(value, Value::Int(0)); // Should default to 0
    
    // Use functions from the loaded library
    let (value, _) = parse_eval_and_extract("double 21", &env).unwrap();
    assert_eq!(value, Value::Int(42));
}

#[test]
fn test_optional_body_backwards_compatible() {
    // Verify old syntax with explicit body still works
    let env = Environment::new();
    
    let (value, env) = parse_eval_and_extract("let x = 10; 0", &env).unwrap();
    assert_eq!(value, Value::Int(0));
    
    let (value, env) = parse_eval_and_extract("let y = 20; x + 5", &env).unwrap();
    assert_eq!(value, Value::Int(15)); // x is 10, so 10 + 5 = 15
    
    let (value, _) = parse_eval_and_extract("y", &env).unwrap();
    assert_eq!(value, Value::Int(20));
}

#[test]
fn test_optional_body_load_with_in() {
    // Verify old load syntax with "in" still works
    let env = Environment::new();
    
    let (value, env) = parse_eval_and_extract("load \"examples/stdlib.par\" in 0", &env).unwrap();
    assert_eq!(value, Value::Int(0));
    
    let (value, _) = parse_eval_and_extract("triple 14", &env).unwrap();
    assert_eq!(value, Value::Int(42));
}

// ========================================
// Tests for Auto-Submit Detection
// ========================================
// These tests verify that various expressions are complete and parseable,
// which is the basis for the REPL's auto-submit behavior

#[test]
fn test_auto_submit_simple_literal() {
    // Simple literals should parse and auto-submit
    assert!(parse("42").is_ok());
    assert!(parse("true").is_ok());
}

#[test]
fn test_auto_submit_arithmetic() {
    // Arithmetic expressions should parse and auto-submit
    assert!(parse("1 + 2").is_ok());
    assert!(parse("10 - 3 * 2").is_ok());
}

#[test]
fn test_auto_submit_function_call() {
    // Function calls should parse and auto-submit (main issue being fixed)
    assert!(parse("(fun x -> x + 1) 41").is_ok());
    assert!(parse("(fun x -> fun y -> x + y) 1 2").is_ok());
}

#[test]
fn test_auto_submit_let_in() {
    // Complete let-in expressions should parse and auto-submit
    assert!(parse("let x = 10 in x + 5").is_ok());
    assert!(parse("let double = fun x -> x + x in double 21").is_ok());
}

#[test]
fn test_auto_submit_let_semicolon() {
    // Let with semicolon should parse and auto-submit
    assert!(parse("let x = 42;").is_ok());
    assert!(parse("let double = fun x -> x + x;").is_ok());
    assert!(parse("let x = 1; let y = 2;").is_ok());
}

#[test]
fn test_auto_submit_load() {
    // Load statements should parse and auto-submit
    assert!(parse("load \"examples/stdlib.par\"").is_ok());
    assert!(parse("load \"examples/stdlib.par\" in 0").is_ok());
}

#[test]
fn test_incomplete_expression_let_in() {
    // Incomplete let-in expressions should NOT parse (will wait for continuation)
    assert!(parse("let x = 10 in").is_err());
    assert!(parse("let x =").is_err());
}

#[test]
fn test_incomplete_expression_function() {
    // Incomplete function definitions should NOT parse (will wait for continuation)
    assert!(parse("fun x ->").is_err());
    assert!(parse("let f = fun x ->").is_err());
}
