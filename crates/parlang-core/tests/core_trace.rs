use parlang_core::builder::{
    alt, app, base_ty, bool_, case_, con, fun_ty, int, lam, let_, letrec, var,
};
use parlang_core::machine::Control;
use parlang_core::Term;
use parlang_core::{reduction_terms, trace, BaseType, Environment, StepKind};

#[test]
fn application_trace_has_raw_and_readback_views() -> Result<(), Box<dyn std::error::Error>> {
    let term = app(
        lam("x", BaseType::Int, con("+", vec![var("x")?, int(1)])?)?,
        int(2),
    );
    let steps = trace(&term, &Environment::new(), None)?;
    assert_eq!(
        reduction_terms(&steps),
        vec![
            "((\\x: Int. +(x, 1)) 2)".to_string(),
            "+(2, 1)".to_string(),
            "3".to_string(),
        ]
    );
    assert!(matches!(steps.first(), Some(step) if step.state.to_string().contains("eval")));
    Ok(())
}

#[test]
fn let_trace_readback_contains_binding_and_result() -> Result<(), Box<dyn std::error::Error>> {
    let term = let_("x", int(1), con("+", vec![var("x")?, int(2)])?)?;
    assert_eq!(
        reduction_terms(&trace(&term, &Environment::new(), None)?),
        vec![
            "(let x = 1 in +(x, 2))".to_string(),
            "+(1, 2)".to_string(),
            "3".to_string(),
        ]
    );
    Ok(())
}

#[test]
fn if_trace_does_not_evaluate_untaken_branch() -> Result<(), Box<dyn std::error::Error>> {
    let term = con("if", vec![bool_(true), int(42), var("nope")?])?;
    let steps = trace(&term, &Environment::new(), None)?;
    let terms = reduction_terms(&steps);
    assert_eq!(terms.last().map(String::as_str), Some("42"));
    assert!(!steps.iter().any(|step| {
        matches!(
            &step.state.control,
            Control::Eval(Term::Var(name)) if name == "nope"
        )
    }));
    Ok(())
}

#[test]
fn factorial_trace_has_expected_reduction_counts() -> Result<(), Box<dyn std::error::Error>> {
    let is_zero = con("eq", vec![var("n")?, int(0)])?;
    let decrement = con("-", vec![var("n")?, int(1)])?;
    let recursive_call = app(var("fac")?, decrement);
    let product = con("*", vec![var("n")?, recursive_call])?;
    let body = con("if", vec![is_zero, int(1), product])?;
    let function = lam("n", BaseType::Int, body)?;
    let term = letrec(
        "fac",
        fun_ty(base_ty(BaseType::Int), base_ty(BaseType::Int)),
        function,
        app(var("fac")?, int(3)),
    )?;
    let steps = trace(&term, &Environment::new(), None)?;
    let terms = reduction_terms(&steps);
    assert_eq!(
        terms.first().map(String::as_str),
        Some("(letrec fac: Int -> Int = (\\n: Int. if(eq(n, 0), 1, *(n, (fac -(n, 1))))) in (fac 3))")
    );
    assert_eq!(terms.last().map(String::as_str), Some("6"));
    assert_eq!(
        steps
            .iter()
            .filter(|step| step.kind == StepKind::Unfold)
            .count(),
        4
    );
    assert_eq!(
        steps
            .iter()
            .filter(|step| step.kind == StepKind::Branch)
            .count(),
        4
    );
    assert_eq!(
        steps
            .iter()
            .filter(|step| step.kind == StepKind::Delta)
            .count(),
        10
    );
    Ok(())
}

#[test]
fn case_trace_readback_selects_constructor_alternative() -> Result<(), Box<dyn std::error::Error>> {
    let term = case_(
        con("Cons", vec![int(1), con("Nil", vec![])?])?,
        vec![
            alt("Cons", &["h", "t"], var("h")?)?,
            alt("Nil", &[], int(0))?,
        ],
    );
    let terms = reduction_terms(&trace(&term, &Environment::new(), None)?);
    assert_eq!(terms.last().map(String::as_str), Some("1"));
    assert!(terms
        .iter()
        .any(|term| term == "(case Cons(1, Nil) of Cons(h, t) -> h | Nil -> 0)"));
    Ok(())
}
