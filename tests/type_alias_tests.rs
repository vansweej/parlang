/// Tests for type alias functionality
use parlang::{eval, parse, typecheck, Environment};

/// Test parsing a simple type alias
#[test]
fn test_parse_simple_type_alias() {
    let input = "type MyInt = Int in 42";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing a function type alias
#[test]
fn test_parse_function_type_alias() {
    let input = "type MyFunc = Int -> Int in fun x -> x + 1";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing a complex function type alias
#[test]
fn test_parse_complex_function_type_alias() {
    let input = "type MyFunc = Int -> Bool -> Int in 42";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test parsing a type alias with higher-order function
#[test]
fn test_parse_higher_order_type_alias() {
    let input = "type HigherOrder = (Int -> Int) -> Int in 42";
    let result = parse(input);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
}

/// Test evaluation of expression with type alias
#[test]
fn test_eval_simple_type_alias() {
    let input = "type MyInt = Int in 42";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test evaluation with type alias and let binding
#[test]
fn test_eval_type_alias_with_let() {
    let input = "type MyInt = Int in let x = 10 in x + 32";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test evaluation with function type alias
#[test]
fn test_eval_function_type_alias() {
    let input = "type MyFunc = Int -> Int in let f = fun x -> x + 1 in f 41";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test type checking with simple type alias
#[test]
fn test_typecheck_simple_type_alias() {
    let input = "type MyInt = Int in 42";
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Typecheck failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "Int");
}

/// Test type checking with function type alias
#[test]
fn test_typecheck_function_type_alias() {
    let input = "type MyFunc = Int -> Int in fun x -> x + 1";
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Typecheck failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "Int -> Int");
}

/// Test type checking with complex function type alias
#[test]
fn test_typecheck_complex_function_type_alias() {
    let input = "type MyFunc = Int -> Bool in fun x -> x == 0";
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Typecheck failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "Int -> Bool");
}

/// Test nested type aliases
#[test]
fn test_nested_type_aliases() {
    let input = "type A = Int in type B = Bool in if true then 1 else 2";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "1");
}

/// Test type checking with nested type aliases
#[test]
fn test_typecheck_nested_type_aliases() {
    let input = "type A = Int in type B = Bool in 42";
    let expr = parse(input).expect("Parse failed");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "Typecheck failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "Int");
}

/// Test type alias with let polymorphism
#[test]
fn test_type_alias_with_polymorphism() {
    let input = "type Identity = Int -> Int in let id = fun x -> x in id 42";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test type alias doesn't affect evaluation result
#[test]
fn test_type_alias_transparent() {
    let input1 = "42";
    let input2 = "type MyInt = Int in 42";
    
    let expr1 = parse(input1).expect("Parse failed");
    let expr2 = parse(input2).expect("Parse failed");
    
    let result1 = eval(&expr1, &Environment::new()).expect("Eval failed");
    let result2 = eval(&expr2, &Environment::new()).expect("Eval failed");
    
    assert_eq!(format!("{}", result1), format!("{}", result2));
}

/// Test multiple type aliases in sequence
#[test]
fn test_multiple_type_aliases() {
    let input = r#"
        type IntFunc = Int -> Int in
        type BoolFunc = Bool -> Bool in
        let f = fun x -> x + 1 in
        f 41
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test type alias with curried function
#[test]
fn test_type_alias_with_currying() {
    let input = "type BinOp = Int -> Int -> Int in let add = fun x -> fun y -> x + y in (add 10) 32";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test type alias with conditional
#[test]
fn test_type_alias_with_conditional() {
    let input = "type MyInt = Int in if true then 42 else 0";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test Display implementation for TypeExpr
#[test]
fn test_type_expr_display() {
    use parlang::ast::TypeExpr;
    
    assert_eq!(format!("{}", TypeExpr::Int), "Int");
    assert_eq!(format!("{}", TypeExpr::Bool), "Bool");
    assert_eq!(
        format!("{}", TypeExpr::Fun(Box::new(TypeExpr::Int), Box::new(TypeExpr::Bool))),
        "Int -> Bool"
    );
    assert_eq!(format!("{}", TypeExpr::Alias("MyType".to_string())), "MyType");
}

/// Test Display implementation for TypeAlias expression
#[test]
fn test_type_alias_expr_display() {
    use parlang::ast::{Expr, TypeExpr};
    
    let expr = Expr::TypeAlias(
        "MyInt".to_string(),
        TypeExpr::Int,
        Box::new(Expr::Int(42)),
    );
    assert_eq!(format!("{}", expr), "(type MyInt = Int in 42)");
}

/// Test that type keyword is reserved
#[test]
fn test_type_keyword_reserved() {
    let input = "let type = 42 in type";
    let result = parse(input);
    assert!(result.is_err(), "Should fail to parse 'type' as identifier");
}

/// Test type alias in complex expression
#[test]
fn test_type_alias_complex_expression() {
    let input = r#"
        type IntFunc = Int -> Int in
        let double = fun x -> x + x in
        let triple = fun x -> x + x + x in
        (double 10) + (triple 7)
    "#;
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "41");
}

/// Test type alias with comparison
#[test]
fn test_type_alias_with_comparison() {
    let input = "type MyInt = Int in if 10 > 5 then 42 else 0";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
    assert_eq!(format!("{}", result.unwrap()), "42");
}

/// Test type alias equality and cloning
#[test]
fn test_type_expr_equality() {
    use parlang::ast::TypeExpr;
    
    let ty1 = TypeExpr::Int;
    let ty2 = TypeExpr::Int;
    assert_eq!(ty1, ty2);
    
    let ty3 = ty1.clone();
    assert_eq!(ty1, ty3);
}

/// Test type alias with parenthesized type expression
#[test]
fn test_type_alias_parenthesized() {
    let input = "type HigherOrder = (Int -> Int) -> Bool in 42";
    let expr = parse(input).expect("Parse failed");
    let result = eval(&expr, &Environment::new());
    assert!(result.is_ok(), "Eval failed: {:?}", result.err());
}
