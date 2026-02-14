/// Integration tests for type inference system
/// These tests verify the complete type inference pipeline
use parlang::{parse, typecheck, Type};

#[test]
fn test_complete_program_int() {
    let expr = parse("let x = 42 in x").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_complete_program_bool() {
    let expr = parse("let b = true in b").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Bool);
}

#[test]
fn test_complete_program_function_application() {
    let expr = parse("let f = fun x -> x in f 10").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_complete_program_if_then_else() {
    let expr = parse("if true then 1 else 2").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_complete_program_arithmetic() {
    let programs = vec![
        "1 + 2",
        "10 - 5",
        "3 * 4",
        "20 / 4",
        "let x = 10 in let y = 20 in x + y",
    ];

    for source in programs {
        let expr = parse(source).unwrap();
        let ty = typecheck(&expr).unwrap();
        assert_eq!(ty, Type::Int, "Failed for program: {source}");
    }
}

#[test]
fn test_complete_program_comparison() {
    let programs = vec![
        "5 > 3",
        "5 >= 3",
        "3 < 5",
        "3 <= 5",
        "1 == 1",
        "1 != 2",
    ];

    for source in programs {
        let expr = parse(source).unwrap();
        let ty = typecheck(&expr).unwrap();
        assert_eq!(ty, Type::Bool, "Failed for program: {source}");
    }
}

#[test]
fn test_complete_program_higher_order_function() {
    let expr = parse("let apply = fun f -> fun x -> f x in apply").unwrap();
    let ty = typecheck(&expr).unwrap();
    // apply should have type: (a -> b) -> a -> b (with some type variables)
    assert!(matches!(ty, Type::Fun(_, _)));
}

#[test]
fn test_complete_program_curried_function() {
    let expr = parse("let add = fun x -> fun y -> x + y in add 5 10").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_complete_program_let_polymorphism() {
    // id should be usable at multiple types
    let expr = parse("let id = fun x -> x in let a = id 42 in let b = id true in b").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Bool);
}

#[test]
fn test_complete_program_nested_let() {
    let expr = parse("let x = 1 in let y = 2 in let z = 3 in x + y + z").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_complete_program_function_composition() {
    let expr = parse("let f = fun x -> x + 1 in let g = fun y -> y * 2 in f (g 5)").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_error_type_mismatch_add() {
    let expr = parse("1 + true").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_err());
}

#[test]
fn test_error_type_mismatch_if_condition() {
    let expr = parse("if 1 then 2 else 3").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_err());
}

#[test]
fn test_error_type_mismatch_if_branches() {
    let expr = parse("if true then 1 else false").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_err());
}

#[test]
fn test_error_unbound_variable() {
    let expr = parse("x + 1").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_err());
}

#[test]
fn test_error_unbound_variable_in_function() {
    let expr = parse("fun x -> y + x").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_err());
}

#[test]
fn test_complex_polymorphic_program() {
    // const function: a -> b -> a
    let expr = parse("let const = fun x -> fun y -> x in let a = const 42 100 in let b = const 10 20 in a + b").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_type_preserving_evaluation() {
    // Programs that type check should also evaluate successfully
    let programs = vec![
        ("42", Type::Int),
        ("true", Type::Bool),
        ("1 + 2", Type::Int),
        ("if true then 1 else 2", Type::Int),
        ("let x = 10 in x", Type::Int),
        ("(fun x -> x) 42", Type::Int),
        ("let id = fun x -> x in id true", Type::Bool),
    ];

    for (source, expected_type) in programs {
        let expr = parse(source).unwrap();
        let ty = typecheck(&expr).unwrap();
        assert_eq!(ty, expected_type, "Failed for program: {source}");
        
        // Also verify it evaluates without runtime errors
        use parlang::{eval, Environment};
        let result = eval(&expr, &Environment::new());
        assert!(result.is_ok(), "Evaluation failed for program: {source}");
    }
}

#[test]
fn test_function_type_structure() {
    let expr = parse("fun x -> x + 1").unwrap();
    let ty = typecheck(&expr).unwrap();
    
    if let Type::Fun(arg, ret) = ty {
        assert_eq!(*arg, Type::Int);
        assert_eq!(*ret, Type::Int);
    } else {
        panic!("Expected function type");
    }
}

#[test]
fn test_curried_function_type_structure() {
    let expr = parse("fun x -> fun y -> x + y").unwrap();
    let ty = typecheck(&expr).unwrap();
    
    // Should be: Int -> (Int -> Int)
    if let Type::Fun(arg1, rest) = ty {
        assert_eq!(*arg1, Type::Int);
        if let Type::Fun(arg2, ret) = *rest {
            assert_eq!(*arg2, Type::Int);
            assert_eq!(*ret, Type::Int);
        } else {
            panic!("Expected function type for second argument");
        }
    } else {
        panic!("Expected function type");
    }
}

#[test]
fn test_identity_function_polymorphic() {
    // Identity function should work with any type
    let programs = vec![
        ("let id = fun x -> x in id 42", Type::Int),
        ("let id = fun x -> x in id true", Type::Bool),
    ];

    for (source, expected_type) in programs {
        let expr = parse(source).unwrap();
        let ty = typecheck(&expr).unwrap();
        assert_eq!(ty, expected_type, "Failed for program: {source}");
    }
}

#[test]
fn test_shadowing() {
    let expr = parse("let x = 1 in let x = true in x").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Bool);
}

#[test]
fn test_nested_functions() {
    let expr = parse("let outer = fun x -> let inner = fun y -> x + y in inner in outer 5 10").unwrap();
    let ty = typecheck(&expr).unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_type_env_default() {
    use parlang::TypeEnv;
    let env1 = TypeEnv::new();
    let env2 = TypeEnv::default();
    // Both should have the same initial counter value
    assert_eq!(format!("{:?}", env1), format!("{:?}", env2));
}

#[test]
fn test_type_error_display_unbound_variable() {
    use parlang::TypeError;
    let error = TypeError::UnboundVariable("x".to_string());
    assert_eq!(format!("{error}"), "Unbound variable: x");
}

#[test]
fn test_type_error_display_unification_error() {
    use parlang::TypeError;
    let error = TypeError::UnificationError(Type::Int, Type::Bool);
    assert_eq!(format!("{error}"), "Cannot unify types: Int and Bool");
}

#[test]
fn test_type_error_display_occurs_check() {
    use parlang::{TypeError, TypeVar};
    let error = TypeError::OccursCheckFailed(TypeVar(0), Type::Int);
    assert_eq!(format!("{error}"), "Occurs check failed: t0 occurs in Int");
}

#[test]
fn test_type_error_display_recursion() {
    use parlang::TypeError;
    let error = TypeError::RecursionRequiresAnnotation;
    assert_eq!(format!("{error}"), "Recursive functions require type annotations");
}

#[test]
fn test_recursion_requires_annotation() {
    // rec is not fully supported in typechecker, should give an appropriate error
    let expr = parse("rec f -> fun x -> f x").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_err());
    if let Err(e) = result {
        use parlang::TypeError;
        assert!(matches!(e, TypeError::RecursionRequiresAnnotation));
    }
}

#[test]
fn test_tuple_type_inference() {
    // Tuples currently return type variables (simplified implementation)
    let expr = parse("(1, 2)").unwrap();
    let result = typecheck(&expr);
    // Should succeed even if it's a simplified type
    assert!(result.is_ok());
}

#[test]
fn test_tuple_proj_type_inference() {
    // Tuple projection currently returns type variables
    let expr = parse("(1, 2).0").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_match_type_inference() {
    // Match expressions currently return type variables
    let expr = parse("match 1 with | 1 -> true | _ -> false").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_load_type_inference() {
    // Load expressions currently return type variables
    let expr = parse("load \"test.par\" in 42").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}

#[test]
fn test_seq_type_inference() {
    // Sequential expressions currently return type variables
    let expr = parse("let x = 1; 2").unwrap();
    let result = typecheck(&expr);
    assert!(result.is_ok());
}
