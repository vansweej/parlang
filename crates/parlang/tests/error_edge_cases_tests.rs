/// Tests for error handling edge cases and boundary conditions
/// These tests verify proper error handling in various edge cases
use parlang::{eval, parse, Environment, Value};

fn parse_and_eval(input: &str) -> Result<Value, String> {
    let expr = parse(input)?;
    eval(&expr, &Environment::new()).map_err(|e| e.to_string())
}

// ============================================================================
// RECURSIVE FUNCTION ERROR TESTS
// ============================================================================

#[test]
fn test_rec_self_application_type_error() {
    // Self-reference: rec f -> f
    // This parses but fails at evaluation because rec body must be a function
    let code = "rec f -> f";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("function") });
}

#[test]
fn test_rec_infinite_loop_simple() {
    // Simple infinite loop - should not crash immediately
    // We can't really test this without timeout, but we can verify it parses
    let code = "rec loop -> fun x -> loop x";
    let result = parse(code);
    assert!(result.is_ok());
}

// ============================================================================
// PATTERN MATCHING ERROR TESTS
// ============================================================================

#[test]
fn test_pattern_match_no_arms_empty() {
    // Pattern match with no arms should be a parse error
    let code = "match 42 with";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_pattern_match_non_exhaustive_constructor() {
    // Pattern matching on sum type without covering all constructors
    let code = r#"
        type Option a = Some a | None in
        let x = Some 42 in
        match x with
        | Some n -> n
    "#;
    let result = parse_and_eval(code);
    // This should fail at runtime if x is None (non-exhaustive)
    // But with Some 42, it succeeds
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_pattern_match_non_exhaustive_runtime_failure() {
    // Pattern matching fails at runtime when no pattern matches
    let code = r#"
        type Option a = Some a | None in
        let x = None in
        match x with
        | Some n -> n
    "#;
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("non-exhaustive") || err.contains("Pattern match") });
}

#[test]
fn test_pattern_match_undefined_constructor() {
    // Using undefined constructor in pattern
    let code = r#"
        match 42 with
        | UndefinedConstructor -> 1
        | n -> n
    "#;
    let result = parse_and_eval(code);
    // Since UndefinedConstructor is not defined, it's treated as a variable pattern
    // So this should actually succeed with the variable binding
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_pattern_match_constructor_arity_mismatch_too_many_args() {
    // Using constructor with wrong number of arguments in pattern
    let code = r#"
        type Option a = Some a | None in
        let x = Some 42 in
        match x with
        | Some x y -> x + y
        | None -> 0
    "#;
    let result = parse_and_eval(code);
    // This should fail - Some expects 1 arg, pattern has 2
    assert!(result.is_err());
}

#[test]
fn test_pattern_match_nested_constructor_mismatch() {
    // Nested pattern with wrong constructor
    let code = r#"
        type Option a = Some a | None in
        let x = Some (Some 42) in
        match x with
        | Some None -> 0
        | Some (Some n) -> n
        | None -> 0
    "#;
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(42)));
}

// ============================================================================
// EVALUATOR EDGE CASE TESTS
// ============================================================================

#[test]
fn test_eval_unbound_variable() {
    // Variable not in environment
    let code = "x + 1";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("Unbound variable") || err.contains("x") });
}

#[test]
fn test_eval_shadowing_same_name() {
    // Shadowing: inner x should hide outer x
    let code = "let x = 1 in let x = 2 in x";
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(2)));
}

#[test]
fn test_eval_shadowing_multiple_levels() {
    // Multiple levels of shadowing
    let code = "let x = 1 in let x = 2 in let x = 3 in x";
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(3)));
}

#[test]
fn test_eval_closure_captures_environment() {
    // Closure should capture outer scope
    let code = "let x = 10 in let f = fun y -> x + y in f 5";
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(15)));
}

#[test]
fn test_eval_closure_captures_at_definition_time() {
    // Closure captures environment at definition, not call time
    let code = r#"
        let x = 10 in
        let f = fun y -> x + y in
        let x = 20 in
        f 5
    "#;
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(15))); // Should use first x = 10
}

#[test]
fn test_eval_non_function_application() {
    // Attempting to call a non-function
    let code = "42 5";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("Type error") || err.contains("not a function") });
}

#[test]
fn test_eval_tuple_projection_out_of_bounds() {
    // Tuple projection with invalid index
    let code = "(1, 2).5";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("out of bounds") || err.contains("Index") });
}

#[test]
fn test_eval_tuple_projection_negative_implicit() {
    // Negative indices don't exist in parlang (only positive usize)
    // But we can test index 0 which should work
    let code = "(10, 20, 30).0";
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(10)));
}

#[test]
fn test_eval_tuple_projection_last_element() {
    // Projecting last element
    let code = "(1, 2, 3, 4, 5).4";
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(5)));
}

#[test]
fn test_eval_empty_tuple_projection() {
    // Empty tuple projection
    let code = "().0";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("out of bounds") || err.contains("Index") });
}

// ============================================================================
// RECORD ERROR TESTS
// ============================================================================

#[test]
fn test_record_access_nonexistent_field() {
    // Accessing field that doesn't exist
    let code = "{ x: 1, y: 2 }.z";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("not found") || err.contains("Field") });
}

#[test]
fn test_record_access_nested_nonexistent() {
    // Nested field access where inner field doesn't exist
    let code = "{ outer: { inner: 42 } }.outer.nonexistent";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("not found") || err.contains("Field") });
}

#[test]
fn test_record_access_on_non_record() {
    // Attempting field access on non-record
    let code = "42.field";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("Expected record") || err.contains("Type error") });
}

#[test]
fn test_record_pattern_extra_fields_allowed() {
    // Pattern with subset of fields should match record with extra fields
    let code = r#"
        let person = { name: 42, age: 30, city: 100 } in
        match person with
        | { name: n } -> n
    "#;
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_record_pattern_nonexistent_field() {
    // Pattern requires field that doesn't exist
    let code = r#"
        let person = { name: 42, age: 30 } in
        match person with
        | { name: n, city: c } -> c
    "#;
    let result = parse_and_eval(code);
    assert!(result.is_err());
    // Should fail because city field doesn't exist
}

#[test]
fn test_record_empty_construction() {
    // Empty record construction
    let code = "{}";
    let result = parse_and_eval(code);
    assert!(result.is_ok());
}

#[test]
fn test_record_field_ordering_irrelevant() {
    // Records with same fields in different order should be equal
    let code = r#"
        let r1 = { x: 1, y: 2 } in
        let r2 = { y: 2, x: 1 } in
        r1.x == r2.x
    "#;
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Bool(true)));
}

// ============================================================================
// CONSTRUCTOR ERROR TESTS
// ============================================================================

#[test]
fn test_constructor_undefined_at_runtime() {
    // Using undefined constructor
    let code = "UndefinedConstructor 42";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("Unknown constructor") || err.contains("Unbound") });
}

#[test]
fn test_constructor_arity_mismatch_too_many() {
    // Constructor called with too many arguments
    let code = r#"
        type Option a = Some a | None in
        Some 1 2
    "#;
    let result = parse_and_eval(code);
    // This might parse as (Some 1) 2, which would fail as non-function application
    assert!(result.is_err());
}

#[test]
fn test_constructor_arity_mismatch_none_with_arg() {
    // None constructor doesn't take arguments
    let code = r#"
        type Option a = Some a | None in
        None 42
    "#;
    let result = parse_and_eval(code);
    // None returns a value, then trying to apply 42 to it fails
    assert!(result.is_err());
}

// ============================================================================
// PARSER EDGE CASE TESTS
// ============================================================================

#[test]
fn test_parse_empty_input() {
    // Empty input defaults to 0 in parlang
    let code = "";
    let result = parse(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), parse("0").unwrap());
}

#[test]
fn test_parse_whitespace_only() {
    // Whitespace-only input also defaults to 0
    let code = "   \n\t  ";
    let result = parse(code);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), parse("0").unwrap());
}

#[test]
fn test_parse_deeply_nested_parens() {
    // Deeply nested parentheses
    let code = "((((((((((42))))))))))";
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_parse_deeply_nested_function_application() {
    // Deeply nested function applications
    let code = r#"
        let id = fun x -> x in
        id (id (id (id (id 42))))
    "#;
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(42)));
}

#[test]
fn test_parse_deeply_nested_let_bindings() {
    // Deep nesting of let bindings
    let code = r#"
        let a = 1 in
        let b = a + 1 in
        let c = b + 1 in
        let d = c + 1 in
        let e = d + 1 in
        e
    "#;
    let result = parse_and_eval(code);
    assert_eq!(result, Ok(Value::Int(5)));
}

#[test]
fn test_parse_unclosed_record() {
    // Unclosed record should fail to parse
    let code = "{ x: 1, y: 2";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_parse_unclosed_tuple() {
    // Unclosed tuple should fail to parse
    let code = "(1, 2, 3";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_parse_unclosed_match() {
    // Unclosed match expression
    let code = "match 42 with | 0 -> 1";
    // Missing closing or additional arms - depends on parser, might be valid
    let result = parse(code);
    // Should parse successfully as a complete match expression
    assert!(result.is_ok());
}

// ============================================================================
// TYPE MISMATCH TESTS (runtime)
// ============================================================================

#[test]
fn test_type_mismatch_adding_bool_and_int() {
    // Runtime type error: adding incompatible types
    let code = "1 + true";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("Type error") || err.contains("type") });
}

#[test]
fn test_type_mismatch_if_non_bool_condition() {
    // If with non-boolean condition
    let code = "if 1 then 2 else 3";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("Type error") || err.contains("type") });
}

#[test]
fn test_type_mismatch_comparison_incompatible_types() {
    // Can't compare different types
    let code = "1 == true";
    let result = parse_and_eval(code);
    assert!(result.is_err());
    assert!({ let err = result.unwrap_err(); err.contains("Type error") || err.contains("type") });
}
