/// Tests for fixed-size array type functionality
use parlang::{parse, eval, Environment, Value};

fn parse_and_eval(input: &str) -> Result<Value, String> {
    let expr = parse(input)?;
    eval(&expr, &Environment::new()).map_err(|e| e.to_string())
}

#[test]
fn test_empty_array() {
    assert_eq!(
        parse_and_eval("[||]"),
        Ok(Value::Array(0, vec![]))
    );
}

#[test]
fn test_array_of_integers() {
    assert_eq!(
        parse_and_eval("[|1, 2, 3|]"),
        Ok(Value::Array(3, vec![Value::Int(1), Value::Int(2), Value::Int(3)]))
    );
}

#[test]
fn test_array_of_booleans() {
    assert_eq!(
        parse_and_eval("[|true, false, true|]"),
        Ok(Value::Array(3, vec![Value::Bool(true), Value::Bool(false), Value::Bool(true)]))
    );
}

#[test]
fn test_array_single_element() {
    assert_eq!(
        parse_and_eval("[|42|]"),
        Ok(Value::Array(1, vec![Value::Int(42)]))
    );
}

#[test]
fn test_array_with_expressions() {
    assert_eq!(
        parse_and_eval("[|1 + 1, 2 * 2, 3|]"),
        Ok(Value::Array(3, vec![Value::Int(2), Value::Int(4), Value::Int(3)]))
    );
}

#[test]
fn test_array_indexing_first() {
    assert_eq!(
        parse_and_eval("[|10, 20, 30|][0]"),
        Ok(Value::Int(10))
    );
}

#[test]
fn test_array_indexing_middle() {
    assert_eq!(
        parse_and_eval("[|10, 20, 30|][1]"),
        Ok(Value::Int(20))
    );
}

#[test]
fn test_array_indexing_last() {
    assert_eq!(
        parse_and_eval("[|10, 20, 30|][2]"),
        Ok(Value::Int(30))
    );
}

#[test]
fn test_array_indexing_with_variable() {
    assert_eq!(
        parse_and_eval("let arr = [|10, 20, 30|] in arr[1]"),
        Ok(Value::Int(20))
    );
}

#[test]
fn test_array_indexing_with_expression() {
    assert_eq!(
        parse_and_eval("[|100, 200, 300|][1 + 1]"),
        Ok(Value::Int(300))
    );
}

#[test]
fn test_array_out_of_bounds() {
    let result = parse_and_eval("[|1, 2, 3|][5]");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of bounds"));
}

#[test]
fn test_array_negative_index() {
    let result = parse_and_eval("[|1, 2, 3|][-1]");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("negative"));
}

#[test]
fn test_array_indexing_non_int_index() {
    let result = parse_and_eval("[|1, 2, 3|][true]");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be an integer"));
}

#[test]
fn test_array_indexing_non_array() {
    let result = parse_and_eval("42[0]");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires an array"));
}

#[test]
fn test_nested_arrays() {
    // Nested arrays of different sizes
    assert_eq!(
        parse_and_eval("let outer = [|[|1, 2|], [|3, 4|]|] in outer[0]"),
        Ok(Value::Array(2, vec![Value::Int(1), Value::Int(2)]))
    );
}

#[test]
fn test_nested_array_indexing() {
    assert_eq!(
        parse_and_eval("let outer = [|[|1, 2|], [|3, 4|]|] in outer[1][0]"),
        Ok(Value::Int(3))
    );
}

#[test]
fn test_array_with_characters() {
    assert_eq!(
        parse_and_eval("[|'a', 'b', 'c'|]"),
        Ok(Value::Array(3, vec![Value::Char('a'), Value::Char('b'), Value::Char('c')]))
    );
}

#[test]
fn test_array_in_function() {
    assert_eq!(
        parse_and_eval("let getFirst = fun arr -> arr[0] in getFirst [|42, 43, 44|]"),
        Ok(Value::Int(42))
    );
}

#[test]
fn test_array_size_preservation() {
    // Test that array size is tracked correctly
    let result = parse_and_eval("[|1, 2, 3, 4, 5|]");
    assert!(result.is_ok());
    if let Ok(Value::Array(size, values)) = result {
        assert_eq!(size, 5);
        assert_eq!(values.len(), 5);
    } else {
        panic!("Expected array value");
    }
}

#[test]
fn test_array_with_let_bindings() {
    assert_eq!(
        parse_and_eval("let x = 10; let y = 20; let z = 30; [|x, y, z|][1]"),
        Ok(Value::Int(20))
    );
}

#[test]
fn test_multiple_array_operations() {
    assert_eq!(
        parse_and_eval("let a1 = [|1, 2|] in let a2 = [|3, 4|] in a1[0] + a2[1]"),
        Ok(Value::Int(5))
    );
}

#[test]
fn test_array_display() {
    let arr = Value::Array(3, vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    let display = format!("{}", arr);
    assert!(display.contains("[|"));
    assert!(display.contains("|]"));
    assert!(display.contains("size: 3"));
}
