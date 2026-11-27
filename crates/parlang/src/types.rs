/// Type representations for the Hindley-Milner type system
use std::fmt;

/// Type representations for the type system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Integer type
    Int,
    /// Boolean type
    Bool,
    /// Character type
    Char,
    /// Floating point type
    Float,
    /// Unit type: ()
    /// Represents the type of the empty tuple, used for side effects
    Unit,
    /// Function type: T1 -> T2
    Fun(Box<Type>, Box<Type>),
    /// Type variable (for polymorphism): α, β, γ
    Var(TypeVar),
    /// Record type: { field1: Type1, field2: Type2, ... }
    /// Uses `HashMap` for O(1) field lookup during type checking
    Record(std::collections::HashMap<String, Type>),
    /// Generic sum type: Type constructor applied to type arguments
    /// E.g., Option Int, List Bool, Either Int Bool
    /// First element is the type constructor name, second is the list of type arguments
    SumType(String, Vec<Type>),
    /// Tuple type: (T1, T2, ...) of arity two or more.
    /// The empty tuple is represented by `Unit`; the parser cannot produce
    /// one-element tuples.
    Tuple(Vec<Type>),
    /// Applied type variable: a type variable applied to one or more type
    /// arguments, e.g. `f a`. The argument list is never empty; build values
    /// with `apply_type_args` to keep applications flat.
    VarApp(TypeVar, Vec<Type>),
}

/// Type variable identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeVar(pub usize);

/// A type class constraint such as `Eq t0`.
///
/// The type checker emits `Eq` constraints for `==` and `!=`; constraints on
/// quantified type variables are recorded in `TypeScheme::constraints`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constraint {
    /// Typeclass name, e.g. `Eq`.
    pub class: String,
    /// Type arguments the class is applied to.
    pub args: Vec<Type>,
}

/// Type scheme for polymorphic types: ∀α.τ
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScheme {
    /// Quantified type variables
    pub vars: Vec<TypeVar>,
    /// The type
    pub ty: Type,
    /// Class constraints over the quantified variables, sorted by rendering.
    pub constraints: Vec<Constraint>,
}

impl TypeScheme {
    /// Build a monomorphic scheme (no quantified variables).
    pub fn mono(ty: Type) -> Self {
        TypeScheme {
            vars: vec![],
            ty,
            constraints: vec![],
        }
    }

    /// Build a polymorphic scheme quantifying `vars` over `ty`.
    pub fn poly(vars: Vec<TypeVar>, ty: Type) -> Self {
        TypeScheme {
            vars,
            ty,
            constraints: vec![],
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::Char => write!(f, "Char"),
            Type::Float => write!(f, "Float"),
            Type::Unit => write!(f, "()"),
            Type::Fun(arg, ret) => {
                // Add parentheses around function arguments if they are also functions
                match arg.as_ref() {
                    Type::Fun(_, _) => write!(f, "({arg}) -> {ret}"),
                    _ => write!(f, "{arg} -> {ret}"),
                }
            }
            Type::Var(var) => write!(f, "t{}", var.0),
            Type::Record(fields) => {
                write!(f, "{{")?;
                // Sort fields by name for consistent display
                let mut sorted: Vec<_> = fields.iter().collect();
                sorted.sort_by_key(|(name, _)| *name);

                for (i, (name, ty)) in sorted.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{name}: {ty}")?;
                }
                write!(f, "}}")
            }
            Type::SumType(name, args) => {
                write!(f, "{name}")?;
                fmt_type_args(f, args)
            }
            Type::VarApp(var, args) => {
                write!(f, "t{}", var.0)?;
                fmt_type_args(f, args)
            }
            Type::Tuple(elements) => {
                write!(f, "(")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, ")")
            }
        }
    }
}

/// Write each type argument preceded by a space, parenthesising arguments
/// that are applications or function types so the rendering is unambiguous
/// (`List (Option Int)`, `Wrap Option Int`).
fn fmt_type_args(f: &mut fmt::Formatter, args: &[Type]) -> fmt::Result {
    for arg in args {
        let needs_parens = match arg {
            Type::SumType(_, inner) | Type::VarApp(_, inner) => !inner.is_empty(),
            Type::Fun(..) => true,
            _ => false,
        };
        if needs_parens {
            write!(f, " ({arg})")?;
        } else {
            write!(f, " {arg}")?;
        }
    }
    Ok(())
}

/// Apply `head` to further type arguments, keeping applications flat: a data
/// type gains arguments (`SumType`) and a type variable becomes an applied
/// variable (`VarApp`).
///
/// Returns `head` unchanged when `args` is empty, and `None` when `head`
/// cannot take arguments (a base, function, tuple or record type).
pub fn apply_type_args(head: Type, args: Vec<Type>) -> Option<Type> {
    if args.is_empty() {
        return Some(head);
    }
    match head {
        Type::Var(var) => Some(Type::VarApp(var, args)),
        Type::VarApp(var, mut prefix) => {
            prefix.extend(args);
            Some(Type::VarApp(var, prefix))
        }
        Type::SumType(name, mut prefix) => {
            prefix.extend(args);
            Some(Type::SumType(name, prefix))
        }
        _ => None,
    }
}

impl fmt::Display for TypeScheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.vars.is_empty() {
            write!(f, "{}", self.ty)
        } else {
            write!(f, "forall ")?;
            let mut first = true;
            for var in &self.vars {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "t{}", var.0)?;
                first = false;
            }
            write!(f, ". ")?;
            if !self.constraints.is_empty() {
                for (i, c) in self.constraints.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{c}")?;
                }
                write!(f, " => ")?;
            }
            write!(f, "{}", self.ty)
        }
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.class)?;
        fmt_type_args(f, &self.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_equality() {
        assert_eq!(Type::Int, Type::Int);
        assert_eq!(Type::Bool, Type::Bool);
        assert_eq!(Type::Float, Type::Float);
        assert_ne!(Type::Int, Type::Bool);
        assert_ne!(Type::Int, Type::Float);
        assert_ne!(Type::Float, Type::Bool);
    }

    #[test]
    fn test_type_var_equality() {
        assert_eq!(TypeVar(0), TypeVar(0));
        assert_ne!(TypeVar(0), TypeVar(1));
    }

    #[test]
    fn test_type_var_ordering() {
        assert!(TypeVar(0) < TypeVar(1));
        assert!(TypeVar(5) > TypeVar(3));
    }

    #[test]
    fn test_function_type_equality() {
        let t1 = Type::Fun(Box::new(Type::Int), Box::new(Type::Bool));
        let t2 = Type::Fun(Box::new(Type::Int), Box::new(Type::Bool));
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_type_scheme_equality() {
        let scheme1 = TypeScheme::poly(vec![TypeVar(0)], Type::Var(TypeVar(0)));
        let scheme2 = TypeScheme::poly(vec![TypeVar(0)], Type::Var(TypeVar(0)));
        assert_eq!(scheme1, scheme2);
    }

    #[test]
    fn test_display_int() {
        assert_eq!(format!("{}", Type::Int), "Int");
    }

    #[test]
    fn test_display_bool() {
        assert_eq!(format!("{}", Type::Bool), "Bool");
    }

    #[test]
    fn test_display_float() {
        assert_eq!(format!("{}", Type::Float), "Float");
    }

    #[test]
    fn test_display_unit() {
        assert_eq!(format!("{}", Type::Unit), "()");
    }

    #[test]
    fn test_type_unit_equality() {
        assert_eq!(Type::Unit, Type::Unit);
        assert_ne!(Type::Unit, Type::Int);
        assert_ne!(Type::Unit, Type::Bool);
    }

    #[test]
    fn test_type_unit_clone() {
        let t1 = Type::Unit;
        let t2 = t1.clone();
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_display_var() {
        assert_eq!(format!("{}", Type::Var(TypeVar(0))), "t0");
        assert_eq!(format!("{}", Type::Var(TypeVar(42))), "t42");
    }

    #[test]
    fn test_display_simple_function() {
        let ty = Type::Fun(Box::new(Type::Int), Box::new(Type::Bool));
        assert_eq!(format!("{ty}"), "Int -> Bool");
    }

    #[test]
    fn test_display_function_with_function_arg() {
        // (Int -> Bool) -> Bool
        let ty = Type::Fun(
            Box::new(Type::Fun(Box::new(Type::Int), Box::new(Type::Bool))),
            Box::new(Type::Bool),
        );
        assert_eq!(format!("{ty}"), "(Int -> Bool) -> Bool");
    }

    #[test]
    fn test_display_function_with_function_ret() {
        // Int -> (Bool -> Int)
        let ty = Type::Fun(
            Box::new(Type::Int),
            Box::new(Type::Fun(Box::new(Type::Bool), Box::new(Type::Int))),
        );
        assert_eq!(format!("{ty}"), "Int -> Bool -> Int");
    }

    #[test]
    fn test_display_function_with_unit_arg() {
        // () -> Int
        let ty = Type::Fun(Box::new(Type::Unit), Box::new(Type::Int));
        assert_eq!(format!("{ty}"), "() -> Int");
    }

    #[test]
    fn test_display_function_with_unit_ret() {
        // Int -> ()
        let ty = Type::Fun(Box::new(Type::Int), Box::new(Type::Unit));
        assert_eq!(format!("{ty}"), "Int -> ()");
    }

    #[test]
    fn test_display_function_unit_to_unit() {
        // () -> ()
        let ty = Type::Fun(Box::new(Type::Unit), Box::new(Type::Unit));
        assert_eq!(format!("{ty}"), "() -> ()");
    }

    #[test]
    fn test_display_type_scheme_monomorphic() {
        let scheme = TypeScheme::mono(Type::Int);
        assert_eq!(format!("{scheme}"), "Int");
    }

    #[test]
    fn test_display_type_scheme_polymorphic() {
        let scheme = TypeScheme::poly(
            vec![TypeVar(0)],
            Type::Fun(
                Box::new(Type::Var(TypeVar(0))),
                Box::new(Type::Var(TypeVar(0))),
            ),
        );
        assert_eq!(format!("{scheme}"), "forall t0. t0 -> t0");
    }

    #[test]
    fn test_display_type_scheme_multiple_vars() {
        let scheme = TypeScheme::poly(
            vec![TypeVar(0), TypeVar(1)],
            Type::Fun(
                Box::new(Type::Var(TypeVar(0))),
                Box::new(Type::Var(TypeVar(1))),
            ),
        );
        assert_eq!(format!("{scheme}"), "forall t0, t1. t0 -> t1");
    }

    #[test]
    fn test_display_type_scheme_unit() {
        let scheme = TypeScheme::mono(Type::Unit);
        assert_eq!(format!("{scheme}"), "()");
    }

    // Test Record type
    #[test]
    fn test_type_record_empty() {
        let ty = Type::Record(std::collections::HashMap::new());
        assert_eq!(ty, Type::Record(std::collections::HashMap::new()));
    }

    #[test]
    fn test_type_record_single_field() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Type::Int);
        let ty = Type::Record(fields.clone());
        assert_eq!(ty, Type::Record(fields));
    }

    #[test]
    fn test_type_record_multiple_fields() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Type::Int);
        fields.insert("age".to_string(), Type::Int);
        let ty = Type::Record(fields.clone());
        assert_eq!(ty, Type::Record(fields));
    }

    #[test]
    fn test_type_record_nested() {
        let mut inner_fields = std::collections::HashMap::new();
        inner_fields.insert("city".to_string(), Type::Int);

        let mut outer_fields = std::collections::HashMap::new();
        outer_fields.insert("address".to_string(), Type::Record(inner_fields.clone()));
        outer_fields.insert("name".to_string(), Type::Int);

        let ty = Type::Record(outer_fields.clone());
        assert_eq!(ty, Type::Record(outer_fields));
    }

    #[test]
    fn test_display_record_empty() {
        let ty = Type::Record(std::collections::HashMap::new());
        assert_eq!(format!("{ty}"), "{}");
    }

    #[test]
    fn test_display_record_single_field() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Type::Int);
        let ty = Type::Record(fields);
        assert_eq!(format!("{ty}"), "{name: Int}");
    }

    #[test]
    fn test_display_record_multiple_fields() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Type::Int);
        fields.insert("age".to_string(), Type::Bool);
        let ty = Type::Record(fields);
        // Fields are sorted alphabetically
        assert_eq!(format!("{ty}"), "{age: Bool, name: Int}");
    }

    #[test]
    fn test_display_record_nested() {
        let mut inner_fields = std::collections::HashMap::new();
        inner_fields.insert("city".to_string(), Type::Int);

        let mut outer_fields = std::collections::HashMap::new();
        outer_fields.insert("address".to_string(), Type::Record(inner_fields));
        outer_fields.insert("name".to_string(), Type::Int);

        let ty = Type::Record(outer_fields);
        assert_eq!(format!("{ty}"), "{address: {city: Int}, name: Int}");
    }

    #[test]
    fn test_record_type_clone() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Type::Int);
        let ty = Type::Record(fields);
        let cloned = ty.clone();
        assert_eq!(ty, cloned);
    }

    // Tests for SumType (generic types)
    #[test]
    fn test_sum_type_no_args() {
        let ty = Type::SumType("Option".to_string(), vec![]);
        assert_eq!(ty, Type::SumType("Option".to_string(), vec![]));
    }

    #[test]
    fn test_sum_type_single_arg() {
        let ty = Type::SumType("Option".to_string(), vec![Type::Int]);
        assert_eq!(ty, Type::SumType("Option".to_string(), vec![Type::Int]));
    }

    #[test]
    fn test_sum_type_multiple_args() {
        let ty = Type::SumType("Either".to_string(), vec![Type::Int, Type::Bool]);
        assert_eq!(
            ty,
            Type::SumType("Either".to_string(), vec![Type::Int, Type::Bool])
        );
    }

    #[test]
    fn test_sum_type_nested() {
        let inner = Type::SumType("Option".to_string(), vec![Type::Int]);
        let outer = Type::SumType("List".to_string(), vec![inner]);
        assert_eq!(
            outer,
            Type::SumType(
                "List".to_string(),
                vec![Type::SumType("Option".to_string(), vec![Type::Int])]
            )
        );
    }

    #[test]
    fn test_display_sum_type_no_args() {
        let ty = Type::SumType("Unit".to_string(), vec![]);
        assert_eq!(format!("{ty}"), "Unit");
    }

    #[test]
    fn test_display_sum_type_single_arg() {
        let ty = Type::SumType("Option".to_string(), vec![Type::Int]);
        assert_eq!(format!("{ty}"), "Option Int");
    }

    #[test]
    fn test_display_sum_type_multiple_args() {
        let ty = Type::SumType("Either".to_string(), vec![Type::Int, Type::Bool]);
        assert_eq!(format!("{ty}"), "Either Int Bool");
    }

    #[test]
    fn test_display_sum_type_with_type_var() {
        let ty = Type::SumType("Option".to_string(), vec![Type::Var(TypeVar(0))]);
        assert_eq!(format!("{ty}"), "Option t0");
    }

    #[test]
    fn test_display_sum_type_nested() {
        let inner = Type::SumType("Option".to_string(), vec![Type::Int]);
        let outer = Type::SumType("List".to_string(), vec![inner]);
        assert_eq!(format!("{outer}"), "List (Option Int)");
    }

    #[test]
    fn test_display_partial_sum_type_and_var_app() {
        let option = Type::SumType("Option".to_string(), vec![]);
        let wrap = Type::SumType("Wrap".to_string(), vec![option, Type::Int]);
        assert_eq!(format!("{wrap}"), "Wrap Option Int");
        let applied = Type::VarApp(TypeVar(0), vec![Type::Var(TypeVar(1))]);
        assert_eq!(format!("{applied}"), "t0 t1");
        let fun = Type::Fun(Box::new(Type::Int), Box::new(Type::Int));
        let list = Type::SumType("List".to_string(), vec![fun]);
        assert_eq!(format!("{list}"), "List (Int -> Int)");
    }

    #[test]
    fn test_apply_type_args_flattens() {
        let option = Type::SumType("Option".to_string(), vec![]);
        let option_int = Type::SumType("Option".to_string(), vec![Type::Int]);
        assert_eq!(apply_type_args(option, vec![Type::Int]), Some(option_int));
        let applied = Type::VarApp(TypeVar(2), vec![Type::Int]);
        assert_eq!(
            apply_type_args(Type::Var(TypeVar(2)), vec![Type::Int]),
            Some(applied)
        );
        assert_eq!(apply_type_args(Type::Int, vec![Type::Bool]), None);
        assert_eq!(apply_type_args(Type::Int, vec![]), Some(Type::Int));
    }

    #[test]
    fn test_sum_type_clone() {
        let ty = Type::SumType("Option".to_string(), vec![Type::Int]);
        let cloned = ty.clone();
        assert_eq!(ty, cloned);
    }

    #[test]
    fn test_sum_type_equality() {
        let ty1 = Type::SumType("Option".to_string(), vec![Type::Int]);
        let ty2 = Type::SumType("Option".to_string(), vec![Type::Int]);
        let ty3 = Type::SumType("Option".to_string(), vec![Type::Bool]);
        let ty4 = Type::SumType("List".to_string(), vec![Type::Int]);

        assert_eq!(ty1, ty2);
        assert_ne!(ty1, ty3); // Different type argument
        assert_ne!(ty1, ty4); // Different type constructor
    }

    #[test]
    fn test_display_tuple_two_elements() {
        let ty = Type::Tuple(vec![Type::Int, Type::Bool]);
        assert_eq!(format!("{ty}"), "(Int, Bool)");
    }

    #[test]
    fn test_display_type_scheme_with_constraint() {
        let mut scheme = TypeScheme::poly(vec![TypeVar(0)], Type::Var(TypeVar(0)));
        scheme.constraints.push(Constraint {
            class: "Eq".to_string(),
            args: vec![Type::Var(TypeVar(0))],
        });
        assert_eq!(format!("{scheme}"), "forall t0. Eq t0 => t0");
    }

    #[test]
    fn test_display_constraint_parenthesises_applied_args() {
        let constraint = Constraint {
            class: "Eq".to_string(),
            args: vec![Type::SumType("List".to_string(), vec![Type::Int])],
        };
        assert_eq!(format!("{constraint}"), "Eq (List Int)");
    }
}
