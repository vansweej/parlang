/// Type representations for the Hindley-Milner type system
use std::fmt;

/// Type representations for the type system
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Integer type
    Int,
    /// Boolean type
    Bool,
    /// Function type: T1 -> T2
    Fun(Box<Type>, Box<Type>),
    /// Type variable (for polymorphism): α, β, γ
    Var(TypeVar),
    /// Record type: { field1: Type1, field2: Type2, ... }
    /// Uses HashMap for O(1) field lookup during type checking
    Record(std::collections::HashMap<String, Type>),
    /// Generic sum type: Type constructor applied to type arguments
    /// E.g., Option Int, List Bool, Either Int Bool
    /// First element is the type constructor name, second is the list of type arguments
    SumType(String, Vec<Type>),
}

/// Type variable identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeVar(pub usize);

/// Type scheme for polymorphic types: ∀α.τ
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScheme {
    /// Quantified type variables
    pub vars: Vec<TypeVar>,
    /// The type
    pub ty: Type,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
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
                if !args.is_empty() {
                    for arg in args {
                        write!(f, " {arg}")?;
                    }
                }
                Ok(())
            }
        }
    }
}

impl fmt::Display for TypeScheme {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.vars.is_empty() {
            write!(f, "{}", self.ty)
        } else {
            write!(f, "forall ")?;
            for (i, var) in self.vars.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "t{}", var.0)?;
            }
            write!(f, ". {}", self.ty)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_equality() {
        assert_eq!(Type::Int, Type::Int);
        assert_eq!(Type::Bool, Type::Bool);
        assert_ne!(Type::Int, Type::Bool);
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
        let scheme1 = TypeScheme {
            vars: vec![TypeVar(0)],
            ty: Type::Var(TypeVar(0)),
        };
        let scheme2 = TypeScheme {
            vars: vec![TypeVar(0)],
            ty: Type::Var(TypeVar(0)),
        };
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
    fn test_display_type_scheme_monomorphic() {
        let scheme = TypeScheme {
            vars: vec![],
            ty: Type::Int,
        };
        assert_eq!(format!("{scheme}"), "Int");
    }

    #[test]
    fn test_display_type_scheme_polymorphic() {
        let scheme = TypeScheme {
            vars: vec![TypeVar(0)],
            ty: Type::Fun(
                Box::new(Type::Var(TypeVar(0))),
                Box::new(Type::Var(TypeVar(0))),
            ),
        };
        assert_eq!(format!("{scheme}"), "forall t0. t0 -> t0");
    }

    #[test]
    fn test_display_type_scheme_multiple_vars() {
        let scheme = TypeScheme {
            vars: vec![TypeVar(0), TypeVar(1)],
            ty: Type::Fun(
                Box::new(Type::Var(TypeVar(0))),
                Box::new(Type::Var(TypeVar(1))),
            ),
        };
        assert_eq!(format!("{scheme}"), "forall t0, t1. t0 -> t1");
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
        assert_eq!(ty, Type::SumType("Either".to_string(), vec![Type::Int, Type::Bool]));
    }

    #[test]
    fn test_sum_type_nested() {
        let inner = Type::SumType("Option".to_string(), vec![Type::Int]);
        let outer = Type::SumType("List".to_string(), vec![inner]);
        assert_eq!(
            outer,
            Type::SumType("List".to_string(), vec![Type::SumType("Option".to_string(), vec![Type::Int])])
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
        assert_eq!(format!("{outer}"), "List Option Int");
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
}
