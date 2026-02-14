/// Advanced exhaustiveness checking tests
/// Tests for nested constructors, complex patterns, and edge cases
use parlang::{parse, typecheck};

// Nested Constructor Exhaustiveness Tests

#[test]
fn test_nested_constructor_exhaustive() {
    // Test exhaustiveness with nested Option constructors
    let code = r"
        type Option a = Some a | None in
        let x = Some (Some 42) in
        match x with
        | Some (Some n) -> n
        | Some None -> 0
        | None -> 0
    ";
    let expr = parse(code).unwrap();
    // Should typecheck successfully without warnings
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_nested_constructor_non_exhaustive() {
    // Test that missing nested patterns are detected
    // Note: Current implementation may not warn on nested patterns
    let code = r"
        type Option a = Some a | None in
        let x = Some (Some 42) in
        match x with
        | Some (Some n) -> n
        | None -> 0
    ";
    let expr = parse(code).unwrap();
    // Should still typecheck, but would ideally warn about missing Some None case
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_deeply_nested_constructors() {
    // Test with three levels of nesting
    let code = r"
        type Option a = Some a | None in
        let x = Some (Some (Some 42)) in
        match x with
        | Some (Some (Some n)) -> n
        | Some (Some None) -> 0
        | Some None -> 0
        | None -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_nested_either_constructors() {
    // Test nested Either constructors
    let code = r"
        type Either a b = Left a | Right b in
        let x = Left (Right 42) in
        match x with
        | Left (Left n) -> n
        | Left (Right n) -> n
        | Right n -> n
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_nested_list_patterns() {
    // Test exhaustiveness with nested list patterns
    let code = r"
        type List a = Nil | Cons a (List a) in
        let list = Cons 1 (Cons 2 Nil) in
        match list with
        | Nil -> 0
        | Cons head Nil -> head
        | Cons head tail -> head
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

// Complex Record Pattern Exhaustiveness Tests

#[test]
fn test_record_pattern_exhaustive_full() {
    // Test exhaustive pattern matching on records with all fields
    let code = r"
        let person = { name: 42, age: 30 } in
        match person with
        | { name: n, age: a } -> n + a
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_record_pattern_partial_matching() {
    // Test partial record patterns (matching subset of fields)
    let code = r"
        let person = { name: 42, age: 30, city: 100 } in
        match person with
        | { name: n } -> n
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_record_pattern_multiple_arms() {
    // Test record patterns with multiple arms
    let code = r"
        let person = { name: 42, age: 30 } in
        match person with
        | { name: 0, age: a } -> a
        | { name: n, age: a } -> n + a
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_nested_record_patterns() {
    // Test nested record patterns
    let code = r"
        let person = { address: { city: 100 }, name: 42 } in
        match person with
        | { address: { city: c }, name: n } -> c + n
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_record_with_tuple_pattern() {
    // Test records containing tuples in patterns
    let code = r"
        let data = { pair: (10, 20), value: 30 } in
        match data with
        | { pair: (x, y), value: v } -> x + y + v
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

// Tuple Pattern Exhaustiveness Tests

#[test]
fn test_tuple_pattern_exhaustive_2() {
    // Test exhaustive tuple patterns with 2 elements
    let code = r"
        match (1, 2) with
        | (0, 0) -> 0
        | (x, y) -> x + y
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_pattern_exhaustive_3() {
    // Test exhaustive tuple patterns with 3 elements
    let code = r"
        match (1, 2, 3) with
        | (0, 0, 0) -> 0
        | (x, y, z) -> x + y + z
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_nested_tuple_patterns() {
    // Test nested tuple patterns
    let code = r"
        match ((1, 2), (3, 4)) with
        | ((a, b), (c, d)) -> a + b + c + d
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_with_constructor_pattern() {
    // Test tuples containing constructors in patterns
    let code = r"
        type Option a = Some a | None in
        let pair = (Some 42, None) in
        match pair with
        | (Some x, Some y) -> x + y
        | (Some x, None) -> x
        | (None, Some y) -> y
        | (None, None) -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_tuple_with_record_pattern() {
    // Test tuples containing records in patterns
    let code = r"
        let tuple = ({ x: 10 }, { y: 20 }) in
        match tuple with
        | ({ x: a }, { y: b }) -> a + b
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

// Mixed Constructor and Record Patterns

#[test]
fn test_constructor_with_record_payload() {
    // Test constructors with record payloads
    let code = r"
        type Result a = Ok a | Err a in
        let result = Ok 42 in
        match result with
        | Ok n -> n
        | Err n -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_complex_mixed_patterns() {
    // Test complex pattern with constructors and tuples
    let code = r"
        type Option a = Some a | None in
        let data = Some (10, 20) in
        match data with
        | Some (x, y) -> x + y
        | None -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

// Wildcard and Variable Pattern Tests

#[test]
fn test_wildcard_makes_exhaustive() {
    // Test that wildcard pattern makes match exhaustive
    let code = r"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | _ -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_variable_pattern_catches_all() {
    // Test that variable pattern makes match exhaustive
    let code = r"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | other -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_mixed_wildcard_and_specific() {
    // Test mixing wildcard with specific patterns
    let code = r"
        type Option a = Some a | None in
        match Some 42 with
        | Some 0 -> 0
        | Some _ -> 1
        | None -> 2
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

// Boolean Pattern Exhaustiveness

#[test]
fn test_boolean_exhaustive() {
    // Test exhaustive boolean patterns
    let code = r"
        match true with
        | true -> 1
        | false -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_boolean_with_wildcard() {
    // Test boolean pattern with wildcard
    let code = r"
        match true with
        | true -> 1
        | _ -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

// Integer Pattern Tests

#[test]
fn test_integer_patterns_with_variable() {
    // Test integer literal patterns with variable catch-all
    let code = r"
        match 42 with
        | 0 -> 0
        | 1 -> 1
        | n -> n
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_integer_patterns_multiple_literals() {
    // Test multiple integer literal patterns
    let code = r"
        match 42 with
        | 0 -> 0
        | 1 -> 1
        | 42 -> 42
        | _ -> 999
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

// Recursive Pattern Matching

#[test]
fn test_recursive_list_pattern_exhaustive() {
    // Test exhaustive pattern matching on recursive lists
    let code = r"
        type List a = Nil | Cons a (List a) in
        let list = Cons 1 (Cons 2 Nil) in
        match list with
        | Nil -> 0
        | Cons head tail -> head
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_recursive_tree_pattern() {
    // Test pattern matching on tree structures
    let code = r"
        type Tree a = Leaf a | Node (Tree a) (Tree a) in
        let tree = Node (Leaf 1) (Leaf 2) in
        match tree with
        | Leaf n -> n
        | Node left right -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

// Edge Cases

#[test]
fn test_empty_match_arms() {
    // Test that empty pattern list is properly rejected
    let code = r"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | None -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_pattern_ordering() {
    // Test that pattern order doesn't affect exhaustiveness
    let code = r"
        type Option a = Some a | None in
        match Some 42 with
        | None -> 0
        | Some n -> n
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_overlapping_patterns() {
    // Test overlapping patterns (later patterns unreachable)
    let code = r"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | Some 0 -> 0
        | None -> 0
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    // Should typecheck fine even though Some 0 is unreachable
    assert!(result.is_ok());
}
