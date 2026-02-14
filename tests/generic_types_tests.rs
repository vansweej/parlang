/// Tests for generic type support in the type checker
use parlang::{parse, typecheck, Type};

/// Test Option type with Int argument
#[test]
fn test_option_int_type() {
    let input = r#"
        type Option a = Some a | None in
        Some 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    // The type should be Option Int
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test Option type with Bool argument
#[test]
fn test_option_bool_type() {
    let input = r#"
        type Option a = Some a | None in
        Some true
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Bool);
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test None constructor
#[test]
fn test_option_none_type() {
    let input = r#"
        type Option a = Some a | None in
        None
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            // None should have a type variable since it's polymorphic
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test Either type with two type parameters
#[test]
fn test_either_left_type() {
    let input = r#"
        type Either a b = Left a | Right b in
        Left 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Either");
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], Type::Int);
            // Second arg should be a type variable
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test Either type with Right constructor
#[test]
fn test_either_right_type() {
    let input = r#"
        type Either a b = Left a | Right b in
        Right true
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Either");
            assert_eq!(args.len(), 2);
            // First arg should be a type variable
            assert_eq!(args[1], Type::Bool);
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test List type with recursive definition
#[test]
fn test_list_nil_type() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        Nil
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "List");
            assert_eq!(args.len(), 1);
            // Nil should have a type variable
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test List Cons with Int
#[test]
fn test_list_cons_int() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        Cons 42 Nil
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "List");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test nested generic types
#[test]
fn test_nested_generic_types() {
    let input = r#"
        type Option a = Some a | None in
        type List a = Nil | Cons a (List a) in
        Some (Cons 1 Nil)
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match &ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            // The inner type should be List Int
            match &args[0] {
                Type::SumType(inner_name, inner_args) => {
                    assert_eq!(inner_name, "List");
                    assert_eq!(inner_args.len(), 1);
                    assert_eq!(inner_args[0], Type::Int);
                }
                _ => panic!("Expected nested SumType, got: {:?}", args[0]),
            }
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test that generic types can be used in let bindings
#[test]
fn test_generic_type_let_binding() {
    let input = r#"
        type Option a = Some a | None in
        let x = Some 42 in
        x
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test that generic types work with functions
#[test]
fn test_generic_type_function_arg() {
    let input = r#"
        type Option a = Some a | None in
        let f = fun x -> Some x in
        f 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType, got: {:?}", ty),
    }
}

/// Test display of generic types
#[test]
fn test_display_option_int() {
    let input = r#"
        type Option a = Some a | None in
        Some 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok());
    
    let ty = result.unwrap();
    assert_eq!(format!("{}", ty), "Option Int");
}

/// Test display of Either type
#[test]
fn test_display_either() {
    let input = r#"
        type Either a b = Left a | Right b in
        Left 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok());
    
    let ty = result.unwrap();
    let display = format!("{}", ty);
    assert!(display.starts_with("Either"));
}

/// Test display of List type
#[test]
fn test_display_list_int() {
    let input = r#"
        type List a = Nil | Cons a (List a) in
        Cons 42 Nil
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok());
    
    let ty = result.unwrap();
    assert_eq!(format!("{}", ty), "List Int");
}

/// Test that multiple uses of the same generic type work
#[test]
fn test_multiple_generic_uses() {
    let input = r#"
        type Option a = Some a | None in
        let x = Some 42 in
        let y = Some true in
        x
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType"),
    }
}

/// Test Result type (similar to Either but with common names)
#[test]
fn test_result_type() {
    let input = r#"
        type Result a b = Ok a | Err b in
        Ok 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Result");
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType"),
    }
}

/// Test Tree type with multiple constructors
#[test]
fn test_tree_type() {
    let input = r#"
        type Tree a = Leaf a | Node (Tree a) (Tree a) in
        Leaf 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
    
    let ty = result.unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Tree");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType"),
    }
}

/// Test constructor arity mismatch error
#[test]
fn test_constructor_arity_mismatch_too_few() {
    let input = r#"
        type Option a = Some a | None in
        Some
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    // Some expects 1 argument but got 0
    assert!(result.is_err(), "Expected type error for arity mismatch");
}

/// Test constructor arity mismatch error - too many args
#[test]
fn test_constructor_arity_mismatch_too_many() {
    let input = r#"
        type Option a = Some a | None in
        None 42
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    // None expects 0 arguments but got 1
    assert!(result.is_err(), "Expected type error for arity mismatch");
}

/// Test correct arity for multi-argument constructor
#[test]
fn test_multi_arg_constructor_correct() {
    let input = r#"
        type Tree a = Leaf a | Node (Tree a) (Tree a) in
        Node (Leaf 1) (Leaf 2)
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Type check failed: {:?}", result.err());
}
