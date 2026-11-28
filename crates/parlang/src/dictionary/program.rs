//! Program-level dictionary insertion: top-level `let`s, instance
//! dictionaries and the implicitly generalised trailing body.

use std::collections::{HashMap, HashSet};

use super::evidence::{
    dictionary_name, dictionary_type, evidence_expr, solve, superclass_field_name, DictError,
};
use super::insert::{dictionary_scheme, lambda_all, Inserter};
use crate::typechecker::{apply_subst, apply_subst_constraint, TypeEnv};
use crate::typed_ast::{TypedDecl, TypedExpr, TypedExprKind, TypedProgram};
use crate::types::{Constraint, Type, TypeScheme, TypeVar};

/// Rewrite the type-checked `program` into dictionary-explicit form.
///
/// `env` is the environment `program` was checked in, after checking, so
/// it holds every class, instance and binding scheme involved. Each
/// `instance` becomes a top-level `let` of its dictionary, a constrained
/// `let` becomes a dictionary lambda, and a body with residual constraints
/// (`body_constraints`) is abstracted over their dictionaries. A program
/// that uses no class comes back unchanged.
///
/// # Errors
///
/// A `DictError` when an invariant of the type checker is broken.
pub fn insert_dictionaries(
    program: &TypedProgram,
    env: &TypeEnv,
) -> Result<TypedProgram, DictError> {
    let program_names: HashSet<String> = program
        .decls
        .iter()
        .filter_map(|decl| match decl {
            TypedDecl::Let { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    let mut inserter = Inserter::new(env, program_names);
    let mut decls = Vec::with_capacity(program.decls.len());
    for decl in &program.decls {
        decls.push(top_decl(&mut inserter, decl)?);
    }
    let body = program
        .body
        .as_ref()
        .map(|body| inserter.abstract_over(&program.body_constraints, body))
        .transpose()?;
    let ty = match &body {
        Some(body) if !program.body_constraints.is_empty() => body.ty.clone(),
        _ => program.ty.clone(),
    };
    Ok(TypedProgram {
        decls,
        body,
        ty,
        method_uses: program.method_uses.clone(),
        body_constraints: program.body_constraints.clone(),
    })
}

fn top_decl(inserter: &mut Inserter<'_>, decl: &TypedDecl) -> Result<TypedDecl, DictError> {
    match decl {
        TypedDecl::Let {
            name,
            ty_ann,
            scheme,
            value,
            doc,
        } => {
            let value_out = inserter.abstract_over(&scheme.constraints, value)?;
            let (ty_ann_out, scheme_out) = if scheme.constraints.is_empty() {
                (ty_ann.clone(), scheme.clone())
            } else {
                (None, dictionary_scheme(scheme, &value_out))
            };
            inserter.top.insert(name.clone(), scheme.clone());
            Ok(TypedDecl::Let {
                name: name.clone(),
                ty_ann: ty_ann_out,
                scheme: scheme_out,
                value: Box::new(value_out),
                doc: doc.clone(),
            })
        }
        TypedDecl::Instance {
            class,
            head,
            vars,
            context,
            methods,
            ..
        } => instance_dictionary(inserter, class, head, vars, context, methods),
        other => Ok(other.clone()),
    }
}

/// The `let` defining the dictionary of one instance: a record of its
/// superclass dictionaries then its method bodies in class declaration
/// order, under one dictionary parameter per context constraint.
fn instance_dictionary(
    inserter: &mut Inserter<'_>,
    class: &str,
    head: &Type,
    vars: &[TypeVar],
    context: &[Constraint],
    methods: &[(String, TypedExpr)],
) -> Result<TypedDecl, DictError> {
    let env = inserter.env;
    let name = dictionary_name(class, head)?;
    let Some(class_info) = env.class_info(class) else {
        return Err(DictError::UnknownClass(class.to_string()));
    };
    if let Some((extra, _)) = methods
        .iter()
        .find(|(method, _)| !class_info.methods.iter().any(|(known, _)| known == method))
    {
        return Err(DictError::MethodShape {
            instance: name,
            method: extra.clone(),
        });
    }
    let mut params = Vec::with_capacity(context.len());
    for constraint in context {
        params.push((inserter.fresh_param(), dictionary_type(env, constraint)?));
    }
    let mut fields = superclass_fields(env, class_info, head, context, &params)?;
    fields.extend(instance_fields(
        inserter, class_info, head, context, &params, methods, &name,
    )?);
    let instance = Constraint {
        class: class.to_string(),
        args: vec![head.clone()],
    };
    let record = TypedExpr {
        kind: TypedExprKind::Record(fields),
        ty: dictionary_type(env, &instance)?,
    };
    let value = lambda_all(params, record);
    Ok(TypedDecl::Let {
        name,
        ty_ann: None,
        scheme: TypeScheme {
            vars: vars.to_vec(),
            ty: value.ty.clone(),
            constraints: Vec::new(),
        },
        value: Box::new(value),
        doc: None,
    })
}

/// The superclass fields of one instance dictionary, in
/// `ClassInfo::superclasses` order: each `$super_<S>` field holds the
/// dictionary for `S head`, solved with the instance context parameters
/// as the only givens, over the head's own (unrenamed) variables.
fn superclass_fields(
    env: &TypeEnv,
    class_info: &crate::typechecker::ClassInfo,
    head: &Type,
    context: &[Constraint],
    params: &[(String, Type)],
) -> Result<Vec<(String, TypedExpr)>, DictError> {
    let givens: Vec<(Constraint, String)> = context
        .iter()
        .cloned()
        .zip(params.iter().map(|(param, _)| param.clone()))
        .collect();
    let mut fields = Vec::with_capacity(class_info.superclasses.len());
    for superclass in &class_info.superclasses {
        let wanted = Constraint {
            class: superclass.clone(),
            args: vec![head.clone()],
        };
        let evidence = solve(env, &givens, &wanted)?;
        fields.push((
            superclass_field_name(superclass),
            evidence_expr(env, &evidence)?,
        ));
    }
    Ok(fields)
}

/// Rewritten method bodies of one instance, in class declaration order.
fn instance_fields(
    inserter: &mut Inserter<'_>,
    class_info: &crate::typechecker::ClassInfo,
    head: &Type,
    context: &[Constraint],
    params: &[(String, Type)],
    methods: &[(String, TypedExpr)],
    dict_name: &str,
) -> Result<Vec<(String, TypedExpr)>, DictError> {
    let mut fields = Vec::with_capacity(class_info.methods.len());
    for (method, scheme) in &class_info.methods {
        let shape_error = || DictError::MethodShape {
            instance: dict_name.to_string(),
            method: method.clone(),
        };
        let Some((_, body)) = methods.iter().find(|(defined, _)| defined == method) else {
            return Err(shape_error());
        };
        let renaming = method_renaming(scheme, head, &body.ty).ok_or_else(shape_error)?;
        let mark = inserter.givens.len();
        for (constraint, (param, _)) in context.iter().zip(params) {
            let given = apply_subst_constraint(&renaming, constraint);
            inserter.givens.push((given, param.clone()));
        }
        let field = inserter.expr(body);
        inserter.givens.truncate(mark);
        fields.push((method.clone(), field?));
    }
    Ok(fields)
}

/// How the type variables of the class signature of a method, taken at
/// `head`, were renamed in the method body's type `actual`.
///
/// Soundness: `check_instance_method` unifies the body type with exactly
/// this expected type (the class parameter replaced by the head), and
/// `check_rigid_images` guarantees every head variable and every other
/// signature variable maps to a distinct, otherwise unconstrained image
/// variable. So `actual` is the expected type with its variables renamed
/// injectively. A one-way match keyed on the expected side is then total,
/// and type-variable ids that happen to coincide across the two sides are
/// harmless because only the expected side is ever looked up. `None`
/// (reported as `MethodShape`) therefore means a broken invariant.
pub(crate) fn method_renaming(
    scheme: &TypeScheme,
    head: &Type,
    actual: &Type,
) -> Option<HashMap<TypeVar, Type>> {
    let param = scheme.vars.first()?;
    let at_head = HashMap::from([(param.clone(), head.clone())]);
    let expected = apply_subst(&at_head, &scheme.ty);
    let mut renaming = HashMap::new();
    match_types(&expected, actual, &mut renaming).then_some(renaming)
}

/// One-way match of `expected` against `actual`, extending `map` with a
/// binding for each type variable of `expected`.
fn match_types(expected: &Type, actual: &Type, map: &mut HashMap<TypeVar, Type>) -> bool {
    match (expected, actual) {
        (Type::Var(var), _) => {
            if let Some(bound) = map.get(var) {
                bound == actual
            } else {
                map.insert(var.clone(), actual.clone());
                true
            }
        }
        (Type::Fun(from, to), Type::Fun(arg, res)) => {
            match_types(from, arg, map) && match_types(to, res, map)
        }
        (Type::SumType(expected_name, expected_args), Type::SumType(actual_name, actual_args)) => {
            expected_name == actual_name && match_all(expected_args, actual_args, map)
        }
        (Type::Tuple(expected_items), Type::Tuple(actual_items)) => {
            match_all(expected_items, actual_items, map)
        }
        (Type::VarApp(head_var, expected_args), Type::VarApp(actual_var, actual_args)) => {
            match_types(
                &Type::Var(head_var.clone()),
                &Type::Var(actual_var.clone()),
                map,
            ) && match_all(expected_args, actual_args, map)
        }
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            expected_fields.len() == actual_fields.len()
                && expected_fields.iter().all(|(field, expected_ty)| {
                    actual_fields
                        .get(field)
                        .is_some_and(|actual_ty| match_types(expected_ty, actual_ty, map))
                })
        }
        _ => expected == actual,
    }
}

fn match_all(expected: &[Type], actual: &[Type], map: &mut HashMap<TypeVar, Type>) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(expected_ty, actual_ty)| match_types(expected_ty, actual_ty, map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_program;
    use crate::typechecker::typecheck_program_typed_with_env;
    use crate::typed_ast::typed_program_to_text;

    fn translate(source: &str) -> (TypedProgram, TypedProgram) {
        let parsed = parse_program(source).unwrap();
        let mut env = TypeEnv::new();
        let typed = typecheck_program_typed_with_env(&parsed, &mut env).unwrap();
        let out = insert_dictionaries(&typed, &env).unwrap();
        (typed, out)
    }

    fn text(source: &str) -> String {
        typed_program_to_text(&translate(source).1)
    }

    /// The rendered declarations only: everything before the top-level
    /// `body:` line (depth 1).
    fn decls_text(rendered: &str) -> &str {
        rendered.split("\n  body:").next().unwrap_or(rendered)
    }

    #[test]
    fn programs_without_classes_are_unchanged() {
        for source in [
            "let id = fun x -> x; id 1",
            "1 == 2",
            "let f = fun x -> x + 1; f 2 != 3",
            "let eq = fun x -> x + 1;\neq 2",
        ] {
            let (typed, out) = translate(source);
            assert_eq!(out, typed, "{source}");
        }
    }

    #[test]
    fn constrained_let_takes_a_dictionary() {
        let (_, out) = translate("let same = fun x -> fun y -> x == y; same 1 2");
        let TypedDecl::Let { scheme, .. } = &out.decls[0] else {
            panic!("expected a let");
        };
        assert!(scheme.constraints.is_empty());
        let rendered = typed_program_to_text(&out);
        assert!(rendered.contains("Fun $d0 : $Dict_Eq"), "{rendered}");
        assert!(rendered.contains("FieldAccess eq"), "{rendered}");
        assert!(rendered.contains("Var $dict_Eq_Int"), "{rendered}");
    }

    #[test]
    fn neq_negates_the_dictionary_call() {
        let rendered = text("let differ = fun x -> fun y -> x != y; differ true false");
        assert!(rendered.contains("If : Bool"), "{rendered}");
        assert!(rendered.contains("FieldAccess eq"), "{rendered}");
        assert!(rendered.contains("Var $dict_Eq_Bool"), "{rendered}");
    }

    #[test]
    fn later_top_level_let_does_not_capture_an_earlier_method_use() {
        let rendered = text("let f = fun x -> eq x x;\nlet eq = fun x -> x + 1;\nf 1");
        assert!(rendered.contains("FieldAccess eq"), "{rendered}");
        assert!(rendered.contains("Var $dict_Eq_Int"), "{rendered}");
        assert!(!rendered.contains("Var eq :"), "{rendered}");
    }

    #[test]
    fn instance_with_context_is_a_dictionary_function() {
        let rendered = text(
            "data Box a = MkBox a;\ninstance Eq a => Eq (Box a) where { eq = fun x -> fun y -> (match x with | MkBox p -> (match y with | MkBox q -> eq p q)) };\nlet same = fun x -> fun y -> x == y;\nsame (MkBox 1) (MkBox 2)",
        );
        assert!(rendered.contains("Let $dict_Eq_Box"), "{rendered}");
        assert!(rendered.contains("Var $dict_Eq_Box"), "{rendered}");
        assert!(rendered.contains("Var $dict_Eq_Int"), "{rendered}");
        assert!(rendered.contains("FieldAccess eq"), "{rendered}");
        assert!(!rendered.contains(": Instance"), "{rendered}");
    }

    #[test]
    fn instance_may_reference_its_own_dictionary() {
        let rendered = text(
            "data Nat = Z | S Nat;\ninstance Eq Nat where { eq = fun x -> fun y -> (match x with | Z -> (match y with | Z -> true | S m -> false) | S n -> (match y with | Z -> false | S m -> eq n m)) };\neq (S Z) (S Z)",
        );
        let decls = decls_text(&rendered);
        assert!(decls.contains("Let $dict_Eq_Nat"), "{rendered}");
        assert!(decls.contains("Var $dict_Eq_Nat"), "{rendered}");
    }

    #[test]
    fn method_use_selects_a_field_in_class_order() {
        let (_, out) = translate(
            "class Pair a where { first : a -> Int; second : a -> Int };\ninstance Pair Bool where { second = fun b -> 2; first = fun b -> 1 };\nfirst true",
        );
        let Some(TypedDecl::Let { value, .. }) = out.decls.get(1) else {
            panic!("expected the instance dictionary let");
        };
        let TypedExprKind::Record(fields) = &value.kind else {
            panic!("expected a record dictionary");
        };
        let order: Vec<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(order, vec!["first", "second"]);
        let rendered = typed_program_to_text(&out);
        assert!(rendered.contains("FieldAccess first"), "{rendered}");
        assert!(rendered.contains("Var $dict_Pair_Bool"), "{rendered}");
    }

    #[test]
    fn body_residual_becomes_a_dictionary_lambda() {
        let (_, out) = translate("let same = fun x -> fun y -> x == y; same");
        assert_eq!(out.body_constraints.len(), 1);
        assert!(matches!(out.ty, Type::Fun(_, _)));
        let body = out.body.as_ref().map(|body| &body.kind);
        assert!(matches!(body, Some(TypedExprKind::Fun(name, None, _)) if name.starts_with("$d")));
    }

    #[test]
    fn stdlib_map_example_is_translated() {
        let rendered = text(include_str!("../../examples/stdlib/map.par"));
        assert!(rendered.contains("FieldAccess eq"), "{rendered}");
        assert!(rendered.contains("Fun $d"), "{rendered}");
    }
}
