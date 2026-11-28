//! Shared Core pipeline used by the parlang and parlang-driver binaries: elaborate, lint, run, dump and trace on Core.

use std::fmt::Write;

use crate::dictionary::{erase_program, insert_dictionaries, EraseError};
use crate::typechecker::TypeEnv;
use crate::typed_ast::{TypedDecl, TypedProgram};
use crate::{
    check_elaborated, elaborate_program, typecheck_program_typed_with_env, Elaborated, Program,
};
use parlang_core::{
    check_function, core_to_dot, eval, reduction_terms, render_value, trace, Environment,
    EvalError, Report, Type,
};

/// Elaborates `program` to Core without running Core Lint: type checks it,
/// rejects a type-class program containing `load` with the same text as
/// every other front end (see [`crate::typed_ast::class_load_conflict`]),
/// then inserts dictionaries and elaborates the translation.
fn elaborate_only(program: &Program) -> Result<Elaborated, String> {
    let mut env = TypeEnv::new();
    let typed = typecheck_program_typed_with_env(program, &mut env)
        .map_err(|e| format!("type error: {e}"))?;
    if let Some(reason) = crate::typed_ast::class_load_conflict(&typed) {
        return Err(reason);
    }
    elaborate_translated(&typed, &env)
}

/// Inserts dictionaries into `typed`, checked in `env`, and elaborates the
/// dictionary-explicit result to Core, without the type-class guard and
/// without Core Lint. A program without classes elaborates exactly as
/// before, because insertion leaves it unchanged.
///
/// # Errors
///
/// `dictionary insertion error: <e>` when insertion fails and
/// `elaboration error: <e>` when elaboration fails.
pub(crate) fn elaborate_translated(
    typed: &TypedProgram,
    env: &TypeEnv,
) -> Result<Elaborated, String> {
    let translated =
        insert_dictionaries(typed, env).map_err(|e| format!("dictionary insertion error: {e}"))?;
    elaborate_program(&translated).map_err(|e| format!("elaboration error: {e}"))
}

/// Elaborates `program` to Core and runs Core Lint, returning the elaborated
/// program together with its synthesized Core type.
fn elaborate_and_lint(program: &Program) -> Result<(Elaborated, Type), String> {
    let elab = elaborate_only(program)?;
    let ty = check_elaborated(&elab).map_err(|e| format!("lint error: {e}"))?;
    Ok((elab, ty))
}

/// Appends a trailing newline to `text` unless it already ends with one.
fn with_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// The untyped program the surface evaluator runs for the type-checked
/// `program`: the dictionary translation of `typed`, erased back to an
/// untyped `Program`, when dictionary insertion changed anything or `typed`
/// declares a class (erasure drops `class` declarations, which the surface
/// evaluator rejects); else `program` itself. A translation containing
/// `load` also keeps `program`,
/// because the untyped loaded body never receives dictionaries.
///
/// `env` is the type environment `typed` was checked in, after checking.
///
/// # Errors
///
/// `dictionary insertion error: <e>` when insertion fails and
/// `dictionary erasure error: <e>` when erasure fails.
pub fn surface_program(
    program: &Program,
    typed: &TypedProgram,
    env: &TypeEnv,
) -> Result<Program, String> {
    let translated =
        insert_dictionaries(typed, env).map_err(|e| format!("dictionary insertion error: {e}"))?;
    let declares_class = typed
        .decls
        .iter()
        .any(|decl| matches!(decl, TypedDecl::Class { .. }));
    if translated == *typed && !declares_class {
        return Ok(program.clone());
    }
    match erase_program(&translated) {
        Ok(erased) => Ok(erased),
        Err(EraseError::Load(_)) => Ok(program.clone()),
        Err(error) => Err(format!("dictionary erasure error: {error}")),
    }
}

/// Elaborates and Core-Lint-checks `program`, returning the elaborated Core program.
///
/// # Errors
///
/// Returns an error string if `program` fails to type-check, fails to
/// elaborate into Core, or fails Core Lint.
pub fn core_of_program(program: &Program) -> Result<Elaborated, String> {
    elaborate_and_lint(program).map(|(elab, _)| elab)
}

/// Elaborates, lints, and evaluates `program` on the Core VM, returning the
/// rendered result value.
///
/// Elaborated programs never reference prelude bindings such as `not`, so no
/// prelude environment is threaded in here. Core `eval` runs on the CEK
/// machine, which has no native recursion, so this call runs directly on the
/// calling thread rather than via `run_on_evaluator_stack`; the renderer
/// likewise walks list spines iteratively.
///
/// # Errors
///
/// Returns an error string if `program` fails to type-check, fails to
/// elaborate into Core, fails Core Lint, or fails to evaluate.
pub fn run_core(program: &Program) -> Result<String, String> {
    let elab = core_of_program(program)?;
    let value = eval(&elab.term, &Environment::new()).map_err(|e| format!("eval error: {e}"))?;
    Ok(with_newline(render_value(&value, &elab.ty, &elab.env)))
}

/// Elaborates `program` to Core and dumps it as text or DOT, skipping Core
/// Lint so a term that fails lint can still be inspected.
///
/// # Errors
///
/// Returns an error string if `program` fails to type-check or fails to
/// elaborate into Core.
pub fn dump_core(program: &Program, dot: bool) -> Result<String, String> {
    let elab = elaborate_only(program)?;
    if dot {
        Ok(with_newline(core_to_dot(&elab.term)))
    } else {
        Ok(with_newline(elab.term.to_text()))
    }
}

/// Elaborates `program` to Core, runs Core Lint, and dumps either a
/// confirmation of the synthesized type or the Core term as DOT.
///
/// # Errors
///
/// Returns an error string if `program` fails to type-check, fails to
/// elaborate into Core, or fails Core Lint.
pub fn dump_lint(program: &Program, dot: bool) -> Result<String, String> {
    let (elab, ty) = elaborate_and_lint(program)?;
    if dot {
        Ok(with_newline(core_to_dot(&elab.term)))
    } else {
        Ok(format!("lint ok: {ty}\n"))
    }
}

/// Elaborates, lints, and traces `program` on the Core VM up to `fuel`
/// transitions, returning a numbered listing of its reductions (or, with
/// `raw` set, of every raw machine transition) followed by the final result.
///
/// One numbered entry may span several lines, because Core term and
/// machine-state text can contain newlines; only the entry's first line
/// carries the `{i}: ` prefix.
///
/// # Errors
///
/// Returns an error string if `program` fails to type-check, fails to
/// elaborate into Core, fails Core Lint, or if tracing/evaluation fails
/// (including exhausting `fuel`).
pub fn trace_core(program: &Program, fuel: usize, raw: bool) -> Result<String, String> {
    let elab = core_of_program(program)?;
    let steps = trace(&elab.term, &Environment::new(), Some(fuel)).map_err(|e| match e {
        EvalError::StepLimit => format!("trace error: step limit of {fuel} exceeded"),
        other => format!("trace error: {other}"),
    })?;

    let mut out = String::new();
    if raw {
        for (i, step) in steps.iter().enumerate() {
            let _ = writeln!(out, "{i}: [{}] {}", step.kind, step.state);
        }
    } else {
        for (i, text) in reduction_terms(&steps).into_iter().enumerate() {
            let _ = writeln!(out, "{i}: {text}");
        }
    }

    // The final `Halt` entry holds `Return(value)`; reuse it instead of re-running.
    let halted = steps.last().map(|step| &step.state.control);
    let Some(parlang_core::machine::Control::Return(value)) = halted else {
        return Err("trace error: trace did not halt with a value".to_string());
    };
    let rendered = render_value(value, &elab.ty, &elab.env);
    let _ = writeln!(out, "result: {rendered}");

    Ok(out)
}

/// Elaborates and lints `program`, then checks whether the top-level function
/// `name` is kernel-eligible under the slice-8 predicate (rules `R1`-`R5`).
///
/// Returns `eligible: <name>` plus a newline when the function passes.
///
/// # Errors
///
/// Returns an error string if `program` fails to type-check, elaborate or
/// lint; `kernel-check error: <e>` if the eligibility check itself fails
/// (for example an unknown function); or a `rejected: <name>` header
/// followed by one `  <id> <rule>: <function>: <node>` line per violation.
pub fn kernel_check(program: &Program, name: &str) -> Result<String, String> {
    let elab = core_of_program(program)?;
    let report = check_function(&elab.term, &elab.env, name, &[])
        .map_err(|e| format!("kernel-check error: {e}"))?;
    match report {
        Report::Eligible => Ok(format!("eligible: {name}\n")),
        Report::Rejected(violations) => {
            let mut lines = vec![format!("rejected: {name}")];
            for v in &violations {
                lines.push(format!(
                    "  {} {}: {}: {}",
                    v.rule.id(),
                    v.rule.name(),
                    v.function,
                    v.node
                ));
            }
            Err(lines.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(src: &str) -> Program {
        crate::parse_program(src).expect("parses")
    }

    fn check_trace_shape(out: &str) {
        let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
        assert!(lines[0].starts_with("0: "), "first line: {}", lines[0]);
        assert!(
            lines.iter().any(|l| l.starts_with("1: ")),
            "no line starting with '1: ' in {out}"
        );
        let last = *lines.last().expect("non-empty trace");
        assert_eq!(last, "result: 7");
        assert!(
            lines[..lines.len() - 1].iter().any(|l| l.contains('7')),
            "no pre-result line contains 7 in {out}"
        );
    }

    #[test]
    fn run_core_arithmetic() {
        assert_eq!(run_core(&prog("1 + 2 * 3")), Ok("7\n".to_string()));
    }

    #[test]
    fn run_core_polymorphic_tuple() {
        assert_eq!(
            run_core(&prog("let id = fun x -> x in (id 1, id true)")),
            Ok("(1, true)\n".to_string())
        );
    }

    #[test]
    fn run_core_string() {
        let src = "data List a = Nil | Cons a (List a) ;\nlet keep = fun s -> s in keep \"ab\"";
        assert_eq!(run_core(&prog(src)), Ok("\"ab\"\n".to_string()));
    }

    #[test]
    fn run_core_type_error() {
        let result = run_core(&prog("1 + true"));
        assert!(matches!(result, Err(ref e) if e.starts_with("type error: ")));
    }

    #[test]
    fn core_runs_type_classes_and_rejects_them_with_load() {
        assert_eq!(run_core(&prog("eq 1 2")), Ok("false\n".to_string()));
        assert!(dump_core(&prog("eq 1 2"), false).is_ok());
        assert_eq!(
            run_core(&prog("class Show a where { show : a -> Int };\n0")),
            Ok("0\n".to_string())
        );
        assert_eq!(run_core(&prog("1 == 2")), Ok("false\n".to_string()));
        let conflict = "class Show a where { show : a -> Int };\nload \"nofile.par\" in 0";
        let expected = "type classes cannot be combined with load yet: class Show".to_string();
        assert_eq!(run_core(&prog(conflict)), Err(expected.clone()));
        assert_eq!(dump_core(&prog(conflict), false), Err(expected));
    }

    #[test]
    fn run_core_unsupported_load() {
        let result = run_core(&prog("load \"nowhere.par\" in 42"));
        assert!(matches!(result, Err(ref e) if e.starts_with("elaboration error: ")));
    }

    #[test]
    fn dump_core_text_identity() {
        let result = dump_core(&prog("(fun x -> x) 100"), false).expect("dump ok");
        assert!(result.contains("100"));
        assert!(result.ends_with('\n'));
    }

    #[test]
    fn dump_core_dot() {
        let result = dump_core(&prog("(fun x -> x) 100"), true).expect("dump ok");
        assert!(result.contains("digraph"));
    }

    #[test]
    fn dump_lint_text() {
        assert_eq!(
            dump_lint(&prog("1 + 2"), false),
            Ok("lint ok: Int\n".to_string())
        );
    }

    #[test]
    fn trace_core_reductions() {
        let out = trace_core(&prog("1 + 2 * 3"), 10_000, false).expect("trace ok");
        check_trace_shape(&out);
    }

    #[test]
    fn trace_core_raw() {
        let out = trace_core(&prog("1 + 2 * 3"), 10_000, true).expect("trace ok");
        let lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
        assert!(lines[0].starts_with("0: ["));
        assert!(lines.iter().any(|l| l.starts_with("1: [")));
        let last = *lines.last().expect("non-empty trace");
        assert_eq!(last, "result: 7");
    }

    #[test]
    fn trace_core_exact_reductions() {
        assert_eq!(
            trace_core(&prog("1 + 2"), 10_000, false),
            Ok("0: +(1, 2)\n1: 3\nresult: 3\n".to_string())
        );
    }

    #[test]
    fn trace_core_exact_raw() {
        let expected = concat!(
            "0: [push] <eval +(1, 2) | kont: []>\n",
            "1: [construct] <eval 1 | kont: [PrimArgs(+, [], [2])]>\n",
            "2: [push] <return 1 | kont: [PrimArgs(+, [], [2])]>\n",
            "3: [construct] <eval 2 | kont: [PrimArgs(+, [1], [])]>\n",
            "4: [delta] <return 2 | kont: [PrimArgs(+, [1], [])]>\n",
            "5: [halt] <return 3 | kont: []>\n",
            "result: 3\n",
        );
        assert_eq!(
            trace_core(&prog("1 + 2"), 10_000, true),
            Ok(expected.to_string())
        );
    }

    #[test]
    fn trace_core_fuel_boundary() {
        // Five transitions plus the halting step need exactly six units of fuel.
        assert_eq!(
            trace_core(&prog("1 + 2"), 6, false),
            Ok("0: +(1, 2)\n1: 3\nresult: 3\n".to_string())
        );
        assert_eq!(
            trace_core(&prog("1 + 2"), 5, false),
            Err("trace error: step limit of 5 exceeded".to_string())
        );
    }

    #[test]
    fn trace_core_step_limit() {
        assert_eq!(
            trace_core(&prog("1 + 2 * 3"), 1, false),
            Err("trace error: step limit of 1 exceeded".to_string())
        );
    }

    #[test]
    fn kernel_check_eligible_function() {
        let out = kernel_check(&prog("let inc = fun x -> x + 1;\n0"), "inc");
        assert_eq!(out, Ok("eligible: inc\n".to_string()));
    }

    #[test]
    fn kernel_check_rejects_recursion() {
        let src = "let factorial = rec f -> fun n ->\n  if n == 0 then 1 else n * f (n - 1);\n0";
        let err = kernel_check(&prog(src), "factorial").expect_err("rejected");
        assert!(err.starts_with("rejected: factorial\n"), "{err}");
        assert!(
            err.contains("  R3 recursion: factorial: (f -(n, 1))"),
            "{err}"
        );
        assert!(!err.ends_with('\n'), "{err}");
    }

    #[test]
    fn kernel_check_unknown_function() {
        assert_eq!(
            kernel_check(&prog("1"), "nope"),
            Err("kernel-check error: unknown function: nope".to_string())
        );
    }

    /// `src` parsed, and the program `surface_program` selects for it.
    fn surface(src: &str) -> (Program, Program) {
        let program = prog(src);
        let mut env = TypeEnv::new();
        let typed =
            crate::typecheck_program_typed_with_env(&program, &mut env).expect("type checks");
        let runnable = surface_program(&program, &typed, &env).expect("surface program");
        (program, runnable)
    }

    #[test]
    fn surface_program_keeps_programs_without_dictionaries() {
        for src in ["1 + 2", "1 == 2", "let id = fun x -> x; id 3"] {
            let (program, runnable) = surface(src);
            assert_eq!(runnable, program, "{src}");
        }
    }

    #[test]
    fn surface_program_erases_inserted_dictionaries() {
        let (program, runnable) = surface("let same = fun x -> fun y -> x == y; same 1 2");
        assert_ne!(runnable, program);
        let first = runnable.decls.first();
        assert!(
            matches!(first, Some(crate::Decl::Let { name, .. }) if name == "$dict_Eq_Int"),
            "{first:?}"
        );
    }

    #[test]
    fn surface_program_keeps_programs_with_load() {
        let src = "let same = fun x -> fun y -> x == y;\nload \"nofile.par\" in same 1 1";
        let (program, runnable) = surface(src);
        assert_eq!(runnable, program);
    }

    #[test]
    fn surface_program_erases_class_declarations() {
        let (_, runnable) = surface("class Show a where { show : a -> Int };\n0");
        let has_class = runnable
            .decls
            .iter()
            .any(|decl| matches!(decl, crate::Decl::Class { .. }));
        assert!(!has_class, "{runnable:?}");
        let value = crate::eval_program(&runnable, &crate::Environment::new());
        assert_eq!(value, Ok(crate::Value::Int(0)));
    }

    #[test]
    fn surface_program_erases_instance_only_programs() {
        let src = "data Box a = MkBox a;\ninstance Eq a => Eq (Box a) where { eq = fun x -> fun y -> true };\n0";
        let (_, runnable) = surface(src);
        let has_instance = runnable
            .decls
            .iter()
            .any(|decl| matches!(decl, crate::Decl::Instance { .. }));
        assert!(!has_instance, "{runnable:?}");
        let value = crate::eval_program(&runnable, &crate::Environment::new());
        assert_eq!(value, Ok(crate::Value::Int(0)));
    }

    /// `src` type-checked, dictionary-translated and elaborated without the
    /// type-class guard, then checked by Core Lint.
    fn translated_core(src: &str) -> Elaborated {
        let program = prog(src);
        let mut env = TypeEnv::new();
        let typed = typecheck_program_typed_with_env(&program, &mut env).expect("type checks");
        let elab = elaborate_translated(&typed, &env).expect("elaborates");
        let checked = check_elaborated(&elab);
        assert!(checked.is_ok(), "{checked:?}\n{}", elab.term.to_text());
        elab
    }

    /// The rendered Core value of `src`, checked against the surface
    /// evaluator running the erased translation.
    fn class_value(src: &str) -> String {
        let elab = translated_core(src);
        let value = eval(&elab.term, &Environment::new()).expect("Core evaluates");
        let core = render_value(&value, &elab.ty, &elab.env);
        let (_, runnable) = surface(src);
        let expected =
            crate::eval_program(&runnable, &crate::Environment::new()).expect("surface evaluates");
        assert_eq!(core, expected.to_string(), "{src}");
        core
    }

    #[test]
    fn core_runs_instance_with_context() {
        let src = "data Box a = MkBox a;\ninstance Eq a => Eq (Box a) where { eq = fun x -> fun y -> (match x with | MkBox p -> (match y with | MkBox q -> eq p q)) };\nlet same = fun x -> fun y -> x == y;\n(if same (MkBox 1) (MkBox 2) then 10 else 0) + (if same (MkBox 3) (MkBox 3) then 1 else 0)";
        assert_eq!(class_value(src), "1");
    }

    #[test]
    fn core_runs_context_instance_using_neq() {
        let src = "data Box a = MkBox a;\ninstance Eq a => Eq (Box a) where { eq = fun x -> fun y -> (match x with | MkBox p -> (match y with | MkBox q -> if p != q then false else true)) };\nlet same = fun x -> fun y -> x == y;\n(if same (MkBox 1) (MkBox 2) then 10 else 0) + (if same (MkBox 3) (MkBox 3) then 1 else 0)";
        assert_eq!(class_value(src), "1");
    }

    #[test]
    fn core_runs_self_referencing_instance() {
        let src = "data Nat = Z | S Nat;\ninstance Eq Nat where { eq = fun x -> fun y -> (match x with | Z -> (match y with | Z -> true | S m -> false) | S n -> (match y with | Z -> false | S m -> eq n m)) };\n(if eq (S Z) (S Z) then 10 else 0) + (if eq (S Z) Z then 1 else 0) + (if (S Z) != Z then 100 else 0)";
        assert_eq!(class_value(src), "110");
    }

    #[test]
    fn core_selects_methods_in_class_order() {
        let src = "class Pair a where { first : a -> Int; second : a -> Int };\ninstance Pair Bool where { second = fun b -> 2; first = fun b -> 1 };\nfirst true + second true";
        assert_eq!(class_value(src), "3");
    }

    #[test]
    fn core_instantiates_extra_method_variables() {
        let src = "class Pick a where { pick : a -> b -> b -> b };\ninstance Pick Bool where { pick = fun c -> fun x -> fun y -> if c then x else y };\n(if pick false true false then 10 else 20) + pick true 1 2";
        assert_eq!(class_value(src), "21");
    }

    #[test]
    fn core_selects_extra_variable_method_at_a_type_variable_head() {
        let src = "class Pick a where { pick : a -> b -> b -> b };\ninstance Pick Bool where { pick = fun c -> fun x -> fun y -> if c then x else y };\nlet choose = fun c -> fun x -> fun y -> pick c x y;\nchoose true 1 2 + (if choose false true false then 10 else 20)";
        assert_eq!(class_value(src), "21");
    }

    #[test]
    fn core_runs_constrained_let_over_a_user_class() {
        let src = "class Size a where { size : a -> Int };\ninstance Size Bool where { size = fun b -> if b then 1 else 0 };\nlet twice = fun x -> size x + size x;\ntwice true";
        assert_eq!(class_value(src), "2");
    }

    /// `Option`, `List`, `Functor` and `Monad` with their four instances.
    const MONAD_DECLS: &str = "data Option a = None | Some a;\n\
        data List a = Nil | Cons a (List a);\n\
        class Functor f where { fmap : (a -> b) -> f a -> f b };\n\
        class Monad m where { return : a -> m a; bind : m a -> (a -> m b) -> m b };\n\
        let append = rec app -> fun xs -> fun ys -> (match xs with | Nil -> ys | Cons h t -> Cons h (app t ys));\n\
        let sumList = rec s -> fun xs -> (match xs with | Nil -> 0 | Cons h t -> h + s t);\n\
        let getOr = fun d -> fun o -> (match o with | None -> d | Some v -> v);\n\
        instance Functor Option where { fmap = fun g -> fun o -> (match o with | None -> None | Some v -> Some (g v)) };\n\
        instance Functor List where { fmap = fun g -> fun xs -> (match xs with | Nil -> Nil | Cons h t -> Cons (g h) (fmap g t)) };\n\
        instance Monad Option where { return = fun x -> Some x; bind = fun o -> fun k -> (match o with | None -> None | Some v -> k v) };\n\
        instance Monad List where { return = fun x -> Cons x Nil; bind = fun xs -> fun k -> (match xs with | Nil -> Nil | Cons h t -> append (k h) (bind t k)) };\n";

    #[test]
    fn core_runs_functor_option() {
        let src = format!("{MONAD_DECLS}getOr 0 (fmap (fun x -> x + 1) (Some 41))");
        assert_eq!(class_value(&src), "42");
    }

    #[test]
    fn core_runs_functor_list() {
        let src = format!("{MONAD_DECLS}sumList (fmap (fun x -> x * 10) (Cons 1 (Cons 2 Nil)))");
        assert_eq!(class_value(&src), "30");
    }

    #[test]
    fn core_runs_monad_option() {
        let src = format!(
            "{MONAD_DECLS}getOr 0 (bind (Some 3) (fun x -> return (x + 4))) + getOr 100 (bind None (fun x -> Some x))"
        );
        assert_eq!(class_value(&src), "107");
    }

    #[test]
    fn core_runs_monad_list() {
        let src = format!(
            "{MONAD_DECLS}sumList (bind (Cons 1 (Cons 2 Nil)) (fun x -> Cons x (Cons (x * 10) Nil))) + sumList (return 5)"
        );
        assert_eq!(class_value(&src), "38");
    }

    #[test]
    fn core_runs_do_option() {
        let src = format!(
            "{MONAD_DECLS}getOr 0 (do {{ x <- Some 3; y <- Some 4; return (x + y) }}) + getOr 100 (do {{ x <- None; return (x + 1) }})"
        );
        assert_eq!(class_value(&src), "107");
    }

    #[test]
    fn core_runs_do_list() {
        let src = format!(
            "{MONAD_DECLS}sumList (do {{ x <- Cons 1 (Cons 2 Nil); y <- Cons 10 (Cons 20 Nil); return (x + y) }})"
        );
        assert_eq!(class_value(&src), "66");
    }

    #[test]
    fn core_runs_do_let_and_bare_statements() {
        let src = format!(
            "{MONAD_DECLS}sumList (do {{ Cons 1 (Cons 2 Nil); let k = 5; return k }}) + getOr 0 (do {{ x <- Some 2; let y : Int = x * 10; return (y + x); }})"
        );
        assert_eq!(class_value(&src), "32");
    }

    #[test]
    fn core_runs_nested_do() {
        let src = format!(
            "{MONAD_DECLS}getOr 0 (do {{ x <- do {{ a <- Some 1; return (a + 1) }}; return (x * 3) }})"
        );
        assert_eq!(class_value(&src), "6");
    }

    #[test]
    fn do_uses_a_user_bind_in_scope() {
        let src = format!(
            "{MONAD_DECLS}getOr 0 (do {{ x <- Some 1; return x }}) + (let bind = fun m -> fun k -> k 100 in do {{ x <- 1; x + 1 }})"
        );
        assert_eq!(class_value(&src), "102");
    }

    #[test]
    fn core_runs_do_in_an_instance_method() {
        let src = format!(
            "{MONAD_DECLS}class Twice m where {{ twice : m Int -> m Int }};\ninstance Twice Option where {{ twice = fun o -> do {{ x <- o; return (x * 2) }} }};\ngetOr 0 (twice (Some 21))"
        );
        assert_eq!(class_value(&src), "42");
    }

    #[test]
    fn do_bind_shadows_an_outer_name() {
        let src = format!(
            "{MONAD_DECLS}let x = 100 in getOr 0 (do {{ x <- Some 1; return (x + 1) }}) + x"
        );
        assert_eq!(class_value(&src), "102");
    }

    #[test]
    fn do_without_bind_in_scope_is_unbound() {
        let result = crate::typechecker::typecheck_program(&prog("do { x <- 1; x }"));
        assert_eq!(
            result.err(),
            Some(crate::typechecker::TypeError::UnboundVariable(
                "bind".to_string()
            ))
        );
    }

    #[test]
    fn core_runs_constrained_let_over_a_constructor_class() {
        let src = format!(
            "{MONAD_DECLS}let incr = fun c -> fmap (fun x -> x + 1) c;\ngetOr 0 (incr (Some 1)) + sumList (incr (Cons 1 (Cons 2 Nil)))"
        );
        assert_eq!(class_value(&src), "7");
    }

    #[test]
    fn core_runs_constructor_class_instance_with_context() {
        let src = format!(
            "{MONAD_DECLS}data Wrap f a = MkWrap (f a);\ninstance Functor f => Functor (Wrap f) where {{ fmap = fun g -> fun w -> (match w with | MkWrap x -> MkWrap (fmap g x)) }};\nlet unwrap = fun w -> (match w with | MkWrap x -> x);\nsumList (unwrap (fmap (fun x -> x + 1) (MkWrap (Cons 1 (Cons 2 Nil)))))"
        );
        assert_eq!(class_value(&src), "5");
    }

    #[test]
    fn residual_monad_body_runs_on_surface_but_not_on_core() {
        let src = format!("{MONAD_DECLS}return 1");
        let (_, runnable) = surface(&src);
        let value = crate::eval_program(&runnable, &crate::Environment::new());
        assert!(value.is_ok(), "{value:?}");
        let core = run_core(&prog(&src));
        assert!(
            core.as_ref()
                .is_err_and(|e| e.contains("residual higher-kinded type variable")),
            "{core:?}"
        );
    }

    #[test]
    fn core_abstracts_a_residual_body_over_its_dictionary() {
        let elab = translated_core("let same = fun x -> fun y -> x == y; same");
        assert!(matches!(elab.ty, Type::Fun(..)), "{}", elab.ty);
    }

    #[test]
    fn run_core_polymorphic_equality_passes_the_guard() {
        let src = "let same = fun x -> fun y -> x == y;\n(same 3 3, same 'a' 'b')";
        assert_eq!(run_core(&prog(src)), Ok("(true, false)\n".to_string()));
        assert!(dump_lint(&prog(src), false).is_ok());
    }

    #[test]
    fn kernel_check_rejects_a_constrained_function() {
        let src = "let same = fun x -> fun y -> x == y;\n0";
        let err = kernel_check(&prog(src), "same").expect_err("rejected");
        assert!(err.starts_with("rejected: same"), "{err}");
    }
}
