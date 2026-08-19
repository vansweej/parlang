use parlang::{eval_program, parse_program, Environment, Value};

#[test]
fn loaded_top_level_constructor_is_visible() {
    // This pins Environment::merge propagating constructors. Construction, not
    // matching, fails when the constructor table is dropped during a load.
    let program = parse_program(r#"load "tests/lib_toplevel_data.par" in Stop"#).unwrap();

    assert_eq!(
        eval_program(&program, &Environment::new()),
        Ok(Value::Variant("Stop".to_string(), vec![]))
    );
}
