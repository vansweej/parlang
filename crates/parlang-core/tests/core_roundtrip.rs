//! Integration test: build hand-written Core via the public builder API and
//! exercise both dump modes (text + DOT). No elaboration or CLI is involved —
//! Arc B Core is dumped only by constructing values directly.

use parlang_core::builder::{
    app, apps, base_ty, bool_, con, fun_ty, int, lam, let_, letrec, unit, var,
};
use parlang_core::{core_to_dot, BaseType, BuildError};

#[test]
fn factorial_shaped_term_text_dump() -> Result<(), BuildError> {
    // letrec fac: Int -> Int = \n: Int. (fac n) in (fac 5)
    let body = app(var("fac")?, int(5));
    let inner = app(var("fac")?, var("n")?);
    let fac = lam("n", BaseType::Int, inner)?;
    let term = letrec(
        "fac",
        fun_ty(base_ty(BaseType::Int), base_ty(BaseType::Int)),
        fac,
        body,
    )?;

    assert_eq!(
        term.to_text(),
        "(letrec fac: Int -> Int = (\\n: Int. (fac n)) in (fac 5))"
    );
    Ok(())
}

#[test]
fn factorial_shaped_term_dot_dump() -> Result<(), BuildError> {
    let term = letrec(
        "fac",
        fun_ty(base_ty(BaseType::Int), base_ty(BaseType::Int)),
        lam("n", BaseType::Int, app(var("fac")?, var("n")?))?,
        app(var("fac")?, int(5)),
    )?;

    let dot = core_to_dot(&term);
    assert!(dot.contains("digraph Core {"));
    assert!(!dot.contains("digraph AST"));
    assert!(dot.contains("node0 [label=\"LetRec fac: Int -> Int\"];"));
    assert!(dot.contains("[label=\"Lam n: Int\"];"));
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
