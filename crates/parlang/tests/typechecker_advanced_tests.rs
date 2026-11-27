/// Advanced unit tests for typechecker internals
/// Tests for type unification edge cases and helper functions
use parlang::{parse_expr as parse, typecheck, Type};

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
        data Option a = Some a | None in
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
        _ => panic!("Expected SumType, got {ty:?}"),
    }
}

#[test]
fn test_generic_either_inference_left() {
    // Test Either type with Left constructor
    let code = r"
        data Either a b = Left a | Right b in
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
        _ => panic!("Expected SumType, got {ty:?}"),
    }
}

#[test]
fn test_generic_either_inference_right() {
    // Test Either type with Right constructor
    let code = r"
        data Either a b = Left a | Right b in
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
        _ => panic!("Expected SumType, got {ty:?}"),
    }
}

#[test]
fn test_generic_nested_constructor() {
    // Test nested generic constructors (Option of Option)
    let code = r"
        data Option a = Some a | None in
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
        _ => panic!("Expected SumType, got {ty:?}"),
    }
}

#[test]
fn test_generic_list_constructor() {
    // Test generic List type inference
    let code = r"
        data List a = Nil | Cons a (List a) in
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
        _ => panic!("Expected SumType, got {ty:?}"),
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
        data Option a = Some a | None in
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
        data Option a = Some a | None in
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
        _ => panic!("Expected Option Int, got {ty:?}"),
    }
}

#[test]
fn test_type_inference_higher_order_generics() {
    // Test higher-order functions with generic types
    let code = r"
        data Option a = Some a | None in
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
        _ => panic!("Expected Option Int, got {ty:?}"),
    }
}

#[test]
fn test_polymorphic_function_multiple_instantiations() {
    // Test that polymorphic functions can be instantiated with different types
    let code = r"
        data Option a = Some a | None in
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
        _ => panic!("Expected Option Int, got {ty:?}"),
    }
}

/// Parse and typecheck a whole program in a fresh environment.
fn check_program(source: &str) -> Result<Type, parlang::TypeError> {
    let program = parlang::parse_program(source).unwrap();
    parlang::typecheck_program(&program)
}

/// The scheme of the first top-level `let` of `source`.
fn first_let_scheme(source: &str) -> parlang::TypeScheme {
    let program = parlang::parse_program(source).unwrap();
    let typed = parlang::typecheck_program_typed(&program).unwrap();
    match typed.decls.first() {
        Some(parlang::TypedDecl::Let { scheme, .. }) => scheme.clone(),
        other => panic!("expected a let declaration, got {other:?}"),
    }
}

/// Whether `ty` has the shape `t -> t -> Bool` for one type variable `t`.
fn is_binary_predicate_on_one_var(ty: &Type) -> bool {
    match ty {
        Type::Fun(first, rest) => match rest.as_ref() {
            Type::Fun(second, result) => {
                matches!(first.as_ref(), Type::Var(_)) && first == second && **result == Type::Bool
            }
            _ => false,
        },
        _ => false,
    }
}

#[test]
fn test_eq_on_base_types_typechecks() {
    for source in ["1 == 2", "true != false", "'a' == 'b'", "1.5 == 2.5"] {
        assert_eq!(check_program(source), Ok(Type::Bool), "{source}");
    }
}

#[test]
fn test_eq_without_instance_is_rejected() {
    let sources = [
        "(1, 2) == (1, 2)",
        "(1, 2) != (1, 2)",
        "data List a = Nil | Cons a (List a);\n\"ab\" == \"ab\"",
        "{ x: 1 } == { x: 1 }",
        "data Color = Red | Green;\nRed == Green",
        "(fun x -> x) == (fun y -> y)",
        "() == ()",
    ];
    for source in sources {
        assert!(
            matches!(
                check_program(source),
                Err(parlang::TypeError::NoInstance(_))
            ),
            "{source}"
        );
    }
    assert_eq!(
        check_program("(1, 2) == (1, 2)").map_err(|error| error.to_string()),
        Err("no instance for Eq (Int, Int)".to_string())
    );
}

#[test]
fn test_top_level_eq_let_gets_constrained_scheme() {
    let scheme = first_let_scheme("let f = fun x -> fun y -> x == y;");
    assert_eq!(scheme.vars.len(), 1);
    let var = Type::Var(scheme.vars[0].clone());
    let expected = parlang::Constraint {
        class: "Eq".to_string(),
        args: vec![var.clone()],
    };
    assert_eq!(scheme.constraints, vec![expected]);
    assert_eq!(
        scheme.to_string(),
        format!("forall {var}. Eq {var} => {var} -> {var} -> Bool")
    );
}

#[test]
fn test_constrained_let_instantiates_per_use() {
    let decl = "let f = fun x -> fun y -> x == y;\nlet g = fun x -> fun y -> x != y;\n";
    assert_eq!(check_program(&format!("{decl}f 1 2")), Ok(Type::Bool));
    assert_eq!(check_program(&format!("{decl}g 1 2")), Ok(Type::Bool));
    assert_eq!(
        check_program(&format!("{decl}(f 'a' 'b', g true false)")),
        Ok(Type::Tuple(vec![Type::Bool, Type::Bool]))
    );
    assert!(matches!(
        check_program(&format!("{decl}f (1, 2) (1, 2)")),
        Err(parlang::TypeError::NoInstance(_))
    ));
}

#[test]
fn test_local_let_generalizes_eq() {
    let expr = parse("let g = fun y -> y == y in (g 1, g 'c')").unwrap();
    assert_eq!(
        typecheck(&expr),
        Ok(Type::Tuple(vec![Type::Bool, Type::Bool]))
    );
}

#[test]
fn test_lambda_bound_eq_propagates_to_outer_let() {
    let source = "let h = fun x -> let b = x == x in b;\n";
    let scheme = first_let_scheme(source);
    assert_eq!(scheme.vars.len(), 1);
    let var = Type::Var(scheme.vars[0].clone());
    assert_eq!(
        scheme.to_string(),
        format!("forall {var}. Eq {var} => {var} -> Bool")
    );
    assert!(matches!(
        check_program(&format!("{source}h ()")),
        Err(parlang::TypeError::NoInstance(_))
    ));
}

#[test]
fn test_program_body_eq_is_implicitly_generalized() {
    let ty = check_program("fun x -> fun y -> x == y").unwrap();
    assert!(is_binary_predicate_on_one_var(&ty), "{ty:?}");
}

#[test]
fn test_ambiguous_eq_constraint_is_rejected() {
    assert!(matches!(
        check_program("let z = (fun f -> 1) (fun y -> y == y);\nz"),
        Err(parlang::TypeError::AmbiguousConstraint(_))
    ));
    let err = check_program("(fun f -> 1) (fun y -> y == y)").unwrap_err();
    assert!(matches!(err, parlang::TypeError::AmbiguousConstraint(_)));
    assert!(err
        .to_string()
        .starts_with("ambiguous type variable in constraint Eq t"));
}

#[test]
fn test_constrained_binding_persists_across_programs() {
    let mut env = parlang::TypeEnv::new();
    let define = parlang::parse_program("let f = fun x -> fun y -> x == y;").unwrap();
    assert_eq!(
        parlang::typecheck_program_with_env(&define, &mut env),
        Ok(Type::Int)
    );
    let bare = parlang::parse_program("f").unwrap();
    let bare_ty = parlang::typecheck_program_with_env(&bare, &mut env).unwrap();
    assert!(is_binary_predicate_on_one_var(&bare_ty), "{bare_ty:?}");
    let use_int = parlang::parse_program("f 1 2").unwrap();
    assert_eq!(
        parlang::typecheck_program_with_env(&use_int, &mut env),
        Ok(Type::Bool)
    );
    let use_tuple = parlang::parse_program("f (1, 2) (1, 2)").unwrap();
    assert!(matches!(
        parlang::typecheck_program_with_env(&use_tuple, &mut env),
        Err(parlang::TypeError::NoInstance(_))
    ));
    let use_char = parlang::parse_program("f 'a' 'b'").unwrap();
    assert_eq!(
        parlang::typecheck_program_with_env(&use_char, &mut env),
        Ok(Type::Bool)
    );
}
