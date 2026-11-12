//! A CEK abstract machine for strict Core evaluation.

use crate::eval::{ClosureData, Environment, EvalError, RecClosureData, Value};
use crate::term::{Alt, Lit, Term};

/// The current term being evaluated or value being returned.
#[derive(Debug, Clone)]
pub enum Control {
    /// Evaluate a Core term.
    Eval(Term),
    /// Return a computed value to the continuation.
    Return(Value),
}

/// A defunctionalized continuation frame.
#[derive(Debug, Clone)]
pub enum Frame {
    /// Evaluate an application argument after its function.
    AppArg(Term, Environment),
    /// Apply a function value to the returned argument.
    AppCall(Value),
    /// Bind a `let` value before evaluating its body.
    LetBody(String, Term, Environment),
    /// Select a conditional branch after evaluating its condition.
    IfBranch(Term, Term, Environment),
    /// Accumulate primitive arguments left to right.
    PrimArgs(String, Vec<Value>, Vec<Term>, Environment),
    /// Accumulate constructor arguments left to right.
    ConArgs(String, Vec<Value>, Vec<Term>, Environment),
    /// Select an alternative after evaluating a case scrutinee.
    CaseAlts(Vec<Alt>, Environment),
}

/// A complete CEK machine state.
#[derive(Debug, Clone)]
pub struct State {
    /// The current control component.
    pub control: Control,
    /// The current environment component.
    pub env: Environment,
    /// The continuation stack.
    pub kont: Vec<Frame>,
}

impl State {
    fn new(term: &Term, env: &Environment) -> Self {
        Self {
            control: Control::Eval(term.clone()),
            env: env.clone(),
            kont: Vec::new(),
        }
    }
}

/// The semantic kind of one machine transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// Resolve a variable from the environment.
    Lookup,
    /// Construct a runtime value.
    Construct,
    /// Push a continuation frame.
    Push,
    /// Apply a non-recursive closure.
    Beta,
    /// Apply a recursive closure and rebind its name.
    Unfold,
    /// Apply a primitive operation.
    Delta,
    /// Select a conditional branch.
    Branch,
    /// Select a case alternative and bind its fields.
    CaseSelect,
    /// Enter a `let` or `letrec` body.
    LetBind,
    /// Erase a type abstraction or application.
    Erase,
}

impl std::fmt::Display for StepKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Lookup => "lookup",
            Self::Construct => "construct",
            Self::Push => "push",
            Self::Beta => "beta",
            Self::Unfold => "unfold",
            Self::Delta => "delta",
            Self::Branch => "branch",
            Self::CaseSelect => "caseselect",
            Self::LetBind => "letbind",
            Self::Erase => "erase",
        };
        write!(formatter, "{name}")
    }
}

impl std::fmt::Display for Control {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Eval(term) => write!(formatter, "eval {term}"),
            Self::Return(value) => write!(formatter, "return {value}"),
        }
    }
}

impl std::fmt::Display for Frame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AppArg(argument, _) => write!(formatter, "AppArg({argument})"),
            Self::AppCall(value) => write!(formatter, "AppCall({value})"),
            Self::LetBody(name, body, _) => write!(formatter, "LetBody({name}, {body})"),
            Self::IfBranch(then_term, else_term, _) => {
                write!(formatter, "IfBranch({then_term}, {else_term})")
            }
            Self::PrimArgs(op, done, rest, _) => write!(
                formatter,
                "PrimArgs({op}, [{}], [{}])",
                join_values(done),
                join_terms(rest)
            ),
            Self::ConArgs(name, done, rest, _) => write!(
                formatter,
                "ConArgs({name}, [{}], [{}])",
                join_values(done),
                join_terms(rest)
            ),
            Self::CaseAlts(alts, _) => write!(formatter, "CaseAlts({} alts)", alts.len()),
        }
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let frames = self
            .kont
            .iter()
            .rev()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        write!(formatter, "<{} | kont: [{frames}]>", self.control)
    }
}

fn join_values(values: &[Value]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn join_terms(terms: &[Term]) -> String {
    terms
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The result of one machine transition.
#[derive(Debug, Clone)]
pub enum StepOutcome {
    /// The machine made a transition and can continue.
    Continue(StepKind),
    /// The machine completed with a value.
    Done(Value),
}

/// A mutable CEK machine.
#[derive(Debug, Clone)]
pub struct Machine {
    state: State,
}

impl Machine {
    /// Creates a machine that evaluates `term` under `env`.
    #[must_use]
    pub fn new(term: &Term, env: &Environment) -> Self {
        Self {
            state: State::new(term, env),
        }
    }

    /// Returns the current machine state.
    #[must_use]
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Advances the machine by one transition.
    ///
    /// # Errors
    /// Returns an [`EvalError`] when the current transition is invalid.
    pub fn step(&mut self) -> Result<StepOutcome, EvalError> {
        match self.state.control.clone() {
            Control::Eval(term) => self.step_eval(term),
            Control::Return(value) => self.step_return(value),
        }
    }

    fn step_eval(&mut self, term: Term) -> Result<StepOutcome, EvalError> {
        match term {
            Term::Var(name) => self.eval_var(name),
            Term::Lit(literal) => {
                self.state.control = Control::Return(value_from_lit(&literal));
                Ok(StepOutcome::Continue(StepKind::Construct))
            }
            Term::Lam(param, param_ty, body) => Ok(self.eval_lam(param, param_ty, body)),
            Term::App(function, argument) => Ok(self.eval_app(*function, *argument)),
            Term::Let(name, value, body) => Ok(self.eval_let(name, *value, *body)),
            Term::LetRec(name, ann, value, body) => self.eval_letrec(name, ann, *value, *body),
            Term::Con(name, _ty_args, args) => self.eval_con(name, &args),
            Term::TyLam(_, inner) | Term::TyApp(inner, _) => Ok(self.eval_erased(*inner)),
            Term::Case(scrutinee, alts) => Ok(self.eval_case(*scrutinee, alts)),
        }
    }

    fn step_return(&mut self, value: Value) -> Result<StepOutcome, EvalError> {
        let Some(frame) = self.state.kont.pop() else {
            return Ok(StepOutcome::Done(value));
        };
        match frame {
            Frame::AppArg(argument, env) => Ok(self.return_app_arg(value, argument, env)),
            Frame::AppCall(callee) => self.return_app_call(value, callee),
            Frame::LetBody(name, body, env) => Ok(self.return_let(value, &name, body, &env)),
            Frame::IfBranch(then_term, else_term, env) => {
                self.return_if(value, then_term, else_term, env)
            }
            Frame::PrimArgs(op, done, rest, env) => self.return_prim(value, op, done, rest, env),
            Frame::ConArgs(name, done, rest, env) => {
                Ok(self.return_con(value, name, done, rest, env))
            }
            Frame::CaseAlts(alts, env) => self.return_case(value, alts, env),
        }
    }

    fn eval_var(&mut self, name: String) -> Result<StepOutcome, EvalError> {
        let value = self
            .state
            .env
            .lookup(&name)
            .cloned()
            .ok_or(EvalError::UnboundVar(name))?;
        self.state.control = Control::Return(value);
        Ok(StepOutcome::Continue(StepKind::Lookup))
    }

    fn eval_lam(
        &mut self,
        param: String,
        param_ty: crate::ty::Type,
        body: Box<Term>,
    ) -> StepOutcome {
        self.state.control = Control::Return(Value::Closure(Box::new(ClosureData {
            param,
            param_ty,
            body,
            env: self.state.env.clone(),
        })));
        StepOutcome::Continue(StepKind::Construct)
    }

    fn eval_app(&mut self, function: Term, argument: Term) -> StepOutcome {
        self.state
            .kont
            .push(Frame::AppArg(argument, self.state.env.clone()));
        self.state.control = Control::Eval(function);
        StepOutcome::Continue(StepKind::Push)
    }

    fn eval_let(&mut self, name: String, value: Term, body: Term) -> StepOutcome {
        self.state
            .kont
            .push(Frame::LetBody(name, body, self.state.env.clone()));
        self.state.control = Control::Eval(value);
        StepOutcome::Continue(StepKind::Push)
    }

    fn eval_letrec(
        &mut self,
        name: String,
        ann: crate::ty::Type,
        value: Term,
        body: Term,
    ) -> Result<StepOutcome, EvalError> {
        let Term::Lam(param, param_ty, lambda_body) = value else {
            return Err(EvalError::TypeMismatch {
                expected: "lambda".to_string(),
                found: "non-lambda".to_string(),
            });
        };
        let binding_name = name.clone();
        let rec = Value::RecClosure(Box::new(RecClosureData {
            name,
            param,
            param_ty,
            ann,
            body: lambda_body,
            env: self.state.env.clone(),
        }));
        self.state.env = self.state.env.extend(&binding_name, rec);
        self.state.control = Control::Eval(body);
        Ok(StepOutcome::Continue(StepKind::LetBind))
    }

    fn eval_con(&mut self, name: String, args: &[Term]) -> Result<StepOutcome, EvalError> {
        match name.as_str() {
            "if" => self.eval_if(args),
            "+" | "-" | "*" | "/" | "<" | "eq" => self.eval_prim(name, args),
            _ => Ok(self.eval_constructor(name, args)),
        }
    }

    fn eval_if(&mut self, args: &[Term]) -> Result<StepOutcome, EvalError> {
        if args.len() != 3 {
            return Err(arity_error(3, args.len()));
        }
        self.state.kont.push(Frame::IfBranch(
            args[1].clone(),
            args[2].clone(),
            self.state.env.clone(),
        ));
        self.state.control = Control::Eval(args[0].clone());
        Ok(StepOutcome::Continue(StepKind::Push))
    }

    fn eval_prim(&mut self, op: String, args: &[Term]) -> Result<StepOutcome, EvalError> {
        if args.len() != 2 {
            return Err(arity_error(2, args.len()));
        }
        self.state.kont.push(Frame::PrimArgs(
            op,
            Vec::new(),
            args[1..].to_vec(),
            self.state.env.clone(),
        ));
        self.state.control = Control::Eval(args[0].clone());
        Ok(StepOutcome::Continue(StepKind::Push))
    }

    fn eval_constructor(&mut self, name: String, args: &[Term]) -> StepOutcome {
        if args.is_empty() {
            self.state.control = Control::Return(Value::Con(name, Vec::new()));
            return StepOutcome::Continue(StepKind::Construct);
        }
        self.state.kont.push(Frame::ConArgs(
            name,
            Vec::new(),
            args[1..].to_vec(),
            self.state.env.clone(),
        ));
        self.state.control = Control::Eval(args[0].clone());
        StepOutcome::Continue(StepKind::Push)
    }

    fn eval_erased(&mut self, inner: Term) -> StepOutcome {
        self.state.control = Control::Eval(inner);
        StepOutcome::Continue(StepKind::Erase)
    }

    fn eval_case(&mut self, scrutinee: Term, alts: Vec<Alt>) -> StepOutcome {
        self.state
            .kont
            .push(Frame::CaseAlts(alts, self.state.env.clone()));
        self.state.control = Control::Eval(scrutinee);
        StepOutcome::Continue(StepKind::Push)
    }

    fn return_app_arg(&mut self, value: Value, argument: Term, env: Environment) -> StepOutcome {
        self.state.kont.push(Frame::AppCall(value));
        self.state.env = env;
        self.state.control = Control::Eval(argument);
        StepOutcome::Continue(StepKind::Push)
    }

    fn return_app_call(&mut self, value: Value, callee: Value) -> Result<StepOutcome, EvalError> {
        match callee {
            Value::Closure(closure) => Ok(self.apply_closure(value, *closure)),
            Value::RecClosure(closure) => Ok(self.apply_rec_closure(value, closure)),
            _ => Err(EvalError::NotAFunction),
        }
    }

    fn apply_closure(&mut self, value: Value, closure: ClosureData) -> StepOutcome {
        self.state.env = closure.env.extend(&closure.param, value);
        self.state.control = Control::Eval(*closure.body);
        StepOutcome::Continue(StepKind::Beta)
    }

    fn apply_rec_closure(&mut self, value: Value, closure: Box<RecClosureData>) -> StepOutcome {
        let recursive = Value::RecClosure(closure.clone());
        self.state.env = closure
            .env
            .extend(&closure.name, recursive)
            .extend(&closure.param, value);
        self.state.control = Control::Eval(*closure.body);
        StepOutcome::Continue(StepKind::Unfold)
    }

    fn return_let(
        &mut self,
        value: Value,
        name: &str,
        body: Term,
        env: &Environment,
    ) -> StepOutcome {
        self.state.env = env.extend(name, value);
        self.state.control = Control::Eval(body);
        StepOutcome::Continue(StepKind::LetBind)
    }

    fn return_if(
        &mut self,
        value: Value,
        then_term: Term,
        else_term: Term,
        env: Environment,
    ) -> Result<StepOutcome, EvalError> {
        let branch = match value {
            Value::Bool(true) => then_term,
            Value::Bool(false) => else_term,
            other => {
                return Err(EvalError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: value_kind(&other).to_string(),
                });
            }
        };
        self.state.env = env;
        self.state.control = Control::Eval(branch);
        Ok(StepOutcome::Continue(StepKind::Branch))
    }

    fn return_prim(
        &mut self,
        value: Value,
        op: String,
        mut done: Vec<Value>,
        mut rest: Vec<Term>,
        env: Environment,
    ) -> Result<StepOutcome, EvalError> {
        done.push(value);
        if rest.is_empty() {
            self.state.control = Control::Return(apply_prim(&op, &done)?);
            return Ok(StepOutcome::Continue(StepKind::Delta));
        }
        let next = rest.remove(0);
        self.state
            .kont
            .push(Frame::PrimArgs(op, done, rest, env.clone()));
        self.state.env = env;
        self.state.control = Control::Eval(next);
        Ok(StepOutcome::Continue(StepKind::Push))
    }

    fn return_con(
        &mut self,
        value: Value,
        name: String,
        mut done: Vec<Value>,
        mut rest: Vec<Term>,
        env: Environment,
    ) -> StepOutcome {
        done.push(value);
        if rest.is_empty() {
            self.state.control = Control::Return(Value::Con(name, done));
            return StepOutcome::Continue(StepKind::Construct);
        }
        let next = rest.remove(0);
        self.state
            .kont
            .push(Frame::ConArgs(name, done, rest, env.clone()));
        self.state.env = env;
        self.state.control = Control::Eval(next);
        StepOutcome::Continue(StepKind::Push)
    }

    fn return_case(
        &mut self,
        value: Value,
        alts: Vec<Alt>,
        env: Environment,
    ) -> Result<StepOutcome, EvalError> {
        let Value::Con(name, args) = value else {
            return Err(EvalError::TypeMismatch {
                expected: "constructor value".to_string(),
                found: value_kind(&value).to_string(),
            });
        };
        let Some(alt) = alts.into_iter().find(|alt| alt.con == name) else {
            return Err(EvalError::NoMatchingAlt(name));
        };
        if alt.binders.len() != args.len() {
            return Err(EvalError::TypeMismatch {
                expected: format!("{} constructor fields", alt.binders.len()),
                found: format!("{} fields", args.len()),
            });
        }
        let mut call_env = env;
        for (binder, arg) in alt.binders.iter().zip(args) {
            call_env = call_env.extend(binder, arg);
        }
        self.state.env = call_env;
        self.state.control = Control::Eval(alt.body);
        Ok(StepOutcome::Continue(StepKind::CaseSelect))
    }
}

/// Runs the machine to completion, or until `fuel` steps elapse.
///
/// `fuel` of `None` runs unbounded (the historical big-step semantics, minus
/// native-stack overflow). `Some(n)` performs at most `n` transitions and
/// returns [`EvalError::StepLimit`] if no value is reached.
///
/// # Errors
/// Propagates any [`EvalError`] raised by a transition, and returns
/// [`EvalError::StepLimit`] when `fuel` is exhausted.
pub fn run(term: &Term, env: &Environment, fuel: Option<usize>) -> Result<Value, EvalError> {
    let mut machine = Machine::new(term, env);
    let mut remaining = fuel;
    loop {
        if matches!(remaining, Some(0)) {
            return Err(EvalError::StepLimit);
        }
        if let Some(steps) = &mut remaining {
            *steps -= 1;
        }
        match machine.step()? {
            StepOutcome::Continue(_) => {}
            StepOutcome::Done(value) => return Ok(value),
        }
    }
}

fn value_from_lit(literal: &Lit) -> Value {
    match literal {
        Lit::Int(value) => Value::Int(*value),
        Lit::Bool(value) => Value::Bool(*value),
        Lit::Float(value) => Value::Float(*value),
        Lit::Unit => Value::Unit,
        Lit::Char(value) => Value::Char(*value),
    }
}

fn arity_error(expected: usize, actual: usize) -> EvalError {
    EvalError::TypeMismatch {
        expected: format!("{expected} arguments"),
        found: format!("{actual} arguments"),
    }
}

fn apply_prim(op: &str, values: &[Value]) -> Result<Value, EvalError> {
    match op {
        "+" | "-" | "*" | "/" => apply_arith(op, values),
        "<" => apply_cmp(values),
        "eq" => apply_eq(values),
        _ => Err(EvalError::TypeMismatch {
            expected: "primitive".to_string(),
            found: op.to_string(),
        }),
    }
}

fn apply_arith(op: &str, values: &[Value]) -> Result<Value, EvalError> {
    let [lhs, rhs] = values else {
        return Err(arity_error(2, values.len()));
    };
    let (Value::Int(lhs), Value::Int(rhs)) = (lhs, rhs) else {
        let found = if matches!(lhs, Value::Int(_)) {
            value_kind(rhs)
        } else {
            value_kind(lhs)
        };
        return Err(EvalError::TypeMismatch {
            expected: "int".to_string(),
            found: found.to_string(),
        });
    };
    let result = match op {
        "+" => lhs.checked_add(*rhs),
        "-" => lhs.checked_sub(*rhs),
        "*" => lhs.checked_mul(*rhs),
        "/" => {
            if *rhs == 0 {
                return Err(EvalError::DivisionByZero);
            }
            lhs.checked_div(*rhs)
        }
        _ => {
            return Err(EvalError::TypeMismatch {
                expected: "primitive".to_string(),
                found: op.to_string(),
            });
        }
    };
    result.map(Value::Int).ok_or(EvalError::ArithmeticOverflow)
}

fn apply_cmp(values: &[Value]) -> Result<Value, EvalError> {
    let [lhs, rhs] = values else {
        return Err(arity_error(2, values.len()));
    };
    match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Ok(Value::Bool(lhs < rhs)),
        (Value::Char(lhs), Value::Char(rhs)) => Ok(Value::Bool(lhs < rhs)),
        _ => {
            let found = if matches!(lhs, Value::Int(_)) {
                value_kind(rhs)
            } else {
                value_kind(lhs)
            };
            Err(EvalError::TypeMismatch {
                expected: "int".to_string(),
                found: found.to_string(),
            })
        }
    }
}

fn apply_eq(values: &[Value]) -> Result<Value, EvalError> {
    let [lhs, rhs] = values else {
        return Err(arity_error(2, values.len()));
    };
    match (lhs, rhs) {
        (Value::Int(lhs), Value::Int(rhs)) => Ok(Value::Bool(lhs == rhs)),
        (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs == rhs)),
        (Value::Unit, Value::Unit) => Ok(Value::Bool(true)),
        (Value::Char(lhs), Value::Char(rhs)) => Ok(Value::Bool(lhs == rhs)),
        (Value::Float(lhs), Value::Float(rhs)) => Ok(Value::Bool(lhs.to_bits() == rhs.to_bits())),
        (Value::Closure(_) | Value::RecClosure(_) | Value::Con(_, _), _)
        | (_, Value::Closure(_) | Value::RecClosure(_) | Value::Con(_, _)) => {
            Err(EvalError::NotComparable(value_kind(lhs).to_string()))
        }
        _ => Err(EvalError::TypeMismatch {
            expected: value_kind(lhs).to_string(),
            found: value_kind(rhs).to_string(),
        }),
    }
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Int(_) => "int",
        Value::Bool(_) => "bool",
        Value::Unit => "unit",
        Value::Char(_) => "char",
        Value::Float(_) => "float",
        Value::Closure(_) => "closure",
        Value::RecClosure(_) => "recclosure",
        Value::Con(_, _) => "con",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_type::BaseType;
    use crate::builder::{
        alt, app, base_ty, bool_, case_, char_, con, con_at, int, lam, let_, letrec, tvar, tyapp,
        tylam, unit, var,
    };

    #[test]
    fn displays_machine_state_and_step_kind() -> Result<(), crate::error::BuildError> {
        assert_eq!(StepKind::Beta.to_string(), "beta");
        let term = app(
            lam("x", BaseType::Int, con("+", vec![var("x")?, int(1)])?)?,
            int(2),
        );
        let mut machine = Machine::new(&term, &Environment::new());
        let initial = machine.state().to_string();
        assert!(initial.contains("eval"));
        assert!(initial.contains("kont: []"));
        assert!(matches!(machine.step(), Ok(StepOutcome::Continue(_))));
        assert!(machine.state().to_string().contains("AppArg(2)"));
        Ok(())
    }

    #[test]
    fn machine_matches_reference_smoke() -> Result<(), crate::error::BuildError> {
        let env = Environment::new();
        let case_term = case_(
            con("Cons", vec![int(1), con("Nil", vec![])?])?,
            vec![
                alt("Cons", &["h", "t"], var("h")?)?,
                alt("Nil", &[], int(0))?,
            ],
        );
        let terms = vec![
            con("if", vec![bool_(true), int(1), int(2)])?,
            con("+", vec![int(2), int(3)])?,
            con("eq", vec![int(1), int(1)])?,
            con("<", vec![int(1), int(2)])?,
            case_term,
        ];
        for term in terms {
            assert_eq!(
                run(&term, &env, None),
                crate::eval::reference_eval(&term, &env)
            );
        }
        Ok(())
    }

    fn factorial_term(input: i64) -> Result<Term, crate::error::BuildError> {
        let is_zero = con("eq", vec![var("n")?, int(0)])?;
        let decrement = con("-", vec![var("n")?, int(1)])?;
        let recursive_call = app(var("fac")?, decrement);
        let product = con("*", vec![var("n")?, recursive_call])?;
        let body = con("if", vec![is_zero, int(1), product])?;
        let function = lam("n", BaseType::Int, body)?;
        let call = app(var("fac")?, int(input));
        letrec("fac", base_ty(BaseType::Int), function, call)
    }

    fn countdown_term(input: i64) -> Result<Term, crate::error::BuildError> {
        let is_zero = con("eq", vec![var("n")?, int(0)])?;
        let decrement = con("-", vec![var("n")?, int(1)])?;
        let recursive_call = app(var("f")?, decrement);
        let body = con("if", vec![is_zero, int(0), recursive_call])?;
        let function = lam("n", BaseType::Int, body)?;
        let call = app(var("f")?, int(input));
        letrec("f", base_ty(BaseType::Int), function, call)
    }

    fn sum_term(input: i64) -> Result<Term, crate::error::BuildError> {
        let is_zero = con("eq", vec![var("n")?, int(0)])?;
        let decrement = con("-", vec![var("n")?, int(1)])?;
        let recursive_call = app(var("s")?, decrement);
        let sum = con("+", vec![var("n")?, recursive_call])?;
        let body = con("if", vec![is_zero, int(0), sum])?;
        let function = lam("n", BaseType::Int, body)?;
        let call = app(var("s")?, int(input));
        letrec("s", base_ty(BaseType::Int), function, call)
    }

    fn divergent_term() -> Result<Term, crate::error::BuildError> {
        let body = app(var("f")?, var("x")?);
        let function = lam("x", BaseType::Int, body)?;
        let call = app(var("f")?, int(0));
        letrec("f", base_ty(BaseType::Int), function, call)
    }

    fn cases() -> Result<Vec<(Term, Environment)>, crate::error::BuildError> {
        let identity = app(lam("x", BaseType::Int, var("x")?)?, int(5));
        let let_binding = let_("x", int(7), var("x")?)?;
        let lazy_if = con("if", vec![bool_(true), int(42), var("nope")?])?;
        let pair = con("Pair", vec![int(1), bool_(true)])?;
        let nil = con("Nil", vec![])?;
        let case_match = case_(
            con("Cons", vec![int(1), con("Nil", vec![])?])?,
            vec![
                alt("Cons", &["h", "t"], var("h")?)?,
                alt("Nil", &[], int(0))?,
            ],
        );
        let case_missing = case_(con("Nil", vec![])?, vec![alt("Cons", &["h", "t"], int(0))?]);
        let case_arity = case_(
            con("Cons", vec![int(1), con("Nil", vec![])?])?,
            vec![alt("Cons", &["only"], int(0))?],
        );
        let erased = app(
            tyapp(
                tylam("a", lam("x", tvar("a")?, var("x")?)?)?,
                base_ty(BaseType::Int),
            ),
            int(5),
        );
        let typed_constructor = con_at("Just", vec![base_ty(BaseType::Int)], vec![int(7)])?;
        let terms = vec![
            int(42),
            bool_(true),
            unit(),
            char_('a'),
            Term::Lit(Lit::Float(1.5)),
            identity,
            let_binding,
            lazy_if,
            con("+", vec![int(10), int(2)])?,
            con("-", vec![int(10), int(2)])?,
            con("*", vec![int(10), int(2)])?,
            con("/", vec![int(10), int(2)])?,
            con("/", vec![int(1), int(0)])?,
            con("+", vec![int(i64::MAX), int(1)])?,
            con("<", vec![int(1), int(2)])?,
            con("<", vec![char_('a'), char_('b')])?,
            con("eq", vec![int(3), int(3)])?,
            con("eq", vec![lam("x", BaseType::Int, var("x")?)?, int(1)])?,
            con("eq", vec![int(1), bool_(true)])?,
            pair,
            nil,
            case_match,
            case_missing,
            case_arity,
            app(int(1), int(2)),
            var("x")?,
            letrec("f", base_ty(BaseType::Int), int(1), int(0))?,
            erased,
            typed_constructor,
            factorial_term(5)?,
            con("+", vec![int(1)])?,
            con("if", vec![bool_(true)])?,
        ];
        Ok(terms
            .into_iter()
            .map(|term| (term, Environment::new()))
            .collect())
    }

    #[test]
    fn machine_matches_reference() -> Result<(), crate::error::BuildError> {
        for (t, e) in cases()? {
            assert_eq!(run(&t, &e, None), crate::eval::reference_eval(&t, &e));
        }
        Ok(())
    }

    #[test]
    fn deep_countdown_20k() -> Result<(), crate::error::BuildError> {
        let term = countdown_term(20_000)?;
        assert_eq!(run(&term, &Environment::new(), None), Ok(Value::Int(0)));
        Ok(())
    }

    #[test]
    fn deep_nontail_sum_5k() -> Result<(), crate::error::BuildError> {
        let term = sum_term(5_000)?;
        assert_eq!(
            run(&term, &Environment::new(), None),
            Ok(Value::Int(12_502_500))
        );
        Ok(())
    }

    #[test]
    fn step_limit_diverges() -> Result<(), crate::error::BuildError> {
        let term = divergent_term()?;
        assert_eq!(
            run(&term, &Environment::new(), Some(1_000)),
            Err(EvalError::StepLimit)
        );
        Ok(())
    }

    #[test]
    fn no_limit_still_terminates() -> Result<(), crate::error::BuildError> {
        let term = factorial_term(5)?;
        assert_eq!(run(&term, &Environment::new(), None), Ok(Value::Int(120)));
        Ok(())
    }
}
