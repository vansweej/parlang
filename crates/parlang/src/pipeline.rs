//! Shared Core pipeline used by the parlang and parlang-driver binaries: elaborate, lint, run, dump and trace on Core.

use std::fmt::Write;

use crate::{check_elaborated, elaborate_program, typecheck_program_typed, Elaborated, Program};
use parlang_core::{
    core_to_dot, eval, reduction_terms, render_value, run, trace, Environment, EvalError, Type,
};

/// Elaborates `program` to Core without running Core Lint.
fn elaborate_only(program: &Program) -> Result<Elaborated, String> {
    let typed = typecheck_program_typed(program).map_err(|e| format!("type error: {e}"))?;
    elaborate_program(&typed).map_err(|e| format!("elaboration error: {e}"))
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

    let value = run(&elab.term, &Environment::new(), Some(fuel))
        .map_err(|e| format!("trace error: {e}"))?;
    let rendered = render_value(&value, &elab.ty, &elab.env);
    let _ = writeln!(out, "result: {rendered}");

    Ok(out)
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
    fn trace_core_step_limit() {
        assert_eq!(
            trace_core(&prog("1 + 2 * 3"), 1, false),
            Err("trace error: step limit of 1 exceeded".to_string())
        );
    }
}
