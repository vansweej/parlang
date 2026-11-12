//! Read-back from CEK machine state to Core [`Term`]s.

use crate::eval::{Environment, Value};
use crate::machine::{Control, Frame};
use crate::term::{Alt, Lit, Term};

fn value_to_term(value: &Value) -> Term {
    match value {
        Value::Int(number) => Term::Lit(Lit::Int(*number)),
        Value::Bool(boolean) => Term::Lit(Lit::Bool(*boolean)),
        Value::Float(number) => Term::Lit(Lit::Float(*number)),
        Value::Unit => Term::Lit(Lit::Unit),
        Value::Char(character) => Term::Lit(Lit::Char(*character)),
        Value::Con(name, values) => Term::Con(
            name.clone(),
            Vec::new(),
            values.iter().map(value_to_term).collect(),
        ),
        Value::Closure(closure) => Term::Lam(
            closure.param.clone(),
            closure.param_ty.clone(),
            Box::new(subst_env(
                &closure.body,
                &closure.env,
                &mut vec![closure.param.clone()],
            )),
        ),
        Value::RecClosure(closure) => Term::LetRec(
            closure.name.clone(),
            closure.ann.clone(),
            Box::new(Term::Lam(
                closure.param.clone(),
                closure.param_ty.clone(),
                Box::new(subst_env(
                    &closure.body,
                    &closure.env,
                    &mut vec![closure.name.clone(), closure.param.clone()],
                )),
            )),
            Box::new(Term::Var(closure.name.clone())),
        ),
    }
}

fn subst_env(term: &Term, env: &Environment, shadowed: &mut Vec<String>) -> Term {
    match term {
        Term::Var(name) if shadowed.contains(name) => Term::Var(name.clone()),
        Term::Var(name) => env
            .lookup(name)
            .map_or_else(|| Term::Var(name.clone()), value_to_term),
        Term::Lam(name, ty, body) => Term::Lam(
            name.clone(),
            ty.clone(),
            Box::new(subst_under_binders(
                body,
                env,
                shadowed,
                std::slice::from_ref(name),
            )),
        ),
        Term::App(function, argument) => Term::App(
            Box::new(subst_env(function, env, shadowed)),
            Box::new(subst_env(argument, env, shadowed)),
        ),
        Term::Let(name, value, body) => Term::Let(
            name.clone(),
            Box::new(subst_env(value, env, shadowed)),
            Box::new(subst_under_binders(
                body,
                env,
                shadowed,
                std::slice::from_ref(name),
            )),
        ),
        Term::LetRec(name, ann, value, body) => Term::LetRec(
            name.clone(),
            ann.clone(),
            Box::new(subst_under_binders(
                value,
                env,
                shadowed,
                std::slice::from_ref(name),
            )),
            Box::new(subst_under_binders(
                body,
                env,
                shadowed,
                std::slice::from_ref(name),
            )),
        ),
        Term::Lit(literal) => Term::Lit(literal.clone()),
        Term::Con(name, ty_args, args) => Term::Con(
            name.clone(),
            ty_args.clone(),
            args.iter()
                .map(|argument| subst_env(argument, env, shadowed))
                .collect(),
        ),
        Term::TyLam(binder, body) => {
            Term::TyLam(binder.clone(), Box::new(subst_env(body, env, shadowed)))
        }
        Term::TyApp(inner, ty) => {
            Term::TyApp(Box::new(subst_env(inner, env, shadowed)), ty.clone())
        }
        Term::Case(scrutinee, alts) => Term::Case(
            Box::new(subst_env(scrutinee, env, shadowed)),
            alts.iter()
                .map(|alt| subst_alt(alt, env, shadowed))
                .collect(),
        ),
    }
}

fn subst_under_binders(
    term: &Term,
    env: &Environment,
    shadowed: &mut Vec<String>,
    binders: &[String],
) -> Term {
    let original_len = shadowed.len();
    shadowed.extend(binders.iter().cloned());
    let result = subst_env(term, env, shadowed);
    shadowed.truncate(original_len);
    result
}

fn subst_alt(alt: &Alt, env: &Environment, shadowed: &mut Vec<String>) -> Alt {
    Alt {
        con: alt.con.clone(),
        binders: alt.binders.clone(),
        body: subst_under_binders(&alt.body, env, shadowed, &alt.binders),
    }
}

impl crate::machine::State {
    /// Reconstructs the Core [`Term`] represented by this machine state.
    #[must_use]
    pub fn readback(&self) -> Term {
        let hole = match &self.control {
            Control::Eval(term) => subst_env(term, &self.env, &mut Vec::new()),
            Control::Return(value) => value_to_term(value),
        };
        self.kont.iter().rev().fold(hole, plug)
    }
}

fn plug(hole: Term, frame: &Frame) -> Term {
    match frame {
        Frame::AppArg(argument, env) => Term::App(
            Box::new(hole),
            Box::new(subst_env(argument, env, &mut Vec::new())),
        ),
        Frame::AppCall(callee) => Term::App(Box::new(value_to_term(callee)), Box::new(hole)),
        Frame::LetBody(name, body, env) => Term::Let(
            name.clone(),
            Box::new(hole),
            Box::new(subst_env(body, env, &mut vec![name.clone()])),
        ),
        Frame::IfBranch(then_term, else_term, env) => Term::Con(
            "if".to_string(),
            Vec::new(),
            vec![
                hole,
                subst_env(then_term, env, &mut Vec::new()),
                subst_env(else_term, env, &mut Vec::new()),
            ],
        ),
        Frame::PrimArgs(op, done, rest, env) => plug_args(hole, op, done, rest, env),
        Frame::ConArgs(name, done, rest, env) => plug_args(hole, name, done, rest, env),
        Frame::CaseAlts(alts, env) => Term::Case(
            Box::new(hole),
            alts.iter()
                .map(|alt| subst_alt(alt, env, &mut Vec::new()))
                .collect(),
        ),
    }
}

fn plug_args(hole: Term, name: &str, done: &[Value], rest: &[Term], env: &Environment) -> Term {
    let mut args = done.iter().map(value_to_term).collect::<Vec<_>>();
    args.push(hole);
    args.extend(
        rest.iter()
            .map(|term| subst_env(term, env, &mut Vec::new())),
    );
    Term::Con(name.to_string(), Vec::new(), args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base_type::BaseType;
    use crate::builder::{app, con, int, lam, let_, var};
    use crate::eval::{Environment, Value};
    use crate::machine::Machine;

    #[test]
    fn values_and_environments_read_back_to_terms() -> Result<(), crate::error::BuildError> {
        let value = Value::Con(
            "Cons".to_string(),
            vec![Value::Int(1), Value::Con("Nil".to_string(), vec![])],
        );
        assert_eq!(value_to_term(&value).to_text(), "Cons(1, Nil)");

        let term = let_("x", int(1), var("x")?)?;
        let env = Environment::new().extend("x", Value::Int(9));
        assert_eq!(
            subst_env(&term, &env, &mut Vec::new()).to_text(),
            "(let x = 1 in x)"
        );
        Ok(())
    }

    #[test]
    fn state_readback_tracks_machine_transitions() -> Result<(), crate::error::BuildError> {
        let term = app(
            lam("x", BaseType::Int, con("+", vec![var("x")?, int(1)])?)?,
            int(2),
        );
        let mut machine = Machine::new(&term, &Environment::new());
        assert_eq!(
            machine.state().readback().to_text(),
            "((\\x: Int. +(x, 1)) 2)"
        );

        for _ in 0..5 {
            let outcome = machine.step();
            assert!(matches!(
                outcome,
                Ok(crate::machine::StepOutcome::Continue(_))
            ));
        }
        assert_eq!(machine.state().readback().to_text(), "+(2, 1)");

        for _ in 0..4 {
            let outcome = machine.step();
            assert!(matches!(
                outcome,
                Ok(crate::machine::StepOutcome::Continue(_))
            ));
        }
        assert_eq!(machine.state().readback().to_text(), "+(2, 1)");

        let outcome = machine.step();
        assert!(matches!(
            outcome,
            Ok(crate::machine::StepOutcome::Continue(_))
        ));
        assert_eq!(machine.state().readback().to_text(), "3");
        Ok(())
    }
}
