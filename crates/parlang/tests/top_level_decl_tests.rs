use parlang::{
    eval_program, eval_program_with_env, parse_program, typecheck_program,
    typecheck_program_with_env, Environment, TypeEnv, TypeError, Value,
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
fn payload_forward_reference_guard_is_vacuous_by_design() {
    // This does not prove payload forward-reference support: payload types are
    // resolved lazily rather than while data declarations are registered.
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
