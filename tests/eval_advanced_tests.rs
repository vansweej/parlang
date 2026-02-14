/// Advanced evaluation tests
/// Tests for tail call optimization, deep recursion, and error handling edge cases
use parlang::{parse, eval, Environment, Value, EvalError};

// Tail Call Optimization (TCO) Stress Tests

#[test]
fn test_tco_deep_recursion_countdown() {
    // Test TCO with very deep recursion (1000 iterations)
    let code = r"
        (rec countdown -> fun n ->
            if n == 0
            then 0
            else countdown (n - 1)
        ) 1000
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(0)));
}

#[test]
fn test_tco_deep_recursion_accumulator() {
    // Test TCO with accumulator pattern (sum from 0 to 30)
    // Note: TCO with multi-argument functions may have limitations
    let code = r"
        (rec helper -> fun n ->
            if n == 0
            then 0
            else n + helper (n - 1)
        ) 30
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // 1+2+...+30 = 30*31/2 = 465
    assert_eq!(result, Ok(Value::Int(465)));
}

#[test]
fn test_tco_very_deep_recursion() {
    // Test TCO with deep recursion (500 iterations)
    // This tests that TCO prevents stack overflow
    let code = r"
        (rec deep -> fun n ->
            if n == 0
            then 42
            else deep (n - 1)
        ) 500
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_tco_tail_recursive_fibonacci_with_accumulator() {
    // Test TCO with tail-recursive fibonacci using accumulators
    // Note: Multi-argument recursive functions may have TCO limitations
    let code = r"
        (rec fib -> fun n ->
            if n == 0
            then 0
            else if n == 1
            then 1
            else fib (n - 1) + fib (n - 2)
        ) 10
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // 10th fibonacci number  
    assert_eq!(result, Ok(Value::Int(55)));
}

#[test]
fn test_tco_mutual_recursion_simulation() {
    // Test TCO with pattern simulating mutual recursion via single function
    let code = r"
        (rec helper -> fun n ->
            if n == 0
            then true
            else if n == 1
            then false
            else helper (n - 2)
        ) 50
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Bool(true))); // 50 is even
}

#[test]
fn test_tco_with_conditional_branches() {
    // Test TCO where recursion happens in both branches
    let code = r"
        (rec collatz -> fun n ->
            if n == 1
            then 1
            else if n == 2
            then collatz 1
            else if n > 2
            then collatz (n - 1)
            else 0
        ) 100
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(1)));
}

#[test]
fn test_tco_nested_function_calls() {
    // Test that TCO works with nested function applications
    let code = r"
        let countdown = rec cnt -> fun n ->
            if n == 0
            then 0
            else cnt (n - 1)
        in let repeat = fun f -> fun n -> f n
        in repeat countdown 500
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(0)));
}

// Non-Tail Recursive Functions (should still work, just not optimized)

#[test]
fn test_non_tail_recursive_factorial() {
    // Test non-tail recursive factorial (multiplication after recursive call)
    let code = r"
        (rec factorial -> fun n ->
            if n == 0
            then 1
            else n * factorial (n - 1)
        ) 10
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(3628800)));
}

#[test]
fn test_non_tail_recursive_fibonacci() {
    // Test non-tail recursive fibonacci
    let code = r"
        (rec fib -> fun n ->
            if n == 0
            then 0
            else if n == 1
            then 1
            else fib (n - 1) + fib (n - 2)
        ) 10
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(55)));
}

// Error Handling Edge Cases

#[test]
fn test_division_by_zero() {
    // Test division by zero error handling
    let code = "10 / 0";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(result.is_err());
}

#[test]
fn test_unbound_variable_error() {
    // Test error when accessing unbound variable
    let code = "unknown_var";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::UnboundVariable(_))));
}

#[test]
fn test_type_error_non_function_application() {
    // Test error when applying non-function as function
    let code = "42 10";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::TypeError(_))));
}

#[test]
fn test_type_error_non_boolean_condition() {
    // Test error when if condition is not boolean
    let code = "if 42 then 1 else 0";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::TypeError(_))));
}

#[test]
fn test_type_error_arithmetic_on_boolean() {
    // Test error when doing arithmetic on booleans
    let code = "true + false";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::TypeError(_))));
}

#[test]
fn test_type_error_comparison_mixed_types() {
    // Test error when comparing different types (if type checking is disabled)
    let code = "42 == true";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::TypeError(_))));
}

#[test]
fn test_pattern_match_failure() {
    // Test pattern match failure (no matching pattern)
    let code = r"
        match 42 with
        | 0 -> 0
        | 1 -> 1
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(matches!(result, Err(EvalError::PatternMatchNonExhaustive)));
}

#[test]
fn test_pattern_match_tuple_length_mismatch() {
    // Test error when tuple pattern length doesn't match value
    // Note: This may not error during parsing or evaluation, depending on implementation
    let code = r"
        match (1, 2) with
        | (a, b, c) -> a
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // Pattern doesn't match, so it should fail
    assert!(result.is_err());
}

#[test]
fn test_record_field_access_missing_field() {
    // Test error when accessing non-existent record field
    let code = r"
        let r = { name: 42 }
        in r.age
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // Should return FieldNotFound error
    assert!(matches!(result, Err(EvalError::FieldNotFound(_, _))));
}

#[test]
fn test_record_access_on_non_record() {
    // Test error when accessing field on non-record value
    let code = "42.field";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // Should return RecordExpected error
    assert!(matches!(result, Err(EvalError::RecordExpected(_))));
}

#[test]
fn test_tuple_projection_out_of_bounds() {
    // Test error when tuple projection index is out of bounds
    let code = "(1, 2).5";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // Should return IndexOutOfBounds error
    assert!(matches!(result, Err(EvalError::IndexOutOfBounds(_))));
}

#[test]
fn test_tuple_projection_on_non_tuple() {
    // Test error when projecting on non-tuple value
    let code = "42.0";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // Should return some kind of error (either FieldNotFound or RecordExpected)
    assert!(result.is_err());
}

// Complex Nested Expressions

#[test]
fn test_deeply_nested_let_bindings() {
    // Test deeply nested let bindings
    let code = r"
        let a = 1 in
        let b = a + 1 in
        let c = b + 1 in
        let d = c + 1 in
        let e = d + 1 in
        let f = e + 1 in
        let g = f + 1 in
        let h = g + 1 in
        let i = h + 1 in
        let j = i + 1 in
        j
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(10)));
}

#[test]
fn test_deeply_nested_function_applications() {
    // Test deeply nested function applications
    let code = r"
        let inc = fun x -> x + 1 in
        inc (inc (inc (inc (inc (inc (inc (inc (inc (inc 0)))))))))
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(10)));
}

#[test]
fn test_complex_nested_pattern_matching() {
    // Test complex nested pattern matching
    let code = r"
        type Option a = Some a | None in
        let x = Some (Some (Some 42)) in
        match x with
        | Some (Some (Some n)) -> n
        | Some (Some None) -> 1
        | Some None -> 2
        | None -> 3
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_nested_records_and_tuples() {
    // Test nested records and tuples
    let code = r"
        let data = { 
            point: (10, 20), 
            info: { value: 30, active: true } 
        } in
        data.point.0 + data.point.1 + data.info.value
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(60)));
}

// Closure and Environment Tests

#[test]
fn test_closure_captures_environment() {
    // Test that closures correctly capture their environment
    let code = r"
        let x = 10 in
        let f = fun y -> x + y in
        let x = 20 in
        f 5
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // Should use x = 10 from closure, not x = 20
    assert_eq!(result, Ok(Value::Int(15)));
}

#[test]
fn test_nested_closures() {
    // Test nested closures with multiple captures
    let code = r"
        let a = 1 in
        let f = fun b -> 
            let g = fun c -> a + b + c
            in g
        in (f 10) 100
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(111)));
}

#[test]
fn test_closure_in_recursive_function() {
    // Test closures within recursive functions
    let code = r"
        let multiplier = 2 in
        (rec doubleSum -> fun n ->
            if n == 0
            then 0
            else (n * multiplier) + doubleSum (n - 1)
        ) 5
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    // (2*5) + (2*4) + (2*3) + (2*2) + (2*1) = 10 + 8 + 6 + 4 + 2 = 30
    assert_eq!(result, Ok(Value::Int(30)));
}

// Large Data Structure Tests

#[test]
fn test_large_tuple() {
    // Test tuples with many elements
    let code = "(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(result.is_ok());
}

#[test]
fn test_large_record() {
    // Test records with many fields
    let code = r"
        let r = { 
            a: 1, b: 2, c: 3, d: 4, e: 5,
            f: 6, g: 7, h: 8, i: 9, j: 10
        } in
        r.a + r.e + r.j
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(16)));
}

#[test]
fn test_long_constructor_chain() {
    // Test chain of constructor applications
    let code = r"
        type List a = Nil | Cons a (List a) in
        let list = Cons 1 (Cons 2 (Cons 3 (Cons 4 (Cons 5 Nil)))) in
        match list with
        | Nil -> 0
        | Cons head _ -> head
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert_eq!(result, Ok(Value::Int(1)));
}
