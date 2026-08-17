use std::error::Error;

use parlang_core::builder::{app, con, str_, var};
use parlang_core::{eval, prelude, Value};

#[test]
fn isempty_on_empty_string_is_true() -> Result<(), Box<dyn Error>> {
    let env = prelude()?;
    let t = app(var("isEmpty")?, str_(""));
    assert_eq!(eval(&t, &env)?, Value::Bool(true));
    Ok(())
}

#[test]
fn isempty_on_nonempty_string_is_false() -> Result<(), Box<dyn Error>> {
    let env = prelude()?;
    let t = app(var("isEmpty")?, str_("x"));
    assert_eq!(eval(&t, &env)?, Value::Bool(false));
    Ok(())
}

#[test]
fn nonempty_on_nonempty_string_is_true() -> Result<(), Box<dyn Error>> {
    let env = prelude()?;
    let t = app(var("nonEmpty")?, str_("x"));
    assert_eq!(eval(&t, &env)?, Value::Bool(true));
    Ok(())
}

#[test]
fn nonempty_on_empty_string_is_false() -> Result<(), Box<dyn Error>> {
    let env = prelude()?;
    let t = app(var("nonEmpty")?, str_(""));
    assert_eq!(eval(&t, &env)?, Value::Bool(false));
    Ok(())
}

#[test]
fn strcat_then_isempty_is_false() -> Result<(), Box<dyn Error>> {
    let env = prelude()?;
    let t = app(var("isEmpty")?, con("strcat", vec![str_("a"), str_("b")])?);
    assert_eq!(eval(&t, &env)?, Value::Bool(false));
    Ok(())
}
