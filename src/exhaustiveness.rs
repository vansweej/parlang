//! Exhaustiveness checking for pattern matching
//!
//! This module implements an algorithm to check if a set of patterns in a match expression
//! covers all possible values of the scrutinee type. This helps catch bugs at "compile time"
//! (before evaluation) by ensuring that all cases are handled.
//!
//! # Algorithm
//!
//! The exhaustiveness checker works by building a "usefulness" matrix that tracks which
//! patterns cover which values. The algorithm is based on the principles described in
//! "Warnings for pattern matching" by Luc Maranget (2007).
//!
//! # Example
//!
//! ```ignore
//! // Exhaustive - all Option constructors covered
//! type Option a = Some a | None in
//! match x with
//! | Some n -> n
//! | None -> 0
//!
//! // Non-exhaustive - missing None case
//! type Option a = Some a | None in
//! match x with
//! | Some n -> n
//! ```

use crate::ast::{Literal, Pattern};
use crate::eval::Environment;
use std::collections::HashSet;

/// Result of exhaustiveness checking
#[derive(Debug, Clone, PartialEq)]
pub enum ExhaustivenessResult {
    /// Patterns are exhaustive
    Exhaustive,
    /// Patterns are non-exhaustive, with examples of missing patterns
    NonExhaustive(Vec<String>),
}

impl ExhaustivenessResult {
    /// Returns true if patterns are exhaustive
    pub fn is_exhaustive(&self) -> bool {
        matches!(self, ExhaustivenessResult::Exhaustive)
    }
}

/// Check if a list of patterns is exhaustive
///
/// This function analyzes the patterns to determine if they cover all possible values.
/// It considers:
/// - Literal patterns (Int, Bool)
/// - Variable and wildcard patterns (always match everything)
/// - Constructor patterns (sum types)
/// - Tuple patterns
/// - Record patterns
///
/// # Arguments
///
/// * `patterns` - The list of patterns from the match arms
/// * `env` - The environment containing constructor information
///
/// # Returns
///
/// An `ExhaustivenessResult` indicating whether the patterns are exhaustive
pub fn check_exhaustiveness(patterns: &[Pattern], env: &Environment) -> ExhaustivenessResult {
    if patterns.is_empty() {
        return ExhaustivenessResult::NonExhaustive(vec!["_".to_string()]);
    }

    // Check if there's a catch-all pattern (wildcard or variable)
    if has_catch_all(patterns) {
        return ExhaustivenessResult::Exhaustive;
    }

    // Analyze patterns by type
    let mut constructors_covered: HashSet<String> = HashSet::new();
    let mut has_bool_true = false;
    let mut has_bool_false = false;
    let mut int_literals: HashSet<i64> = HashSet::new();
    let mut has_tuple_pattern = false;
    let mut has_record_pattern = false;

    for pattern in patterns {
        analyze_pattern(
            pattern,
            &mut constructors_covered,
            &mut has_bool_true,
            &mut has_bool_false,
            &mut int_literals,
            &mut has_tuple_pattern,
            &mut has_record_pattern,
        );
    }

    // Check for exhaustiveness based on pattern types

    // 1. Constructor patterns - need to check all constructors of the type are covered
    if !constructors_covered.is_empty() {
        if let Some(missing) = check_constructor_exhaustiveness(&constructors_covered, env) {
            return ExhaustivenessResult::NonExhaustive(missing);
        }
    }

    // 2. Boolean patterns - need both true and false
    if has_bool_true || has_bool_false {
        if !has_bool_true {
            return ExhaustivenessResult::NonExhaustive(vec!["true".to_string()]);
        }
        if !has_bool_false {
            return ExhaustivenessResult::NonExhaustive(vec!["false".to_string()]);
        }
    }

    // 3. Integer patterns - integers are infinite, so if we only have literals, it's non-exhaustive
    if !int_literals.is_empty() {
        // Integer patterns alone (without a catch-all) are never exhaustive
        return ExhaustivenessResult::NonExhaustive(vec!["<other integers>".to_string()]);
    }

    // 4. Tuple and record patterns
    // For tuples and records, if we don't have a catch-all, we consider them non-exhaustive
    // (proper nested exhaustiveness checking would be more complex)
    if has_tuple_pattern || has_record_pattern {
        // These patterns alone without catch-all are considered non-exhaustive
        return ExhaustivenessResult::NonExhaustive(vec!["_".to_string()]);
    }

    // If we get here, we have no patterns or only catch-all patterns
    ExhaustivenessResult::Exhaustive
}

/// Check if patterns contain a catch-all (wildcard or variable)
fn has_catch_all(patterns: &[Pattern]) -> bool {
    patterns.iter().any(|p| matches!(p, Pattern::Wildcard | Pattern::Var(_)))
}

/// Recursively analyze a pattern to collect information
fn analyze_pattern(
    pattern: &Pattern,
    constructors: &mut HashSet<String>,
    has_bool_true: &mut bool,
    has_bool_false: &mut bool,
    int_literals: &mut HashSet<i64>,
    has_tuple_pattern: &mut bool,
    has_record_pattern: &mut bool,
) {
    match pattern {
        Pattern::Literal(Literal::Bool(true)) => *has_bool_true = true,
        Pattern::Literal(Literal::Bool(false)) => *has_bool_false = true,
        Pattern::Literal(Literal::Int(n)) => {
            int_literals.insert(*n);
        }
        Pattern::Constructor(name, args) => {
            constructors.insert(name.clone());
            // Recursively analyze nested patterns
            for arg in args {
                analyze_pattern(
                    arg,
                    constructors,
                    has_bool_true,
                    has_bool_false,
                    int_literals,
                    has_tuple_pattern,
                    has_record_pattern,
                );
            }
        }
        Pattern::Tuple(patterns) => {
            *has_tuple_pattern = true;
            // Recursively analyze nested patterns
            for p in patterns {
                analyze_pattern(
                    p,
                    constructors,
                    has_bool_true,
                    has_bool_false,
                    int_literals,
                    has_tuple_pattern,
                    has_record_pattern,
                );
            }
        }
        Pattern::Record(fields) => {
            *has_record_pattern = true;
            // Recursively analyze nested patterns
            for (_, p) in fields {
                analyze_pattern(
                    p,
                    constructors,
                    has_bool_true,
                    has_bool_false,
                    int_literals,
                    has_tuple_pattern,
                    has_record_pattern,
                );
            }
        }
        Pattern::Wildcard | Pattern::Var(_) => {
            // These are catch-all patterns, handled separately
        }
    }
}

/// Check if all constructors of a sum type are covered
///
/// This function finds all constructors that belong to the same type as the covered
/// constructors and checks if all are present in the set.
fn check_constructor_exhaustiveness(
    covered: &HashSet<String>,
    env: &Environment,
) -> Option<Vec<String>> {
    // Find the type name from one of the covered constructors
    let first_ctor = covered.iter().next()?;
    let type_info = env.get_constructor(first_ctor)?;
    let type_name = &type_info.type_name;

    // Find all constructors for this type
    let all_constructors = env.get_constructors_for_type(type_name);

    // Check which constructors are missing
    let missing: Vec<String> = all_constructors
        .into_iter()
        .filter(|ctor| !covered.contains(ctor))
        .collect();

    if missing.is_empty() {
        None
    } else {
        Some(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Literal;
    use crate::eval::ConstructorInfo;

    #[test]
    fn test_empty_patterns() {
        let patterns: Vec<Pattern> = vec![];
        let env = Environment::new();
        let result = check_exhaustiveness(&patterns, &env);
        assert!(!result.is_exhaustive());
    }

    #[test]
    fn test_wildcard_exhaustive() {
        let patterns = vec![Pattern::Wildcard];
        let env = Environment::new();
        let result = check_exhaustiveness(&patterns, &env);
        assert!(result.is_exhaustive());
    }

    #[test]
    fn test_variable_exhaustive() {
        let patterns = vec![Pattern::Var("x".to_string())];
        let env = Environment::new();
        let result = check_exhaustiveness(&patterns, &env);
        assert!(result.is_exhaustive());
    }

    #[test]
    fn test_bool_exhaustive() {
        let patterns = vec![
            Pattern::Literal(Literal::Bool(true)),
            Pattern::Literal(Literal::Bool(false)),
        ];
        let env = Environment::new();
        let result = check_exhaustiveness(&patterns, &env);
        assert!(result.is_exhaustive());
    }

    #[test]
    fn test_bool_non_exhaustive_missing_false() {
        let patterns = vec![Pattern::Literal(Literal::Bool(true))];
        let env = Environment::new();
        let result = check_exhaustiveness(&patterns, &env);
        assert!(!result.is_exhaustive());
        if let ExhaustivenessResult::NonExhaustive(missing) = result {
            assert!(missing.contains(&"false".to_string()));
        }
    }

    #[test]
    fn test_bool_non_exhaustive_missing_true() {
        let patterns = vec![Pattern::Literal(Literal::Bool(false))];
        let env = Environment::new();
        let result = check_exhaustiveness(&patterns, &env);
        assert!(!result.is_exhaustive());
        if let ExhaustivenessResult::NonExhaustive(missing) = result {
            assert!(missing.contains(&"true".to_string()));
        }
    }

    #[test]
    fn test_int_non_exhaustive() {
        let patterns = vec![
            Pattern::Literal(Literal::Int(0)),
            Pattern::Literal(Literal::Int(1)),
        ];
        let env = Environment::new();
        let result = check_exhaustiveness(&patterns, &env);
        assert!(!result.is_exhaustive());
    }

    #[test]
    fn test_int_with_wildcard_exhaustive() {
        let patterns = vec![
            Pattern::Literal(Literal::Int(0)),
            Pattern::Literal(Literal::Int(1)),
            Pattern::Wildcard,
        ];
        let env = Environment::new();
        let result = check_exhaustiveness(&patterns, &env);
        assert!(result.is_exhaustive());
    }

    #[test]
    fn test_constructor_exhaustive() {
        let mut env = Environment::new();
        // Register Option type constructors
        env.register_constructor("Some".to_string(), ConstructorInfo {
            type_name: "Option".to_string(),
            arity: 1,
        });
        env.register_constructor("None".to_string(), ConstructorInfo {
            type_name: "Option".to_string(),
            arity: 0,
        });

        let patterns = vec![
            Pattern::Constructor("Some".to_string(), vec![Pattern::Wildcard]),
            Pattern::Constructor("None".to_string(), vec![]),
        ];
        let result = check_exhaustiveness(&patterns, &env);
        assert!(result.is_exhaustive());
    }

    #[test]
    fn test_constructor_non_exhaustive() {
        let mut env = Environment::new();
        // Register Option type constructors
        env.register_constructor("Some".to_string(), ConstructorInfo {
            type_name: "Option".to_string(),
            arity: 1,
        });
        env.register_constructor("None".to_string(), ConstructorInfo {
            type_name: "Option".to_string(),
            arity: 0,
        });

        // Only match Some, missing None
        let patterns = vec![
            Pattern::Constructor("Some".to_string(), vec![Pattern::Wildcard]),
        ];
        let result = check_exhaustiveness(&patterns, &env);
        assert!(!result.is_exhaustive());
        if let ExhaustivenessResult::NonExhaustive(missing) = result {
            assert!(missing.contains(&"None".to_string()));
        }
    }

    #[test]
    fn test_nested_constructor() {
        let mut env = Environment::new();
        env.register_constructor("Some".to_string(), ConstructorInfo {
            type_name: "Option".to_string(),
            arity: 1,
        });
        env.register_constructor("None".to_string(), ConstructorInfo {
            type_name: "Option".to_string(),
            arity: 0,
        });

        let patterns = vec![
            Pattern::Constructor(
                "Some".to_string(),
                vec![Pattern::Constructor("Some".to_string(), vec![Pattern::Wildcard])],
            ),
            Pattern::Constructor(
                "Some".to_string(),
                vec![Pattern::Constructor("None".to_string(), vec![])],
            ),
            Pattern::Constructor("None".to_string(), vec![]),
        ];
        let result = check_exhaustiveness(&patterns, &env);
        assert!(result.is_exhaustive());
    }
}
