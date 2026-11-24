//! Sound match-coverage checking over typed patterns, using a usefulness algorithm on a pattern matrix.

use crate::ast::Literal;
use crate::typechecker::TypeEnv;
use crate::typed_ast::{TypedPattern, TypedPatternKind};
use crate::types::Type;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Ctor {
    Bool(bool),
    Unit,
    Tuple(usize),
    Record(Vec<String>),
    Named(String, usize),
    Lit,
}

#[derive(Clone, Debug)]
enum Pat {
    Wild(Option<Type>),
    Con(Ctor, Vec<Pat>, Type),
}

type Row = Vec<Pat>;

#[derive(Clone, Debug)]
enum Witness {
    Wild,
    Con(Ctor, Vec<Witness>),
}

fn lower(p: &TypedPattern) -> Pat {
    let ty = p.ty.clone();
    match &p.kind {
        TypedPatternKind::Literal(Literal::Bool(b)) => Pat::Con(Ctor::Bool(*b), vec![], ty),
        TypedPatternKind::Literal(_) => Pat::Con(Ctor::Lit, vec![], ty),
        TypedPatternKind::Var(_) | TypedPatternKind::Wildcard => Pat::Wild(Some(ty)),
        TypedPatternKind::Tuple(ps) => {
            if ps.is_empty() {
                Pat::Con(Ctor::Unit, vec![], ty)
            } else {
                Pat::Con(Ctor::Tuple(ps.len()), ps.iter().map(lower).collect(), ty)
            }
        }
        TypedPatternKind::Constructor(name, ps) => Pat::Con(
            Ctor::Named(name.clone(), ps.len()),
            ps.iter().map(lower).collect(),
            ty,
        ),
        TypedPatternKind::Record(fs) => {
            let mut fields: Vec<String> = if let Type::Record(m) = &p.ty {
                m.keys().cloned().collect()
            } else {
                fs.iter().map(|(n, _)| n.clone()).collect()
            };
            fields.sort();
            fields.dedup();
            let subs: Vec<Pat> = fields
                .iter()
                .map(|f| {
                    fs.iter()
                        .find(|(n, _)| n == f)
                        .map_or(Pat::Wild(None), |(_, sp)| lower(sp))
                })
                .collect();
            Pat::Con(Ctor::Record(fields), subs, ty)
        }
    }
}

fn ty_of_column(rows: &[Row], fallback: Option<&Type>) -> Option<Type> {
    rows.iter()
        .find_map(|r| match r.first() {
            Some(Pat::Wild(Some(t)) | Pat::Con(_, _, t)) => Some(t.clone()),
            _ => None,
        })
        .or_else(|| fallback.cloned())
}

fn signature(ty: Option<&Type>, env: &TypeEnv) -> Option<Vec<Ctor>> {
    match ty? {
        Type::Bool => Some(vec![Ctor::Bool(false), Ctor::Bool(true)]),
        Type::Unit => Some(vec![Ctor::Unit]),
        Type::Tuple(ts) => Some(vec![Ctor::Tuple(ts.len())]),
        Type::Record(m) => {
            let mut keys: Vec<String> = m.keys().cloned().collect();
            keys.sort();
            Some(vec![Ctor::Record(keys)])
        }
        Type::SumType(n, _) => {
            let ctors = env.constructors_of(n);
            if ctors.is_empty() {
                None
            } else {
                Some(
                    ctors
                        .into_iter()
                        .map(|(name, k)| Ctor::Named(name, k))
                        .collect(),
                )
            }
        }
        _ => None,
    }
}

fn arity(c: &Ctor) -> usize {
    match c {
        Ctor::Bool(_) | Ctor::Unit | Ctor::Lit => 0,
        Ctor::Tuple(n) | Ctor::Named(_, n) => *n,
        Ctor::Record(fields) => fields.len(),
    }
}

fn sub_types(c: &Ctor, col_ty: Option<&Type>, rows: &[Row]) -> Vec<Option<Type>> {
    let k = arity(c);
    match (c, col_ty) {
        (Ctor::Tuple(_), Some(Type::Tuple(ts))) => ts.iter().cloned().map(Some).collect(),
        (Ctor::Record(fields), Some(Type::Record(m))) => {
            fields.iter().map(|f| m.get(f).cloned()).collect()
        }
        (Ctor::Tuple(_) | Ctor::Record(_), _) => (0..k).map(|_| None).collect(),
        _ => (0..k)
            .map(|i| {
                rows.iter().find_map(|r| match r.first() {
                    Some(Pat::Con(c2, subs, _)) if c2 == c => match subs.get(i) {
                        Some(Pat::Wild(Some(t)) | Pat::Con(_, _, t)) => Some(t.clone()),
                        _ => None,
                    },
                    _ => None,
                })
            })
            .collect(),
    }
}

fn specialise(rows: &[Row], c: &Ctor, k: usize) -> Vec<Row> {
    rows.iter()
        .filter_map(|r| {
            let (head, rest) = r.split_first()?;
            match head {
                Pat::Con(c2, subs, _) if c2 == c => {
                    Some(subs.iter().cloned().chain(rest.iter().cloned()).collect())
                }
                Pat::Wild(_) => Some(
                    (0..k)
                        .map(|_| Pat::Wild(None))
                        .chain(rest.iter().cloned())
                        .collect(),
                ),
                Pat::Con(..) => None,
            }
        })
        .collect()
}

fn default_rows(rows: &[Row]) -> Vec<Row> {
    rows.iter()
        .filter_map(|r| match r.split_first() {
            Some((Pat::Wild(_), rest)) => Some(rest.to_vec()),
            _ => None,
        })
        .collect()
}

/// Usefulness of a wildcard row against `rows` (Maranget): returns a witness
/// vector (one entry per column in `tys`) for a value no row matches, or
/// `None` when the rows are exhaustive. A column is split on its full
/// signature only when the row heads mention every constructor of it;
/// otherwise the default matrix is used, so witnesses stay minimal.
fn useful(rows: &[Row], tys: &[Option<Type>], env: &TypeEnv) -> Option<Vec<Witness>> {
    let Some((head_ty, rest_tys)) = tys.split_first() else {
        return if rows.is_empty() { Some(vec![]) } else { None };
    };
    let col = ty_of_column(rows, head_ty.as_ref());
    let present: Vec<&Ctor> = rows
        .iter()
        .filter_map(|r| match r.first() {
            Some(Pat::Con(c, _, _)) => Some(c),
            _ => None,
        })
        .collect();
    let head = match signature(col.as_ref(), env) {
        Some(sig) => {
            let missing: Option<Ctor> = sig.iter().find(|c| !present.contains(c)).cloned();
            match missing {
                None => return useful_complete(rows, sig, col.as_ref(), rest_tys, env),
                Some(m) if !present.is_empty() => {
                    let k = arity(&m);
                    Witness::Con(m, vec![Witness::Wild; k])
                }
                Some(_) => Witness::Wild,
            }
        }
        None => Witness::Wild,
    };
    let w = useful(&default_rows(rows), rest_tys, env)?;
    let mut out = vec![head];
    out.extend(w);
    Some(out)
}

/// The complete-signature case of `useful`: specialise the first column on
/// every constructor of `sig` and return the first witness found.
fn useful_complete(
    rows: &[Row],
    sig: Vec<Ctor>,
    col: Option<&Type>,
    rest_tys: &[Option<Type>],
    env: &TypeEnv,
) -> Option<Vec<Witness>> {
    for c in sig {
        let k = arity(&c);
        let st = sub_types(&c, col, rows);
        let next_tys: Vec<Option<Type>> = st.into_iter().chain(rest_tys.iter().cloned()).collect();
        if let Some(w) = useful(&specialise(rows, &c, k), &next_tys, env) {
            let (subs, rest) = w.split_at(k.min(w.len()));
            let mut out = vec![Witness::Con(c, subs.to_vec())];
            out.extend(rest.iter().cloned());
            return Some(out);
        }
    }
    None
}

fn render(w: &Witness) -> String {
    match w {
        Witness::Wild => "_".to_string(),
        Witness::Con(c, subs) => match c {
            Ctor::Bool(b) => b.to_string(),
            Ctor::Unit => "()".to_string(),
            Ctor::Tuple(_) => {
                let parts: Vec<String> = subs.iter().map(render).collect();
                format!("({})", parts.join(", "))
            }
            Ctor::Record(fields) => {
                let parts: Vec<String> = fields
                    .iter()
                    .zip(subs.iter())
                    .map(|(f, s)| format!("{f}: {}", render(s)))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
            Ctor::Named(name, _) => {
                if subs.is_empty() {
                    name.clone()
                } else {
                    let parts: Vec<String> = subs
                        .iter()
                        .map(|s| match s {
                            Witness::Con(Ctor::Named(_, _), inner) if !inner.is_empty() => {
                                format!("({})", render(s))
                            }
                            _ => render(s),
                        })
                        .collect();
                    format!("{name} {}", parts.join(" "))
                }
            }
            Ctor::Lit => "_".to_string(),
        },
    }
}

/// Checks that `patterns` cover every value of `scrutinee_ty`.
///
/// # Errors
///
/// Returns rendered witnesses of uncovered values.
pub fn check_match(
    scrutinee_ty: &Type,
    patterns: &[&TypedPattern],
    env: &TypeEnv,
) -> Result<(), Vec<String>> {
    let rows: Vec<Row> = patterns.iter().map(|p| vec![lower(p)]).collect();
    match useful(&rows, &[Some(scrutinee_ty.clone())], env) {
        None => Ok(()),
        Some(w) => Err(vec![w.first().map_or_else(|| "_".to_string(), render)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typechecker::ConstructorInfo;
    use std::collections::HashMap;

    fn pat(kind: TypedPatternKind, ty: &Type) -> TypedPattern {
        TypedPattern {
            kind,
            ty: ty.clone(),
        }
    }

    fn wild(ty: &Type) -> TypedPattern {
        pat(TypedPatternKind::Wildcard, ty)
    }

    fn var(name: &str, ty: &Type) -> TypedPattern {
        pat(TypedPatternKind::Var(name.to_string()), ty)
    }

    fn con(name: &str, args: Vec<TypedPattern>, ty: &Type) -> TypedPattern {
        pat(TypedPatternKind::Constructor(name.to_string(), args), ty)
    }

    fn register(env: &mut TypeEnv, name: &str, k: usize, sum: &str) {
        env.register_constructor(
            name.to_string(),
            ConstructorInfo {
                type_params: vec!["a".to_string()],
                payload_types: vec![crate::ast::TypeAnnotation::Var("a".to_string()); k],
                sum_type_name: sum.to_string(),
            },
        );
    }

    fn test_env() -> TypeEnv {
        let mut env = TypeEnv::new();
        register(&mut env, "Some", 1, "Option");
        register(&mut env, "None", 0, "Option");
        register(&mut env, "Cons", 2, "List");
        register(&mut env, "Nil", 0, "List");
        env
    }

    fn list_ty() -> Type {
        Type::SumType("List".to_string(), vec![Type::Int])
    }

    fn tup(a: TypedPattern, b: TypedPattern, ty: &Type) -> TypedPattern {
        pat(TypedPatternKind::Tuple(vec![a, b]), ty)
    }

    fn run(ty: &Type, pats: &[TypedPattern]) -> Result<(), Vec<String>> {
        let refs: Vec<&TypedPattern> = pats.iter().collect();
        check_match(ty, &refs, &test_env())
    }

    fn rec_ty() -> Type {
        let mut m = HashMap::new();
        m.insert("x".to_string(), Type::Int);
        m.insert("y".to_string(), Type::Int);
        Type::Record(m)
    }

    fn lit_int(n: i64, ty: &Type) -> TypedPattern {
        pat(TypedPatternKind::Literal(Literal::Int(n)), ty)
    }

    #[test]
    fn nested_option_gap() {
        let inner = Type::SumType("Option".to_string(), vec![Type::Int]);
        let outer = Type::SumType("Option".to_string(), vec![inner.clone()]);
        let some_some = con(
            "Some",
            vec![con("Some", vec![var("n", &Type::Int)], &inner)],
            &outer,
        );
        let none = con("None", vec![], &outer);
        assert_eq!(
            run(&outer, &[some_some, none]),
            Err(vec!["Some None".to_string()])
        );
    }

    #[test]
    fn char_literals_only() {
        let pats = [
            pat(TypedPatternKind::Literal(Literal::Char('a')), &Type::Char),
            pat(TypedPatternKind::Literal(Literal::Char('b')), &Type::Char),
        ];
        assert_eq!(run(&Type::Char, &pats), Err(vec!["_".to_string()]));
    }

    #[test]
    fn int_literal_without_catch_all() {
        let pats = [lit_int(0, &Type::Int)];
        assert_eq!(run(&Type::Int, &pats), Err(vec!["_".to_string()]));
    }

    #[test]
    fn tuple_of_lists_gap() {
        let l = list_ty();
        let t = Type::Tuple(vec![l.clone(), l.clone()]);
        let nil = || con("Nil", vec![], &l);
        let cons = || con("Cons", vec![wild(&Type::Int), wild(&l)], &l);
        let pats = [
            tup(nil(), nil(), &t),
            tup(nil(), cons(), &t),
            tup(cons(), cons(), &t),
        ];
        assert_eq!(run(&t, &pats), Err(vec!["(Cons _ _, Nil)".to_string()]));
    }

    #[test]
    fn strcmp_shape_ok() {
        let l = list_ty();
        let t = Type::Tuple(vec![l.clone(), l.clone()]);
        let nil = || con("Nil", vec![], &l);
        let cons = |n: &str| con("Cons", vec![var(n, &Type::Int), wild(&l)], &l);
        let pats = [
            tup(nil(), nil(), &t),
            tup(nil(), wild(&l), &t),
            tup(wild(&l), nil(), &t),
            tup(cons("a"), cons("b"), &t),
        ];
        assert_eq!(run(&t, &pats), Ok(()));
    }

    #[test]
    fn partial_record_with_catch_all_ok() {
        let t = rec_ty();
        let r = pat(
            TypedPatternKind::Record(vec![("x".to_string(), lit_int(1, &Type::Int))]),
            &t,
        );
        assert_eq!(run(&t, &[r, wild(&t)]), Ok(()));
    }

    #[test]
    fn partial_record_patterns_cover_all_ok() {
        let t = rec_ty();
        let rx = pat(
            TypedPatternKind::Record(vec![("x".to_string(), lit_int(1, &Type::Int))]),
            &t,
        );
        let ry = pat(
            TypedPatternKind::Record(vec![("y".to_string(), var("n", &Type::Int))]),
            &t,
        );
        assert_eq!(run(&t, &[rx, ry]), Ok(()));
    }

    #[test]
    fn bool_both_arms_ok() {
        let pats = [
            pat(TypedPatternKind::Literal(Literal::Bool(true)), &Type::Bool),
            pat(TypedPatternKind::Literal(Literal::Bool(false)), &Type::Bool),
        ];
        assert_eq!(run(&Type::Bool, &pats), Ok(()));
    }

    #[test]
    fn unit_pattern_ok() {
        let pats = [pat(TypedPatternKind::Tuple(vec![]), &Type::Unit)];
        assert_eq!(run(&Type::Unit, &pats), Ok(()));
    }
}
