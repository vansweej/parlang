//! Arc C "REGULAR PROGRAMS RUN" milestone integration tests.
//!
//! Drives the Core VM [`eval`] entry point on builder-constructed terms.
//! Runs inside `parlang-core`; the driver is intentionally not wired to the
//! Core VM in Arc C.

use parlang_core::builder::{app, apps, bool_, con, int, lam, let_, letrec, unit, var};
use parlang_core::{eval, BaseType, Environment, EvalError, Value};

fn env() -> Environment {
    Environment::new()
}

// ---------------------------------------------------------------------------
// Recursion / milestone
// ---------------------------------------------------------------------------

/// Primary milestone assertion: factorial(5) = 120.
#[test]
fn factorial_of_five() -> Result<(), Box<dyn std::error::Error>> {
    // letrec fac = \n:int. if (eq n 0) 1 (* n (fac (- n 1))) in fac 5
    let body = con(
        "if",
        vec![
            con("eq", vec![var("n")?, int(0)])?,
            int(1),
            con(
                "*",
                vec![
                    var("n")?,
                    app(var("fac")?, con("-", vec![var("n")?, int(1)])?),
                ],
            )?,
        ],
    )?;
    let fac_lam = lam("n", BaseType::Int, body)?;
    let t = letrec("fac", fac_lam, app(var("fac")?, int(5)))?;
    assert_eq!(eval(&t, &env())?, Value::Int(120));
    Ok(())
}

/// Recursion countdown: counts n down to 0, returning 0.
#[test]
fn recursion_countdown() -> Result<(), Box<dyn std::error::Error>> {
    // letrec f = \n:int. if (eq n 0) 0 (f (- n 1)) in f 5
    let body = con(
        "if",
        vec![
            con("eq", vec![var("n")?, int(0)])?,
            int(0),
            app(var("f")?, con("-", vec![var("n")?, int(1)])?),
        ],
    )?;
    let f_lam = lam("n", BaseType::Int, body)?;
    let t = letrec("f", f_lam, app(var("f")?, int(5)))?;
    assert_eq!(eval(&t, &env())?, Value::Int(0));
    Ok(())
}

// ---------------------------------------------------------------------------
// Conditionals
// ---------------------------------------------------------------------------

#[test]
fn conditional_selects_true_branch() -> Result<(), Box<dyn std::error::Error>> {
    let t = con(
        "if",
        vec![con("<", vec![int(1), int(2)])?, int(10), int(20)],
    )?;
    assert_eq!(eval(&t, &env())?, Value::Int(10));
    Ok(())
}

#[test]
fn conditional_selects_false_branch() -> Result<(), Box<dyn std::error::Error>> {
    let t = con(
        "if",
        vec![con("eq", vec![int(1), int(2)])?, int(10), int(20)],
    )?;
    assert_eq!(eval(&t, &env())?, Value::Int(20));
    Ok(())
}

/// The untaken branch references an unbound variable — eval must not touch it.
#[test]
fn conditional_is_lazy_in_untaken_branch() -> Result<(), Box<dyn std::error::Error>> {
    let t = con("if", vec![bool_(true), int(42), var("nope")?])?;
    assert_eq!(eval(&t, &env())?, Value::Int(42));
    Ok(())
}

// ---------------------------------------------------------------------------
// Arithmetic primitives
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_primitives() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        eval(&con("+", vec![int(2), int(3)])?, &env())?,
        Value::Int(5)
    );
    assert_eq!(
        eval(&con("-", vec![int(10), int(4)])?, &env())?,
        Value::Int(6)
    );
    assert_eq!(
        eval(&con("*", vec![int(3), int(7)])?, &env())?,
        Value::Int(21)
    );
    assert_eq!(
        eval(&con("/", vec![int(10), int(2)])?, &env())?,
        Value::Int(5)
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Higher-order application
// ---------------------------------------------------------------------------

/// let k = \x:int. \y:int. x in (k 7) 9  =>  7
#[test]
fn higher_order_application() -> Result<(), Box<dyn std::error::Error>> {
    let inner = lam("y", BaseType::Int, var("x")?)?;
    let k = lam("x", BaseType::Int, inner)?;
    let t = let_("k", k, apps(var("k")?, vec![int(7), int(9)]))?;
    assert_eq!(eval(&t, &env())?, Value::Int(7));
    Ok(())
}

// ---------------------------------------------------------------------------
// Church-encoded list fold — honest "list ops evaluate correctly" milestone
// ---------------------------------------------------------------------------

/// Demonstrates real list COMPUTATION via a Church-encoded fold.
///
/// No list eliminator (`case`/`match`) is needed — the fold is expressed
/// purely with lambdas and the reserved `+` primitive.
///
/// ```text
/// nil  = \c. \n. n
/// cons = \h. \t. \c. \n.  c h (t c n)
/// sum  = \l.  l (\h. \acc. + h acc) 0
///
/// sum (cons 1 (cons 2 (cons 3 nil)))  =>  Int(6)
/// ```
#[test]
fn church_encoded_list_sum() -> Result<(), Box<dyn std::error::Error>> {
    // nil = \c. \n. n  (BaseType::Int used as placeholder; evaluator ignores it)
    let nil = lam("c", BaseType::Int, lam("n", BaseType::Int, var("n")?)?)?;

    // cons = \h. \t. \c. \n.  c h (t c n)
    let t_c_n = apps(var("t")?, vec![var("c")?, var("n")?]);
    let c_h_tcn = apps(var("c")?, vec![var("h")?, t_c_n]);
    let cons = lam(
        "h",
        BaseType::Int,
        lam(
            "t",
            BaseType::Int,
            lam("c", BaseType::Int, lam("n", BaseType::Int, c_h_tcn)?)?,
        )?,
    )?;

    // sum = \l. l (\h. \acc. + h acc) 0
    let add_h_acc = con("+", vec![var("h")?, var("acc")?])?;
    let combiner = lam("h", BaseType::Int, lam("acc", BaseType::Int, add_h_acc)?)?;
    let sum = lam("l", BaseType::Int, apps(var("l")?, vec![combiner, int(0)]))?;

    // list: cons 1 (cons 2 (cons 3 nil))
    let list = apps(
        cons.clone(),
        vec![
            int(1),
            apps(cons.clone(), vec![int(2), apps(cons, vec![int(3), nil])]),
        ],
    );

    let t = app(sum, list);
    assert_eq!(eval(&t, &env())?, Value::Int(6));
    Ok(())
}

// ---------------------------------------------------------------------------
// Value-level constructor evaluation
// ---------------------------------------------------------------------------

/// Validates STRICT CONSTRUCTOR EVALUATION (construction).
///
/// Complementary to the Church fold above: proves `Con` args are evaluated
/// strictly left-to-right and nested `Value::Con`s are produced correctly.
#[test]
fn list_construction_value_level() -> Result<(), Box<dyn std::error::Error>> {
    // let xs = Cons(1, Cons(2, Nil)) in xs
    let nil_term = con("Nil", vec![])?;
    let inner = con("Cons", vec![int(2), nil_term])?;
    let outer = con("Cons", vec![int(1), inner])?;
    let t = let_("xs", outer, var("xs")?)?;
    assert_eq!(
        eval(&t, &env())?,
        Value::Con(
            "Cons".to_string(),
            vec![
                Value::Int(1),
                Value::Con(
                    "Cons".to_string(),
                    vec![Value::Int(2), Value::Con("Nil".to_string(), vec![])]
                )
            ]
        )
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn division_by_zero_errors() -> Result<(), Box<dyn std::error::Error>> {
    let t = con("/", vec![int(1), int(0)])?;
    assert_eq!(eval(&t, &env()), Err(EvalError::DivisionByZero));
    Ok(())
}

#[test]
fn arithmetic_overflow_errors() -> Result<(), Box<dyn std::error::Error>> {
    let t = con("+", vec![int(i64::MAX), int(1)])?;
    assert_eq!(eval(&t, &env()), Err(EvalError::ArithmeticOverflow));
    Ok(())
}

#[test]
fn unbound_variable_errors() -> Result<(), Box<dyn std::error::Error>> {
    let t = var("x")?;
    assert!(matches!(eval(&t, &env()), Err(EvalError::UnboundVar(_))));
    Ok(())
}

#[test]
fn not_a_function_errors() {
    let t = app(int(1), int(2));
    assert_eq!(eval(&t, &env()), Err(EvalError::NotAFunction));
}

// ---------------------------------------------------------------------------
// Unused-import guard
// ---------------------------------------------------------------------------

#[test]
fn unit_literal() {
    assert_eq!(eval(&unit(), &env()), Ok(Value::Unit));
}

#[test]
fn bool_literal() {
    assert_eq!(eval(&bool_(false), &env()), Ok(Value::Bool(false)));
}
