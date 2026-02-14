/// Integration tests for record types
use parlang::{eval, parse, typecheck, Environment, EvalError, Type, TypeError};

#[test]
fn test_simple_record_construction() {
    let source = "{ name: 42, age: 30 }";
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    // Check the result contains both fields
    let result_str = format!("{}", result);
    assert!(result_str.contains("name: 42"));
    assert!(result_str.contains("age: 30"));
}

#[test]
fn test_empty_record() {
    let source = "{}";
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "{}");
}

#[test]
fn test_field_access() {
    let source = r#"
        let person = { name: 42, age: 30 }
        in person.age
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "30");
}

#[test]
fn test_field_access_multiple_fields() {
    let source = r#"
        let person = { name: 42, age: 30, city: 100 }
        in person.name
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "42");
}

#[test]
fn test_nested_records() {
    let source = r#"
        let address = { street: 123, city: 456 }
        in let person = { name: 789, address: address }
        in person.address.city
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "456");
}

#[test]
fn test_record_with_boolean_fields() {
    let source = "{ active: true, verified: false }";
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    let result_str = format!("{}", result);
    assert!(result_str.contains("active: true"));
    assert!(result_str.contains("verified: false"));
}

#[test]
fn test_record_pattern_matching_full() {
    let source = r#"
        let person = { name: 42, age: 30 }
        in match person with
        | { name: n, age: a } -> n + a
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "72");
}

#[test]
fn test_record_pattern_matching_partial() {
    let source = r#"
        let person = { name: 42, age: 30, city: 100 }
        in match person with
        | { name: n } -> n
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "42");
}

#[test]
fn test_record_pattern_matching_with_wildcard() {
    let source = r#"
        let person = { name: 42, age: 30 }
        in match person with
        | { name: _, age: a } -> a
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "30");
}

#[test]
fn test_record_in_function_parameter() {
    let source = r#"
        let getAge = fun p -> p.age
        in let person = { name: 42, age: 25 }
        in getAge person
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "25");
}

#[test]
fn test_record_in_function_return() {
    let source = r#"
        let makePerson = fun n -> fun a -> { name: n, age: a }
        in makePerson 42 30
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    let result_str = format!("{}", result);
    assert!(result_str.contains("name: 42"));
    assert!(result_str.contains("age: 30"));
}

#[test]
fn test_record_field_access_in_expression() {
    let source = r#"
        let person = { x: 10, y: 20 }
        in person.x + person.y
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "30");
}

#[test]
fn test_record_update_functional_style() {
    // Records are immutable, so "update" means creating a new record
    let source = r#"
        let person = { name: 42, age: 30 }
        in let updatedPerson = { name: person.name, age: 31 }
        in updatedPerson.age
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "31");
}

#[test]
fn test_field_not_found_error() {
    let source = r#"
        let person = { name: 42, age: 30 }
        in person.salary
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new());
    
    assert!(result.is_err());
    match result {
        Err(EvalError::FieldNotFound(field, _)) => {
            assert_eq!(field, "salary");
        }
        _ => panic!("Expected FieldNotFound error"),
    }
}

#[test]
fn test_record_expected_error() {
    let source = "let x = 42 in x.field";
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new());
    
    assert!(result.is_err());
    assert!(matches!(result, Err(EvalError::RecordExpected(_))));
}

#[test]
fn test_record_type_inference_simple() {
    let source = "{ name: 42, age: 30 }";
    
    let expr = parse(source).expect("Parse error");
    let ty = typecheck(&expr).expect("Type error");
    
    let type_str = format!("{}", ty);
    assert!(type_str.contains("name: Int"));
    assert!(type_str.contains("age: Int"));
}

#[test]
fn test_record_type_inference_field_access() {
    let source = r#"
        let person = { name: 42, age: 30 }
        in person.age
    "#;
    
    let expr = parse(source).expect("Parse error");
    let ty = typecheck(&expr).expect("Type error");
    
    assert_eq!(ty, Type::Int);
}

#[test]
fn test_record_type_inference_function() {
    let source = "fun p -> p.age";
    
    let expr = parse(source).expect("Parse error");
    let ty = typecheck(&expr).expect("Type error");
    
    // Should be a function from a record with at least 'age' field to the type of age
    let type_str = format!("{}", ty);
    assert!(type_str.contains("->"));
    assert!(type_str.contains("age:"));
}

#[test]
fn test_record_type_inference_nested() {
    let source = r#"
        let person = { address: { city: 100 } }
        in person.address
    "#;
    
    let expr = parse(source).expect("Parse error");
    let ty = typecheck(&expr).expect("Type error");
    
    let type_str = format!("{}", ty);
    assert!(type_str.contains("city: Int"));
}

#[test]
fn test_empty_record_type() {
    let source = "{}";
    
    let expr = parse(source).expect("Parse error");
    let ty = typecheck(&expr).expect("Type error");
    
    assert_eq!(format!("{}", ty), "{}");
}

#[test]
fn test_record_with_mixed_types() {
    let source = "{ num: 42, flag: true }";
    
    let expr = parse(source).expect("Parse error");
    let ty = typecheck(&expr).expect("Type error");
    
    let type_str = format!("{}", ty);
    assert!(type_str.contains("flag: Bool"));
    assert!(type_str.contains("num: Int"));
}

#[test]
fn test_deeply_nested_field_access() {
    let source = r#"
        let data = { a: { b: { c: 42 } } }
        in data.a.b.c
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "42");
}

#[test]
fn test_record_pattern_not_matching() {
    let source = r#"
        let record = { name: 42 }
        in match record with
        | { age: a } -> a
        | _ -> 0
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "0");
}

#[test]
fn test_record_comparison_in_match() {
    let source = r#"
        let person = { status: 1, name: 42 }
        in match person with
        | { status: 1, name: n } -> n
        | _ -> 0
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "42");
}

#[test]
fn test_record_in_nested_let() {
    let source = r#"
        let x = 10
        in let record = { a: x, b: 20 }
        in record.a + record.b
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "30");
}

#[test]
fn test_record_with_function_field() {
    let source = r#"
        let obj = { value: 42, getValue: fun x -> x }
        in (obj.getValue) obj.value
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "42");
}

#[test]
fn test_higher_order_function_with_records() {
    let source = r#"
        let mapField = fun f -> fun record -> { value: f record.value }
        in let inc = fun x -> x + 1
        in let data = { value: 41 }
        in (mapField inc data).value
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = eval(&expr, &Environment::new()).expect("Eval error");
    
    assert_eq!(format!("{}", result), "42");
}

#[test]
fn test_record_type_error_field_not_found() {
    let source = r#"
        let makePerson = fun x -> { name: x }
        in let getPerson = fun p -> p.age
        in getPerson (makePerson 42)
    "#;
    
    let expr = parse(source).expect("Parse error");
    let result = typecheck(&expr);
    
    // This should fail type checking because 'age' field doesn't exist
    // The error may be either FieldNotFound or RecordFieldMismatch depending on
    // when the type checker discovers the incompatibility
    assert!(result.is_err());
    match result {
        Err(TypeError::FieldNotFound(field, _)) => {
            assert_eq!(field, "age");
        }
        Err(TypeError::RecordFieldMismatch) => {
            // Also acceptable - unification fails due to field mismatch
        }
        other => panic!("Expected FieldNotFound or RecordFieldMismatch type error, got {:?}", other),
    }
}
