use parlang::ast::TypeAnnotation;
use parlang::{
    eval_program, eval_program_with_env, parse_program, program_to_dot, typecheck_program,
    typecheck_program_with_env, Decl, Environment, EvalError, Expr, Program, TypeEnv, TypeError,
    Value,
};

fn run(source: &str) -> Result<Value, String> {
    let program = parse_program(source)?;
    typecheck_program(&program).map_err(|error| error.to_string())?;
    eval_program(&program, &Environment::new()).map_err(|error| error.to_string())
}

#[test]
fn top_level_data_constructs_a_value() {
    assert_eq!(
        run("data Color = Red | Green | Blue ; Red"),
        Ok(Value::Variant("Red".to_string(), vec![]))
    );
}

#[test]
fn generic_top_level_data_constructs_a_value() {
    assert_eq!(
        run("data Option a = None | Some a ; Some 1"),
        Ok(Value::Variant("Some".to_string(), vec![Value::Int(1)]))
    );
}

#[test]
fn top_level_type_alias_is_available_to_annotations() {
    assert_eq!(
        run("type Name = Int ; let name : Name = 5 ; name"),
        Ok(Value::Int(5))
    );
}

#[test]
fn top_level_let_uses_a_predeclared_data_constructor() {
    assert_eq!(
        run("data Box = Mk ; let value = Mk ; value"),
        Ok(Value::Variant("Mk".to_string(), vec![]))
    );
}

#[test]
fn top_level_constructor_is_available_before_its_data_declaration() {
    assert_eq!(
        run("let value = Mk ; data Box = Mk ; value"),
        Ok(Value::Variant("Mk".to_string(), vec![]))
    );
}

#[test]
fn type_alias_can_name_a_later_data_type() {
    let program =
        parse_program("type MyList = List ; data List a = Nil | Cons a (List a) ; Nil").unwrap();

    assert!(typecheck_program(&program).is_ok());
}

#[test]
fn type_alias_can_reference_a_later_alias() {
    assert_eq!(
        run("type A = B ; type B = Int ; let value : A = 3 ; value"),
        Ok(Value::Int(3))
    );
}

#[test]
fn payload_forward_reference_to_later_data_type_is_accepted() {
    // Payload names are checked at declaration time after every top-level
    // data arity is registered, so `B` may be declared after its use in `A`.
    assert_eq!(
        run("data A = MkA B ; data B = MkB ; MkB"),
        Ok(Value::Variant("MkB".to_string(), vec![]))
    );
}

#[test]
fn alias_cycles_are_rejected() {
    for source in ["type A = B ; type B = A ; 0", "type A = A ; 0"] {
        let program = parse_program(source).unwrap();
        assert!(matches!(
            typecheck_program(&program),
            Err(TypeError::TypeAliasCycle(_))
        ));
    }
}

#[test]
fn duplicate_top_level_type_names_are_rejected() {
    let program = parse_program("data Foo = A ; data Foo = B ; 0").unwrap();
    assert!(matches!(
        typecheck_program(&program),
        Err(TypeError::DuplicateTypeName(name)) if name == "Foo"
    ));
}

#[test]
fn duplicate_top_level_constructor_names_are_rejected() {
    let program = parse_program("data Foo = Dup ; data Bar = Dup ; 0").unwrap();
    assert!(matches!(
        typecheck_program(&program),
        Err(TypeError::DuplicateConstructor(name)) if name == "Dup"
    ));
}

fn single_parsed_decl(source: &str) -> Decl {
    let program = parse_program(source).unwrap();
    assert_eq!(program.body, None);
    assert_eq!(program.decls.len(), 1, "expected one decl in {source}");
    program.decls[0].clone()
}

#[test]
fn class_declaration_parses_name_param_and_methods() {
    let Decl::Class {
        name,
        param,
        superclasses,
        methods,
        doc,
    } = single_parsed_decl("class Eq a where { eq : a -> a -> Bool };")
    else {
        panic!("expected a class declaration");
    };
    assert_eq!(name, "Eq");
    assert_eq!(param, "a");
    assert!(superclasses.is_empty());
    assert_eq!(doc, None);
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].0, "eq");
    assert_eq!(methods[0].1.to_string(), "(a -> (a -> Bool))");
}

#[test]
fn class_declaration_accepts_trailing_semicolon_and_higher_kinded_param() {
    let Decl::Class { param, methods, .. } =
        single_parsed_decl("class Container f where { empty : f Int; size : f Int -> Int; };")
    else {
        panic!("expected a class declaration");
    };
    assert_eq!(param, "f");
    let names: Vec<&str> = methods.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["empty", "size"]);
    assert_eq!(
        methods[0].1,
        TypeAnnotation::VarApp(
            "f".to_string(),
            vec![TypeAnnotation::Concrete("Int".to_string())]
        )
    );
}

#[test]
fn class_declaration_accepts_empty_body() {
    let Decl::Class { name, methods, .. } = single_parsed_decl("class Marker a where {};") else {
        panic!("expected a class declaration");
    };
    assert_eq!(name, "Marker");
    assert!(methods.is_empty());
}

#[test]
fn instance_without_context_parses() {
    let Decl::Instance {
        class,
        context,
        head,
        methods,
        ..
    } = single_parsed_decl("instance Eq Int where { eq = fun x -> fun y -> x == y };")
    else {
        panic!("expected an instance declaration");
    };
    assert_eq!(class, "Eq");
    assert!(context.is_empty());
    assert_eq!(head, TypeAnnotation::Concrete("Int".to_string()));
    assert_eq!(methods.len(), 1);
    assert_eq!(methods[0].0, "eq");
}

#[test]
fn instance_with_single_context_parses() {
    let Decl::Instance { context, head, .. } = single_parsed_decl(
        "instance Eq a => Eq (Option a) where { eq = fun x -> fun y -> true; };",
    ) else {
        panic!("expected an instance declaration");
    };
    assert_eq!(context, vec![("Eq".to_string(), "a".to_string())]);
    assert_eq!(
        head,
        TypeAnnotation::App(
            "Option".to_string(),
            vec![TypeAnnotation::Var("a".to_string())]
        )
    );
}

#[test]
fn instance_with_parenthesised_context_parses() {
    let Decl::Instance { context, head, .. } = single_parsed_decl(
        "instance (Eq a, Eq b) => Eq (Pair a b) where { eq = fun x -> fun y -> true };",
    ) else {
        panic!("expected an instance declaration");
    };
    assert_eq!(
        context,
        vec![
            ("Eq".to_string(), "a".to_string()),
            ("Eq".to_string(), "b".to_string())
        ]
    );
    assert_eq!(
        head,
        TypeAnnotation::App(
            "Pair".to_string(),
            vec![
                TypeAnnotation::Var("a".to_string()),
                TypeAnnotation::Var("b".to_string())
            ]
        )
    );
}

#[test]
fn instance_for_type_constructor_parses() {
    let Decl::Instance { head, methods, .. } =
        single_parsed_decl("instance Container List where { empty = Nil; size = fun xs -> 0 };")
    else {
        panic!("expected an instance declaration");
    };
    assert_eq!(head, TypeAnnotation::Concrete("List".to_string()));
    let names: Vec<&str> = methods.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, vec!["empty", "size"]);
}

#[test]
fn class_and_instance_mix_with_ordinary_declarations() {
    let source = "let one = 1; class Marker a where {}; instance Marker Int where {}; one";
    let program = parse_program(source).unwrap();
    assert_eq!(program.decls.len(), 3);
    assert!(matches!(program.decls[1], Decl::Class { .. }));
    assert!(matches!(program.decls[2], Decl::Instance { .. }));
    assert!(program.body.is_some());
}

#[test]
fn class_keywords_are_not_identifiers() {
    for source in [
        "let where = 1; 0",
        "let class = 1; 0",
        "let instance = 1; 0",
    ] {
        assert!(parse_program(source).is_err(), "{source} should not parse");
    }
}

#[test]
fn malformed_class_and_instance_headers_are_rejected() {
    for source in [
        "class Eq a => Ord a where {};",
        "class Eq a b where {};",
        "class Eq a where {}",
        "class C in where {};",
        "class C where where {};",
        "class Eq a where { eq = 1 };",
        "instance Eq a >= Eq Int where {};",
        "instance Eq a = Eq Int where {};",
        "instance Eq where {};",
        "instance Eq Int where { eq : Int };",
    ] {
        assert!(parse_program(source).is_err(), "{source} should not parse");
    }
}

#[test]
fn redeclaring_the_builtin_eq_class_is_rejected() {
    assert_eq!(
        run("class Eq a where { eq : a -> a -> Bool }; 1"),
        Err("Duplicate class: Eq".to_string())
    );
}

#[test]
fn parsed_user_class_program_is_rejected_by_the_evaluator() {
    assert_eq!(
        run("class Show a where { show : a -> Int }; 1"),
        Err("type classes are not executable yet: class Show".to_string())
    );
}

#[test]
fn parsed_instance_program_is_rejected_by_the_evaluator() {
    let source = "instance Eq Int where { eq = fun x -> fun y -> x == y }; 1";
    let program = parse_program(source).unwrap();
    assert_eq!(
        eval_program(&program, &Environment::new()),
        Err(EvalError::ClassesNotExecutable("instance Eq".to_string()))
    );
}

#[test]
fn identifiers_prefixed_by_class_keywords_still_work() {
    let source = "let classify = fun x -> x ; let whereabouts = 2 ; let instances = 3 ; \
                  classify whereabouts + instances";
    assert_eq!(run(source), Ok(Value::Int(5)));
}

#[test]
fn top_level_data_requires_a_semicolon() {
    assert!(parse_program("data Foo = A B").is_err());
}

#[test]
fn type_remains_alias_only() {
    assert!(parse_program("type Color = Red | Green ; 0").is_err());
}

#[test]
fn repl_api_persists_top_level_data_declarations() {
    let mut type_env = TypeEnv::new();
    let declaration = parse_program("data Color = Red ;").unwrap();
    typecheck_program_with_env(&declaration, &mut type_env).unwrap();
    let (_, value_env) = eval_program_with_env(&declaration, &Environment::new()).unwrap();

    let use_site = parse_program("Red").unwrap();
    assert!(typecheck_program_with_env(&use_site, &mut type_env).is_ok());
    assert_eq!(
        eval_program(&use_site, &value_env),
        Ok(Value::Variant("Red".to_string(), vec![]))
    );
}

fn sample_class_decl() -> Decl {
    Decl::Class {
        name: "Eq".to_string(),
        param: "a".to_string(),
        superclasses: vec![],
        methods: vec![("eq".to_string(), TypeAnnotation::Var("a".to_string()))],
        doc: None,
    }
}

fn sample_instance_decl() -> Decl {
    Decl::Instance {
        class: "Eq".to_string(),
        context: vec![("Eq".to_string(), "a".to_string())],
        head: TypeAnnotation::App(
            "Option".to_string(),
            vec![TypeAnnotation::Var("a".to_string())],
        ),
        methods: vec![("eq".to_string(), Expr::Int(0))],
        doc: None,
    }
}

fn single_decl_program(decl: Decl) -> Program {
    Program {
        decls: vec![decl],
        body: None,
    }
}

#[test]
fn class_decl_displays_with_braced_methods() {
    assert_eq!(
        single_decl_program(sample_class_decl()).to_string(),
        "(class Eq a where { eq : a })"
    );
    let marker = Decl::Class {
        name: "Marker".to_string(),
        param: "a".to_string(),
        superclasses: vec![],
        methods: vec![],
        doc: None,
    };
    assert_eq!(
        single_decl_program(marker).to_string(),
        "(class Marker a where { })"
    );
}

#[test]
fn class_decl_displays_superclass_context() {
    let ord = Decl::Class {
        name: "Ord".to_string(),
        param: "a".to_string(),
        superclasses: vec![("Eq".to_string(), "a".to_string())],
        methods: vec![("cmp".to_string(), TypeAnnotation::Var("a".to_string()))],
        doc: None,
    };
    assert_eq!(
        single_decl_program(ord).to_string(),
        "(class (Eq a) => Ord a where { cmp : a })"
    );
}

#[test]
fn instance_decl_displays_context_and_parenthesised_head() {
    assert_eq!(
        single_decl_program(sample_instance_decl()).to_string(),
        "(instance (Eq a) => Eq (Option a) where { eq = 0 })"
    );
}

#[test]
fn class_and_instance_decls_render_as_dot() {
    let program = Program {
        decls: vec![sample_class_decl(), sample_instance_decl()],
        body: None,
    };
    let dot = program_to_dot(&program);
    assert!(dot.contains("Class\\nEq a"), "{dot}");
    assert!(dot.contains("Method\\neq : a"), "{dot}");
    assert!(dot.contains("Instance\\nEq (Option a)"), "{dot}");
    assert!(dot.contains("[label=\"decl 1\"]"), "{dot}");
}

#[test]
fn class_declaration_renders_in_typed_text_and_dot() {
    let source = "class Container f where { empty : f Int; size : f Int -> Int };\n0";
    let program = parse_program(source).unwrap();
    let typed = parlang::typecheck_program_typed(&program).unwrap();
    let text = parlang::typed_program_to_text(&typed);
    assert!(
        text.contains("decl 0: Class Container f :: * -> *"),
        "{text}"
    );
    assert!(text.contains("method: empty : forall "), "{text}");
    let dot = parlang::typed_program_to_dot(&typed);
    assert!(dot.contains("Class\\nContainer f"), "{dot}");
    assert!(dot.contains("Method\\nsize : forall "), "{dot}");
}

#[test]
fn repl_api_persists_class_declarations() {
    let mut type_env = TypeEnv::new();
    let declaration = parse_program("class Show a where { show : a -> Int };").unwrap();
    assert_eq!(
        typecheck_program_with_env(&declaration, &mut type_env),
        Ok(parlang::Type::Int)
    );
    let use_site = parse_program("let g = fun x -> show x;").unwrap();
    assert!(typecheck_program_with_env(&use_site, &mut type_env).is_ok());
    assert_eq!(
        typecheck_program_with_env(&declaration, &mut type_env),
        Err(TypeError::DuplicateClass("Show".to_string()))
    );
}

#[test]
fn typechecker_checks_class_and_instance_decls() {
    assert_eq!(
        typecheck_program(&single_decl_program(sample_class_decl())),
        Err(TypeError::DuplicateClass("Eq".to_string()))
    );
    let marker = Decl::Class {
        name: "Marker".to_string(),
        param: "a".to_string(),
        superclasses: vec![],
        methods: vec![("mark".to_string(), TypeAnnotation::Var("a".to_string()))],
        doc: None,
    };
    assert_eq!(
        typecheck_program(&single_decl_program(marker)),
        Ok(parlang::Type::Int)
    );
    assert_eq!(
        typecheck_program(&single_decl_program(sample_instance_decl())),
        Err(TypeError::UnboundVariable("Option".to_string()))
    );
}

#[test]
fn evaluator_rejects_class_and_instance_decls() {
    assert_eq!(
        eval_program(
            &single_decl_program(sample_class_decl()),
            &Environment::new()
        ),
        Err(EvalError::ClassesNotExecutable("class Eq".to_string()))
    );
    assert_eq!(
        eval_program(
            &single_decl_program(sample_instance_decl()),
            &Environment::new()
        ),
        Err(EvalError::ClassesNotExecutable("instance Eq".to_string()))
    );
    assert_eq!(
        EvalError::ClassesNotExecutable("instance Eq".to_string()).to_string(),
        "type classes are not executable yet: instance Eq"
    );
}

const OPT_SAME: &str = "data Opt a = N | S a;\nclass Same a where { same : a -> Bool };\n";

const SAME_INSTANCES: &str = "instance Same Int where { same = fun x -> x == 0 };\n\
     instance Same a => Same (Opt a) where { same = fun x -> (match x with | N -> true | S v -> same v) };\n";

fn check_source(parts: &[&str]) -> Result<parlang::Type, TypeError> {
    let program = parse_program(&parts.concat()).unwrap();
    typecheck_program(&program)
}

#[test]
fn instance_with_context_resolves_through_the_context() {
    let parsed = parse_program(&[OPT_SAME, SAME_INSTANCES, "0"].concat());
    assert!(parsed.is_ok(), "{parsed:?}");
    let ty = check_source(&[OPT_SAME, SAME_INSTANCES, "same (S (S 1))"]);
    assert_eq!(ty, Ok(parlang::Type::Bool));
    let missing = check_source(&[OPT_SAME, SAME_INSTANCES, "same (S true)"]);
    assert_eq!(missing, Err(TypeError::NoInstance("Same Bool".to_string())));
    let eq_opt = "data Opt a = N | S a;\n\
                  instance Eq a => Eq (Opt a) where { eq = fun x -> fun y -> true };\n\
                  eq (S 1) (S 2)";
    assert_eq!(check_source(&[eq_opt]), Ok(parlang::Type::Bool));
}

#[test]
fn instance_body_needs_its_context() {
    let source = "instance Same (Opt a) where { same = fun x -> (match x with | N -> true | S v -> same v) };\n0";
    let result = check_source(&[OPT_SAME, source]);
    assert!(
        matches!(result, Err(TypeError::NoInstance(_))),
        "{result:?}"
    );
    let wrong = check_source(&[
        OPT_SAME,
        "instance Same Int where { same = fun x -> 1 };\n0",
    ]);
    assert!(
        matches!(wrong, Err(TypeError::UnificationError(..))),
        "{wrong:?}"
    );
}

#[test]
fn constructor_class_instance_is_kind_checked() {
    let container = "data Opt a = N | S a;\nclass Container f where { size : f Int -> Int };\n";
    let opt = "instance Container Opt where { size = fun x -> (match x with | N -> 0 | S v -> v) };\nsize (S 3)";
    assert_eq!(check_source(&[container, opt]), Ok(parlang::Type::Int));
    let int = check_source(&[
        container,
        "instance Container Int where { size = fun x -> 0 };\n0",
    ]);
    assert!(matches!(int, Err(TypeError::KindMismatch(..))), "{int:?}");
    let bare = check_source(&[
        OPT_SAME,
        "instance Same Opt where { same = fun x -> true };\n0",
    ]);
    assert!(
        matches!(bare, Err(TypeError::TypeConstructorArityMismatch(..))),
        "{bare:?}"
    );
}

#[test]
fn instance_declaration_errors() {
    let method_error = |instance: &str, method: &str| (instance.to_string(), method.to_string());
    let (inst, meth) = method_error("Same Int", "same");
    let missing = TypeError::MissingInstanceMethod {
        instance: inst,
        method: meth,
    };
    let (inst, meth) = method_error("Same Int", "other");
    let unknown = TypeError::UnknownInstanceMethod {
        instance: inst,
        method: meth,
    };
    let (inst, meth) = method_error("Same Int", "same");
    let twice = TypeError::DuplicateInstanceMethod {
        instance: inst,
        method: meth,
    };
    let (inst, meth) = method_error("Ident (Opt a)", "ident");
    let specific = TypeError::InstanceMethodTooSpecific {
        instance: inst,
        method: meth,
    };
    let cases = [
        (
            "instance Nope Int where { };\n0",
            TypeError::UnknownClass("Nope".to_string()),
        ),
        (
            "instance Eq Int where { eq = fun x -> fun y -> x == y };\n0",
            TypeError::DuplicateInstance("Eq Int".to_string()),
        ),
        (
            "instance Same (Opt a) where { same = fun x -> true };\n\
             instance Same (Opt b) where { same = fun x -> false };\n0",
            TypeError::DuplicateInstance("Same (Opt b)".to_string()),
        ),
        (
            "instance Same a where { same = fun x -> true };\n0",
            TypeError::InvalidInstanceHead("Same a".to_string()),
        ),
        (
            "data Pr a b = MkPr a b;\ninstance Same (Pr a a) where { same = fun x -> true };\n0",
            TypeError::InvalidInstanceHead("Same (Pr a a)".to_string()),
        ),
        (
            "instance Same (Opt Int) where { same = fun x -> true };\n0",
            TypeError::InvalidInstanceHead("Same (Opt Int)".to_string()),
        ),
        (
            "instance Same (a -> a) where { same = fun x -> true };\n0",
            TypeError::InvalidInstanceHead("Same (a -> a)".to_string()),
        ),
        (
            "type Num = Int;\ninstance Same Num where { same = fun x -> true };\n0",
            TypeError::TypeSynonymInstance("Same Num".to_string()),
        ),
        (
            "instance Same Foo where { same = fun x -> true };\n0",
            TypeError::UnboundVariable("Foo".to_string()),
        ),
        (
            "instance Same b => Same (Opt a) where { same = fun x -> true };\n0",
            TypeError::InvalidInstanceContext("Same b".to_string()),
        ),
        (
            "instance Nope a => Same (Opt a) where { same = fun x -> true };\n0",
            TypeError::UnknownClass("Nope".to_string()),
        ),
        ("instance Same Int where { };\n0", missing),
        (
            "instance Same Int where { same = fun x -> true; other = 1 };\n0",
            unknown,
        ),
        (
            "instance Same Int where { same = fun x -> true; same = fun x -> false };\n0",
            twice,
        ),
        (
            "class Ident a where { ident : a -> a };\n\
             instance Ident (Opt a) where { ident = fun x -> S 1 };\n0",
            specific,
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(check_source(&[OPT_SAME, source]), Err(expected), "{source}");
    }
}

#[test]
fn instance_programs_render_and_run_through_dictionaries() {
    let source = [OPT_SAME, SAME_INSTANCES, "same 1"].concat();
    let program = parse_program(&source).unwrap();
    let typed = parlang::typecheck_program_typed(&program).unwrap();
    let text = parlang::typed_program_to_text(&typed);
    assert!(text.contains("decl 2: Instance Same Int"), "{text}");
    assert!(text.contains("context: Same t"), "{text}");
    assert!(text.contains("method same: "), "{text}");
    let dot = parlang::typed_program_to_dot(&typed);
    assert!(dot.contains("Instance\\nSame Int"), "{dot}");
    assert!(parlang::typed_ast::is_class_program(&typed));
    assert_eq!(parlang::typed_ast::class_load_conflict(&typed), None);
    // Raw evaluation of an untranslated program still rejects the instance;
    // the front ends run its dictionary translation instead (ADR 0013).
    let eq_only = "data Opt a = N | S a;\n\
                   instance Eq a => Eq (Opt a) where { eq = fun x -> fun y -> true };\n0";
    assert_eq!(
        run(eq_only),
        Err("type classes are not executable yet: instance Eq".to_string())
    );
}
