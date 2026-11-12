//! Strict, call-by-value, environment-based tree-walking VM over [`Term`].
//!
//! Evaluates Arc B Core IR terms to runtime [`Value`]s. Branching and
//! arithmetic are encoded as reserved constructor names dispatched here
//! (ADR 0001 Decision 3 Option A). Recursion uses [`Value::RecClosure`]
//! re-bound at every application (ADR 0001 Decision 4). See
//! `docs/CORE_OPERATIONAL_SEMANTICS.md` and `docs/adr/0001-arc-c-eval-model.md`.

#[cfg(test)]
use crate::term::Lit;
use crate::term::Term;
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
    pub(crate) param_ty: crate::ty::Type,
    pub(crate) body: Box<Term>,
    pub(crate) env: Environment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecClosureData {
    pub(crate) name: String,
    pub(crate) param: String,
    pub(crate) param_ty: crate::ty::Type,
    pub(crate) ann: crate::ty::Type,
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
    /// A Unicode scalar value (a code point), matching Haskell's `Char`.
    Char(char),
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

#[derive(Debug)]
struct Scope {
    name: String,
    value: Value,
    parent: Option<std::rc::Rc<Scope>>,
}

/// A persistent environment mapping names to [`Value`]s, represented as an
/// `Rc` chain of [`Scope`] nodes so that [`extend`](Environment::extend) is
/// O(1) and shares structure with the parent (roadmap Amendment 1: no
/// whole-map clone).
#[derive(Debug, Clone, Default)]
pub struct Environment {
    head: Option<std::rc::Rc<Scope>>,
}

impl PartialEq for Environment {
    fn eq(&self, other: &Self) -> bool {
        self.visible_bindings() == other.visible_bindings()
    }
}

impl Environment {
    /// Creates an empty environment.
    #[must_use]
    pub fn new() -> Self {
        Self { head: None }
    }

    /// Looks up `name` in the environment.
    #[must_use]
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        let mut current = &self.head;
        while let Some(scope) = current {
            if scope.name == name {
                return Some(&scope.value);
            }
            current = &scope.parent;
        }
        None
    }

    /// Returns a new environment with `name` bound to `value`.
    #[must_use]
    pub fn extend(&self, name: &str, value: Value) -> Self {
        Self {
            head: Some(std::rc::Rc::new(Scope {
                name: name.to_string(),
                value,
                parent: self.head.clone(),
            })),
        }
    }

    fn visible_bindings(&self) -> std::collections::BTreeMap<&str, &Value> {
        let mut bindings = std::collections::BTreeMap::new();
        let mut current = &self.head;
        while let Some(scope) = current {
            bindings.entry(scope.name.as_str()).or_insert(&scope.value);
            current = &scope.parent;
        }
        bindings
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
    /// A `Case` scrutinee's constructor matched no alternative.
    NoMatchingAlt(String),
    /// The machine exhausted its configured transition limit.
    StepLimit,
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
            Self::NoMatchingAlt(con) => {
                write!(f, "no matching alternative for constructor {con}")
            }
            Self::StepLimit => write!(f, "step limit exceeded"),
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
/// zero, arithmetic overflow, or application of a non-function. Runs
/// unbounded, replacing native-stack recursion.
pub fn eval(term: &Term, env: &Environment) -> EvalResult {
    crate::machine::run(term, env, None)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_type::BaseType;
    use crate::builder::{
        app, base_ty, bool_, char_, con, fun_ty, int, lam, let_, letrec, unit, var,
    };

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
        let term = Term::Con(
            "eq".to_string(),
            vec![],
            vec![
                Term::Lit(Lit::Float(f64::NAN)),
                Term::Lit(Lit::Float(f64::NAN)),
            ],
        );
        assert_eq!(eval(&term, &Environment::new()), Ok(Value::Bool(true)));
    }

    #[test]
    fn eq_closure_not_comparable() -> Result<(), crate::error::BuildError> {
        let cl = lam("x", BaseType::Int, var("x")?)?;
        let t = con("eq", vec![cl, int(1)])?;
        assert!(matches!(eval(&t, &env()), Err(EvalError::NotComparable(_))));
        Ok(())
    }

    #[test]
    fn literal_char() {
        assert_eq!(eval(&char_('a'), &env()), Ok(Value::Char('a')));
    }

    #[test]
    fn char_eq_true() -> Result<(), crate::error::BuildError> {
        let t = con("eq", vec![char_('a'), char_('a')])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Bool(true)));
        Ok(())
    }

    #[test]
    fn char_cmp_less_than() -> Result<(), crate::error::BuildError> {
        let t = con("<", vec![char_('a'), char_('b')])?;
        assert_eq!(eval(&t, &env()), Ok(Value::Bool(true)));
        Ok(())
    }

    #[test]
    fn eq_mismatched_base_is_type_error() -> Result<(), crate::error::BuildError> {
        // Lint-clean Core never compares mismatched base types (slice 6e), so
        // eval rejects them as a hard type error instead of returning false.
        let t = con("eq", vec![int(1), bool_(true)])?;
        assert!(matches!(
            eval(&t, &env()),
            Err(EvalError::TypeMismatch { .. })
        ));
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
    fn strlen_is_ordinary_con() -> Result<(), crate::error::BuildError> {
        // After slice 6a this name is no longer reserved: a Con with that
        // name is an ordinary constructor value.
        let t = con("strlen", vec![char_('a')])?;
        assert_eq!(
            eval(&t, &env()),
            Ok(Value::Con("strlen".to_string(), vec![Value::Char('a')]))
        );
        Ok(())
    }

    #[test]
    fn strcat_is_ordinary_con() -> Result<(), crate::error::BuildError> {
        // After slice 6a this name is no longer reserved: a Con with that
        // name is an ordinary constructor value.
        let t = con("strcat", vec![char_('a'), char_('b')])?;
        assert_eq!(
            eval(&t, &env()),
            Ok(Value::Con(
                "strcat".to_string(),
                vec![Value::Char('a'), Value::Char('b')]
            ))
        );
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
        let fac_ty = fun_ty(base_ty(BaseType::Int), base_ty(BaseType::Int));
        let t = letrec("fac", fac_ty, fac_lam, app(var("fac")?, int(3)))?;
        assert_eq!(eval(&t, &env()), Ok(Value::Int(6)));
        Ok(())
    }

    #[test]
    fn tylam_tyapp_are_erased() -> Result<(), crate::error::BuildError> {
        use crate::builder::{tvar, tyapp, tylam};
        // /\\a. \\x: a. x  instantiated at Int applied to 5, and at Bool to true.
        let poly = tylam("a", lam("x", tvar("a")?, var("x")?)?)?;
        let at_int = app(tyapp(poly.clone(), base_ty(BaseType::Int)), int(5));
        assert_eq!(eval(&at_int, &env()), Ok(Value::Int(5)));
        let at_bool = app(tyapp(poly, base_ty(BaseType::Bool)), bool_(true));
        assert_eq!(eval(&at_bool, &env()), Ok(Value::Bool(true)));
        Ok(())
    }

    #[test]
    fn typed_con_evaluates_and_ignores_ty_args() -> Result<(), crate::error::BuildError> {
        use crate::builder::con_at;
        let t = con_at("Just", vec![base_ty(BaseType::Int)], vec![int(7)])?;
        assert_eq!(
            eval(&t, &env()),
            Ok(Value::Con("Just".to_string(), vec![Value::Int(7)]))
        );
        Ok(())
    }
}
