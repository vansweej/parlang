//! Evidence for class constraints: which dictionary satisfies `C t`, and
//! the typed expression that builds that dictionary.

use std::fmt;

use crate::typechecker::TypeEnv;
use crate::typed_ast::{TypedExpr, TypedExprKind};
use crate::types::{Constraint, Type};

/// Why dictionary insertion failed. Every case breaks an invariant of a
/// program that type-checked; it is reported instead of panicking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictError {
    /// No dictionary parameter in scope and no instance satisfies the
    /// rendered constraint.
    NoEvidence(String),
    /// The class of a constraint is not declared.
    UnknownClass(String),
    /// An instance head that is neither a base type nor a data type.
    UnsupportedInstanceHead(String),
    /// An instance method that is missing, extra, or whose type is not a
    /// renaming of its class signature.
    MethodShape {
        /// Dictionary name of the instance.
        instance: String,
        /// Name of the method.
        method: String,
    },
}

impl fmt::Display for DictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DictError::NoEvidence(constraint) => {
                write!(f, "no dictionary for constraint {constraint}")
            }
            DictError::UnknownClass(class) => write!(f, "unknown class {class}"),
            DictError::UnsupportedInstanceHead(head) => {
                write!(f, "unsupported instance head {head}")
            }
            DictError::MethodShape { instance, method } => write!(
                f,
                "method {method} of {instance} does not match its class signature"
            ),
        }
    }
}

impl std::error::Error for DictError {}

/// How one class constraint is satisfied.
#[derive(Debug, Clone, PartialEq)]
pub enum Evidence {
    /// The dictionary parameter `name` of an enclosing constrained binding,
    /// instance or trailing body.
    Param {
        /// Dictionary parameter name, e.g. `$d0`.
        name: String,
        /// The constraint the parameter stands for.
        constraint: Constraint,
    },
    /// The instance dictionary `name` at the head arguments `args`, applied
    /// to the evidence for its context in context order.
    Instance {
        /// Instance dictionary name, e.g. `$dict_Eq_Int`.
        name: String,
        /// The constraint this instance satisfies.
        constraint: Constraint,
        /// Type arguments of the instance head (empty for a base type).
        args: Vec<Type>,
        /// Evidence for each context constraint of the instance.
        context: Vec<Evidence>,
    },
}

/// The nominal dictionary type constructor of `class`: `$Dict_<class>`.
/// The `$` prefix cannot clash with a user type name.
pub fn dictionary_type_name(class: &str) -> String {
    format!("$Dict_{class}")
}

/// The deterministic dictionary name of the `class` instance with `head`:
/// `$dict_<class>_<tycon>`. The `$` prefix cannot clash with user names.
///
/// # Errors
///
/// `UnsupportedInstanceHead` when `head` is not a base type or a data type.
pub fn dictionary_name(class: &str, head: &Type) -> Result<String, DictError> {
    let key = match head {
        Type::Int => "Int",
        Type::Bool => "Bool",
        Type::Char => "Char",
        Type::Float => "Float",
        Type::SumType(name, _) => name.as_str(),
        other => return Err(DictError::UnsupportedInstanceHead(other.to_string())),
    };
    Ok(format!("$dict_{class}_{key}"))
}

/// The nominal type of the dictionary for `constraint`:
/// `$Dict_<class> arg`.
///
/// # Errors
///
/// `UnknownClass` for an undeclared class; `NoEvidence` when the
/// constraint does not have exactly one argument.
pub fn dictionary_type(env: &TypeEnv, constraint: &Constraint) -> Result<Type, DictError> {
    if env.class_info(&constraint.class).is_none() {
        return Err(DictError::UnknownClass(constraint.class.clone()));
    }
    let [arg] = constraint.args.as_slice() else {
        return Err(DictError::NoEvidence(constraint.to_string()));
    };
    Ok(Type::SumType(
        dictionary_type_name(&constraint.class),
        vec![arg.clone()],
    ))
}

/// Evidence for `constraint`: an equal given first (innermost wins), else
/// the matching instance with evidence for its context, solved in turn.
/// Recursion terminates because a Haskell 98 instance context only
/// constrains variables of the instance head.
///
/// # Errors
///
/// `NoEvidence` when neither a given nor an instance applies;
/// `UnsupportedInstanceHead` from `dictionary_name`.
pub fn solve(
    env: &TypeEnv,
    givens: &[(Constraint, String)],
    constraint: &Constraint,
) -> Result<Evidence, DictError> {
    if let Some((_, name)) = givens.iter().rev().find(|(given, _)| given == constraint) {
        return Ok(Evidence::Param {
            name: name.clone(),
            constraint: constraint.clone(),
        });
    }
    let Some((head, context)) = env.matching_instance(constraint) else {
        return Err(DictError::NoEvidence(constraint.to_string()));
    };
    let name = dictionary_name(&constraint.class, &head)?;
    let args = match constraint.args.as_slice() {
        [Type::SumType(_, args)] => args.clone(),
        _ => Vec::new(),
    };
    let context = context
        .iter()
        .map(|wanted| solve(env, givens, wanted))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Evidence::Instance {
        name,
        constraint: constraint.clone(),
        args,
        context,
    })
}

/// The typed expression building the dictionary `evidence` denotes: the
/// parameter itself, or the instance dictionary applied to its context
/// dictionaries.
///
/// # Errors
///
/// Any `dictionary_type` error.
pub fn evidence_expr(env: &TypeEnv, evidence: &Evidence) -> Result<TypedExpr, DictError> {
    match evidence {
        Evidence::Param { name, constraint } => Ok(TypedExpr {
            kind: TypedExprKind::Var(name.clone(), Vec::new()),
            ty: dictionary_type(env, constraint)?,
        }),
        Evidence::Instance {
            name,
            constraint,
            args,
            context,
        } => {
            let dict_ty = dictionary_type(env, constraint)?;
            let parts = context
                .iter()
                .map(|inner| evidence_expr(env, inner))
                .collect::<Result<Vec<_>, _>>()?;
            let fun_ty = parts.iter().rev().fold(dict_ty, |acc, part| {
                Type::Fun(Box::new(part.ty.clone()), Box::new(acc))
            });
            let head = TypedExpr {
                kind: TypedExprKind::Var(name.clone(), args.clone()),
                ty: fun_ty,
            };
            Ok(apply_all(head, parts))
        }
    }
}

/// Apply `func` to each of `args` in order. Each `App` node is typed with
/// the codomain of its function type.
pub fn apply_all(func: TypedExpr, args: Vec<TypedExpr>) -> TypedExpr {
    args.into_iter().fold(func, |acc, arg| {
        let ty = match &acc.ty {
            Type::Fun(_, result) => (**result).clone(),
            other => other.clone(),
        };
        TypedExpr {
            kind: TypedExprKind::App(Box::new(acc), Box::new(arg)),
            ty,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_program;
    use crate::typechecker::typecheck_program_typed_with_env;
    use crate::types::TypeVar;

    const BOX: &str = "data Box a = MkBox a;\ninstance Eq a => Eq (Box a) where { eq = fun x -> fun y -> true };\n0";

    fn env_for(source: &str) -> TypeEnv {
        let program = parse_program(source).unwrap();
        let mut env = TypeEnv::new();
        typecheck_program_typed_with_env(&program, &mut env).unwrap();
        env
    }

    fn eq_on(ty: Type) -> Constraint {
        Constraint {
            class: "Eq".to_string(),
            args: vec![ty],
        }
    }

    fn boxed_int() -> Type {
        Type::SumType("Box".to_string(), vec![Type::Int])
    }

    #[test]
    fn builtin_eq_int_is_an_instance_dictionary() {
        let env = TypeEnv::new();
        let evidence = solve(&env, &[], &eq_on(Type::Int)).unwrap();
        assert_eq!(
            evidence,
            Evidence::Instance {
                name: "$dict_Eq_Int".to_string(),
                constraint: eq_on(Type::Int),
                args: Vec::new(),
                context: Vec::new(),
            }
        );
    }

    #[test]
    fn a_matching_given_wins() {
        let env = TypeEnv::new();
        let wanted = eq_on(Type::Var(TypeVar(7)));
        let givens = vec![(wanted.clone(), "$d0".to_string())];
        assert_eq!(
            solve(&env, &givens, &wanted).unwrap(),
            Evidence::Param {
                name: "$d0".to_string(),
                constraint: wanted,
            }
        );
    }

    #[test]
    fn instance_context_is_solved_recursively() {
        let env = env_for(BOX);
        let Evidence::Instance {
            name,
            args,
            context,
            ..
        } = solve(&env, &[], &eq_on(boxed_int())).unwrap()
        else {
            panic!("expected an instance");
        };
        assert_eq!(name, "$dict_Eq_Box");
        assert_eq!(args, vec![Type::Int]);
        assert_eq!(context.len(), 1);
    }

    #[test]
    fn unsolvable_constraints_are_errors() {
        let env = TypeEnv::new();
        let free = eq_on(Type::Var(TypeVar(3)));
        assert!(matches!(
            solve(&env, &[], &free),
            Err(DictError::NoEvidence(_))
        ));
        assert!(matches!(
            solve(&env, &[], &eq_on(Type::Unit)),
            Err(DictError::NoEvidence(_))
        ));
    }

    #[test]
    fn dictionary_type_is_nominal() {
        let env = TypeEnv::new();
        assert_eq!(
            dictionary_type(&env, &eq_on(Type::Int)).unwrap(),
            Type::SumType("$Dict_Eq".to_string(), vec![Type::Int])
        );
        let unknown = Constraint {
            class: "Nope".to_string(),
            args: vec![Type::Int],
        };
        assert!(matches!(
            dictionary_type(&env, &unknown),
            Err(DictError::UnknownClass(_))
        ));
    }

    #[test]
    fn evidence_expr_applies_the_instance_to_its_context() {
        let env = env_for(BOX);
        let evidence = solve(&env, &[], &eq_on(boxed_int())).unwrap();
        let built = evidence_expr(&env, &evidence).unwrap();
        assert!(matches!(built.kind, TypedExprKind::App(_, _)));
        assert_eq!(
            built.ty,
            Type::SumType("$Dict_Eq".to_string(), vec![boxed_int()])
        );
    }
}
