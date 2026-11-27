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
    assert_eq!(format!("{expr}"), "(fun (x : Int) -> x)");
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

#[test]
fn test_display_round_trips_annotated_fun() {
    let sources = [
        "fun (x : Int) -> x + 1",
        "fun (f : Int -> Int) -> f 1",
        "fun (x : a) -> x",
        "fun (b : Bool) -> fun (n : Int) -> if b then n else 0",
        "fun (x : Either (List a) b) -> x",
        "fun (t : Pair (Option Int) (List (List a))) -> t",
        "fun (g : List (a -> b)) -> g",
    ];
    for src in sources {
        let parsed = parse_expr(src).unwrap();
        let printed = format!("{parsed}");
        let reparsed = parse_expr(&printed).unwrap();
        assert_eq!(
            parsed, reparsed,
            "round-trip mismatch for {src:?}: printed as {printed:?}",
        );
    }

    // Exact printed forms for the two simplest cases.
    assert_eq!(
        format!("{}", parse_expr("fun (x : Int) -> x + 1").unwrap()),
        "(fun (x : Int) -> (x + 1))",
    );
    assert_eq!(
        format!("{}", parse_expr("fun (f : Int -> Int) -> f 1").unwrap()),
        "(fun (f : (Int -> Int)) -> (f 1))",
    );

    // Nested applied and function arguments keep their parentheses.
    let either_src = "fun (x : Either (List a) b) -> x";
    let either = parse_expr(either_src).unwrap();
    assert_eq!(format!("{either}"), "(fun (x : Either (List a) b) -> x)");
    let lambda_src = "fun (g : List (a -> b)) -> g";
    let lambda = parse_expr(lambda_src).unwrap();
    assert_eq!(format!("{lambda}"), "(fun (g : List (a -> b)) -> g)");
}

#[test]
fn test_display_parenthesises_nested_applied_annotation_args() {
    let src = "let p : Either (List a) b = q in p";
    let parsed = parse_expr(src).unwrap();
    let printed = format!("{parsed}");
    assert_eq!(printed, "(let p : Either (List a) b = q in p)");
    let reparsed = parse_expr(&printed).unwrap();
    assert_eq!(
        parsed, reparsed,
        "round-trip mismatch for {src:?}: printed as {printed:?}",
    );

    let deep_src = "fun (t : Pair (Option Int) (List (List a))) -> t";
    let deep = parse_expr(deep_src).unwrap();
    assert_eq!(
        format!("{deep}"),
        "(fun (t : Pair (Option Int) (List (List a))) -> t)",
    );
    let deep_reparsed = parse_expr(&format!("{deep}")).unwrap();
    assert_eq!(deep, deep_reparsed);
}

const LIST_DECL: &str = "data List a = Nil | Cons a (List a) in ";

fn string_example_with_body(body: &str) -> String {
    let src = include_str!("../examples/string.par");
    let prefix = src
        .trim_end()
        .strip_suffix('0')
        .expect("string.par ends with body 0");
    format!("{prefix}{body}")
}

fn list_char() -> Type {
    Type::SumType("List".to_string(), vec![Type::Char])
}

#[test]
fn test_typecheck_applied_type_annotation_with_declared_list() {
    let src = format!(
        "{LIST_DECL}(fun (xs : List Int) -> match xs with | Nil -> 0 | Cons h t -> h) (Cons 1 Nil)"
    );
    let expr = parse_expr(&src).unwrap();
    assert_eq!(typecheck(&expr), Ok(Type::Int));
}

#[test]
fn test_nullary_data_annotation_resolves() {
    let src =
        "data Color = Red | Green in (fun (c : Color) -> match c with | Red -> 1 | Green -> 2) Red";
    let expr = parse_expr(src).unwrap();
    assert_eq!(typecheck(&expr), Ok(Type::Int));
}

#[test]
fn test_applied_base_type_annotation_is_arity_error() {
    let expr = parse_expr("fun (x : Int Int) -> x").unwrap();
    assert_eq!(
        typecheck(&expr),
        Err(parlang::TypeError::TypeConstructorArityMismatch(
            "Int".to_string(),
            0,
            1
        ))
    );
}

#[test]
fn test_bare_parameterised_type_annotation_is_arity_error() {
    let src = format!("{LIST_DECL}fun (x : List) -> x");
    let expr = parse_expr(&src).unwrap();
    assert_eq!(
        typecheck(&expr),
        Err(parlang::TypeError::TypeConstructorArityMismatch(
            "List".to_string(),
            1,
            0
        ))
    );
}

#[test]
fn test_unknown_applied_type_annotation_is_rejected() {
    let expr = parse_expr("fun (x : Foo Int) -> x").unwrap();
    assert!(matches!(
        typecheck(&expr),
        Err(parlang::TypeError::UnboundVariable(_))
    ));
}

#[test]
fn test_string_example_strcmp_is_list_char() {
    let program = parse_program(&string_example_with_body("strcmp")).unwrap();
    let expected = Type::Fun(
        Box::new(list_char()),
        Box::new(Type::Fun(Box::new(list_char()), Box::new(Type::Int))),
    );
    assert_eq!(parlang::typecheck_program(&program), Ok(expected));

    let program = parse_program(&string_example_with_body("strcmp \"ab\" \"ac\"")).unwrap();
    assert_eq!(parlang::typecheck_program(&program), Ok(Type::Int));
}

#[test]
fn test_string_example_strcmp_rejects_int_lists() {
    let body = "strcmp (Cons 1 Nil) (Cons 2 Nil)";
    let program = parse_program(&string_example_with_body(body)).unwrap();
    assert!(parlang::typecheck_program(&program).is_err());
}

#[test]
fn test_annotation_type_variables_are_shared_within_one_annotation() {
    let expr = parse_expr("fun (f : a -> a) -> f").unwrap();
    let ty = typecheck(&expr).unwrap();
    let Type::Fun(param, result) = &ty else {
        panic!("expected a function type, got {ty:?}");
    };
    assert_eq!(param, result);
    let Type::Fun(arg, ret) = &**param else {
        panic!("expected a function parameter, got {param:?}");
    };
    assert_eq!(arg, ret);
    assert!(matches!(**arg, Type::Var(_)), "{arg:?}");
}

#[test]
fn test_shared_annotation_variable_rejects_mismatched_uses() {
    let src = "let f : a -> a = fun x -> if x then 1 else 2 in f";
    let mismatched = parse_expr(src).unwrap();
    assert!(typecheck(&mismatched).is_err());
    let src = "let k : a -> b -> a = fun x -> fun y -> y in k 1 true";
    let linked = parse_expr(src).unwrap();
    assert!(typecheck(&linked).is_err());
    let src = "let k : a -> b -> a = fun x -> fun y -> y in k 1 2";
    let consistent = parse_expr(src).unwrap();
    assert_eq!(typecheck(&consistent), Ok(Type::Int));
}

#[test]
fn test_annotation_type_variables_are_independent_across_annotations() {
    let src = "fun (x : a) -> fun (y : a) -> if y then x else 0";
    let expr = parse_expr(src).unwrap();
    let bool_to_int = Type::Fun(Box::new(Type::Bool), Box::new(Type::Int));
    let expected = Type::Fun(Box::new(Type::Int), Box::new(bool_to_int));
    assert_eq!(typecheck(&expr), Ok(expected));
}

#[test]
fn test_annotation_type_variables_stay_flexible() {
    let src = "let id : a -> a = fun x -> x + 1 in id 2";
    let expr = parse_expr(src).unwrap();
    assert_eq!(typecheck(&expr), Ok(Type::Int));
}

#[test]
fn test_variable_headed_annotation_typechecks() {
    let expr = parse_expr("fun (x : f a) -> x").unwrap();
    let ty = typecheck(&expr).unwrap();
    let Type::Fun(arg, ret) = &ty else {
        panic!("expected a function type, got {ty}");
    };
    assert_eq!(arg, ret);
    assert!(matches!(**arg, Type::VarApp(..)), "got {ty}");
}

#[test]
fn test_partially_applied_constructor_annotation() {
    let src = "data Option a = None | Some a ;\n\
               data Wrap f a = MkWrap (f a) ;\n\
               (fun (w : Wrap Option Int) -> w) (MkWrap (Some 1))";
    let program = parse_program(src).unwrap();
    let ty = parlang::typecheck_program(&program).unwrap();
    assert_eq!(ty.to_string(), "Wrap Option Int");
}

#[test]
fn test_wrap_annotation_with_variable_heads() {
    let src = "data Wrap f a = MkWrap (f a) ;\n\
               fun (w : Wrap f a) -> match w with | MkWrap x -> x";
    let program = parse_program(src).unwrap();
    let ty = parlang::typecheck_program(&program).unwrap();
    let Type::Fun(arg, ret) = &ty else {
        panic!("expected a function type, got {ty}");
    };
    assert!(matches!(&**arg, Type::SumType(name, args) if name == "Wrap" && args.len() == 2));
    assert!(matches!(**ret, Type::VarApp(..)), "got {ty}");
}

#[test]
fn test_type_application_is_left_associative_in_annotations() {
    let src = format!("{LIST_DECL}data Option a = None | Some a in fun (x : List Option Int) -> x");
    let expr = parse_expr(&src).unwrap();
    assert_eq!(
        typecheck(&expr),
        Err(parlang::TypeError::TypeConstructorArityMismatch(
            "List".to_string(),
            1,
            2
        ))
    );
}

/// Unconstrained data parameters default to kind `*` (Haskell 98), so a
/// constructor cannot be passed to them
#[test]
fn test_phantom_parameter_defaults_to_star() {
    let src = "data Option a = None | Some a ;\n\
               data Phantom f = P ;\n\
               fun (x : Phantom Option) -> x";
    let program = parse_program(src).unwrap();
    assert_eq!(
        parlang::typecheck_program(&program),
        Err(parlang::TypeError::TypeConstructorArityMismatch(
            "Option".to_string(),
            1,
            0
        ))
    );
}

/// An alias of a bare constructor is a partial application, rejected where
/// a plain type is expected
#[test]
fn test_constructor_alias_is_rejected_at_star_position() {
    let src = "type MyList = List ;\n\
               data List a = Nil | Cons a (List a) ;\n\
               fun (x : MyList) -> x";
    let program = parse_program(src).unwrap();
    assert_eq!(
        parlang::typecheck_program(&program),
        Err(parlang::TypeError::TypeConstructorArityMismatch(
            "MyList".to_string(),
            1,
            0
        ))
    );
}
