//! Text rendering for runtime [`Value`]s.

impl std::fmt::Display for crate::eval::Value {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(value) => write!(formatter, "{value}"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::Float(value) => write!(formatter, "{value}"),
            Self::Unit => write!(formatter, "()"),
            Self::Char(value) => write!(formatter, "{value:?}"),
            Self::Closure(closure) => write!(
                formatter,
                "<closure (\\{}: {}. {})>",
                closure.param, closure.param_ty, closure.body
            ),
            Self::RecClosure(closure) => write!(
                formatter,
                "<recclosure {} = (\\{}: {}. {})>",
                closure.name, closure.param, closure.param_ty, closure.body
            ),
            Self::Con(name, values) if values.is_empty() => write!(formatter, "{name}"),
            Self::Con(name, values) => {
                let rendered = values
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(formatter, "{name}({rendered})")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::base_type::BaseType;
    use crate::builder::{con, int, lam, var};
    use crate::eval::{Environment, Value};

    #[test]
    fn displays_runtime_values() -> Result<(), crate::error::BuildError> {
        assert_eq!(Value::Int(3).to_string(), "3");
        assert_eq!(Value::Char('a').to_string(), "'a'");
        assert_eq!(Value::Con("Nil".to_string(), vec![]).to_string(), "Nil");
        assert_eq!(
            Value::Con(
                "Cons".to_string(),
                vec![Value::Int(1), Value::Con("Nil".to_string(), vec![])],
            )
            .to_string(),
            "Cons(1, Nil)"
        );

        let term = lam("x", BaseType::Int, con("+", vec![var("x")?, int(1)])?)?;
        assert!(matches!(
            crate::machine::run(&term, &Environment::new(), None),
            Ok(ref value) if value.to_string() == "<closure (\\x: Int. +(x, 1))>"
        ));
        Ok(())
    }
}
