/// Unit tests for reference/pointer types
use parlang::{parse, eval, Environment, Value, EvalError, typecheck};

#[test]
fn test_ref_creation() {
    // Test creating a reference to an integer
    let expr = parse("ref 42").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    // Should be a reference
    match result {
        Value::Reference(_, cell) => {
            assert_eq!(*cell.borrow(), Value::Int(42));
        }
        _ => panic!("Expected a reference"),
    }
}

#[test]
fn test_ref_deref() {
    // Test creating and dereferencing a reference
    let expr = parse("!(ref 42)").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_ref_assignment() {
    // Test assigning to a reference
    let expr = parse("let r = ref 10 in r := 20").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    // Assignment returns unit
    assert_eq!(result, Value::Tuple(vec![]));
}

#[test]
fn test_ref_assignment_with_deref() {
    // Test assigning and then dereferencing
    let expr = parse("let r = ref 10 in let dummy = r := 20 in !r").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_ref_bool() {
    // Test reference with boolean
    let expr = parse("let r = ref true in !r").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_ref_float() {
    // Test reference with float
    let expr = parse("let r = ref 3.14 in !r").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Float(3.14));
}

#[test]
fn test_ref_in_function() {
    // Test passing a reference to a function
    let code = r"
        let increment = fun r -> r := !r + 1 in
        let x = ref 5 in
        let dummy = increment x in
        !x
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_counter() {
    // Test a simple counter using references
    let code = r"
        let counter = ref 0 in
        let increment = fun x -> counter := !counter + 1 in
        let dummy1 = increment 0 in
        let dummy2 = increment 0 in
        let dummy3 = increment 0 in
        !counter
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_ref_closure_capture() {
    // Test that closures can capture and mutate references
    let code = r"
        let make_counter = fun initial ->
            let count = ref initial in
            fun x -> (
                let old = !count in
                let dummy = count := old + 1 in
                old
            )
        in
        let counter = make_counter 10 in
        let a = counter 0 in
        let b = counter 0 in
        let c = counter 0 in
        a + b + c
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    // Should be 10 + 11 + 12 = 33
    assert_eq!(result, Value::Int(33));
}

#[test]
fn test_ref_tuple() {
    // Test reference to a tuple
    let code = r"
        let r = ref (1, 2) in
        !r
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Tuple(vec![Value::Int(1), Value::Int(2)]));
}

#[test]
fn test_multiple_refs() {
    // Test multiple independent references
    let code = r"
        let r1 = ref 1 in
        let r2 = ref 2 in
        let dummy = r1 := 10 in
        let dummy = r2 := 20 in
        !r1 + !r2
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_ref_aliasing() {
    // Test that two variables can reference the same reference
    let code = r"
        let r = ref 5 in
        let alias = r in
        let dummy = r := 10 in
        !alias
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    // Both r and alias point to the same reference
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_deref_non_reference() {
    // Test that dereferencing a non-reference fails
    let expr = parse("!42").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    
    assert!(matches!(result, Err(EvalError::TypeError(_))));
}

#[test]
fn test_assign_non_reference() {
    // Test that assigning to a non-reference fails
    let expr = parse("42 := 10").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    
    assert!(matches!(result, Err(EvalError::TypeError(_))));
}

// Type checking tests
#[test]
fn test_ref_type() {
    // Test type of ref expr
    let expr = parse("ref 42").unwrap();
    let ty = typecheck(&expr).unwrap();
    
    assert_eq!(ty.to_string(), "Ref Int");
}

#[test]
fn test_deref_type() {
    // Test type of deref expr
    let expr = parse("!(ref 42)").unwrap();
    let ty = typecheck(&expr).unwrap();
    
    assert_eq!(ty.to_string(), "Int");
}

#[test]
fn test_ref_assign_type() {
    // Test type of assignment expr
    let expr = parse("let r = ref 10 in r := 20").unwrap();
    let ty = typecheck(&expr).unwrap();
    
    // Assignment returns unit type
    assert_eq!(ty.to_string(), "()");
}

#[test]
fn test_ref_polymorphic() {
    // Test polymorphic reference in function
    let expr = parse("fun x -> ref x").unwrap();
    let ty = typecheck(&expr).unwrap();
    
    // Should be: t0 -> Ref t0
    assert!(ty.to_string().contains("Ref"));
}

#[test]
fn test_ref_type_mismatch() {
    // Test that assigning wrong type fails type checking
    let expr = parse("let r = ref 10 in r := true").unwrap();
    let result = typecheck(&expr);
    
    assert!(result.is_err());
}

#[test]
fn test_deref_type_inference() {
    // Test that type inference works with dereferencing
    let code = r"
        let r = ref 42 in
        !r + 1
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    
    assert_eq!(ty.to_string(), "Int");
}

#[test]
fn test_ref_in_record() {
    // Test reference inside a record
    let code = r"
        let r = { count: ref 0 } in
        let dummy = r.count := 5 in
        !r.count
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_swap_function() {
    // Test a swap function using references
    let code = r"
        let swap = fun r1 -> fun r2 ->
            let temp = !r1 in
            let dummy = r1 := !r2 in
            r2 := temp
        in
        let x = ref 10 in
        let y = ref 20 in
        let dummy = swap x y in
        (!x, !y)
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Tuple(vec![Value::Int(20), Value::Int(10)]));
}

#[test]
fn test_accumulator() {
    // Test accumulator pattern with references
    let code = r"
        let accumulator = fun initial ->
            let sum = ref initial in
            fun x -> (
                let dummy = sum := !sum + x in
                !sum
            )
        in
        let acc = accumulator 0 in
        let a = acc 5 in
        let b = acc 10 in
        let c = acc 15 in
        c
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    // Should be 0 + 5 + 10 + 15 = 30
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_ref_with_if() {
    // Test references with conditional expressions
    let code = r"
        let r = ref 5 in
        let dummy = if !r > 0 then r := !r * 2 else r := 0 in
        !r
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_ref_recursive() {
    // Test references in recursive functions
    let code = r"
        let count_down = rec f -> fun r ->
            if !r == 0
            then 0
            else (
                let dummy = r := !r - 1 in
                1 + f r
            )
        in
        let counter = ref 5 in
        count_down counter
    ";
    let expr = parse(code).unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    
    assert_eq!(result, Value::Int(5));
}
