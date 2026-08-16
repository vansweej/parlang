//! Integration test: build hand-written Core via the public builder API and
//! exercise both dump modes (text + DOT). No elaboration or CLI is involved —
//! Arc B Core is dumped only by constructing values directly.

use parlang_core::builder::{app, apps, bool_, con, int, lam, let_, letrec, unit, var};
use parlang_core::{core_to_dot, BaseType, BuildError};

#[test]
fn factorial_shaped_term_text_dump() -> Result<(), BuildError> {
    // letrec fac = \n: int. (fac n) in (fac 5)
    let body = app(var("fac")?, int(5));
    let inner = app(var("fac")?, var("n")?);
    let fac = lam("n", BaseType::Int, inner)?;
    let term = letrec("fac", fac, body)?;

    assert_eq!(
        term.to_text(),
        "(letrec fac = (\\n: int. (fac n)) in (fac 5))"
    );
    Ok(())
}

#[test]
fn factorial_shaped_term_dot_dump() -> Result<(), BuildError> {
    let term = letrec(
        "fac",
        lam("n", BaseType::Int, app(var("fac")?, var("n")?))?,
        app(var("fac")?, int(5)),
    )?;

    let dot = core_to_dot(&term);
    assert!(dot.contains("digraph Core {"));
    assert!(!dot.contains("digraph AST"));
    assert!(dot.contains("node0 [label=\"LetRec fac\"];"));
    assert!(dot.contains("[label=\"Lam n: int\"];"));
    Ok(())
}

#[test]
fn constructor_and_apps_text_dump() -> Result<(), BuildError> {
    // let p = Pair(1, true) in ((g p) unit)  -- via apps
    let g = apps(var("g")?, vec![var("p")?, unit()]);
    let term = let_("p", con("Pair", vec![int(1), bool_(true)])?, g)?;

    assert_eq!(term.to_text(), "(let p = Pair(1, true) in ((g p) ()))");
    Ok(())
}
