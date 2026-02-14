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

// ============================================================================
// Recursion Tests
// ============================================================================

#[test]
fn test_rec_simple_identity() {
    // Simple recursive function that just returns its argument (base case)
    let result = parse_and_eval("(rec f -> fun n -> n) 42");
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_rec_factorial() {
    // Classic factorial using recursion
    let factorial = r"
        (rec factorial -> fun n ->
            if n == 0
            then 1
            else n * factorial (n - 1)
        ) 5
    ";
    assert_eq!(parse_and_eval(factorial), Ok(Value::Int(120)));
}

#[test]
fn test_rec_factorial_larger() {
    // Factorial of 10
    let factorial = r"
        (rec factorial -> fun n ->
            if n == 0
            then 1
            else n * factorial (n - 1)
        ) 10
    ";
    assert_eq!(parse_and_eval(factorial), Ok(Value::Int(3_628_800)));
}

#[test]
fn test_rec_fibonacci() {
    // Fibonacci sequence using recursion
    let fib = r"
        (rec fib -> fun n ->
            if n == 0
            then 0
            else if n == 1
            then 1
            else fib (n - 1) + fib (n - 2)
        ) 10
    ";
    assert_eq!(parse_and_eval(fib), Ok(Value::Int(55)));
}

#[test]
fn test_rec_sum_to_n() {
    // Sum from 1 to n using recursion (not tail recursive, so keep it small)
    let sum = r"
        (rec sum -> fun n ->
            if n == 0
            then 0
            else n + sum (n - 1)
        ) 10
    ";
    assert_eq!(parse_and_eval(sum), Ok(Value::Int(55)));
}

#[test]
fn test_rec_countdown() {
    // Countdown to zero
    let countdown = r"
        (rec countdown -> fun n ->
            if n == 0
            then 0
            else countdown (n - 1)
        ) 10
    ";
    assert_eq!(parse_and_eval(countdown), Ok(Value::Int(0)));
}

#[test]
fn test_rec_with_let() {
    // Recursive function with let binding
    let code = r"
        let fact = rec factorial -> fun n ->
            if n == 0
            then 1
            else n * factorial (n - 1)
        in fact 6
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(720)));
}

#[test]
fn test_rec_power() {
    // Compute x^n using recursion
    let power = r"
        (rec pow -> fun n ->
            if n == 0
            then 1
            else 2 * pow (n - 1)
        ) 8
    ";
    assert_eq!(parse_and_eval(power), Ok(Value::Int(256)));
}

#[test]
fn test_rec_even_odd() {
    // Test even using mutual recursion simulation (via helper)
    let even = r"
        (rec is_even -> fun n ->
            if n == 0
            then true
            else if n == 1
            then false
            else is_even (n - 2)
        ) 10
    ";
    assert_eq!(parse_and_eval(even), Ok(Value::Bool(true)));
    
    let odd = r"
        (rec is_even -> fun n ->
            if n == 0
            then true
            else if n == 1
            then false
            else is_even (n - 2)
        ) 11
    ";
    assert_eq!(parse_and_eval(odd), Ok(Value::Bool(false)));
}

#[test]
fn test_rec_tail_call_optimization() {
    // Test that tail call optimization prevents stack overflow for deep recursion
    // Uses a simple countdown function that recurses 1000 times
    let countdown = r"
        (rec countdown -> fun n ->
            if n == 0
            then 0
            else countdown (n - 1)
        ) 1000
    ";
    assert_eq!(parse_and_eval(countdown), Ok(Value::Int(0)));
}

#[test]
fn test_rec_gcd() {
    // Greatest common divisor using Euclidean algorithm
    let gcd = r"
        (rec gcd -> fun a -> fun b ->
            if b == 0
            then a
            else gcd b (a - (a / b) * b)
        ) 48 18
    ";
    assert_eq!(parse_and_eval(gcd), Ok(Value::Int(6)));
}

#[test]
fn test_rec_nested_calls() {
    // Test recursive function with nested arithmetic
    let code = r"
        (rec f -> fun n ->
            if n == 0
            then 0
            else (f (n - 1)) + n
        ) 5
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(15)));
}

#[test]
fn test_rec_comparison_in_recursion() {
    // Test comparison operators in recursive function
    let max_value = r"
        (rec find_max -> fun current -> fun n ->
            if n == 0
            then current
            else if n > current
            then find_max n (n - 1)
            else find_max current (n - 1)
        ) 0 10
    ";
    assert_eq!(parse_and_eval(max_value), Ok(Value::Int(10)));
}

#[test]
fn test_rec_multiple_base_cases() {
    // Recursive function with multiple base cases
    let fib_like = r"
        (rec f -> fun n ->
            if n == 0
            then 1
            else if n == 1
            then 1
            else if n == 2
            then 2
            else f (n - 1) + f (n - 2)
        ) 6
    ";
    assert_eq!(parse_and_eval(fib_like), Ok(Value::Int(13)));
}

#[test]
fn test_rec_with_boolean_result() {
    // Recursive function returning boolean
    let is_positive = r"
        (rec check -> fun n ->
            if n > 0
            then true
            else false
        ) 5
    ";
    assert_eq!(parse_and_eval(is_positive), Ok(Value::Bool(true)));
}

#[test]
fn test_rec_seq_binding() {
    // Test recursive function with sequential bindings
    let code = r"
        let factorial = rec f -> fun n ->
            if n == 0
            then 1
            else n * f (n - 1);
        factorial 5
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(120)));
}

#[test]
fn test_rec_repl_persistence() {
    // Test that recursive functions persist in REPL environment
    let env = Environment::new();
    let (_, env) = parse_eval_and_extract(
        "let factorial = rec f -> fun n -> if n == 0 then 1 else n * f (n - 1);",
        &env,
    )
    .unwrap();
    let (result, _) = parse_eval_and_extract("factorial 5", &env).unwrap();
    assert_eq!(result, Value::Int(120));
}

#[test]
fn test_rec_curried_function() {
    // Test recursive function with currying
    let code = r"
        let add_up_to = rec f -> fun acc -> fun n ->
            if n == 0
            then acc
            else f (acc + n) (n - 1)
        in let add_from_zero = add_up_to 0
        in add_from_zero 10
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(55)));
}

#[test]
fn test_rec_error_non_function_body() {
    // rec expression body must be a function
    let result = parse_and_eval("(rec f -> 42) 5");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be a function"));
}

#[test]
fn test_rec_non_recursive_function() {
    // A recursive function that doesn't actually recurse (valid but non-recursive)
    let result = parse_and_eval("(rec f -> fun x -> x + 1) 41");
    assert_eq!(result, Ok(Value::Int(42)));
}

// Pattern matching tests
#[test]
fn test_match_literal_int() {
    let code = "match 0 with | 0 -> 1 | n -> n";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(1)));
}

#[test]
fn test_match_literal_bool() {
    let code = "match true with | true -> 1 | false -> 0";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(1)));
}

#[test]
fn test_match_variable_pattern() {
    let code = "match 42 with | 0 -> 1 | n -> n";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(42)));
}

#[test]
fn test_match_wildcard_pattern() {
    let code = "match 100 with | 0 -> 1 | _ -> 999";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(999)));
}

#[test]
fn test_match_multiple_arms() {
    let code = "match 2 with | 0 -> 10 | 1 -> 20 | 2 -> 30 | _ -> 40";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(30)));
}

#[test]
fn test_match_with_expr() {
    let code = "match 1 + 1 with | 0 -> 10 | 2 -> 20 | _ -> 30";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(20)));
}

#[test]
fn test_match_in_function() {
    let code = r"
        let abs = fun n -> match n with | 0 -> 0 | n -> if n < 0 then 0 - n else n
        in abs (-5)
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(5)));
}

#[test]
fn test_match_factorial() {
    let code = r"
        let factorial = rec fact -> fun n ->
            match n with
            | 0 -> 1
            | n -> n * fact (n - 1)
        in factorial 5
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(120)));
}

#[test]
fn test_match_fibonacci() {
    let code = r"
        let fib = rec f -> fun n ->
            match n with
            | 0 -> 0
            | 1 -> 1
            | n -> f (n - 1) + f (n - 2)
        in fib 7
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(13)));
}

#[test]
fn test_match_nested() {
    let code = r"
        match 1 with
        | 0 -> 10
        | 1 -> match 2 with | 2 -> 20 | _ -> 30
        | _ -> 40
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(20)));
}

#[test]
fn test_match_negative_literal() {
    let code = "match -1 with | -1 -> 100 | 0 -> 0 | _ -> 1";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(100)));
}

#[test]
fn test_match_bool_false() {
    let code = "match false with | true -> 1 | false -> 0";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(0)));
}

#[test]
fn test_match_pattern_order_matters() {
    // First matching pattern wins
    let code = "match 5 with | n -> 100 | 5 -> 200";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(100)));
}

#[test]
fn test_match_variable_binding() {
    let code = "match 42 with | x -> x + 1";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(43)));
}

#[test]
fn test_match_with_let() {
    let code = r"
        let x = 5
        in match x with
        | 0 -> 1
        | 5 -> 42
        | _ -> 0
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(42)));
}

#[test]
fn test_match_error_no_match() {
    // If no pattern matches, should return an error
    let code = "match 100 with | 0 -> 1 | 1 -> 2";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Pattern match is non-exhaustive"));
}

#[test]
fn test_match_replaces_nested_if() {
    // Demonstrate match replacing nested if-then-else
    // This would work with strings, but we only have ints and bools
    // So we use a numeric example
    let if_code = r"
        let category = fun n ->
            if n == 0 then 0
            else if n == 1 then 1
            else if n == 2 then 2
            else 999
        in category 1
    ";
    
    let match_code = r"
        let category = fun n ->
            match n with
            | 0 -> 0
            | 1 -> 1
            | 2 -> 2
            | _ -> 999
        in category 1
    ";
    
    assert_eq!(parse_and_eval(if_code), parse_and_eval(match_code));
    assert_eq!(parse_and_eval(match_code), Ok(Value::Int(1)));
}

// Tuple tests
#[test]
fn test_tuple_simple() {
    let result = parse_and_eval("(1, 2, 3)");
    assert_eq!(
        result,
        Ok(Value::Tuple(vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3)
        ]))
    );
}

#[test]
fn test_tuple_empty() {
    let result = parse_and_eval("()");
    assert_eq!(result, Ok(Value::Tuple(vec![])));
}

#[test]
fn test_tuple_mixed_types() {
    let result = parse_and_eval("(42, true)");
    assert_eq!(
        result,
        Ok(Value::Tuple(vec![Value::Int(42), Value::Bool(true)]))
    );
}

#[test]
fn test_tuple_nested() {
    let result = parse_and_eval("((1, 2), (3, 4))");
    assert_eq!(
        result,
        Ok(Value::Tuple(vec![
            Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
            Value::Tuple(vec![Value::Int(3), Value::Int(4)]),
        ]))
    );
}

#[test]
fn test_tuple_projection_first() {
    assert_eq!(parse_and_eval("(10, 20).0"), Ok(Value::Int(10)));
}

#[test]
fn test_tuple_projection_second() {
    assert_eq!(parse_and_eval("(10, 20).1"), Ok(Value::Int(20)));
}

#[test]
fn test_tuple_projection_nested() {
    // ((1, 2), (3, 4)).0.1 => 2
    assert_eq!(parse_and_eval("((1, 2), (3, 4)).0.1"), Ok(Value::Int(2)));
}

#[test]
fn test_tuple_projection_complex() {
    // ((10, 20), 30).0.0 => 10
    assert_eq!(parse_and_eval("((10, 20), 30).0.0"), Ok(Value::Int(10)));
}

#[test]
fn test_tuple_with_let() {
    let code = "let pair = (42, true) in pair";
    assert_eq!(
        parse_and_eval(code),
        Ok(Value::Tuple(vec![Value::Int(42), Value::Bool(true)]))
    );
}

#[test]
fn test_tuple_projection_with_let() {
    let code = "let point = (10, 20) in point.0";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(10)));
}

#[test]
fn test_tuple_in_binop() {
    let code = "(10, 20).0 + (5, 15).1";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(25)));
}

#[test]
fn test_function_returning_tuple() {
    let code = "let f = fun x -> (x, x + 1) in f 5";
    assert_eq!(
        parse_and_eval(code),
        Ok(Value::Tuple(vec![Value::Int(5), Value::Int(6)]))
    );
}

#[test]
fn test_tuple_swap() {
    let code = "let swap = fun p -> (p.1, p.0) in swap (5, 10)";
    assert_eq!(
        parse_and_eval(code),
        Ok(Value::Tuple(vec![Value::Int(10), Value::Int(5)]))
    );
}

#[test]
fn test_tuple_with_function() {
    let code = "(42, fun x -> x * 2)";
    let result = parse_and_eval(code);
    assert!(result.is_ok());
    if let Ok(Value::Tuple(values)) = result {
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], Value::Int(42));
        assert!(matches!(values[1], Value::Closure(_, _, _)));
    }
}

#[test]
fn test_tuple_function_projection_call_result() {
    let code = "let data = (42, fun x -> x * 2) in data.1 21";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(42)));
}

// Pattern matching with tuples
#[test]
fn test_match_tuple_simple() {
    let code = "match (10, 20) with | (0, 0) -> 0 | (x, y) -> x + y";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(30)));
}

#[test]
fn test_match_tuple_with_literal() {
    let code = "match (0, 5) with | (0, y) -> y | (x, y) -> x";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(5)));
}

#[test]
fn test_match_tuple_with_wildcard() {
    let code = "match (10, 20) with | (x, _) -> x";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(10)));
}

#[test]
fn test_match_tuple_nested() {
    let code = "match ((1, 2), 3) with | ((a, b), c) -> a + b + c";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(6)));
}

#[test]
fn test_match_tuple_multiple_arms() {
    let code = r"
        match (1, 2) with
        | (0, 0) -> 0
        | (1, 1) -> 1
        | (1, 2) -> 12
        | (x, y) -> x + y
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(12)));
}

#[test]
fn test_match_empty_tuple() {
    let code = "match () with | () -> 42";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(42)));
}

// Error cases
#[test]
fn test_tuple_projection_out_of_bounds() {
    let result = parse_and_eval("(10, 20).2");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of bounds"));
}

#[test]
fn test_tuple_projection_non_tuple() {
    let result = parse_and_eval("42.0");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Tuple projection requires a tuple"));
}

#[test]
fn test_match_tuple_wrong_pattern_size() {
    let code = "match (1, 2) with | (x, y, z) -> x";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Pattern match is non-exhaustive"));
}

#[test]
fn test_match_tuple_wrong_literal() {
    let code = "match (1, 2) with | (0, 0) -> 0";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Pattern match is non-exhaustive"));
}

// Complex realistic examples
#[test]
fn test_tuple_point_distance() {
    let code = r"
        let p1 = (0, 0) in
        let p2 = (3, 4) in
        let dx = p2.0 - p1.0 in
        let dy = p2.1 - p1.1 in
        dx * dx + dy * dy
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(25)));
}

#[test]
fn test_tuple_fibonacci_pair() {
    let code = r"
        let fib = rec fib -> fun n ->
            if n == 0 then (0, 1)
            else if n == 1 then (1, 1)
            else
                let prev = fib (n - 1) in
                (prev.1, prev.0 + prev.1)
        in (fib 6).0
    ";
    assert_eq!(parse_and_eval(code), Ok(Value::Int(8)));
}

#[test]
fn test_tuple_with_match_and_rec() {
    let code = r"
        let divmod = rec divmod -> fun p ->
            match p with
            | (n, d) ->
                if n < d then (0, n)
                else
                    let result = divmod (n - d, d) in
                    (result.0 + 1, result.1)
        in divmod (17, 5)
    ";
    assert_eq!(
        parse_and_eval(code),
        Ok(Value::Tuple(vec![Value::Int(3), Value::Int(2)]))
    );
}

#[test]
fn test_rec_closure_display() {
    // Test that recursive closures display correctly
    let code = r"rec factorial -> fun n -> if n == 0 then 1 else n * factorial (n - 1)";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    let display_str = format!("{result}");
    assert!(display_str.contains("<recursive function"));
    assert!(display_str.contains("factorial"));
}

#[test]
fn test_tuple_display() {
    // Test tuple display formatting
    let code = "(1, 2, 3)";
    let result = parse_and_eval(code).unwrap();
    assert_eq!(format!("{result}"), "(1, 2, 3)");
}

#[test]
fn test_nested_tuple_display() {
    // Test nested tuple display
    let code = "((1, 2), (3, 4))";
    let result = parse_and_eval(code).unwrap();
    assert_eq!(format!("{result}"), "((1, 2), (3, 4))");
}

#[test]
fn test_empty_tuple_display() {
    // Test empty tuple display
    let code = "()";
    let result = parse_and_eval(code).unwrap();
    assert_eq!(format!("{result}"), "()");
}

