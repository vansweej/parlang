//! Guard: every program that reaches a binary eval path must typecheck.
use parlang::{parse, typecheck};

fn assert_well_typed(label: &str, src: &str) {
    let expr = parse(src).expect("must parse");
    let result = typecheck(&expr);
    assert!(result.is_ok(), "{label} must typecheck: {:?}", result.err());
}

#[test]
fn driver_golden_fixtures_typecheck() {
    assert_well_typed("arithmetic.par", "1 + 2 * 3");
    assert_well_typed("let_binding.par", "let x = 40 in x + 2");
    assert_well_typed("identity.par", "(fun x -> x) 100");
}

#[test]
fn cli_inline_programs_typecheck() {
    assert_well_typed("cli_complex_program", "1 + 2 + 3");
    assert_well_typed("cli_eval_error", "1 / 0");
    assert_well_typed(
        "cli_factorial",
        r"
        let factorial = rec f -> fun n ->
            if n == 0
            then 1
            else n * f (n - 1)
        in factorial 5
        ",
    );
    assert_well_typed(
        "cli_multiline_program",
        r"
        let x = 10 in
        let y = 20 in
        let z = 30 in
        x + y + z
        ",
    );
}
