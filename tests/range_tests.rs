/// Tests for Range type functionality
use parlang::{parse, eval, typecheck, Environment, Value};

fn parse_and_eval(input: &str) -> Result<Value, String> {
    let expr = parse(input)?;
    eval(&expr, &Environment::new()).map_err(|e| e.to_string())
}

fn parse_and_typecheck(input: &str) -> Result<String, String> {
    let expr = parse(input)?;
    typecheck(&expr)
        .map(|ty| format!("{}", ty))
        .map_err(|e| e.to_string())
}

// Basic range creation tests
#[test]
fn test_simple_range() {
    assert_eq!(
        parse_and_eval("1..10"),
        Ok(Value::Range(1, 10))
    );
}

#[test]
fn test_range_with_negative_start() {
    assert_eq!(
        parse_and_eval("-5..5"),
        Ok(Value::Range(-5, 5))
    );
}

#[test]
fn test_range_with_negative_end() {
    assert_eq!(
        parse_and_eval("0..-10"),
        Ok(Value::Range(0, -10))
    );
}

#[test]
fn test_range_both_negative() {
    assert_eq!(
        parse_and_eval("-10..-1"),
        Ok(Value::Range(-10, -1))
    );
}

#[test]
fn test_range_zero_to_zero() {
    assert_eq!(
        parse_and_eval("0..0"),
        Ok(Value::Range(0, 0))
    );
}

#[test]
fn test_range_descending() {
    assert_eq!(
        parse_and_eval("10..1"),
        Ok(Value::Range(10, 1))
    );
}

#[test]
fn test_range_large_numbers() {
    assert_eq!(
        parse_and_eval("1000..2000"),
        Ok(Value::Range(1000, 2000))
    );
}

// Range with expressions
#[test]
fn test_range_with_arithmetic() {
    assert_eq!(
        parse_and_eval("1+1..10*2"),
        Ok(Value::Range(2, 20))
    );
}

#[test]
fn test_range_with_variables() {
    assert_eq!(
        parse_and_eval("let start = 5 in let end = 15 in start..end"),
        Ok(Value::Range(5, 15))
    );
}

#[test]
fn test_range_with_complex_expressions() {
    assert_eq!(
        parse_and_eval("let x = 10 in (x - 5)..(x + 5)"),
        Ok(Value::Range(5, 15))
    );
}

// Range in let bindings
#[test]
fn test_range_in_let_binding() {
    assert_eq!(
        parse_and_eval("let r = 1..10 in r"),
        Ok(Value::Range(1, 10))
    );
}

#[test]
fn test_multiple_ranges() {
    assert_eq!(
        parse_and_eval("let r1 = 1..5 in let r2 = 10..20 in r2"),
        Ok(Value::Range(10, 20))
    );
}

// Range with functions
#[test]
fn test_range_from_function() {
    assert_eq!(
        parse_and_eval("let makeRange = fun start -> fun end -> start..end in makeRange 1 10"),
        Ok(Value::Range(1, 10))
    );
}

#[test]
fn test_function_returning_range() {
    assert_eq!(
        parse_and_eval("let tenRange = fun n -> 0..n in tenRange 10"),
        Ok(Value::Range(0, 10))
    );
}

// Range display
#[test]
fn test_range_display() {
    let range = Value::Range(1, 10);
    assert_eq!(format!("{}", range), "1..10");
}

#[test]
fn test_range_display_negative() {
    let range = Value::Range(-5, 5);
    assert_eq!(format!("{}", range), "-5..5");
}

// Type checking tests
#[test]
fn test_range_type_inference() {
    assert_eq!(parse_and_typecheck("1..10"), Ok("Range".to_string()));
}

#[test]
fn test_range_type_with_variables() {
    assert_eq!(
        parse_and_typecheck("let start = 1 in let end = 10 in start..end"),
        Ok("Range".to_string())
    );
}

#[test]
fn test_range_type_in_function() {
    let result = parse_and_typecheck("fun start -> fun end -> start..end");
    assert!(result.is_ok());
    let ty = result.unwrap();
    assert!(ty.contains("->") && ty.contains("Range"));
}

#[test]
fn test_range_type_with_let() {
    assert_eq!(
        parse_and_typecheck("let r = 1..10 in r"),
        Ok("Range".to_string())
    );
}

// Error cases - type errors
#[test]
fn test_range_with_non_integer_start() {
    let result = parse_and_typecheck("true..10");
    assert!(result.is_err());
}

#[test]
fn test_range_with_non_integer_end() {
    let result = parse_and_typecheck("1..false");
    assert!(result.is_err());
}

#[test]
fn test_range_with_both_non_integers() {
    let result = parse_and_typecheck("true..false");
    assert!(result.is_err());
}

#[test]
fn test_range_with_float_start() {
    let result = parse_and_typecheck("1.5..10");
    assert!(result.is_err());
}

#[test]
fn test_range_with_float_end() {
    let result = parse_and_typecheck("1..10.5");
    assert!(result.is_err());
}

// Parsing tests
#[test]
fn test_range_parsing() {
    let expr = parse("1..10");
    assert!(expr.is_ok());
}

#[test]
fn test_range_parsing_with_spaces() {
    let expr = parse("1 .. 10");
    assert!(expr.is_ok());
}

#[test]
fn test_range_parsing_without_spaces() {
    let expr = parse("1..10");
    assert!(expr.is_ok());
}

#[test]
fn test_nested_range_expressions() {
    let expr = parse("(1+1)..(10-1)");
    assert!(expr.is_ok());
}

// Range precedence tests
#[test]
fn test_range_precedence_with_addition() {
    // Should parse as (1+2)..(3+4) = 3..7
    assert_eq!(
        parse_and_eval("1+2..3+4"),
        Ok(Value::Range(3, 7))
    );
}

#[test]
fn test_range_precedence_with_multiplication() {
    // Should parse as (2*3)..(4*5) = 6..20
    assert_eq!(
        parse_and_eval("2*3..4*5"),
        Ok(Value::Range(6, 20))
    );
}

#[test]
fn test_range_with_comparison() {
    // Range has higher precedence than comparison
    let result = parse_and_typecheck("1..10 == 1..10");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Bool");
}

// Range equality
#[test]
fn test_range_equality_true() {
    assert_eq!(
        parse_and_eval("1..10 == 1..10"),
        Ok(Value::Bool(true))
    );
}

#[test]
fn test_range_equality_false_different_start() {
    assert_eq!(
        parse_and_eval("1..10 == 2..10"),
        Ok(Value::Bool(false))
    );
}

#[test]
fn test_range_equality_false_different_end() {
    assert_eq!(
        parse_and_eval("1..10 == 1..11"),
        Ok(Value::Bool(false))
    );
}

#[test]
fn test_range_inequality() {
    assert_eq!(
        parse_and_eval("1..10 != 2..10"),
        Ok(Value::Bool(true))
    );
}

// Edge cases
#[test]
fn test_range_very_large() {
    assert_eq!(
        parse_and_eval("0..1000000"),
        Ok(Value::Range(0, 1000000))
    );
}

#[test]
fn test_range_max_int() {
    // Test with large numbers (but not MAX to avoid overflow)
    assert_eq!(
        parse_and_eval("1000000..2000000"),
        Ok(Value::Range(1000000, 2000000))
    );
}

// Range in if expressions
#[test]
fn test_range_in_if_then() {
    assert_eq!(
        parse_and_eval("if true then 1..10 else 5..15"),
        Ok(Value::Range(1, 10))
    );
}

#[test]
fn test_range_in_if_else() {
    assert_eq!(
        parse_and_eval("if false then 1..10 else 5..15"),
        Ok(Value::Range(5, 15))
    );
}

#[test]
fn test_range_type_in_if() {
    assert_eq!(
        parse_and_typecheck("if true then 1..10 else 5..15"),
        Ok("Range".to_string())
    );
}

// Sequential let bindings with ranges
#[test]
fn test_range_in_sequential_let() {
    assert_eq!(
        parse_and_eval("let x = 1; let y = 10; x..y"),
        Ok(Value::Range(1, 10))
    );
}

#[test]
fn test_multiple_ranges_sequential() {
    assert_eq!(
        parse_and_eval("let r1 = 1..5; let r2 = 10..20; r2"),
        Ok(Value::Range(10, 20))
    );
}

// Recursive function with ranges
#[test]
fn test_range_in_recursive_function() {
    let result = parse_and_eval(
        "let makeRange = rec f -> fun n -> if n == 0 then 0..0 else 0..n in makeRange 10"
    );
    assert_eq!(result, Ok(Value::Range(0, 10)));
}
