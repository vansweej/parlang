use std::error::Error;

use parlang_core::builder::{app, bool_, var};
use parlang_core::{eval, prelude, Value};

#[test]
fn not_true_is_false() -> Result<(), Box<dyn Error>> {
    let env = prelude()?;
    let t = app(var("not")?, bool_(true));
    assert_eq!(eval(&t, &env)?, Value::Bool(false));
    Ok(())
}

#[test]
fn not_false_is_true() -> Result<(), Box<dyn Error>> {
    let env = prelude()?;
    let t = app(var("not")?, bool_(false));
    assert_eq!(eval(&t, &env)?, Value::Bool(true));
    Ok(())
}

#[test]
fn isempty_and_nonempty_are_absent() -> Result<(), Box<dyn Error>> {
    let env = prelude()?;
    assert!(env.lookup("isEmpty").is_none());
    assert!(env.lookup("nonEmpty").is_none());
    Ok(())
}
