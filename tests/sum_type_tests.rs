/// Tests for sum type (algebraic data type) functionality
use parlang::{eval, parse, Environment, Value};

/// Test parsing a simple Option type definition
#[test]
fn test_parse_option_type() {
    let input = "type Option a = Some a | None in 42";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing an Either type definition
#[test]
fn test_parse_either_type() {
    let input = "type Either a b = Left a | Right b in 0";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing a List type definition
#[test]
fn test_parse_list_type() {
    let input = "type List a = Nil | Cons a (List a) in 0";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing a constructor with no arguments
#[test]
fn test_parse_constructor_no_args() {
    let input = "None";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing a constructor with one argument
#[test]
fn test_parse_constructor_one_arg() {
    let input = "Some 42";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing a constructor with multiple arguments
#[test]
fn test_parse_constructor_multiple_args() {
    let input = "Cons 1 rest";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing constructor pattern in match
#[test]
fn test_parse_constructor_pattern() {
    let input = r#"
        type Option a = Some a | None in
        match None with
        | Some x -> x
        | None -> 0
    "#;
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test Option type with Some constructor
#[test]
fn test_option_some() {
    let input = r#"
        type Option a = Some a | None in
        let x = Some 42 in
        match x with
        | Some n -> n
        | None -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test Option type with None constructor
#[test]
fn test_option_none() {
    let input = r#"
        type Option a = Some a | None in
        let x = None in
        match x with
        | Some n -> n
        | None -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "0");
}

/// Test Either type with Left constructor
#[test]
fn test_either_left() {
    let input = r#"
        type Either a b = Left a | Right b in
        let x = Left 10
        in match x with
        | Left n -> n
        | Right m -> m
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "10");
}

/// Test Either type with Right constructor
#[test]
fn test_either_right() {
    let input = r#"
        type Either a b = Left a | Right b in
        let x = Right 20
        in match x with
        | Left n -> n
        | Right m -> m
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "20");
}

/// Test List type with Nil
#[test]
fn test_list_nil() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        let list = Nil
        in match list with
        | Nil -> 0
        | Cons head tail -> head
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "0");
}

/// Test List type with Cons
#[test]
fn test_list_cons() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        let list = Cons 1 (Cons 2 (Cons 3 Nil))
        in match list with
        | Nil -> 0
        | Cons head tail -> head
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "1");
}

/// Test nested pattern matching
#[test]
fn test_nested_pattern_matching() {
    let input = r#"
        type Option a = Some a | None in
        let x = Some (Some 42)
        in match x with
        | Some (Some n) -> n
        | Some None -> 0
        | None -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test recursive list sum function
#[test]
fn test_recursive_list_sum() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        let sum = rec sum -> fun list ->
            match list with
            | Nil -> 0
            | Cons head tail -> head + sum tail
        in let list = Cons 1 (Cons 2 (Cons 3 Nil))
        in sum list
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "6");
}

/// Test recursive list length function
#[test]
fn test_recursive_list_length() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        let length = rec length -> fun list ->
            match list with
            | Nil -> 0
            | Cons head tail -> 1 + length tail
        in let list = Cons 1 (Cons 2 (Cons 3 (Cons 4 Nil)))
        in length list
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "4");
}

/// Test wildcard pattern with constructor
#[test]
fn test_constructor_wildcard_pattern() {
    let input = r#"
        type Option a = Some a | None in
        let x = Some 99
        in match x with
        | Some _ -> 1
        | None -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "1");
}

/// Test constructor arity error
#[test]
fn test_constructor_arity_error() {
    let input = r#"
        type Option a = Some a | None in
        Some 1 2
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_err(), "Should fail with arity error");
}

/// Test unknown constructor error
#[test]
fn test_unknown_constructor_error() {
    let input = "UnknownConstructor 42";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_err(), "Should fail with unknown constructor");
}

/// Test pattern match on wrong constructor
#[test]
fn test_pattern_match_wrong_constructor() {
    let input = r#"
        type Option a = Some a | None in
        let x = None
        in match x with
        | Some n -> n
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_err(), "Should fail - pattern doesn't match");
}

/// Test Display for Variant values
#[test]
fn test_variant_display() {
    let input = r#"
        type Option a = Some a | None in
        Some 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "Some(42)");
}

/// Test Display for Variant with no arguments
#[test]
fn test_variant_display_no_args() {
    let input = r#"
        type Option a = Some a | None in
        None
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "None");
}

/// Test Display for nested Variant
#[test]
fn test_variant_display_nested() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        Cons 1 (Cons 2 Nil)
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "Cons(1, Cons(2, Nil))");
}

/// Test multiple type definitions in scope
#[test]
fn test_multiple_type_definitions() {
    let input = r#"
        type Option a = Some a | None in
        type Either a b = Left a | Right b in
        let x = Some 10
        in let y = Left 20
        in match x with
        | Some n -> n
        | None -> match y with
            | Left m -> m
            | Right m -> m
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "10");
}

/// Test constructor equality in pattern matching
#[test]
fn test_constructor_equality() {
    let input = r#"
        type Bool2 = True2 | False2 in
        let x = True2
        in match x with
        | True2 -> 1
        | False2 -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "1");
}

/// Test Tree type (binary tree)
#[test]
fn test_tree_type() {
    let input = r#"
        type Tree a = Leaf | Node a (Tree a) (Tree a) in
        let tree = Node 5 (Node 3 Leaf Leaf) (Node 7 Leaf Leaf)
        in match tree with
        | Leaf -> 0
        | Node value left right -> value
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "5");
}

/// Test Result type pattern
#[test]
fn test_result_type() {
    let input = r#"
        type Result a b = Ok a | Err b in
        let r = Ok 100
        in match r with
        | Ok value -> value
        | Err _ -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "100");
}
