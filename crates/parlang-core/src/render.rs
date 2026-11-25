//! Temporary host-side, type-directed rendering of Core runtime values (replaced by a Show class later).

use crate::base_type::BaseType;
use crate::eval::Value;
use crate::lint::{DataType, LintEnv};
use crate::ty::Type;

/// Strips all leading `Forall` binders from `ty`, returning the first non-`Forall` type.
fn strip_foralls(ty: &Type) -> &Type {
    let mut current = ty;
    while let Type::Forall(_, body) = current {
        current = body;
    }
    current
}

/// Walks the application spine of `ty`, returning the head constructor name and its
/// arguments in source order, or `None` if the head is not a `Type::Con`.
fn spine(ty: &Type) -> Option<(&str, Vec<&Type>)> {
    let mut args = Vec::new();
    let mut current = ty;
    loop {
        match current {
            Type::App(head, arg) => {
                args.push(arg.as_ref());
                current = head;
            }
            Type::Con(name) => {
                args.reverse();
                return Some((name.as_str(), args));
            }
            _ => return None,
        }
    }
}

/// Returns the field types of constructor `ctor` (with `arity` fields) of data type `dt`,
/// instantiated at the type arguments `args`, or `None` on arity/shape mismatch.
fn field_types(dt: &DataType, args: &[&Type], ctor: &str, arity: usize) -> Option<Vec<Type>> {
    if dt.params.len() != args.len() {
        return None;
    }
    let con = dt
        .ctors
        .iter()
        .find(|c| c.name == ctor && c.fields.len() == arity)?;
    let mapping: Vec<(&str, &Type)> = dt
        .params
        .iter()
        .map(String::as_str)
        .zip(args.iter().copied())
        .collect();
    Some(
        con.fields
            .iter()
            // Simultaneous and capture-avoiding (`Type::subst_many`).
            .map(|field| field.subst_many(&mapping))
            .collect(),
    )
}

/// Returns true if `dt` is the canonical `List a = Nil | Cons a (List a)` shape.
fn is_list(dt: &DataType) -> bool {
    dt.name == "List"
        && dt.params.len() == 1
        && dt.ctors.len() == 2
        && dt
            .ctors
            .iter()
            .any(|c| c.name == "Nil" && c.fields.is_empty())
        && dt
            .ctors
            .iter()
            .any(|c| c.name == "Cons" && c.fields.len() == 2)
}

/// Renders a `Char` the way the surface evaluator's `Value::Char` Display does.
fn render_char(c: char) -> String {
    let mut out = String::from("'");
    match c {
        '\n' => out.push_str("\\n"),
        '\t' => out.push_str("\\t"),
        '\r' => out.push_str("\\r"),
        '\\' => out.push_str("\\\\"),
        '\'' => out.push_str("\\'"),
        _ => out.push(c),
    }
    out.push('\'');
    out
}

/// Renders `value` of Core type `ty` the way the surface language shows values, using
/// `env` for data-type field types. Falls back to the value's plain `Display` on shape mismatch.
#[must_use]
pub fn render_value(value: &Value, ty: &Type, env: &LintEnv) -> String {
    let ty = strip_foralls(ty);

    match (value, ty) {
        (Value::Closure(_) | Value::RecClosure(_), _) => "<function>".to_string(),
        (Value::Int(n), Type::Base(BaseType::Int)) => format!("{n}"),
        (Value::Bool(b), Type::Base(BaseType::Bool)) => format!("{b}"),
        (Value::Float(x), Type::Base(BaseType::Float)) => format!("{x}"),
        (Value::Unit, Type::Base(BaseType::Unit)) => "()".to_string(),
        (Value::Char(c), Type::Base(BaseType::Char)) => render_char(*c),
        _ => render_data(value, ty, env).unwrap_or_else(|| value.to_string()),
    }
}

/// Attempts to render a data-type (list, tuple, record, or user ADT) value.
fn render_data(value: &Value, ty: &Type, env: &LintEnv) -> Option<String> {
    let (name, args) = spine(ty)?;
    let dt = env.data_type(name)?;

    if is_list(dt) && args.len() == 1 {
        return render_list(value, args[0], env);
    }

    let Value::Con(ctor, vals) = value else {
        return None;
    };
    let fields = field_types(dt, &args, ctor, vals.len())?;
    let rendered: Vec<String> = vals
        .iter()
        .zip(fields.iter())
        .map(|(v, t)| render_value(v, t, env))
        .collect();

    if dt.name.starts_with('(') {
        return Some(format!("({})", rendered.join(", ")));
    }
    if dt.name.starts_with('{') {
        let field_names: Vec<&str> = dt
            .name
            .trim_start_matches('{')
            .trim_end_matches('}')
            .split(',')
            .collect();
        let parts: Vec<String> = field_names
            .iter()
            .zip(rendered.iter())
            .map(|(n, r)| format!("{n}: {r}"))
            .collect();
        return Some(format!("{{{}}}", parts.join(", ")));
    }
    if rendered.is_empty() {
        Some(ctor.clone())
    } else {
        Some(format!("{ctor}({})", rendered.join(", ")))
    }
}

/// Renders a `List elem_ty` value, special-casing `List Char` as a quoted string.
fn render_list(mut value: &Value, elem_ty: &Type, env: &LintEnv) -> Option<String> {
    let mut heads: Vec<&Value> = Vec::new();
    loop {
        match value {
            Value::Con(name, args) if name == "Cons" => {
                let [head, tail] = &args[..] else {
                    return None;
                };
                heads.push(head);
                value = tail;
            }
            Value::Con(name, args) if name == "Nil" && args.is_empty() => break,
            _ => return None,
        }
    }

    if matches!(elem_ty, Type::Base(BaseType::Char)) {
        let mut s = String::new();
        for head in &heads {
            let Value::Char(c) = head else {
                return Some(render_list_generic(&heads, elem_ty, env));
            };
            s.push(*c);
        }
        return Some(format!("{s:?}"));
    }

    Some(render_list_generic(&heads, elem_ty, env))
}

fn render_list_generic(heads: &[&Value], elem_ty: &Type, env: &LintEnv) -> String {
    let rendered: Vec<String> = heads
        .iter()
        .map(|h| render_value(h, elem_ty, env))
        .collect();
    format!("[{}]", rendered.join(", "))
}

#[cfg(test)]
mod tests {
    use super::render_value;
    use crate::base_type::BaseType;
    use crate::eval::Value;
    use crate::lint::{DataCon, DataType, LintEnv};
    use crate::ty::Type;

    fn list_env() -> LintEnv {
        let list = DataType {
            name: "List".to_string(),
            params: vec!["a".to_string()],
            ctors: vec![
                DataCon {
                    name: "Nil".to_string(),
                    fields: vec![],
                },
                DataCon {
                    name: "Cons".to_string(),
                    fields: vec![
                        Type::Var("a".to_string()),
                        Type::App(
                            Box::new(Type::Con("List".to_string())),
                            Box::new(Type::Var("a".to_string())),
                        ),
                    ],
                },
            ],
        };
        let pair = DataType {
            name: "(,)".to_string(),
            params: vec!["a".to_string(), "b".to_string()],
            ctors: vec![DataCon {
                name: "(,)".to_string(),
                fields: vec![Type::Var("a".to_string()), Type::Var("b".to_string())],
            }],
        };
        let record = DataType {
            name: "{x,y}".to_string(),
            params: vec![],
            ctors: vec![DataCon {
                name: "{x,y}".to_string(),
                fields: vec![Type::Base(BaseType::Int), Type::Base(BaseType::Bool)],
            }],
        };
        LintEnv::new()
            .with_data(list)
            .with_data(pair)
            .with_data(record)
    }

    fn list_ty(t: Type) -> Type {
        Type::App(Box::new(Type::Con("List".to_string())), Box::new(t))
    }

    fn pair_ty(l: Type, r: Type) -> Type {
        Type::App(
            Box::new(Type::App(
                Box::new(Type::Con("(,)".to_string())),
                Box::new(l),
            )),
            Box::new(r),
        )
    }

    fn cons(h: Value, t: Value) -> Value {
        Value::Con("Cons".to_string(), vec![h, t])
    }

    fn nil() -> Value {
        Value::Con("Nil".to_string(), vec![])
    }

    #[test]
    fn renders_base_values() {
        let env = list_env();
        assert_eq!(
            render_value(&Value::Int(7), &Type::Base(BaseType::Int), &env),
            "7"
        );
        assert_eq!(
            render_value(&Value::Bool(true), &Type::Base(BaseType::Bool), &env),
            "true"
        );
        assert_eq!(
            render_value(&Value::Float(2.5), &Type::Base(BaseType::Float), &env),
            "2.5"
        );
        assert_eq!(
            render_value(&Value::Unit, &Type::Base(BaseType::Unit), &env),
            "()"
        );
        assert_eq!(
            render_value(&Value::Char('a'), &Type::Base(BaseType::Char), &env),
            "'a'"
        );
        assert_eq!(
            render_value(&Value::Char('\n'), &Type::Base(BaseType::Char), &env),
            "'\\n'"
        );
    }

    #[test]
    fn renders_char_list_as_string() {
        let env = list_env();
        let v = cons(Value::Char('a'), cons(Value::Char('b'), nil()));
        assert_eq!(
            render_value(&v, &list_ty(Type::Base(BaseType::Char)), &env),
            "\"ab\""
        );
    }

    #[test]
    fn renders_empty_char_list_as_empty_string() {
        let env = list_env();
        assert_eq!(
            render_value(&nil(), &list_ty(Type::Base(BaseType::Char)), &env),
            "\"\""
        );
    }

    #[test]
    fn renders_int_list() {
        let env = list_env();
        let v = cons(Value::Int(1), cons(Value::Int(2), nil()));
        assert_eq!(
            render_value(&v, &list_ty(Type::Base(BaseType::Int)), &env),
            "[1, 2]"
        );
    }

    #[test]
    fn renders_empty_int_list() {
        let env = list_env();
        assert_eq!(
            render_value(&nil(), &list_ty(Type::Base(BaseType::Int)), &env),
            "[]"
        );
    }

    #[test]
    fn renders_tuple() {
        let env = list_env();
        let v = Value::Con("(,)".to_string(), vec![Value::Int(1), Value::Bool(true)]);
        assert_eq!(
            render_value(
                &v,
                &pair_ty(Type::Base(BaseType::Int), Type::Base(BaseType::Bool)),
                &env
            ),
            "(1, true)"
        );
    }

    #[test]
    fn renders_record() {
        let env = list_env();
        let v = Value::Con("{x,y}".to_string(), vec![Value::Int(1), Value::Bool(false)]);
        assert_eq!(
            render_value(&v, &Type::Con("{x,y}".to_string()), &env),
            "{x: 1, y: false}"
        );
    }

    #[test]
    fn renders_forall_nil() {
        let env = list_env();
        let ty = Type::Forall(
            "a".to_string(),
            Box::new(list_ty(Type::Var("a".to_string()))),
        );
        assert_eq!(render_value(&nil(), &ty, &env), "[]");
    }

    #[test]
    fn strips_all_leading_foralls() {
        let env = list_env();
        let v = Value::Con("(,)".to_string(), vec![nil(), nil()]);
        let ty = Type::Forall(
            "a".to_string(),
            Box::new(Type::Forall(
                "b".to_string(),
                Box::new(pair_ty(
                    list_ty(Type::Var("a".to_string())),
                    list_ty(Type::Var("b".to_string())),
                )),
            )),
        );
        assert_eq!(render_value(&v, &ty, &env), "([], [])");
    }

    #[test]
    fn simultaneous_substitution_avoids_capture() {
        let env = list_env();
        let v = Value::Con(
            "(,)".to_string(),
            vec![cons(Value::Char('a'), nil()), cons(Value::Char('b'), nil())],
        );
        let ty = pair_ty(
            Type::Var("b".to_string()),
            list_ty(Type::Base(BaseType::Char)),
        );
        assert_eq!(render_value(&v, &ty, &env), "(Cons('a', Nil), \"b\")");
    }

    #[test]
    fn falls_back_on_shape_mismatch() {
        let env = list_env();
        assert_eq!(
            render_value(&Value::Int(3), &Type::Base(BaseType::Bool), &env),
            "3"
        );
        let v = Value::Con("Foo".to_string(), vec![]);
        assert_eq!(render_value(&v, &Type::Con("Foo".to_string()), &env), "Foo");
    }
}
