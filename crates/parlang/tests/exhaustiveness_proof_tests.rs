//! Type-check-time proofs that match exhaustiveness checking is sound.
use parlang::{parse_program, typecheck_program, Type, TypeError};

/// Type-check `src` as a whole program.
fn check(src: &str) -> Result<Type, TypeError> {
    let program = parse_program(src).expect("program should parse");
    typecheck_program(&program)
}

/// The witnesses reported for a non-exhaustive `src`, or `None` if it is
/// accepted or rejected for another reason.
fn witnesses(src: &str) -> Option<Vec<String>> {
    match check(src) {
        Err(TypeError::NonExhaustiveMatch { missing }) => Some(missing),
        _ => None,
    }
}

#[test]
fn nested_option_gap_is_reported() {
    let src = "data Option a = Some a | None;\n\
               match Some (Some 1) with | Some (Some n) -> n | None -> 0";
    assert_eq!(witnesses(src), Some(vec!["Some None".to_string()]));
}

#[test]
fn int_literals_without_catch_all_are_reported() {
    let src = "match 1 with | 0 -> 1 | 1 -> 2";
    assert_eq!(witnesses(src), Some(vec!["_".to_string()]));
}

#[test]
fn char_literals_without_catch_all_are_reported() {
    let src = "match 'a' with | 'a' -> 1 | 'b' -> 2";
    assert_eq!(witnesses(src), Some(vec!["_".to_string()]));
}

#[test]
fn bool_missing_false_is_reported() {
    let src = "match true with | true -> 1";
    assert_eq!(witnesses(src), Some(vec!["false".to_string()]));
}

#[test]
fn tuple_of_lists_gap_is_reported() {
    let src = "data List a = Nil | Cons a (List a);\n\
               match (Nil, Nil) with | (Nil, _) -> 0 | (_, Nil) -> 1";
    assert!(witnesses(src).is_some());
}

#[test]
fn strcmp_shape_is_exhaustive() {
    let src = "data List a = Nil | Cons a (List a);\n\
               let f = fun p -> match p with \
               | (Nil, Nil) -> 0 | (Nil, _) -> 1 | (_, Nil) -> 2 \
               | (Cons a _, Cons b _) -> a + b in f (Nil, Nil)";
    assert!(check(src).is_ok());
}

#[test]
fn bool_both_arms_are_exhaustive() {
    assert!(check("match true with | true -> 1 | false -> 0").is_ok());
}

#[test]
fn unit_pattern_is_exhaustive() {
    assert!(check("match () with | () -> 1").is_ok());
}

#[test]
fn partial_record_patterns_are_exhaustive() {
    assert!(check("match {x: 1, y: 2} with | {x: 1} -> 10 | {y: n} -> n").is_ok());
}
