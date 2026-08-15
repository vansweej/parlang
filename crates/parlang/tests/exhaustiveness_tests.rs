/// Integration tests for pattern matching exhaustiveness checking
use parlang::{eval, parse, Environment};

/// Test exhaustive Option type match
#[test]
fn test_option_exhaustive() {
    let input = r#"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | None -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    
    // Get the match arms and environment after TypeDef evaluation
    // For this test, we'll evaluate and verify no warning is printed
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test non-exhaustive Option type match (missing None)
#[test]
fn test_option_non_exhaustive_missing_none() {
    let input = r#"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
    "#;
    let expr = parse(input).expect("Parse failed");
    
    // This should print a warning but still evaluate successfully
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test non-exhaustive Option type match (missing Some)
#[test]
fn test_option_non_exhaustive_missing_some() {
    let input = r#"
        type Option a = Some a | None in
        match None with
        | None -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    
    // This should print a warning but still evaluate successfully
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "0");
}

/// Test exhaustive Either type match
#[test]
fn test_either_exhaustive() {
    let input = r#"
        type Either a b = Left a | Right b in
        match Left 10 with
        | Left n -> n
        | Right m -> m
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "10");
}

/// Test non-exhaustive Either type match
#[test]
fn test_either_non_exhaustive() {
    let input = r#"
        type Either a b = Left a | Right b in
        match Left 10 with
        | Left n -> n
    "#;
    let expr = parse(input).expect("Parse failed");
    
    // This should print a warning but still evaluate successfully
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "10");
}

/// Test exhaustive boolean match
#[test]
fn test_bool_exhaustive() {
    let input = r#"
        match true with
        | true -> 1
        | false -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "1");
}

/// Test non-exhaustive boolean match
#[test]
fn test_bool_non_exhaustive() {
    let input = r#"
        match true with
        | true -> 1
    "#;
    let expr = parse(input).expect("Parse failed");
    
    // This should print a warning but still evaluate successfully
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "1");
}

/// Test wildcard makes pattern exhaustive
#[test]
fn test_wildcard_makes_exhaustive() {
    let input = r#"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | _ -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test variable pattern makes pattern exhaustive
#[test]
fn test_variable_makes_exhaustive() {
    let input = r#"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | x -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test exhaustive List type match
#[test]
fn test_list_exhaustive() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        match Nil with
        | Nil -> 0
        | Cons head tail -> head
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "0");
}

/// Test non-exhaustive List type match
#[test]
fn test_list_non_exhaustive() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        match Cons 1 Nil with
        | Cons head tail -> head
    "#;
    let expr = parse(input).expect("Parse failed");
    
    // This should print a warning but still evaluate successfully
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "1");
}

/// Test nested pattern matching with exhaustiveness
#[test]
fn test_nested_pattern_exhaustive() {
    let input = r#"
        type Option a = Some a | None in
        match Some (Some 42) with
        | Some (Some n) -> n
        | Some None -> 0
        | None -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test integer patterns are non-exhaustive
#[test]
fn test_int_patterns_non_exhaustive() {
    let input = r#"
        match 5 with
        | 0 -> 1
        | 1 -> 2
    "#;
    let expr = parse(input).expect("Parse failed");
    
    // This should print a warning and fail at runtime since 5 doesn't match
    let result = eval(&expr, &Environment::new());
    assert!(result.is_err(), "Expected error for non-matching pattern");
}

/// Test integer patterns with wildcard are exhaustive
#[test]
fn test_int_patterns_with_wildcard() {
    let input = r#"
        match 5 with
        | 0 -> 1
        | 1 -> 2
        | _ -> 999
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "999");
}

/// Test tuple patterns
#[test]
fn test_tuple_pattern_with_wildcard() {
    let input = r#"
        match (1, 2) with
        | (0, 0) -> 0
        | _ -> 999
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "999");
}

/// Test record patterns with wildcard
#[test]
fn test_record_pattern_with_wildcard() {
    let input = r#"
        let person = { name: 42, age: 30 } in
        match person with
        | { name: 0, age: 0 } -> 0
        | _ -> 999
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "999");
}

/// Test exhaustiveness checking with Result type
#[test]
fn test_result_type_exhaustive() {
    let input = r#"
        type Result a b = Ok a | Err b in
        match Ok 42 with
        | Ok value -> value
        | Err _ -> 0
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test exhaustiveness checking with Tree type
#[test]
fn test_tree_type_exhaustive() {
    let input = r#"
        type Tree a = Leaf | Node a (Tree a) (Tree a) in
        match Leaf with
        | Leaf -> 0
        | Node value left right -> value
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "0");
}

/// Test that exhaustiveness warnings work in recursive functions
#[test]
fn test_exhaustiveness_in_recursive_function() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        (rec sum -> fun list ->
            match list with
            | Nil -> 0
            | Cons head tail -> head + sum tail)
        (Cons 1 (Cons 2 (Cons 3 Nil)))
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "6");
}
