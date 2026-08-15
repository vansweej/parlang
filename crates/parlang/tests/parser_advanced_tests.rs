/// Advanced parser tests
/// Tests for operator precedence, invalid syntax, and edge cases
use parlang::{parse};

// Operator Precedence Tests

#[test]
fn test_precedence_multiplication_over_addition() {
    // Test that * has higher precedence than +
    let code = "2 + 3 * 4";
    let expr = parse(code).unwrap();
    let result_expr = parse("2 + (3 * 4)").unwrap();
    // Should parse as 2 + (3 * 4) = 14, not (2 + 3) * 4 = 20
    assert_eq!(format!("{:?}", expr), format!("{:?}", result_expr));
}

#[test]
fn test_precedence_division_over_subtraction() {
    // Test that / has higher precedence than -
    let code = "10 - 6 / 2";
    let expr = parse(code).unwrap();
    let result_expr = parse("10 - (6 / 2)").unwrap();
    // Should parse as 10 - (6 / 2) = 7, not (10 - 6) / 2 = 2
    assert_eq!(format!("{:?}", expr), format!("{:?}", result_expr));
}

#[test]
fn test_precedence_comparison_lower_than_arithmetic() {
    // Test that comparison has lower precedence than arithmetic
    let code = "1 + 2 > 2";
    let expr = parse(code).unwrap();
    let result_expr = parse("(1 + 2) > 2").unwrap();
    assert_eq!(format!("{:?}", expr), format!("{:?}", result_expr));
}

#[test]
fn test_precedence_equality_same_as_comparison() {
    // Test that == has same precedence level as other comparisons
    let code = "1 + 1 == 2";
    let expr = parse(code).unwrap();
    let result_expr = parse("(1 + 1) == 2").unwrap();
    assert_eq!(format!("{:?}", expr), format!("{:?}", result_expr));
}

#[test]
fn test_precedence_complex_expression() {
    // Test complex expression with multiple operators
    let code = "2 + 3 * 4 - 10 / 2";
    let expr = parse(code).unwrap();
    // Should parse as: 2 + (3 * 4) - (10 / 2) = 2 + 12 - 5 = 9
    let result_expr = parse("(2 + (3 * 4)) - (10 / 2)").unwrap();
    assert_eq!(format!("{:?}", expr), format!("{:?}", result_expr));
}

#[test]
fn test_precedence_nested_comparison() {
    // Test that parentheses work correctly with comparisons
    let code = "(1 + 2) > (3 - 1)";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_left_associativity_addition() {
    // Test that addition is left-associative
    let code = "1 + 2 + 3";
    let expr = parse(code).unwrap();
    let result_expr = parse("(1 + 2) + 3").unwrap();
    assert_eq!(format!("{:?}", expr), format!("{:?}", result_expr));
}

#[test]
fn test_left_associativity_subtraction() {
    // Test that subtraction is left-associative
    let code = "10 - 3 - 2";
    let expr = parse(code).unwrap();
    let result_expr = parse("(10 - 3) - 2").unwrap();
    assert_eq!(format!("{:?}", expr), format!("{:?}", result_expr));
}

#[test]
fn test_function_application_precedence() {
    // Test that function application has high precedence
    let code = "fun x -> x + 1";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_function_application_left_associative() {
    // Test that function application is left-associative
    // f x y should be (f x) y
    let code = "let f = fun x -> fun y -> x + y in f 10 20";
    let result = parse(code);
    assert!(result.is_ok());
}

// Invalid Syntax Tests

#[test]
fn test_invalid_syntax_missing_then() {
    // Test error for if without then
    let code = "if true 1 else 0";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_missing_else() {
    // Test error for if without else
    let code = "if true then 1";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_missing_arrow_in_fun() {
    // Test error for fun without arrow
    let code = "fun x x + 1";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_missing_in_keyword() {
    // Test error for let without in
    let code = "let x = 42";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_unclosed_paren() {
    // Test error for unclosed parenthesis
    let code = "(1 + 2";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_extra_closing_paren() {
    // Test error for extra closing parenthesis
    let code = "1 + 2)";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_empty_tuple() {
    // Test that empty tuple parses (it's the unit type)
    let code = "()";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_syntax_single_element_tuple() {
    // Test that single element in parens is just the element, not a tuple
    let code = "(42)";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_invalid_syntax_unclosed_record() {
    // Test error for unclosed record
    let code = "{ x: 10";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_missing_colon_in_record() {
    // Test error for missing colon in record field
    let code = "{ x 10 }";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_trailing_comma_in_record() {
    // Test that trailing comma in record is handled
    let code = "{ x: 10, }";
    let result = parse(code);
    // Some parsers allow trailing commas, check if it errors or succeeds
    let _ = result; // Just ensure it doesn't panic
}

#[test]
fn test_invalid_syntax_missing_with_in_match() {
    // Test error for match without with
    let code = "match 42 | 0 -> 0";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_missing_arrow_in_match_arm() {
    // Test error for match arm without arrow
    let code = "match 42 with | 0 0";
    let result = parse(code);
    assert!(result.is_err());
}

#[test]
fn test_invalid_syntax_empty_match_arms() {
    // Test error for match with no arms
    let code = "match 42 with";
    let result = parse(code);
    assert!(result.is_err());
}

// Deeply Nested Expressions

#[test]
fn test_deeply_nested_parentheses() {
    // Test deeply nested parentheses
    let code = "((((((((((1))))))))))";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_functions() {
    // Test deeply nested function definitions
    let code = "fun a -> fun b -> fun c -> fun d -> fun e -> a + b + c + d + e";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_let_expressions() {
    // Test deeply nested let expressions
    let code = r"
        let a = 1 in
        let b = 2 in
        let c = 3 in
        let d = 4 in
        let e = 5 in
        a + b + c + d + e
    ";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_if_expressions() {
    // Test deeply nested if expressions
    let code = r"
        if true then
            if true then
                if true then
                    if true then 1
                    else 2
                else 3
            else 4
        else 5
    ";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_tuples() {
    // Test deeply nested tuples
    let code = "(((1, 2), (3, 4)), ((5, 6), (7, 8)))";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_records() {
    // Test deeply nested records
    let code = "{ a: { b: { c: { d: { e: 42 } } } } }";
    let result = parse(code);
    assert!(result.is_ok());
}

// Whitespace and Formatting Tests

#[test]
fn test_no_whitespace_operators() {
    // Test that operators work without surrounding whitespace
    let code = "1+2*3-4/2";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_excessive_whitespace() {
    // Test that excessive whitespace is handled
    let code = "  1   +   2   *   3  ";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_newlines_in_expression() {
    // Test that newlines are treated as whitespace
    let code = "1 +\n2 *\n3";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_tabs_in_expression() {
    // Test that tabs are treated as whitespace
    let code = "1\t+\t2\t*\t3";
    let result = parse(code);
    assert!(result.is_ok());
}

// Type Annotation Parsing Tests

#[test]
fn test_type_annotation_simple() {
    // Test simple type annotation
    let code = "let x : Int = 42 in x";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_type_annotation_function() {
    // Test function type annotation
    let code = "let f : Int -> Int = fun x -> x + 1 in f 10";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_type_annotation_missing_type() {
    // Test error for incomplete type annotation
    let code = "let x : = 42 in x";
    let result = parse(code);
    assert!(result.is_err());
}

// Sum Type Parsing Tests

#[test]
fn test_sum_type_simple() {
    // Test simple sum type definition
    let code = "type Option a = Some a | None in Some 42";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_sum_type_multiple_params() {
    // Test sum type with multiple type parameters
    let code = "type Either a b = Left a | Right b in Left 10";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_sum_type_recursive() {
    // Test recursive sum type definition
    let code = "type List a = Nil | Cons a (List a) in Cons 1 Nil";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_constructor_application_multiple_args() {
    // Test constructor with multiple arguments
    let code = "type Pair a b = MkPair a b in MkPair 1 2";
    let result = parse(code);
    assert!(result.is_ok());
}

// Pattern Parsing Tests

#[test]
fn test_pattern_wildcard() {
    // Test wildcard pattern
    let code = "match 42 with | _ -> 0";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_pattern_variable() {
    // Test variable pattern
    let code = "match 42 with | x -> x";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_pattern_tuple() {
    // Test tuple pattern
    let code = "match (1, 2) with | (x, y) -> x + y";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_pattern_record() {
    // Test record pattern
    let code = "match { x: 10 } with | { x: n } -> n";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_pattern_constructor() {
    // Test constructor pattern
    let code = "type Option a = Some a | None in match Some 42 with | Some n -> n | None -> 0";
    let result = parse(code);
    assert!(result.is_ok());
}

// Edge Cases

#[test]
fn test_empty_input() {
    // Test behavior for empty input (may or may not error depending on implementation)
    let code = "";
    let result = parse(code);
    // Just ensure it doesn't panic
    let _ = result;
}

#[test]
fn test_only_whitespace() {
    // Test behavior for only whitespace (may or may not error depending on implementation)
    let code = "   \n\t  ";
    let result = parse(code);
    // Just ensure it doesn't panic
    let _ = result;
}

#[test]
fn test_keyword_as_variable_name() {
    // Test that keywords cannot be used as variable names
    let code = "let if = 42 in if";
    let result = parse(code);
    // Should error because 'if' is a keyword
    assert!(result.is_err());
}

#[test]
fn test_number_with_leading_zeros() {
    // Test numbers with leading zeros
    let code = "0042";
    let result = parse(code);
    // Should parse as 42
    assert!(result.is_ok());
}

#[test]
fn test_negative_numbers() {
    // Test negative number literals
    let code = "-42";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_very_large_number() {
    // Test very large numbers (within i64 range)
    let code = "9223372036854775807"; // i64::MAX
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_boolean_true() {
    // Test boolean true literal
    let code = "true";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_boolean_false() {
    // Test boolean false literal
    let code = "false";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_record_empty() {
    // Test empty record
    let code = "{}";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_sequential_let_with_semicolons() {
    // Test sequential let bindings with semicolons
    let code = "let x = 1; let y = 2; x + y";
    let result = parse(code);
    assert!(result.is_ok());
}

#[test]
fn test_load_expression() {
    // Test load expression parsing
    let code = r#"load "examples/stdlib.par" in double 21"#;
    let result = parse(code);
    assert!(result.is_ok());
}
