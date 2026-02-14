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
    /// Uses HashMap for O(1) field lookup during type checking
    Record(std::collections::HashMap<String, Type>),
    /// Record type with row polymorphism: { field1: Type1, field2: Type2 | r }
    /// The row variable represents "the rest of the fields"
    /// This enables functions like `fun r -> r.field` to work with any record having that field
    RecordRow(std::collections::HashMap<String, Type>, RowVar),
    /// Row variable (for row polymorphism): ρ
    /// Represents an unknown set of record fields
    Row(RowVar),
    /// Generic sum type: Type constructor applied to type arguments
    /// E.g., Option Int, List Bool, Either Int Bool
    /// First element is the type constructor name, second is the list of type arguments
    SumType(String, Vec<Type>),
}

/// Type variable identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TypeVar(pub usize);

/// Row variable identifier for row polymorphism
/// Represents "the rest of the fields" in a record type
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RowVar(pub usize);

/// Type scheme for polymorphic types: ∀α.τ
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScheme {
    /// Quantified type variables
    pub vars: Vec<TypeVar>,
    /// Quantified row variables
    pub row_vars: Vec<RowVar>,
    /// The type
    pub ty: Type,
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
            Type::RecordRow(fields, row) => {
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
                if !fields.is_empty() {
                    write!(f, " | ")?;
                }
                write!(f, "r{}}}", row.0)
            }
            Type::Row(row) => write!(f, "r{}", row.0),
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
        if self.vars.is_empty() && self.row_vars.is_empty() {
            write!(f, "{}", self.ty)
        } else {
            write!(f, "forall ")?;
            let mut first = true;
            for var in self.vars.iter() {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "t{}", var.0)?;
                first = false;
            }
            for row_var in self.row_vars.iter() {
                if !first {
                    write!(f, ", ")?;
                }
                write!(f, "r{}", row_var.0)?;
                first = false;
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
        let scheme1 = TypeScheme {
            vars: vec![TypeVar(0)],
            row_vars: vec![],
            ty: Type::Var(TypeVar(0)),
        };
        let scheme2 = TypeScheme {
            vars: vec![TypeVar(0)],
            row_vars: vec![],
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
        let scheme = TypeScheme {
            vars: vec![],
            row_vars: vec![],
            ty: Type::Int,
        };
        assert_eq!(format!("{scheme}"), "Int");
    }

    #[test]
    fn test_display_type_scheme_polymorphic() {
        let scheme = TypeScheme {
            vars: vec![TypeVar(0)],
            row_vars: vec![],
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
            row_vars: vec![],
            ty: Type::Fun(
                Box::new(Type::Var(TypeVar(0))),
                Box::new(Type::Var(TypeVar(1))),
            ),
        };
        assert_eq!(format!("{scheme}"), "forall t0, t1. t0 -> t1");
    }

    #[test]
    fn test_display_type_scheme_unit() {
        let scheme = TypeScheme {
            vars: vec![],
            row_vars: vec![],
            ty: Type::Unit,
        };
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

    // Tests for row polymorphism
    #[test]
    fn test_row_var_equality() {
        assert_eq!(RowVar(0), RowVar(0));
        assert_ne!(RowVar(0), RowVar(1));
    }

    #[test]
    fn test_row_var_ordering() {
        assert!(RowVar(0) < RowVar(1));
        assert!(RowVar(5) > RowVar(3));
    }

    #[test]
    fn test_display_row() {
        let ty = Type::Row(RowVar(0));
        assert_eq!(format!("{ty}"), "r0");
        
        let ty = Type::Row(RowVar(42));
        assert_eq!(format!("{ty}"), "r42");
    }

    #[test]
    fn test_display_record_row() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Type::Int);
        let ty = Type::RecordRow(fields, RowVar(0));
        assert_eq!(format!("{ty}"), "{name: Int | r0}");
    }

    #[test]
    fn test_display_record_row_multiple_fields() {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".to_string(), Type::Int);
        fields.insert("age".to_string(), Type::Bool);
        let ty = Type::RecordRow(fields, RowVar(1));
        // Fields are sorted alphabetically
        assert_eq!(format!("{ty}"), "{age: Bool, name: Int | r1}");
    }

    #[test]
    fn test_display_record_row_empty() {
        let fields = std::collections::HashMap::new();
        let ty = Type::RecordRow(fields, RowVar(2));
        assert_eq!(format!("{ty}"), "{r2}");
    }

    #[test]
    fn test_record_row_equality() {
        let mut fields1 = std::collections::HashMap::new();
        fields1.insert("name".to_string(), Type::Int);
        let ty1 = Type::RecordRow(fields1.clone(), RowVar(0));
        let ty2 = Type::RecordRow(fields1, RowVar(0));
        assert_eq!(ty1, ty2);
    }

    #[test]
    fn test_record_row_inequality() {
        let mut fields1 = std::collections::HashMap::new();
        fields1.insert("name".to_string(), Type::Int);
        let ty1 = Type::RecordRow(fields1.clone(), RowVar(0));
        
        let mut fields2 = std::collections::HashMap::new();
        fields2.insert("name".to_string(), Type::Bool);
        let ty2 = Type::RecordRow(fields2, RowVar(0));
        
        let ty3 = Type::RecordRow(fields1, RowVar(1));
        
        assert_ne!(ty1, ty2); // Different field type
        assert_ne!(ty1, ty3); // Different row variable
    }

    #[test]
    fn test_display_type_scheme_with_row_vars() {
        let scheme = TypeScheme {
            vars: vec![TypeVar(0)],
            row_vars: vec![RowVar(0)],
            ty: Type::Fun(
                Box::new(Type::RecordRow(
                    {
                        let mut fields = std::collections::HashMap::new();
                        fields.insert("age".to_string(), Type::Var(TypeVar(0)));
                        fields
                    },
                    RowVar(0),
                )),
                Box::new(Type::Var(TypeVar(0))),
            ),
        };
        assert_eq!(format!("{scheme}"), "forall t0, r0. {age: t0 | r0} -> t0");
    }

    #[test]
    fn test_display_type_scheme_only_row_vars() {
        let scheme = TypeScheme {
            vars: vec![],
            row_vars: vec![RowVar(0), RowVar(1)],
            ty: Type::Fun(
                Box::new(Type::Row(RowVar(0))),
                Box::new(Type::Row(RowVar(1))),
            ),
        };
        assert_eq!(format!("{scheme}"), "forall r0, r1. r0 -> r1");
    }
}
