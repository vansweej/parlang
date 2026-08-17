//! Strict, call-by-value, environment-based tree-walking VM over [`Term`].
//!
//! Evaluates Arc B Core IR terms to runtime [`Value`]s. Branching and
//! arithmetic are encoded as reserved constructor names dispatched here
//! (ADR 0001 Decision 3 Option A). Recursion uses [`Value::RecClosure`]
//! re-bound at every application (ADR 0001 Decision 4). See
//! `docs/CORE_OPERATIONAL_SEMANTICS.md` and `docs/adr/0001-arc-c-eval-model.md`.

use crate::term::{Lit, Term};
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

// ---------------------------------------------------------------------------
// Closure payloads — boxed to preempt clippy::large_enum_variant.
//
// These structs are `pub` so that the `pub Value` enum variants holding
// `Box<ClosureData>` / `Box<RecClosureData>` satisfy the `private_interfaces`
// lint. They are not re-exported from the crate root and remain effectively
// internal despite the `pub` visibility.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureData {
    pub(crate) param: String,
    pub(crate) body: Box<Term>,
    pub(crate) env: Environment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecClosureData {
    pub(crate) name: String,
    pub(crate) param: String,
    pub(crate) body: Box<Term>,
    pub(crate) env: Environment,
}

// ---------------------------------------------------------------------------
// Value
// ---------------------------------------------------------------------------

/// A runtime value produced by the Core VM.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A 64-bit signed integer.
    Int(i64),
    /// A boolean.
    Bool(bool),
    /// The unit value.
    Unit,
    /// A UTF-8 string.
    Str(String),
    /// A 64-bit float.
    Float(f64),
    /// A non-recursive closure (lambda + captured environment).
    Closure(Box<ClosureData>),
    /// A recursive closure produced by `letrec`; re-binds its own name at
    /// each application (ADR 0001 Decision 4).
    RecClosure(Box<RecClosureData>),
    /// A constructor value with evaluated arguments.
    Con(String, Vec<Value>),
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

/// A persistent, value-semantics environment mapping names to [`Value`]s.
///
/// [`extend`](Environment::extend) returns a new environment; the original
/// is unchanged.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Environment(HashMap<String, Value>);

impl Environment {
    /// Creates an empty environment.
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Looks up `name` in the environment.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }

    /// Returns a new environment with `name` bound to `value`.
    #[must_use]
    pub fn extend(&self, name: &str, value: Value) -> Self {
        let mut map = self.0.clone();
        map.insert(name.to_string(), value);
        Self(map)
    }
}

// ---------------------------------------------------------------------------
// EvalError
// ---------------------------------------------------------------------------

/// A runtime evaluation error.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// A variable was not found in the environment.
    UnboundVar(String),
    /// A non-function value was applied as a function.
    NotAFunction,
    /// A value of an unexpected type was encountered by a primitive.
    TypeMismatch {
        /// The expected type or shape.
        expected: String,
        /// The actual type tag found.
        found: String,
    },
    /// Integer division by zero.
    DivisionByZero,
    /// A checked integer arithmetic operation overflowed.
    ArithmeticOverflow,
    /// An operand to `eq` was not a comparable base value.
    NotComparable(String),
}

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnboundVar(n) => write!(f, "unbound variable: {n}"),
            Self::NotAFunction => write!(f, "applied a non-function value"),
            Self::TypeMismatch { expected, found } => {
                write!(f, "type mismatch: expected {expected}, found {found}")
            }
            Self::DivisionByZero => write!(f, "integer division by zero"),
            Self::ArithmeticOverflow => write!(f, "integer arithmetic overflow"),
            Self::NotComparable(kind) => {
                write!(f, "eq: {kind} values are not comparable")
            }
        }
    }
}

impl std::error::Error for EvalError {}

/// Shorthand result type for evaluation.
pub type EvalResult = Result<Value, EvalError>;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Evaluates `term` under `env`, returning a [`Value`] or an [`EvalError`].
///
/// # Errors
/// Returns [`EvalError`] on unbound variables, type mismatches, division by
/// zero, arithmetic overflow, or application of a non-function.
pub fn eval(term: &Term, env: &Environment) -> EvalResult {
    match term {
        Term::Var(n) => env
            .lookup(n)
            .cloned()
            .ok_or_else(|| EvalError::UnboundVar(n.clone())),

        Term::Lit(l) => Ok(match l {
            Lit::Int(n) => Value::Int(*n),
            Lit::Bool(b) => Value::Bool(*b),
            Lit::Float(x) => Value::Float(*x),
            Lit::Unit => Value::Unit,
            Lit::Str(s) => Value::Str(s.clone()),
        }),

        Term::Lam(p, _ty, body) => Ok(Value::Closure(Box::new(ClosureData {
            param: p.clone(),
            body: body.clone(),
            env: env.clone(),
        }))),

        Term::Let(n, v, b) => {
            let vv = eval(v, env)?;
            eval(b, &env.extend(n, vv))
        }

        // ADR 0001 Decision 4: LetRec builds a RecClosure capturing the
        // CURRENT env WITHOUT pre-inserting the name. Re-binding happens at
        // every App on a RecClosure, enabling arbitrary-depth recursion.
        Term::LetRec(n, v, b) => {
            let Term::Lam(param, _ty, body) = v.as_ref() else {
                return Err(EvalError::TypeMismatch {
                    expected: "lambda".to_string(),
                    found: "non-lambda".to_string(),
                });
            };
            let rc = Value::RecClosure(Box::new(RecClosureData {
                name: n.clone(),
                param: param.clone(),
                body: body.clone(),
                env: env.clone(),
            }));
            eval(b, &env.extend(n, rc))
        }

        // ADR 0001 Decision 4: App re-binds the rec-name at every application.
        Term::App(f, a) => {
            let callee = eval(f, env)?;
            match callee {
                Value::Closure(ref cd) => {
                    let v_arg = eval(a, env)?;
                    eval(&cd.body, &cd.env.extend(&cd.param, v_arg))
                }
                Value::RecClosure(ref rcd) => {
                    let v_arg = eval(a, env)?;
                    let call_env = rcd
                        .env
                        .extend(&rcd.name, callee.clone())
                        .extend(&rcd.param, v_arg);
                    eval(&rcd.body, &call_env)
                }
                _ => Err(EvalError::NotAFunction),
            }
        }

        Term::Con(name, args) => {
            // Reserved primitive names per ADR 0001 Decision 3 (Option A).
            match name.as_str() {
                "if" => eval_if(args, env),
                "+" => eval_arith('+', args, env),
                "-" => eval_arith('-', args, env),
                "*" => eval_arith('*', args, env),
                "/" => eval_arith('/', args, env),
                "<" => eval_cmp(args, env),
                "eq" => eval_eq(args, env),
                "strlen" => eval_strlen(args, env),
                "strcat" => eval_strcat(args, env),
                _ => {
                    let mut vals = Vec::with_capacity(args.len());
                    for a in args {
                        vals.push(eval(a, env)?);
                    }
                    Ok(Value::Con(name.clone(), vals))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Primitive helpers
// ---------------------------------------------------------------------------

fn eval_if(args: &[Term], env: &Environment) -> EvalResult {
    if args.len() != 3 {
        return Err(EvalError::TypeMismatch {
            expected: "3 arguments".to_string(),
            found: format!("{} arguments", args.len()),
        });
    }
    match eval(&args[0], env)? {
        Value::Bool(true) => eval(&args[1], env),
        Value::Bool(false) => eval(&args[2], env),
        other => Err(EvalError::TypeMismatch {
            expected: "bool".to_string(),
            found: value_kind(&other).to_string(),
        }),
    }
}

/// Evaluate the private `strcat` primitive, concatenating two `Value::Str` values.
///
/// Requires exactly two arguments, both evaluating to `Value::Str`.
fn eval_strcat(args: &[Term], env: &Environment) -> EvalResult {
    if args.len() != 2 {
        return Err(EvalError::TypeMismatch {
            expected: "2 arguments".to_string(),
            found: format!("{} arguments", args.len()),
        });
    }
    let a = eval(&args[0], env)?;
    let b = eval(&args[1], env)?;
    match (&a, &b) {
        (Value::Str(a), Value::Str(b)) => {
            let mut joined = String::with_capacity(a.len() + b.len());
            joined.push_str(a);
            joined.push_str(b);
            Ok(Value::Str(joined))
        }
        (Value::Str(_), v) | (v, _) => Err(EvalError::TypeMismatch {
            expected: "str".to_string(),
            found: value_kind(v).to_string(),
        }),
    }
}

fn eval_arith(op: char, args: &[Term], env: &Environment) -> EvalResult {
    if args.len() != 2 {
        return Err(EvalError::TypeMismatch {
            expected: "2 arguments".to_string(),
            found: format!("{} arguments", args.len()),
        });
    }
    let lhs = eval(&args[0], env)?;
    let rhs = eval(&args[1], env)?;
    let (Value::Int(a), Value::Int(b)) = (&lhs, &rhs) else {
        let found = if matches!(lhs, Value::Int(_)) {
            value_kind(&rhs)
        } else {
            value_kind(&lhs)
        };
        return Err(EvalError::TypeMismatch {
            expected: "int".to_string(),
            found: found.to_string(),
        });
    };
    let (a, b) = (*a, *b);
    let result = match op {
        '+' => a.checked_add(b),
        '-' => a.checked_sub(b),
        '*' => a.checked_mul(b),
        '/' => {
            if b == 0 {
                return Err(EvalError::DivisionByZero);
            }
            a.checked_div(b)
        }
        _ => unreachable!("eval_arith called with unknown op '{op}'"),
    };
    result.map(Value::Int).ok_or(EvalError::ArithmeticOverflow)
}

fn eval_cmp(args: &[Term], env: &Environment) -> EvalResult {
    if args.len() != 2 {
        return Err(EvalError::TypeMismatch {
            expected: "2 arguments".to_string(),
            found: format!("{} arguments", args.len()),
        });
    }
    let lhs = eval(&args[0], env)?;
    let rhs = eval(&args[1], env)?;
    if let (Value::Int(a), Value::Int(b)) = (&lhs, &rhs) {
        Ok(Value::Bool(a < b))
    } else {
        let found = if matches!(lhs, Value::Int(_)) {
            value_kind(&rhs)
        } else {
            value_kind(&lhs)
        };
        Err(EvalError::TypeMismatch {
            expected: "int".to_string(),
            found: found.to_string(),
        })
    }
}

fn eval_eq(args: &[Term], env: &Environment) -> EvalResult {
    if args.len() != 2 {
        return Err(EvalError::TypeMismatch {
            expected: "2 arguments".to_string(),
            found: format!("{} arguments", args.len()),
        });
    }
    let lhs = eval(&args[0], env)?;
    let rhs = eval(&args[1], env)?;
    // eq is restricted to base-type operands; closures and constructors are
    // not comparable (ADR 0001 Decision 3).
    match (&lhs, &rhs) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(a == b)),
        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a == b)),
        (Value::Unit, Value::Unit) => Ok(Value::Bool(true)),
        (Value::Str(a), Value::Str(b)) => Ok(Value::Bool(a == b)),
        // Float comparison via to_bits: NaN == NaN is true; 0.0 != -0.0.
        // Both are intentional consequences of bit-equality (ADR 0001).
        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(a.to_bits() == b.to_bits())),
        (Value::Closure(_) | Value::RecClosure(_) | Value::Con(_, _), _)
        | (_, Value::Closure(_) | Value::RecClosure(_) | Value::Con(_, _)) => {
            Err(EvalError::NotComparable(value_kind(&lhs).to_string()))
        }
        // Mismatched base types — not equal rather than a hard error.
        _ => Ok(Value::Bool(false)),
    }
}

/// Evaluates the `strlen` built-in: returns the UTF-8 character count of a
/// [`Value::Str`] as an [`Value::Int`].
fn eval_strlen(args: &[Term], env: &Environment) -> EvalResult {
    if args.len() != 1 {
        return Err(EvalError::TypeMismatch {
            expected: "1 argument".to_string(),
            found: format!("{} arguments", args.len()),
        });
    }
    let v = eval(&args[0], env)?;
    match v {
        Value::Str(s) => {
            let n = i64::try_from(s.chars().count()).map_err(|_| EvalError::ArithmeticOverflow)?;
            Ok(Value::Int(n))
        }
        other => Err(EvalError::TypeMismatch {
            expected: "str".to_string(),
            found: value_kind(&other).to_string(),
        }),
    }
}

/// Maps a [`Value`] to a short type-tag string for error messages.
fn value_kind(v: &Value) -> &'static str {
    match v {
        Value::Int(_) => "int",
        Value::Bool(_) => "bool",
        Value::Unit => "unit",
        Value::Str(_) => "str",
        Value::Float(_) => "float",
        Value::Closure(_) => "closure",
        Value::RecClosure(_) => "recclosure",
        Value::Con(_, _) => "con",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_type::BaseType;
    use crate::builder::{app, bool_, con, int, lam, let_, letrec, str_, unit, var};

    fn env() -> Environment {
        Environment::new()
    }

    #[test]
    fn literal_int() {
        assert_eq!(eval(&int(42), &env()), Ok(Value::Int(42)));
    }

    #[test]
    fn literal_bool() {
        assert_eq!(eval(&bool_(true), &env()), Ok(Value::Bool(true)));
    }

    #[test]
    fn literal_unit() {
        assert_eq!(eval(&unit(), &env()), Ok(Value::Unit));
    }

    #[test]
    fn identity_application() -> Result<(), crate::error::BuildError> {
        let id = lam("x", BaseType::Int, var("x")?)?;
        let t = app(id, int(5));
        assert_eq!(eval(&t, &env()), Ok(Value::Int(5)));
        Ok(())
    }

    #[test]
    fn let_binding() -> Result<(), crate::error::BuildError> {
        let t = let_("x", int(7), var("x")?)?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(7)));
        Ok(())
    }

    #[test]
    fn if_true_branch() -> Result<(), crate::error::BuildError> {
        let t = con("if", vec![bool_(true), int(1), int(2)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(1)));
        Ok(())
    }

    #[test]
    fn if_false_branch() -> Result<(), crate::error::BuildError> {
        let t = con("if", vec![bool_(false), int(1), int(2)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(2)));
        Ok(())
    }

    #[test]
    fn if_lazy_untaken_branch() -> Result<(), crate::error::BuildError> {
        // The else branch references an unbound variable — eval must NOT touch it.
        let t = con("if", vec![bool_(true), int(42), var("nope")?])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(42)));
        Ok(())
    }

    #[test]
    fn arith_add() -> Result<(), crate::error::BuildError> {
        let t = con("+", vec![int(3), int(4)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(7)));
        Ok(())
    }

    #[test]
    fn arith_sub() -> Result<(), crate::error::BuildError> {
        let t = con("-", vec![int(10), int(4)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(6)));
        Ok(())
    }

    #[test]
    fn arith_mul() -> Result<(), crate::error::BuildError> {
        let t = con("*", vec![int(3), int(7)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(21)));
        Ok(())
    }

    #[test]
    fn arith_div() -> Result<(), crate::error::BuildError> {
        let t = con("/", vec![int(10), int(2)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(5)));
        Ok(())
    }

    #[test]
    fn division_by_zero() -> Result<(), crate::error::BuildError> {
        let t = con("/", vec![int(1), int(0)])?;
        assert_eq!(eval(&t, &env()), Err(EvalError::DivisionByZero));
        Ok(())
    }

    #[test]
    fn arithmetic_overflow() -> Result<(), crate::error::BuildError> {
        let t = con("+", vec![int(i64::MAX), int(1)])?;
        assert_eq!(eval(&t, &env()), Err(EvalError::ArithmeticOverflow));
        Ok(())
    }

    #[test]
    fn cmp_less_than_true() -> Result<(), crate::error::BuildError> {
        let t = con("<", vec![int(1), int(2)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Bool(true)));
        Ok(())
    }

    #[test]
    fn cmp_less_than_false() -> Result<(), crate::error::BuildError> {
        let t = con("<", vec![int(5), int(2)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Bool(false)));
        Ok(())
    }

    #[test]
    fn cmp_non_int_type_mismatch() -> Result<(), crate::error::BuildError> {
        let t = con("<", vec![bool_(true), int(1)])?;
        assert!(matches!(
            eval(&t, &env()),
            Err(EvalError::TypeMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn eq_equal_ints() -> Result<(), crate::error::BuildError> {
        let t = con("eq", vec![int(3), int(3)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Bool(true)));
        Ok(())
    }

    #[test]
    fn eq_unequal_ints() -> Result<(), crate::error::BuildError> {
        let t = con("eq", vec![int(3), int(4)])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Bool(false)));
        Ok(())
    }

    #[test]
    fn eq_float_nan_reflexive() {
        // NaN == NaN via to_bits is intentionally true (ADR 0001).
        let nan = f64::NAN;
        let args = vec![
            crate::term::Term::Lit(crate::term::Lit::Float(nan)),
            crate::term::Term::Lit(crate::term::Lit::Float(nan)),
        ];
        assert_eq!(eval_eq(&args, &Environment::new()), Ok(Value::Bool(true)));
    }

    #[test]
    fn eq_closure_not_comparable() -> Result<(), crate::error::BuildError> {
        let cl = lam("x", BaseType::Int, var("x")?)?;
        let t = con("eq", vec![cl, int(1)])?;
        assert!(matches!(eval(&t, &env()), Err(EvalError::NotComparable(_))));
        Ok(())
    }

    #[test]
    fn strlen_counts_utf8_chars() -> Result<(), crate::error::BuildError> {
        let t = con("strlen", vec![str_("héllo")])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(5)));
        Ok(())
    }

    #[test]
    fn strcat_joins_two_strings() -> Result<(), crate::error::BuildError> {
        let t = con("strcat", vec![str_("foo"), str_("bar")])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Str("foobar".to_string())));
        Ok(())
    }

    #[test]
    fn non_reserved_con() -> Result<(), crate::error::BuildError> {
        let t = con("Pair", vec![int(1), bool_(true)])?;
        assert_eq!(
            eval(&t, &env()),
            Ok(Value::Con(
                "Pair".to_string(),
                vec![Value::Int(1), Value::Bool(true)]
            ))
        );
        Ok(())
    }

    #[test]
    fn strlen_rejects_non_string() -> Result<(), crate::error::BuildError> {
        let t = con("strlen", vec![int(1)])?;
        assert!(matches!(
            eval(&t, &env()),
            Err(EvalError::TypeMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn strcat_rejects_wrong_arity() -> Result<(), crate::error::BuildError> {
        let t = con("strcat", vec![str_("x")])?;
        assert!(matches!(
            eval(&t, &env()),
            Err(EvalError::TypeMismatch { .. })
        ));
        Ok(())
    }

    #[test]
    fn strlen_name_is_not_bound() -> Result<(), crate::error::BuildError> {
        let t = var("strlen")?;
        let result = eval(&t, &env());
        assert!(matches!(result, Err(EvalError::UnboundVar(_))));
        Ok(())
    }

    #[test]
    fn unbound_variable() {
        let t = crate::term::Term::Var("x".to_string());
        assert_eq!(
            eval(&t, &env()),
            Err(EvalError::UnboundVar("x".to_string()))
        );
    }

    #[test]
    fn not_a_function() {
        let t = app(int(1), int(2));
        assert_eq!(eval(&t, &env()), Err(EvalError::NotAFunction));
    }

    /// Belt-and-suspenders guard: requires >=2 levels of recursion unrolling.
    /// A plain-Closure one-shot self-bind would fail here with UnboundVar("fac").
    #[test]
    fn recursion_depth_two_or_more() -> Result<(), crate::error::BuildError> {
        // letrec fac = \n:int. if (eq n 0) 1 (* n (fac (- n 1))) in fac 3
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
        let t = letrec("fac", fac_lam, app(var("fac")?, int(3)))?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(6)));
        Ok(())
    }
}
