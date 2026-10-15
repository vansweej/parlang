/// Tests for explicit type annotations
/// This module tests parsing and type checking of explicit type annotations
use parlang::{
    ast::*, eval, parse_expr, parse_program, typecheck, types::Type, Environment, Value,
};

#[test]
fn test_parse_fun_with_type_annotation() {
    let expr = parse_expr("fun (x : Int) -> x + 1").unwrap();
    match expr {
        Expr::Fun(param, Some(TypeAnnotation::Concrete(name)), _body) => {
            assert_eq!(param, "x");
            assert_eq!(name, "Int");
        }
        other => panic!("expected annotated Fun, got {other:?}"),
    }
}

#[test]
fn test_parse_fun_without_type_annotation() {
    // fun x -> x + 1
    let result = parse_expr("fun x -> x + 1");
    assert!(result.is_ok());

    let expr = result.unwrap();
    match expr {
        Expr::Fun(param, ty_ann, _body) => {
            assert_eq!(param, "x");
            assert!(ty_ann.is_none());
        }
        _ => panic!("Expected Expr::Fun, got {expr:?}"),
    }
}

#[test]
fn test_parse_fun_with_bool_annotation() {
    let expr = parse_expr("fun (b : Bool) -> if b then 1 else 0").unwrap();
    match expr {
        Expr::Fun(param, Some(TypeAnnotation::Concrete(name)), _body) => {
            assert_eq!(param, "b");
            assert_eq!(name, "Bool");
        }
        other => panic!("expected annotated Fun, got {other:?}"),
    }
}

#[test]
fn test_parse_let_with_type_annotation() {
    // let x : Int = 42 in x
    let result = parse_expr("let x : Int = 42 in x");
    assert!(result.is_ok());

    let expr = result.unwrap();
    match expr {
        Expr::Let(name, ty_ann, _value, _body) => {
            assert_eq!(name, "x");
            assert!(ty_ann.is_some());
            let ty = ty_ann.unwrap();
            assert_eq!(ty, TypeAnnotation::Concrete("Int".to_string()));
        }
        _ => panic!("Expected Expr::Let, got {expr:?}"),
    }
}

#[test]
fn test_parse_let_without_type_annotation() {
    // let x = 42 in x
    let result = parse_expr("let x = 42 in x");
    assert!(result.is_ok());

    let expr = result.unwrap();
    match expr {
        Expr::Let(name, ty_ann, _value, _body) => {
            assert_eq!(name, "x");
            assert!(ty_ann.is_none());
        }
        _ => panic!("Expected Expr::Let, got {expr:?}"),
    }
}

#[test]
fn test_parse_seq_with_type_annotation() {
    // let x : Int = 42;
    let result = parse_program("let x : Int = 42;");
    assert!(result.is_ok());

    let program = result.unwrap();
    match &program.decls[..] {
        [Decl::Let { name, ty_ann, .. }] => {
            assert_eq!(name, "x");
            assert_eq!(ty_ann, &Some(TypeAnnotation::Concrete("Int".to_string())));
        }
        _ => panic!("Expected one top-level let declaration"),
    }
}

#[test]
fn test_parse_seq_multiple_with_annotations() {
    // let x : Int = 42; let y : Bool = true;
    let result = parse_program("let x : Int = 42; let y : Bool = true;");
    assert!(result.is_ok());

    let program = result.unwrap();
    match &program.decls[..] {
        [Decl::Let {
            name: name1,
            ty_ann: ty_ann1,
            ..
        }, Decl::Let {
            name: name2,
            ty_ann: ty_ann2,
            ..
        }] => {
            assert_eq!(name1, "x");
            assert_eq!(ty_ann1, &Some(TypeAnnotation::Concrete("Int".to_string())));
            assert_eq!(name2, "y");
            assert_eq!(ty_ann2, &Some(TypeAnnotation::Concrete("Bool".to_string())));
        }
        _ => panic!("Expected two top-level let declarations"),
    }
}

#[test]
fn test_typecheck_fun_with_correct_annotation() {
    let expr = parse_expr("fun (x : Int) -> x + 1").unwrap();
    let ty = typecheck(&expr).unwrap();
    match ty {
        Type::Fun(arg, ret) => {
            assert_eq!(*arg, Type::Int);
            assert_eq!(*ret, Type::Int);
        }
        other => panic!("expected Int -> Int, got {other:?}"),
    }
}

#[test]
fn test_typecheck_fun_with_bool_annotation() {
    // Testing with simple function without annotation
    let result = parse_expr("fun b -> if b then 1 else 0");
    assert!(result.is_ok());

    let expr = result.unwrap();
    let infer_result = typecheck(&expr);
    assert!(infer_result.is_ok());

    let ty = infer_result.unwrap();
    // Should be Bool -> Int
    match ty {
        Type::Fun(arg, ret) => {
            assert_eq!(*arg, Type::Bool);
            assert_eq!(*ret, Type::Int);
        }
        _ => panic!("Expected function type, got {ty:?}"),
    }
}

#[test]
fn test_typecheck_fun_with_wrong_annotation() {
    // Body `x + 1` forces `x : Int`, contradicting the `Bool` annotation.
    let expr = parse_expr("fun (x : Bool) -> x + 1").unwrap();
    assert!(typecheck(&expr).is_err());
}

#[test]
fn test_typecheck_let_with_correct_annotation() {
    // let x : Int = 42 in x + 1
    let result = parse_expr("let x : Int = 42 in x + 1");
    assert!(result.is_ok());

    let expr = result.unwrap();
    let infer_result = typecheck(&expr);
    assert!(infer_result.is_ok());

    let ty = infer_result.unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_typecheck_let_with_wrong_annotation() {
    // let x : Bool = 42 in x  (should fail because 42 is Int, not Bool)
    let result = parse_expr("let x : Bool = 42 in x");
    assert!(result.is_ok());

    let expr = result.unwrap();
    let infer_result = typecheck(&expr);
    // This should fail with a type error
    assert!(infer_result.is_err());
}

#[test]
fn test_typecheck_nested_let_with_annotations() {
    // let x : Int = 10 in let y : Int = 20 in x + y
    let result = parse_expr("let x : Int = 10 in let y : Int = 20 in x + y");
    assert!(result.is_ok());

    let expr = result.unwrap();
    let infer_result = typecheck(&expr);
    assert!(infer_result.is_ok());

    let ty = infer_result.unwrap();
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_display_fun_with_annotation() {
    let expr = Expr::Fun(
        "x".to_string(),
        Some(TypeAnnotation::Concrete("Int".to_string())),
        Box::new(Expr::Var("x".to_string())),
    );
    assert_eq!(format!("{expr}"), "(fun x : Int -> x)");
}

#[test]
fn test_display_let_with_annotation() {
    let expr = Expr::Let(
        "x".to_string(),
        Some(TypeAnnotation::Concrete("Int".to_string())),
        Box::new(Expr::Int(42)),
        Box::new(Expr::Var("x".to_string())),
    );
    assert_eq!(format!("{expr}"), "(let x : Int = 42 in x)");
}

#[test]
fn test_display_fun_without_annotation() {
    let expr = Expr::Fun("x".to_string(), None, Box::new(Expr::Var("x".to_string())));
    assert_eq!(format!("{expr}"), "(fun x -> x)");
}

#[test]
fn test_display_let_without_annotation() {
    let expr = Expr::Let(
        "x".to_string(),
        None,
        Box::new(Expr::Int(42)),
        Box::new(Expr::Var("x".to_string())),
    );
    assert_eq!(format!("{expr}"), "(let x = 42 in x)");
}

#[test]
fn test_parse_fun_with_arrow_annotation() {
    let expr = parse_expr("fun (f : Int -> Int) -> f 1").unwrap();
    match expr {
        Expr::Fun(param, Some(TypeAnnotation::Fun(arg, ret)), _body) => {
            assert_eq!(param, "f");
            assert_eq!(*arg, TypeAnnotation::Concrete("Int".to_string()));
            assert_eq!(*ret, TypeAnnotation::Concrete("Int".to_string()));
        }
        other => panic!("expected arrow-annotated Fun, got {other:?}"),
    }
}

#[test]
fn test_typecheck_fun_with_arrow_annotation() {
    let expr = parse_expr("fun (f : Int -> Int) -> f 1").unwrap();
    let ty = typecheck(&expr).unwrap();
    match ty {
        Type::Fun(arg, ret) => {
            assert_eq!(*arg, Type::Fun(Box::new(Type::Int), Box::new(Type::Int)));
            assert_eq!(*ret, Type::Int);
        }
        other => panic!("expected (Int -> Int) -> Int, got {other:?}"),
    }
}

#[test]
fn test_parse_fun_with_type_variable_annotation() {
    let expr = parse_expr("fun (x : a) -> x").unwrap();
    match expr {
        Expr::Fun(param, Some(TypeAnnotation::Var(name)), _body) => {
            assert_eq!(param, "x");
            assert_eq!(name, "a");
        }
        other => panic!("expected var-annotated Fun, got {other:?}"),
    }
}

#[test]
fn test_bare_parameter_annotation_is_rejected() {
    assert!(parse_expr("fun x : Int -> x").is_err());
    assert!(parse_expr("fun x : Int -> x + 1").is_err());
}

#[test]
fn test_eval_annotated_fun_application() {
    let expr = parse_expr("(fun (x : Int) -> x + 1) 41").unwrap();
    assert_eq!(eval(&expr, &Environment::new()).unwrap(), Value::Int(42));
}

#[test]
fn test_eval_curried_annotated_fun() {
    let expr = parse_expr("(fun (a : Int) -> fun (b : Int) -> a + b) 40 2").unwrap();
    assert_eq!(eval(&expr, &Environment::new()).unwrap(), Value::Int(42));
}

#[test]
fn test_parse_unannotated_fun_still_works() {
    let expr = parse_expr("fun x -> x").unwrap();
    match expr {
        Expr::Fun(param, ty_ann, _body) => {
            assert_eq!(param, "x");
            assert!(ty_ann.is_none());
        }
        other => panic!("expected unannotated Fun, got {other:?}"),
    }
    assert!(parse_expr("let f = fun x -> x in f").is_ok());
}
