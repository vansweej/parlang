/// Integration tests to prove exhaustiveness checking works
/// These tests verify warnings by checking the exhaustiveness results directly
use parlang::{check_exhaustiveness, parse, Environment};
use parlang::ast::Pattern;

/// Test that exhaustive Option match has no warnings
#[test]
fn prove_exhaustive_option_match() {
    let code = r#"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | None -> 0
    "#;
    
    let expr = parse(code).expect("Failed to parse");
    let env = Environment::new();
    let result = parlang::eval(&expr, &env);
    
    // Should succeed without runtime errors
    assert!(result.is_ok(), "Exhaustive match should succeed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test that non-exhaustive Option match (missing None) gets warning
#[test]
fn prove_non_exhaustive_option_missing_none() {
    use parlang::eval::ConstructorInfo;
    
    // Create environment with Option constructors
    let mut env = Environment::new();
    env.register_constructor("Some".to_string(), ConstructorInfo {
        type_name: "Option".to_string(),
        arity: 1,
    });
    env.register_constructor("None".to_string(), ConstructorInfo {
        type_name: "Option".to_string(),
        arity: 0,
    });
    
    // Patterns: only Some
    let patterns = vec![
        Pattern::Constructor("Some".to_string(), vec![Pattern::Wildcard]),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Should be non-exhaustive");
    
    if let parlang::ExhaustivenessResult::NonExhaustive(missing) = result {
        assert!(missing.contains(&"None".to_string()), 
                "Should report None as missing, got: {:?}", missing);
    }
}

/// Test that non-exhaustive Option match (missing Some) gets warning
#[test]
fn prove_non_exhaustive_option_missing_some() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Some".to_string(), ConstructorInfo {
        type_name: "Option".to_string(),
        arity: 1,
    });
    env.register_constructor("None".to_string(), ConstructorInfo {
        type_name: "Option".to_string(),
        arity: 0,
    });
    
    // Patterns: only None
    let patterns = vec![
        Pattern::Constructor("None".to_string(), vec![]),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Should be non-exhaustive");
    
    if let parlang::ExhaustivenessResult::NonExhaustive(missing) = result {
        assert!(missing.contains(&"Some".to_string()), 
                "Should report Some as missing, got: {:?}", missing);
    }
}

/// Test exhaustive Either match
#[test]
fn prove_exhaustive_either_match() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Left".to_string(), ConstructorInfo {
        type_name: "Either".to_string(),
        arity: 1,
    });
    env.register_constructor("Right".to_string(), ConstructorInfo {
        type_name: "Either".to_string(),
        arity: 1,
    });
    
    // Patterns: both Left and Right
    let patterns = vec![
        Pattern::Constructor("Left".to_string(), vec![Pattern::Wildcard]),
        Pattern::Constructor("Right".to_string(), vec![Pattern::Wildcard]),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(result.is_exhaustive(), "Should be exhaustive");
}

/// Test non-exhaustive Either match (missing Right)
#[test]
fn prove_non_exhaustive_either_missing_right() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Left".to_string(), ConstructorInfo {
        type_name: "Either".to_string(),
        arity: 1,
    });
    env.register_constructor("Right".to_string(), ConstructorInfo {
        type_name: "Either".to_string(),
        arity: 1,
    });
    
    // Patterns: only Left
    let patterns = vec![
        Pattern::Constructor("Left".to_string(), vec![Pattern::Wildcard]),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Should be non-exhaustive");
    
    if let parlang::ExhaustivenessResult::NonExhaustive(missing) = result {
        assert!(missing.contains(&"Right".to_string()), 
                "Should report Right as missing, got: {:?}", missing);
    }
}

/// Test exhaustive boolean match
#[test]
fn prove_exhaustive_bool_match() {
    use parlang::ast::Literal;
    
    let env = Environment::new();
    
    // Patterns: true and false
    let patterns = vec![
        Pattern::Literal(Literal::Bool(true)),
        Pattern::Literal(Literal::Bool(false)),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(result.is_exhaustive(), "Should be exhaustive");
}

/// Test non-exhaustive boolean match (missing false)
#[test]
fn prove_non_exhaustive_bool_missing_false() {
    use parlang::ast::Literal;
    
    let env = Environment::new();
    
    // Patterns: only true
    let patterns = vec![
        Pattern::Literal(Literal::Bool(true)),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Should be non-exhaustive");
    
    if let parlang::ExhaustivenessResult::NonExhaustive(missing) = result {
        assert!(missing.contains(&"false".to_string()), 
                "Should report false as missing, got: {:?}", missing);
    }
}

/// Test non-exhaustive boolean match (missing true)
#[test]
fn prove_non_exhaustive_bool_missing_true() {
    use parlang::ast::Literal;
    
    let env = Environment::new();
    
    // Patterns: only false
    let patterns = vec![
        Pattern::Literal(Literal::Bool(false)),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Should be non-exhaustive");
    
    if let parlang::ExhaustivenessResult::NonExhaustive(missing) = result {
        assert!(missing.contains(&"true".to_string()), 
                "Should report true as missing, got: {:?}", missing);
    }
}

/// Test wildcard makes match exhaustive
#[test]
fn prove_wildcard_makes_exhaustive() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Some".to_string(), ConstructorInfo {
        type_name: "Option".to_string(),
        arity: 1,
    });
    env.register_constructor("None".to_string(), ConstructorInfo {
        type_name: "Option".to_string(),
        arity: 0,
    });
    
    // Patterns: Some and wildcard
    let patterns = vec![
        Pattern::Constructor("Some".to_string(), vec![Pattern::Wildcard]),
        Pattern::Wildcard,
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(result.is_exhaustive(), "Wildcard should make it exhaustive");
}

/// Test variable pattern makes match exhaustive
#[test]
fn prove_variable_makes_exhaustive() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Some".to_string(), ConstructorInfo {
        type_name: "Option".to_string(),
        arity: 1,
    });
    env.register_constructor("None".to_string(), ConstructorInfo {
        type_name: "Option".to_string(),
        arity: 0,
    });
    
    // Patterns: Some and variable
    let patterns = vec![
        Pattern::Constructor("Some".to_string(), vec![Pattern::Wildcard]),
        Pattern::Var("x".to_string()),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(result.is_exhaustive(), "Variable pattern should make it exhaustive");
}

/// Test exhaustive List type match
#[test]
fn prove_exhaustive_list_match() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Nil".to_string(), ConstructorInfo {
        type_name: "List".to_string(),
        arity: 0,
    });
    env.register_constructor("Cons".to_string(), ConstructorInfo {
        type_name: "List".to_string(),
        arity: 2,
    });
    
    // Patterns: Nil and Cons
    let patterns = vec![
        Pattern::Constructor("Nil".to_string(), vec![]),
        Pattern::Constructor("Cons".to_string(), vec![Pattern::Wildcard, Pattern::Wildcard]),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(result.is_exhaustive(), "Should be exhaustive");
}

/// Test non-exhaustive List match (missing Nil)
#[test]
fn prove_non_exhaustive_list_missing_nil() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Nil".to_string(), ConstructorInfo {
        type_name: "List".to_string(),
        arity: 0,
    });
    env.register_constructor("Cons".to_string(), ConstructorInfo {
        type_name: "List".to_string(),
        arity: 2,
    });
    
    // Patterns: only Cons
    let patterns = vec![
        Pattern::Constructor("Cons".to_string(), vec![Pattern::Wildcard, Pattern::Wildcard]),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Should be non-exhaustive");
    
    if let parlang::ExhaustivenessResult::NonExhaustive(missing) = result {
        assert!(missing.contains(&"Nil".to_string()), 
                "Should report Nil as missing, got: {:?}", missing);
    }
}

/// Test integer patterns are non-exhaustive without wildcard
#[test]
fn prove_int_patterns_non_exhaustive() {
    use parlang::ast::Literal;
    
    let env = Environment::new();
    
    // Patterns: 0 and 1
    let patterns = vec![
        Pattern::Literal(Literal::Int(0)),
        Pattern::Literal(Literal::Int(1)),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Integer patterns without wildcard should be non-exhaustive");
}

/// Test integer patterns with wildcard are exhaustive
#[test]
fn prove_int_patterns_with_wildcard_exhaustive() {
    use parlang::ast::Literal;
    
    let env = Environment::new();
    
    // Patterns: 0, 1, and wildcard
    let patterns = vec![
        Pattern::Literal(Literal::Int(0)),
        Pattern::Literal(Literal::Int(1)),
        Pattern::Wildcard,
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(result.is_exhaustive(), "Wildcard should make integer patterns exhaustive");
}

/// Test that empty pattern list is non-exhaustive
#[test]
fn prove_empty_patterns_non_exhaustive() {
    let env = Environment::new();
    let patterns: Vec<Pattern> = vec![];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Empty pattern list should be non-exhaustive");
}

/// Test multiple constructors with all covered
#[test]
fn prove_multiple_constructors_all_covered() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Active".to_string(), ConstructorInfo {
        type_name: "Status".to_string(),
        arity: 0,
    });
    env.register_constructor("Inactive".to_string(), ConstructorInfo {
        type_name: "Status".to_string(),
        arity: 0,
    });
    env.register_constructor("Pending".to_string(), ConstructorInfo {
        type_name: "Status".to_string(),
        arity: 0,
    });
    env.register_constructor("Archived".to_string(), ConstructorInfo {
        type_name: "Status".to_string(),
        arity: 0,
    });
    
    // Patterns: all four constructors
    let patterns = vec![
        Pattern::Constructor("Active".to_string(), vec![]),
        Pattern::Constructor("Inactive".to_string(), vec![]),
        Pattern::Constructor("Pending".to_string(), vec![]),
        Pattern::Constructor("Archived".to_string(), vec![]),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(result.is_exhaustive(), "All constructors covered should be exhaustive");
}

/// Test multiple constructors with some missing
#[test]
fn prove_multiple_constructors_some_missing() {
    use parlang::eval::ConstructorInfo;
    
    let mut env = Environment::new();
    env.register_constructor("Active".to_string(), ConstructorInfo {
        type_name: "Status".to_string(),
        arity: 0,
    });
    env.register_constructor("Inactive".to_string(), ConstructorInfo {
        type_name: "Status".to_string(),
        arity: 0,
    });
    env.register_constructor("Pending".to_string(), ConstructorInfo {
        type_name: "Status".to_string(),
        arity: 0,
    });
    env.register_constructor("Archived".to_string(), ConstructorInfo {
        type_name: "Status".to_string(),
        arity: 0,
    });
    
    // Patterns: only Active and Inactive
    let patterns = vec![
        Pattern::Constructor("Active".to_string(), vec![]),
        Pattern::Constructor("Inactive".to_string(), vec![]),
    ];
    
    let result = check_exhaustiveness(&patterns, &env);
    assert!(!result.is_exhaustive(), "Should be non-exhaustive");
    
    if let parlang::ExhaustivenessResult::NonExhaustive(missing) = result {
        // Should report both Pending and Archived as missing
        assert!(missing.len() == 2, "Should have 2 missing constructors, got: {:?}", missing);
        assert!(missing.contains(&"Pending".to_string()), "Should have Pending");
        assert!(missing.contains(&"Archived".to_string()), "Should have Archived");
    }
}

/// Integration test: Full program execution with exhaustive match
#[test]
fn prove_full_program_exhaustive() {
    let code = r#"
        type Option a = Some a | None in
        match Some 42 with
        | Some n -> n
        | None -> 0
    "#;
    
    let expr = parse(code).expect("Failed to parse");
    let env = Environment::new();
    let result = parlang::eval(&expr, &env);
    
    assert!(result.is_ok(), "Exhaustive match should succeed");
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Integration test: Full program execution with recursive function
#[test]
fn prove_full_program_with_recursion() {
    let code = r#"
        type List a = Nil | Cons a (List a) in
        (rec sum -> fun list ->
            match list with
            | Nil -> 0
            | Cons head tail -> head + sum tail)
        (Cons 1 (Cons 2 (Cons 3 Nil)))
    "#;
    
    let expr = parse(code).expect("Failed to parse");
    let env = Environment::new();
    let result = parlang::eval(&expr, &env);
    
    assert!(result.is_ok(), "Exhaustive recursive match should succeed");
    assert_eq!(format!("{}", result.unwrap()), "6");
}
