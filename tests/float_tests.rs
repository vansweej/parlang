/// Tests for floating point type support
use parlang::{parse, eval, typecheck, Environment, Value};

// Parser tests for Float literals

#[test]
fn test_parse_float_positive() {
    let expr = parse("3.14").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(3.14)));
}

#[test]
fn test_parse_float_negative() {
    let expr = parse("-2.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(-2.5)));
}

#[test]
fn test_parse_float_zero() {
    let expr = parse("0.0").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(0.0)));
}

#[test]
fn test_parse_float_small() {
    let expr = parse("0.001").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(0.001)));
}

#[test]
fn test_parse_float_large() {
    let expr = parse("123456.789").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(123456.789)));
}

// Arithmetic operations with Float

#[test]
fn test_float_addition() {
    let expr = parse("1.5 + 2.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(4.0)));
}

#[test]
fn test_float_subtraction() {
    let expr = parse("5.0 - 2.0").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(3.0)));
}

#[test]
fn test_float_multiplication() {
    let expr = parse("2.5 * 4.0").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(10.0)));
}

#[test]
fn test_float_division() {
    let expr = parse("10.0 / 4.0").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(2.5)));
}

#[test]
fn test_float_division_by_zero() {
    let expr = parse("10.0 / 0.0").unwrap();
    let env = Environment::new();
    assert!(eval(&expr, &env).is_err());
}

#[test]
fn test_float_complex_expression() {
    let expr = parse("1.5 + 2.5 * 3.0").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(9.0)));
}

// Comparison operations with Float

#[test]
fn test_float_equality() {
    let expr = parse("3.14 == 3.14").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_float_inequality() {
    let expr = parse("3.14 != 2.71").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_float_less_than() {
    let expr = parse("2.5 < 3.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_float_less_than_or_equal() {
    let expr = parse("2.5 <= 2.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_float_greater_than() {
    let expr = parse("3.5 > 2.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

#[test]
fn test_float_greater_than_or_equal() {
    let expr = parse("3.5 >= 3.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Bool(true)));
}

// Type checking tests

#[test]
fn test_typecheck_float_literal() {
    let expr = parse("3.14").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(format!("{}", ty), "Float");
}

#[test]
fn test_typecheck_float_addition() {
    let expr = parse("1.5 + 2.5").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(format!("{}", ty), "Float");
}

#[test]
fn test_typecheck_float_comparison() {
    let expr = parse("1.5 < 2.5").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(format!("{}", ty), "Bool");
}

// Let bindings with Float

#[test]
fn test_float_let_binding() {
    let expr = parse("let x = 3.14 in x + 1.0").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    // Use approximate comparison for floating point
    if let Value::Float(f) = result {
        assert!((f - 4.14).abs() < 0.0001);
    } else {
        panic!("Expected Float value");
    }
}

#[test]
fn test_float_function() {
    let expr = parse("let double = fun x -> x + x in double 2.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(5.0)));
}

#[test]
fn test_typecheck_float_function() {
    let expr = parse("fun x -> x + x").unwrap();
    let ty = typecheck(&expr).unwrap();
    // Since x + x requires x to be numeric, type checker should infer Int by default
    // but we can test with explicit Float usage
    let expr2 = parse("fun x -> x + 1.0").unwrap();
    let ty2 = typecheck(&expr2).unwrap();
    assert_eq!(format!("{}", ty2), "Float -> Float");
}

// Mixed type operations should fail

#[test]
fn test_cannot_add_int_and_float() {
    let expr = parse("1 + 2.5").unwrap();
    let env = Environment::new();
    // This should fail at evaluation
    assert!(eval(&expr, &env).is_err());
}

#[test]
fn test_typecheck_cannot_add_int_and_float() {
    let expr = parse("1 + 2.5").unwrap();
    // This should fail type checking
    assert!(typecheck(&expr).is_err());
}

// If expressions with Float

#[test]
fn test_float_in_if_expression() {
    let expr = parse("if true then 1.5 else 2.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(1.5)));
}

#[test]
fn test_typecheck_float_in_if_expression() {
    let expr = parse("if true then 1.5 else 2.5").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(format!("{}", ty), "Float");
}

// Record with Float fields

#[test]
fn test_float_in_record() {
    let expr = parse("{ pi: 3.14, e: 2.71 }").unwrap();
    let env = Environment::new();
    let result = eval(&expr, &env).unwrap();
    match result {
        Value::Record(fields) => {
            assert_eq!(fields.get("pi"), Some(&Value::Float(3.14)));
            assert_eq!(fields.get("e"), Some(&Value::Float(2.71)));
        }
        _ => panic!("Expected record"),
    }
}

#[test]
fn test_float_record_field_access() {
    let expr = parse("let r = { pi: 3.14 } in r.pi").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(3.14)));
}

// Tuple with Float elements

#[test]
fn test_float_in_tuple() {
    let expr = parse("(1.5, 2.5, 3.5)").unwrap();
    let env = Environment::new();
    assert_eq!(
        eval(&expr, &env),
        Ok(Value::Tuple(vec![
            Value::Float(1.5),
            Value::Float(2.5),
            Value::Float(3.5)
        ]))
    );
}

#[test]
fn test_float_tuple_projection() {
    let expr = parse("let t = (1.5, 2.5) in t.0").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(1.5)));
}

// Negative numbers

#[test]
fn test_float_negative_arithmetic() {
    let expr = parse("-3.5 + 1.5").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(-2.0)));
}

#[test]
fn test_float_negative_multiplication() {
    let expr = parse("-2.5 * 4.0").unwrap();
    let env = Environment::new();
    assert_eq!(eval(&expr, &env), Ok(Value::Float(-10.0)));
}
