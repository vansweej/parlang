/// Tests for Byte type support
use parlang::{parse, eval, typecheck, Environment, Value};

// Parser tests for Byte literals

#[test]
fn test_parse_byte_zero() {
    let expr = parse("0b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(0)));
}

#[test]
fn test_parse_byte_one() {
    let expr = parse("1b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(1)));
}

#[test]
fn test_parse_byte_max() {
    let expr = parse("255b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(255)));
}

#[test]
fn test_parse_byte_mid_range() {
    let expr = parse("128b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(128)));
}

#[test]
fn test_parse_byte_various_values() {
    let test_cases = vec![
        ("0b", 0u8),
        ("1b", 1u8),
        ("10b", 10u8),
        ("42b", 42u8),
        ("100b", 100u8),
        ("200b", 200u8),
        ("255b", 255u8),
    ];
    
    let env = Environment::new();
    for (input, expected) in test_cases {
        let expr = parse(input).unwrap();
        assert_eq!(eval(&expr, &env), Ok(Value::Byte(expected)), "Failed for input: {}", input);
    }
}

// Error cases: out of range

#[test]
fn test_byte_out_of_range_256() {
    // 256b will parse as 256 followed by variable 'b', which will fail during evaluation
    let result = parse("256b");
    if result.is_ok() {
        // If it parses (as 256 followed by b), it should fail during eval due to unbound variable
        let env = Environment::new();
        let result = eval(&result.unwrap(), &env);
        assert!(result.is_err(), "Should fail to evaluate 256b");
    }
    // If byte parser rejects it, that's also ok
}

#[test]
fn test_byte_out_of_range_1000() {
    // 1000b will parse as 1000 followed by variable 'b', which will fail during evaluation
    let result = parse("1000b");
    if result.is_ok() {
        // If it parses (as 1000 followed by b), it should fail during eval due to unbound variable
        let env = Environment::new();
        let result = eval(&result.unwrap(), &env);
        assert!(result.is_err(), "Should fail to evaluate 1000b");
    }
    // If byte parser rejects it, that's also ok
}

#[test]
fn test_byte_out_of_range_999999() {
    // 999999b will parse as 999999 followed by variable 'b', which will fail during evaluation
    let result = parse("999999b");
    if result.is_ok() {
        // If it parses (as 999999 followed by b), it should fail during eval due to unbound variable
        let env = Environment::new();
        let result = eval(&result.unwrap(), &env);
        assert!(result.is_err(), "Should fail to evaluate 999999b");
    }
    // If byte parser rejects it, that's also ok
}

// Arithmetic operations with Byte

#[test]
fn test_byte_addition() {
    let expr = parse("10b + 20b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(30)));
}

#[test]
fn test_byte_addition_zero() {
    let expr = parse("0b + 0b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(0)));
}

#[test]
fn test_byte_addition_max_components() {
    let expr = parse("100b + 155b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(255)));
}

#[test]
fn test_byte_addition_overflow() {
    let expr = parse("200b + 100b").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(result.is_err(), "Should overflow when adding 200 + 100");
}

#[test]
fn test_byte_subtraction() {
    let expr = parse("30b - 10b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(20)));
}

#[test]
fn test_byte_subtraction_to_zero() {
    let expr = parse("42b - 42b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(0)));
}

#[test]
fn test_byte_subtraction_underflow() {
    let expr = parse("10b - 20b").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(result.is_err(), "Should underflow when subtracting 20 from 10");
}

#[test]
fn test_byte_multiplication() {
    let expr = parse("10b * 5b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(50)));
}

#[test]
fn test_byte_multiplication_by_zero() {
    let expr = parse("100b * 0b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(0)));
}

#[test]
fn test_byte_multiplication_by_one() {
    let expr = parse("42b * 1b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(42)));
}

#[test]
fn test_byte_multiplication_overflow() {
    let expr = parse("20b * 20b").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env);
    assert!(result.is_err(), "Should overflow when multiplying 20 * 20 = 400");
}

#[test]
fn test_byte_division() {
    let expr = parse("100b / 5b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(20)));
}

#[test]
fn test_byte_division_truncation() {
    let expr = parse("10b / 3b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(3)));
}

#[test]
fn test_byte_division_by_one() {
    let expr = parse("42b / 1b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(42)));
}

#[test]
fn test_byte_division_by_zero() {
    let expr = parse("42b / 0b").unwrap();
    let env = Environment::new();
    assert!(eval(&expr, &env).is_err(), "Should error on division by zero");
}

#[test]
fn test_byte_complex_expression() {
    let expr = parse("10b + 20b * 2b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(50)));
}

// Comparison operations with Byte

#[test]
fn test_byte_equality() {
    let expr = parse("42b == 42b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_equality_false() {
    let expr = parse("42b == 43b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
}

#[test]
fn test_byte_inequality() {
    let expr = parse("42b != 43b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_inequality_false() {
    let expr = parse("42b != 42b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
}

#[test]
fn test_byte_less_than() {
    let expr = parse("10b < 20b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_less_than_false() {
    let expr = parse("20b < 10b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
}

#[test]
fn test_byte_less_than_equal() {
    let expr = parse("10b <= 10b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_less_than_equal_less() {
    let expr = parse("10b <= 20b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_greater_than() {
    let expr = parse("20b > 10b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_greater_than_false() {
    let expr = parse("10b > 20b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(false)));
}

#[test]
fn test_byte_greater_than_equal() {
    let expr = parse("10b >= 10b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_greater_than_equal_greater() {
    let expr = parse("20b >= 10b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

// Let bindings with Byte

#[test]
fn test_byte_let_binding() {
    let expr = parse("let x = 42b in x").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(42)));
}

#[test]
fn test_byte_let_binding_arithmetic() {
    let expr = parse("let x = 10b in let y = 20b in x + y").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(30)));
}

#[test]
fn test_byte_let_binding_expression() {
    let expr = parse("let x = 5b * 2b in x + 10b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(20)));
}

// Functions with Byte

#[test]
fn test_byte_function() {
    let expr = parse("(fun x -> x) 42b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(42)));
}

#[test]
fn test_byte_function_arithmetic() {
    let expr = parse("(fun x -> x + 10b) 32b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(42)));
}

#[test]
fn test_byte_function_comparison() {
    let expr = parse("(fun x -> x > 100b) 150b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

// Type checking

#[test]
fn test_byte_type_inference() {
    let expr = parse("42b").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type checking should succeed for byte literal");
}

#[test]
fn test_byte_type_inference_arithmetic() {
    let expr = parse("10b + 20b").unwrap();
    let result = typecheck(&expr);
    match &result {
        Ok(_) => {},
        Err(e) => panic!("Type checking failed with error: {:?}", e),
    }
    assert!(result.is_ok(), "Type checking should succeed for byte arithmetic");
}

#[test]
fn test_byte_type_inference_function() {
    let expr = parse("fun x -> x + 1b").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type checking should succeed for byte function");
}

// Display tests

#[test]
fn test_byte_display() {
    let value = Value::Byte(42);
    assert_eq!(format!("{}", value), "42b");
}

#[test]
fn test_byte_display_zero() {
    let value = Value::Byte(0);
    assert_eq!(format!("{}", value), "0b");
}

#[test]
fn test_byte_display_max() {
    let value = Value::Byte(255);
    assert_eq!(format!("{}", value), "255b");
}

// If expressions with Byte

#[test]
fn test_byte_if_expression() {
    let expr = parse("if 10b < 20b then 1b else 0b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(1)));
}

#[test]
fn test_byte_if_expression_false() {
    let expr = parse("if 30b < 20b then 1b else 0b").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(0)));
}

// Pattern matching with Byte

#[test]
fn test_byte_match_literal() {
    let expr = parse("match 42b with | 42b -> true | _ -> false").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_match_wildcard() {
    let expr = parse("match 99b with | _ -> true").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_byte_match_variable() {
    let expr = parse("match 42b with | x -> x").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Byte(42)));
}

// Boundary value tests

#[test]
fn test_byte_boundary_values() {
    let test_cases = vec![
        ("0b", 0u8),
        ("1b", 1u8),
        ("127b", 127u8),
        ("128b", 128u8),
        ("254b", 254u8),
        ("255b", 255u8),
    ];
    
    let env = Environment::new();
    for (input, expected) in test_cases {
        let expr = parse(input).unwrap();
        assert_eq!(eval(&expr, &env), Ok(Value::Byte(expected)), "Failed for input: {}", input);
    }
}
