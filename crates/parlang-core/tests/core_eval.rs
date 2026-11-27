//! Arc C "REGULAR PROGRAMS RUN" milestone integration tests.
//!
//! Drives the Core VM [`eval`] entry point on builder-constructed terms.
//! Runs inside `parlang-core`; the driver is intentionally not wired to the
//! Core VM in Arc C.

use parlang_core::builder::{
    alt, app, apps, base_ty, bool_, case_, char_, con, fun_ty, int, lam, let_, letrec, tapp, tcon,
    tvar, unit, var,
};
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
    let t = letrec(
        "fac",
        fun_ty(base_ty(BaseType::Int), base_ty(BaseType::Int)),
        fac_lam,
        app(var("fac")?, int(5)),
    )?;
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
    let t = letrec(
        "f",
        fun_ty(base_ty(BaseType::Int), base_ty(BaseType::Int)),
        f_lam,
        app(var("f")?, int(5)),
    )?;
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
/// list = forall r. (Int -> r -> r) -> r -> r
/// nil  = /\r. \c:(Int -> r -> r). \n:r. n
/// cons = \h:Int. \t:list. /\r. \c:(Int -> r -> r). \n:r.  c h (t [r] c n)
/// sum  = \l:list.  l [Int] (\h:Int. \acc:Int. + h acc) 0
///
/// sum (cons 1 (cons 2 (cons 3 nil)))  =>  Int(6)
/// ```
#[test]
fn church_encoded_list_sum() -> Result<(), Box<dyn std::error::Error>> {
    use parlang_core::builder::{forall, tyapp, tylam};
    use parlang_core::{lint, LintEnv};
    // step = Int -> r -> r;  list = forall r. step -> r -> r
    let step_ty = fun_ty(base_ty(BaseType::Int), fun_ty(tvar("r")?, tvar("r")?));
    let list_ty = forall("r", fun_ty(step_ty.clone(), fun_ty(tvar("r")?, tvar("r")?)))?;

    // nil = /\r. \c:step. \n:r. n
    let nil = tylam(
        "r",
        lam("c", step_ty.clone(), lam("n", tvar("r")?, var("n")?)?)?,
    )?;

    // cons = \h:Int. \t:list. /\r. \c:step. \n:r.  c h (t [r] c n)
    let t_c_n = apps(tyapp(var("t")?, tvar("r")?), vec![var("c")?, var("n")?]);
    let c_h_tcn = apps(var("c")?, vec![var("h")?, t_c_n]);
    let handler = lam("c", step_ty, lam("n", tvar("r")?, c_h_tcn)?)?;
    let cons = lam(
        "h",
        BaseType::Int,
        lam("t", list_ty.clone(), tylam("r", handler)?)?,
    )?;

    // sum = \l:list. l [Int] (\h:Int. \acc:Int. + h acc) 0
    let add_h_acc = con("+", vec![var("h")?, var("acc")?])?;
    let combiner = lam("h", BaseType::Int, lam("acc", BaseType::Int, add_h_acc)?)?;
    let folded = apps(
        tyapp(var("l")?, base_ty(BaseType::Int)),
        vec![combiner, int(0)],
    );
    let sum = lam("l", list_ty, folded)?;

    // list: cons 1 (cons 2 (cons 3 nil))
    let list = apps(
        cons.clone(),
        vec![
            int(1),
            apps(cons.clone(), vec![int(2), apps(cons, vec![int(3), nil])]),
        ],
    );

    let t = app(sum, list);
    assert_eq!(lint(&t, &LintEnv::new())?, base_ty(BaseType::Int));
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

#[test]
fn case_selects_matching_alt() -> Result<(), parlang_core::BuildError> {
    // case Cons(1, Nil) of Cons(h, t) -> h | Nil -> 0  ==> 1
    let scrutinee = con("Cons", vec![int(1), con("Nil", vec![])?])?;
    let term = case_(
        scrutinee,
        vec![
            alt("Cons", &["h", "t"], var("h")?)?,
            alt("Nil", &[], int(0))?,
        ],
    );
    assert_eq!(eval(&term, &Environment::new()), Ok(Value::Int(1)));
    Ok(())
}

#[test]
fn case_list_length_via_letrec() -> Result<(), Box<dyn std::error::Error>> {
    use parlang_core::builder::{con_at, tyapp, tylam};
    use parlang_core::{lint, prelude_env};
    // len = /\a. letrec len : List a -> Int =
    //         \xs:List a. case xs of Cons(h, t) -> 1 + len t | Nil -> 0
    //       in len
    // len [Int] (Cons @Int 7 (Cons @Int 8 (Nil @Int)))  ==> 2
    let body = case_(
        var("xs")?,
        vec![
            alt(
                "Cons",
                &["h", "t"],
                con("+", vec![int(1), app(var("len")?, var("t")?)])?,
            )?,
            alt("Nil", &[], int(0))?,
        ],
    );
    let len_lam = lam("xs", tapp(tcon("List")?, tvar("a")?), body)?;
    let mono_len = letrec(
        "len",
        fun_ty(tapp(tcon("List")?, tvar("a")?), base_ty(BaseType::Int)),
        len_lam,
        var("len")?,
    )?;
    let poly_len = tylam("a", mono_len)?;
    let nil = con_at("Nil", vec![base_ty(BaseType::Int)], vec![])?;
    let inner = con_at("Cons", vec![base_ty(BaseType::Int)], vec![int(8), nil])?;
    let list = con_at("Cons", vec![base_ty(BaseType::Int)], vec![int(7), inner])?;
    let term = app(tyapp(poly_len, base_ty(BaseType::Int)), list);
    assert_eq!(lint(&term, &prelude_env())?, base_ty(BaseType::Int));
    assert_eq!(eval(&term, &Environment::new()), Ok(Value::Int(2)));
    Ok(())
}

#[test]
fn case_no_matching_alt_errors() -> Result<(), parlang_core::BuildError> {
    // case Nil of Cons(h, t) -> h   ==> NoMatchingAlt("Nil")
    let term = case_(con("Nil", vec![])?, vec![alt("Cons", &["h", "t"], int(0))?]);
    assert_eq!(
        eval(&term, &Environment::new()),
        Err(EvalError::NoMatchingAlt("Nil".to_string()))
    );
    Ok(())
}

#[test]
fn case_binder_arity_mismatch_errors() -> Result<(), parlang_core::BuildError> {
    // case Cons(1, Nil) of Cons(only) -> only   ==> TypeMismatch
    let scrutinee = con("Cons", vec![int(1), con("Nil", vec![])?])?;
    let term = case_(scrutinee, vec![alt("Cons", &["only"], int(0))?]);
    assert!(matches!(
        eval(&term, &Environment::new()),
        Err(EvalError::TypeMismatch { .. })
    ));
    Ok(())
}

// ---------------------------------------------------------------------------
// Strings as List Char (ADR 0002 Cons/Nil encoding; ADR 0004 Char model)
// ---------------------------------------------------------------------------

/// A string is a list of `Char`s under ADR 0002's `Cons`/`Nil` encoding;
/// its length is computed with the ordinary `Case` eliminator.
#[test]
fn string_as_list_char_length_is_two() -> Result<(), Box<dyn std::error::Error>> {
    use parlang_core::builder::con_at;
    use parlang_core::{lint, prelude_env};
    // "ab" = Cons @Char 'a' (Cons @Char 'b' (Nil @Char))
    let nil = con_at("Nil", vec![base_ty(BaseType::Char)], vec![])?;
    let inner = con_at("Cons", vec![base_ty(BaseType::Char)], vec![char_('b'), nil])?;
    let s = con_at(
        "Cons",
        vec![base_ty(BaseType::Char)],
        vec![char_('a'), inner],
    )?;
    // letrec len = \xs. case xs of Cons(h, t) -> 1 + len t | Nil -> 0 in len s
    let body = case_(
        var("xs")?,
        vec![
            alt(
                "Cons",
                &["h", "t"],
                con("+", vec![int(1), app(var("len")?, var("t")?)])?,
            )?,
            alt("Nil", &[], int(0))?,
        ],
    );
    let list_char = tapp(tcon("List")?, base_ty(BaseType::Char));
    let len_lam = lam("xs", list_char.clone(), body)?;
    let term = letrec(
        "len",
        fun_ty(list_char, base_ty(BaseType::Int)),
        len_lam,
        app(var("len")?, s),
    )?;
    assert_eq!(lint(&term, &prelude_env())?, base_ty(BaseType::Int));
    assert_eq!(eval(&term, &env())?, Value::Int(2));
    Ok(())
}

/// A non-ASCII Unicode scalar round-trips through eval and renders readably.
#[test]
fn non_ascii_char_roundtrips() -> Result<(), Box<dyn std::error::Error>> {
    let scalar = '\u{1F600}';
    let v = eval(&char_(scalar), &env())?;
    assert_eq!(v, Value::Char(scalar));
    assert_eq!(char_(scalar).to_text(), format!("{scalar:?}"));
    Ok(())
}

// ---------------------------------------------------------------------------
// Standing guard: lint explicitly-typed Core, then confirm it still evaluates.
// ---------------------------------------------------------------------------

/// factorial via letrec: lint-clean at `Int -> Int`, evaluates to 120.
#[test]
fn lint_and_eval_factorial() -> Result<(), Box<dyn std::error::Error>> {
    use parlang_core::{lint, LintEnv};
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
    let t = letrec(
        "fac",
        fun_ty(base_ty(BaseType::Int), base_ty(BaseType::Int)),
        fac_lam,
        app(var("fac")?, int(5)),
    )?;
    assert_eq!(lint(&t, &LintEnv::new())?, base_ty(BaseType::Int));
    assert_eq!(eval(&t, &env())?, Value::Int(120));
    Ok(())
}

/// Higher-kinded Core (M2.1a): `data Wrap f a = MkWrap (f a)` with
/// `f : * -> *`, and a type abstraction over `f` instantiated at `Option`
/// and at `List`. Both instantiations lint, then evaluate on the machine.
#[test]
fn lint_and_eval_higher_kinded_wrap() -> Result<(), Box<dyn std::error::Error>> {
    use parlang_core::builder::{con_at, tyapp, tylam, tylam_k};
    use parlang_core::{lint, prelude_env, DataCon, DataType, Kind};
    let star_to_star = Kind::arrow(Kind::Star, Kind::Star);
    let option = DataType {
        name: "Option".to_string(),
        params: vec!["a".to_string()],
        param_kinds: vec![Kind::Star],
        ctors: vec![
            DataCon {
                name: "None".to_string(),
                fields: vec![],
            },
            DataCon {
                name: "Some".to_string(),
                fields: vec![tvar("a")?],
            },
        ],
    };
    let wrap = DataType {
        name: "Wrap".to_string(),
        params: vec!["f".to_string(), "a".to_string()],
        param_kinds: vec![star_to_star.clone(), Kind::Star],
        ctors: vec![DataCon {
            name: "MkWrap".to_string(),
            fields: vec![tapp(tvar("f")?, tvar("a")?)],
        }],
    };
    let lint_env = prelude_env().try_with_data(option)?.try_with_data(wrap)?;
    // /\(f : * -> *). /\a. \x : f a. MkWrap @f @a (x)
    let wrap_body = con_at("MkWrap", vec![tvar("f")?, tvar("a")?], vec![var("x")?])?;
    let wrap_lam = lam("x", tapp(tvar("f")?, tvar("a")?), wrap_body)?;
    let wrap_fn = tylam_k("f", star_to_star, tylam("a", wrap_lam)?)?;
    let int_ty = base_ty(BaseType::Int);
    // case wrapFn @Option @Int (Some @Int 3) of MkWrap(o) -> case o of Some(n) -> n | None -> 0
    let some = con_at("Some", vec![int_ty.clone()], vec![int(3)])?;
    let at_option = tyapp(tyapp(wrap_fn.clone(), tcon("Option")?), int_ty.clone());
    let unwrap_option = case_(
        var("o")?,
        vec![alt("Some", &["n"], var("n")?)?, alt("None", &[], int(0))?],
    );
    let t = case_(
        app(at_option, some),
        vec![alt("MkWrap", &["o"], unwrap_option)?],
    );
    assert_eq!(lint(&t, &lint_env)?, int_ty);
    assert_eq!(eval(&t, &env())?, Value::Int(3));
    // The same abstraction instantiated at List: wrapFn @List @Int (Nil @Int).
    let nil = con_at("Nil", vec![int_ty.clone()], vec![])?;
    let at_list = app(tyapp(tyapp(wrap_fn, tcon("List")?), int_ty), nil);
    assert_eq!(lint(&at_list, &lint_env)?.to_string(), "Wrap List Int");
    let expected = Value::Con(
        "MkWrap".to_string(),
        vec![Value::Con("Nil".to_string(), vec![])],
    );
    assert_eq!(eval(&at_list, &env())?, expected);
    Ok(())
}

/// Polymorphic identity instantiated at two types: lint-clean, evaluates.
#[test]
fn lint_and_eval_poly_identity() -> Result<(), Box<dyn std::error::Error>> {
    use parlang_core::builder::{tyapp, tylam};
    use parlang_core::{lint, LintEnv};
    let poly = tylam("a", lam("x", tvar("a")?, var("x")?)?)?;
    let at_int = app(tyapp(poly.clone(), base_ty(BaseType::Int)), int(5));
    assert_eq!(lint(&at_int, &LintEnv::new())?, base_ty(BaseType::Int));
    assert_eq!(eval(&at_int, &env())?, Value::Int(5));
    let at_bool = app(tyapp(poly, base_ty(BaseType::Bool)), bool_(true));
    assert_eq!(lint(&at_bool, &LintEnv::new())?, base_ty(BaseType::Bool));
    assert_eq!(eval(&at_bool, &env())?, Value::Bool(true));
    Ok(())
}

/// List length over Cons @Int / Nil @Int via letrec + Case under `prelude_env`:
/// lint-clean at `Int`, evaluates to 2.
#[test]
fn lint_and_eval_list_length() -> Result<(), Box<dyn std::error::Error>> {
    use parlang_core::builder::con_at;
    use parlang_core::{lint, prelude_env};
    let list_int = tapp(tcon("List")?, base_ty(BaseType::Int));
    let body = case_(
        var("xs")?,
        vec![
            alt(
                "Cons",
                &["h", "t"],
                con("+", vec![int(1), app(var("len")?, var("t")?)])?,
            )?,
            alt("Nil", &[], int(0))?,
        ],
    );
    let len_lam = lam("xs", list_int.clone(), body)?;
    let nil = con_at("Nil", vec![base_ty(BaseType::Int)], vec![])?;
    let inner = con_at("Cons", vec![base_ty(BaseType::Int)], vec![int(8), nil])?;
    let list = con_at("Cons", vec![base_ty(BaseType::Int)], vec![int(7), inner])?;
    let t = letrec(
        "len",
        fun_ty(list_int, base_ty(BaseType::Int)),
        len_lam,
        app(var("len")?, list),
    )?;
    assert_eq!(lint(&t, &prelude_env())?, base_ty(BaseType::Int));
    assert_eq!(eval(&t, &env())?, Value::Int(2));
    Ok(())
}
