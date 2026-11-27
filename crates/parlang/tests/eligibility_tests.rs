//! Gated tests for the `parlang_core::eligibility` kernel-eligibility
//! predicate, on builder-made Core and on real elaborated programs.

use std::error::Error;

use parlang::parse_program;
use parlang::pipeline::core_of_program;
use parlang_core::builder::{int, let_, var};
use parlang_core::eligibility::{find_binding, is_join_point, JOIN_POINT_TAG};
use parlang_core::{check_function, DataCon, DataType, EligibilityError, LintEnv};
use parlang_core::{Lit, Report, Rule, Term};

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
