//! Gated tests for the `parlang_core::eligibility` kernel-eligibility
//! predicate, on builder-made Core and on real elaborated programs.

use std::error::Error;

use parlang::parse_program;
use parlang::pipeline::core_of_program;
use parlang_core::builder::{base_ty, int, lam, let_, tapp, tapps, tcon, tvar, var};
use parlang_core::eligibility::{find_binding, is_join_point, JOIN_POINT_TAG};
use parlang_core::{check_function, DataCon, DataType, EligibilityError, LintEnv};
use parlang_core::{BaseType, Lit, Report, Rule, Term, Type, Violation};

#[test]
fn is_join_point_accepts_match_compiler_names() {
    assert_eq!(JOIN_POINT_TAG, "j");
    assert!(is_join_point("$j0"));
    assert!(is_join_point("$j42"));
}

#[test]
fn is_join_point_rejects_other_names() {
    for name in ["$j", "$jx", "j1", "$s3", "$u0", "$j1a", ""] {
        assert!(!is_join_point(name), "{name} must not be a join point");
    }
}

#[test]
fn rule_ids_and_names_are_stable() {
    let expected = [
        (Rule::KernelTypes, "R1", "kernel types"),
        (Rule::FirstOrder, "R2", "first-order"),
        (Rule::Recursion, "R3", "recursion"),
        (Rule::Data, "R4", "data"),
        (Rule::Polymorphism, "R5", "polymorphism"),
    ];
    for (rule, id, name) in expected {
        assert_eq!(rule.id(), id);
        assert_eq!(rule.name(), name);
    }
}

#[test]
fn owner_of_reports_declaring_type() {
    let env = LintEnv::new().with_data(DataType {
        name: "Pair".to_string(),
        params: vec![],
        param_kinds: vec![],
        ctors: vec![DataCon {
            name: "MkPair".to_string(),
            fields: vec![],
        }],
    });
    assert_eq!(env.owner_of("MkPair"), Some("Pair"));
    assert_eq!(env.owner_of("Missing"), None);
}

#[test]
fn unknown_function_is_an_error() {
    let result = check_function(&int(1), &LintEnv::new(), "f", &[]);
    let expected = EligibilityError::UnknownFunction("f".to_string());
    assert_eq!(result, Err(expected.clone()));
    assert_eq!(expected.to_string(), "unknown function: f");
}

#[test]
fn find_binding_takes_innermost() -> Result<(), Box<dyn Error>> {
    let root = let_("f", int(1), let_("f", int(2), var("f")?)?)?;
    let found = find_binding(&root, "f");
    assert!(matches!(found, Some(Term::Lit(Lit::Int(2)))));
    Ok(())
}

#[test]
fn real_program_function_is_found() -> Result<(), Box<dyn Error>> {
    let program = parse_program("let inc = fun x -> x + 1;\ninc 41")?;
    let elab = core_of_program(&program)?;
    let report = check_function(&elab.term, &elab.env, "inc", &[])?;
    assert_eq!(report, Report::Eligible);
    Ok(())
}

fn data(name: &str, params: &[&str], ctors: Vec<(&str, Vec<Type>)>) -> DataType {
    DataType {
        name: name.to_string(),
        params: params.iter().map(ToString::to_string).collect(),
        param_kinds: vec![parlang_core::Kind::Star; params.len()],
        ctors: ctors
            .into_iter()
            .map(|(ctor, fields)| DataCon {
                name: ctor.to_string(),
                fields,
            })
            .collect(),
    }
}

fn list_type() -> Result<DataType, Box<dyn Error>> {
    let tail = tapp(tcon("List")?, tvar("a")?);
    let cons = ("Cons", vec![tvar("a")?, tail]);
    Ok(data("List", &["a"], vec![("Nil", vec![]), cons]))
}

fn identity_fn(name: &str, param: &str, ty: Type) -> Result<Term, Box<dyn Error>> {
    let body = lam(param, ty, var(param)?)?;
    Ok(let_(name, body, var(name)?)?)
}

fn r1_rejection(function: &str, node: &str) -> Report {
    Report::Rejected(vec![Violation {
        rule: Rule::KernelTypes,
        function: function.to_string(),
        node: node.to_string(),
    }])
}

#[test]
fn int_param_function_is_eligible() -> Result<(), Box<dyn Error>> {
    let root = identity_fn("f", "x", base_ty(BaseType::Int))?;
    assert_eq!(
        check_function(&root, &LintEnv::new(), "f", &[])?,
        Report::Eligible
    );
    Ok(())
}

#[test]
fn list_param_is_rejected_r1() -> Result<(), Box<dyn Error>> {
    let env = LintEnv::new().with_data(list_type()?);
    let ty = tapp(tcon("List")?, base_ty(BaseType::Int));
    let root = identity_fn("len", "xs", ty)?;
    let expected = r1_rejection("len", "(\\xs: List Int. xs)");
    assert_eq!(check_function(&root, &env, "len", &[])?, expected);
    Ok(())
}

#[test]
fn generalised_id_is_rejected_r5() -> Result<(), Box<dyn Error>> {
    let program = parse_program("let id = fun x -> x;\nid 1")?;
    let elab = core_of_program(&program)?;
    let report = check_function(&elab.term, &elab.env, "id", &[])?;
    let Report::Rejected(violations) = report else {
        return Err("expected a rejection".into());
    };
    let first = violations.first().ok_or("no violations")?;
    assert_eq!(first.rule, Rule::Polymorphism);
    assert_eq!(first.function, "id");
    assert!(first.node.starts_with("(/\\t"), "{}", first.node);
    Ok(())
}

#[test]
fn tuple_param_is_eligible() -> Result<(), Box<dyn Error>> {
    let pair = data(
        "(,)",
        &["a", "b"],
        vec![("(,)", vec![tvar("a")?, tvar("b")?])],
    );
    let env = LintEnv::new().with_data(pair);
    let int_ty = base_ty(BaseType::Int);
    let ty = tapps(tcon("(,)")?, vec![int_ty.clone(), int_ty]);
    let root = identity_fn("swap", "p", ty)?;
    assert_eq!(check_function(&root, &env, "swap", &[])?, Report::Eligible);
    Ok(())
}

#[test]
fn option_like_param_is_eligible() -> Result<(), Box<dyn Error>> {
    let maybe = data(
        "Maybe",
        &["a"],
        vec![("Nothing", vec![]), ("Just", vec![tvar("a")?])],
    );
    let env = LintEnv::new().with_data(maybe);
    let ty = tapp(tcon("Maybe")?, base_ty(BaseType::Int));
    let root = identity_fn("get", "m", ty)?;
    assert_eq!(check_function(&root, &env, "get", &[])?, Report::Eligible);
    Ok(())
}

#[test]
fn recursive_data_param_is_rejected_r1() -> Result<(), Box<dyn Error>> {
    let fields = vec![tcon("Tree")?, base_ty(BaseType::Int), tcon("Tree")?];
    let tree = data("Tree", &[], vec![("Leaf", vec![]), ("Node", fields)]);
    let env = LintEnv::new().with_data(tree);
    let root = identity_fn("depth", "t", tcon("Tree")?)?;
    let expected = r1_rejection("depth", "(\\t: Tree. t)");
    assert_eq!(check_function(&root, &env, "depth", &[])?, expected);
    Ok(())
}

#[test]
fn data_reaching_list_is_rejected_r1() -> Result<(), Box<dyn Error>> {
    let items = tapp(tcon("List")?, base_ty(BaseType::Int));
    let bag = data("Bag", &[], vec![("MkBag", vec![items])]);
    let env = LintEnv::new().with_data(list_type()?).with_data(bag);
    let root = identity_fn("size", "b", tcon("Bag")?)?;
    let expected = r1_rejection("size", "(\\b: Bag. b)");
    assert_eq!(check_function(&root, &env, "size", &[])?, expected);
    Ok(())
}

const FACTORIAL: &str = "let factorial = rec f -> fun n ->
    if n == 0
    then 1
    else n * f (n - 1);
";

fn report_of(src: &str, name: &str) -> Result<Report, Box<dyn Error>> {
    let elab = core_of_program(&parse_program(src)?)?;
    Ok(check_function(&elab.term, &elab.env, name, &[])?)
}

fn rules_of(report: &Report) -> Vec<Rule> {
    match report {
        Report::Eligible => vec![],
        Report::Rejected(violations) => violations.iter().map(|v| v.rule).collect(),
    }
}

fn factorial_r3() -> Report {
    Report::Rejected(vec![Violation {
        rule: Rule::Recursion,
        function: "factorial".to_string(),
        node: "(f -(n, 1))".to_string(),
    }])
}

#[test]
fn recursion_sum_to_n_and_gcd_are_eligible() -> Result<(), Box<dyn Error>> {
    let src = "let sum_to_n = rec sum_helper -> fun acc -> fun n ->
    if n == 0
    then acc
    else sum_helper (acc + n) (n - 1);

let gcd = rec gcd_helper -> fun a -> fun b ->
    if b == 0
    then a
    else gcd_helper b (a - (a / b) * b);

0";
    assert_eq!(report_of(src, "sum_to_n")?, Report::Eligible);
    assert_eq!(report_of(src, "gcd")?, Report::Eligible);
    Ok(())
}

#[test]
fn factorial_non_tail_recursion_is_rejected_r3() -> Result<(), Box<dyn Error>> {
    let src = format!("{FACTORIAL}\n0");
    assert_eq!(report_of(&src, "factorial")?, factorial_r3());
    Ok(())
}

#[test]
fn non_recursive_adt_match_is_eligible() -> Result<(), Box<dyn Error>> {
    let src = "data Shape = Circle Int | Square Int;
let area = fun s -> match s with | Circle r -> r * r | Square w -> w * w;
0";
    assert_eq!(report_of(src, "area")?, Report::Eligible);
    Ok(())
}

#[test]
fn list_function_is_rejected_r4() -> Result<(), Box<dyn Error>> {
    let src = "data List a = Nil | Cons a (List a);
let len = fun xs -> match xs with | Nil -> 0 | Cons x rest -> 1;
0";
    let rules = rules_of(&report_of(src, "len")?);
    assert!(rules.contains(&Rule::Data), "{rules:?}");
    assert!(rules.contains(&Rule::KernelTypes), "{rules:?}");
    Ok(())
}

#[test]
fn higher_order_function_is_rejected_r2() -> Result<(), Box<dyn Error>> {
    let src = "let apply = fun f -> fun x -> f x;\n0";
    let rules = rules_of(&report_of(src, "apply")?);
    assert!(rules.contains(&Rule::FirstOrder), "{rules:?}");
    assert!(rules.contains(&Rule::KernelTypes), "{rules:?}");
    Ok(())
}

#[test]
fn tuple_match_uses_join_points_and_is_eligible() -> Result<(), Box<dyn Error>> {
    let src = "data AB = A | B;
let pairs = fun p -> match p with | (A, A) -> 1 | _ -> 2;
0";
    let elab = core_of_program(&parse_program(src)?)?;
    let text = find_binding(&elab.term, "pairs").map(Term::to_text);
    assert!(text.is_some_and(|t| t.contains("$j")));
    let report = check_function(&elab.term, &elab.env, "pairs", &[])?;
    assert_eq!(report, Report::Eligible);
    Ok(())
}

#[test]
fn ineligible_callee_violations_are_appended() -> Result<(), Box<dyn Error>> {
    let src = format!("{FACTORIAL}let g = fun n -> factorial n;\n0");
    assert_eq!(report_of(&src, "g")?, factorial_r3());
    Ok(())
}

#[test]
fn unbound_variable_is_malformed() -> Result<(), Box<dyn Error>> {
    let root = let_("f", lam("x", base_ty(BaseType::Int), var("y")?)?, var("f")?)?;
    let result = check_function(&root, &LintEnv::new(), "f", &[]);
    let expected = EligibilityError::Malformed("unbound variable y".to_string());
    assert_eq!(result, Err(expected));
    Ok(())
}
