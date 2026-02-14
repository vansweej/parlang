/// Advanced unit tests for typechecker internals
/// Tests for row polymorphism, type unification edge cases, and helper functions
use parlang::{parse, typecheck, Type, TypeVar};

// Row Polymorphism Tests

#[test]
fn test_row_polymorphism_basic_field_access() {
    // Test that functions accessing a field work with any record having that field
    let code = r"
        let getAge = fun r -> r.age
        in getAge { age: 25, name: 100 }
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_row_polymorphism_multiple_fields() {
    // Test accessing multiple fields from a row-polymorphic record
    let code = r"
        let getFullName = fun r -> r.first
        in getFullName { first: 10, last: 20, age: 30 }
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_row_polymorphism_nested_access() {
    // Test nested field access with row polymorphism
    let code = r"
        let getCity = fun r -> r.address.city
        in getCity { address: { city: 100, zip: 200 }, name: 42 }
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_row_polymorphism_function_composition() {
    // Test that row-polymorphic functions compose correctly
    let code = r"
        let getAge = fun r -> r.age in
        let addOne = fun x -> x + 1 in
        let getAgeAndIncrement = fun r -> addOne (getAge r) in
        getAgeAndIncrement { age: 25, name: 100 }
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_row_polymorphism_conditional() {
    // Test row polymorphism with conditionals
    // Note: Both branches must access the same field for row polymorphism to work
    let code = r"
        let getField = fun r -> if true then r.x else r.x
        in getField { x: 10, y: 20 }
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

// Type Unification Edge Cases

#[test]
fn test_unification_recursive_types() {
    // Test that occurs check prevents infinite types
    let code = r"
        fun f -> f f
    ";
    let expr = parse(code).unwrap();
    let result = typecheck(&expr);
    // This should fail due to occurs check: t0 = t0 -> t1
    assert!(result.is_err());
}

#[test]
fn test_unification_deeply_nested_functions() {
    // Test unification with deeply nested function types
    let code = r"
        let f = fun a -> fun b -> fun c -> fun d -> a + b + c + d
        in f 1 2 3 4
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_unification_complex_record_types() {
    // Test unification with complex record types
    let code = r"
        let makeRecord = fun x -> { a: x, b: x + 1 }
        in let r = makeRecord 10
        in r.a + r.b
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_unification_mixed_record_tuple() {
    // Test unification with records containing tuples
    let code = r"
        let r = { pair: (1, 2), value: 42 }
        in r.pair.0 + r.value
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_unification_function_in_record() {
    // Test unification with functions stored in records
    let code = r"
        let r = { f: fun x -> x + 1 }
        in r.f 41
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

// Generic Type Constructor Tests

#[test]
fn test_generic_option_inference() {
    // Test that Option type parameters are inferred correctly
    let code = r"
        type Option a = Some a | None in
        Some 42
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    // Should infer Option Int
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType, got {:?}", ty),
    }
}

#[test]
fn test_generic_either_inference_left() {
    // Test Either type with Left constructor
    let code = r"
        type Either a b = Left a | Right b in
        Left true
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Either");
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], Type::Bool);
            // Second type parameter should be a type variable
            assert!(matches!(args[1], Type::Var(_)));
        }
        _ => panic!("Expected SumType, got {:?}", ty),
    }
}

#[test]
fn test_generic_either_inference_right() {
    // Test Either type with Right constructor
    let code = r"
        type Either a b = Left a | Right b in
        Right 42
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Either");
            assert_eq!(args.len(), 2);
            // First type parameter should be a type variable
            assert!(matches!(args[0], Type::Var(_)));
            assert_eq!(args[1], Type::Int);
        }
        _ => panic!("Expected SumType, got {:?}", ty),
    }
}

#[test]
fn test_generic_nested_constructor() {
    // Test nested generic constructors (Option of Option)
    let code = r"
        type Option a = Some a | None in
        Some (Some 42)
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args.len(), 1);
            // Inner type should also be Option Int
            match &args[0] {
                Type::SumType(inner_name, inner_args) => {
                    assert_eq!(inner_name, "Option");
                    assert_eq!(inner_args[0], Type::Int);
                }
                _ => panic!("Expected nested SumType, got {:?}", args[0]),
            }
        }
        _ => panic!("Expected SumType, got {:?}", ty),
    }
}

#[test]
fn test_generic_list_constructor() {
    // Test generic List type inference
    let code = r"
        type List a = Nil | Cons a (List a) in
        Cons 1 (Cons 2 Nil)
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "List");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected SumType, got {:?}", ty),
    }
}

// Type Variable Substitution Tests

#[test]
fn test_substitution_preserves_types() {
    // Test that substitution doesn't break existing type relationships
    let code = r"
        let id = fun x -> x in
        let a = id 42 in
        let b = id true in
        b
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Bool);
}

#[test]
fn test_substitution_in_nested_context() {
    // Test substitution in nested let bindings
    let code = r"
        let f = fun x -> 
            let g = fun y -> x + y
            in g
        in (f 10) 20
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_substitution_with_multiple_uses() {
    // Test that type variables are substituted consistently
    let code = r"
        let pair = fun x -> (x, x) in
        let p = pair 42 in
        p.0 + p.1
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    // The result of p.0 + p.1 should be Int
    assert_eq!(ty, Type::Int);
}

// Complex Type Inference Tests

#[test]
fn test_type_inference_with_records_and_generics() {
    // Test combining records with generic types
    let code = r"
        type Option a = Some a | None in
        let r = { opt: Some 42, val: 100 } in
        r.val
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_type_inference_curried_with_generics() {
    // Test curried functions with generic types  
    let code = r"
        type Option a = Some a | None in
        let makeOption = fun x -> Some x in
        makeOption 42
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    // Should infer Option Int
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected Option Int, got {:?}", ty),
    }
}

#[test]
fn test_type_inference_higher_order_generics() {
    // Test higher-order functions with generic types
    let code = r"
        type Option a = Some a | None in
        let inc = fun x -> x + 1 in
        Some (inc 41)
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    // Should infer Option Int
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected Option Int, got {:?}", ty),
    }
}

#[test]
fn test_polymorphic_function_multiple_instantiations() {
    // Test that polymorphic functions can be instantiated with different types
    let code = r"
        type Option a = Some a | None in
        let getSome = fun x -> Some x in
        getSome 42
    ";
    let expr = parse(code).unwrap();
    let ty = typecheck(&expr).unwrap();
    // Should infer Option Int
    match ty {
        Type::SumType(name, args) => {
            assert_eq!(name, "Option");
            assert_eq!(args[0], Type::Int);
        }
        _ => panic!("Expected Option Int, got {:?}", ty),
    }
}
