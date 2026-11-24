use std::process::Command;

use assert_cmd::prelude::*;

fn run_fixture(name: &str, expected: &str) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest}/tests/fixtures/{name}");

    let output = Command::cargo_bin("parlang-driver")
        .expect("driver binary should be built")
        .arg(&fixture)
        .output()
        .expect("driver should run");

    assert!(
        output.status.success(),
        "driver failed for {name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is valid utf-8");
    assert_eq!(stdout.trim(), expected, "unexpected output for {name}");
}

#[test]
fn arithmetic_precedence() {
    run_fixture("arithmetic.par", "7");
}

#[test]
fn let_binding() {
    run_fixture("let_binding.par", "42");
}

#[test]
fn identity_application() {
    run_fixture("identity.par", "100");
}

fn run_fixture_with_flag(name: &str, flag: &str, expected: &str) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest}/tests/fixtures/{name}");

    let output = Command::cargo_bin("parlang-driver")
        .expect("driver binary should be built")
        .arg(flag)
        .arg(&fixture)
        .output()
        .expect("driver should run");

    assert!(
        output.status.success(),
        "driver failed for {name} {flag}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is valid utf-8");
    assert_eq!(
        stdout.trim(),
        expected,
        "unexpected output for {name} {flag}"
    );
}

#[test]
fn dump_text_arithmetic() {
    run_fixture_with_flag("arithmetic.par", "--dump", "(1 + (2 * 3))");
}

#[test]
fn dump_text_let_binding() {
    run_fixture_with_flag("let_binding.par", "--dump", "(let x = 40 in (x + 2))");
}

#[test]
fn dump_text_identity() {
    run_fixture_with_flag("identity.par", "--dump", "((fun x -> x) 100)");
}

#[test]
fn dump_dot_arithmetic() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest}/tests/fixtures/arithmetic.par");

    let output = Command::cargo_bin("parlang-driver")
        .expect("driver binary should be built")
        .arg("--dump-dot")
        .arg(&fixture)
        .output()
        .expect("driver should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is valid utf-8");

    assert!(stdout.contains("digraph AST"), "stdout was: {stdout}");
    assert!(
        stdout.contains("node0 [label=\"Program\"]"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("node1 [label=\"BinOp\\n+\"]"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("node3 [label=\"BinOp\\n*\"]"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("node0 -> node1 [label=\"body\"]"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("node1 -> node3 [label=\"right\"]"),
        "stdout was: {stdout}"
    );
}

fn run_fixture_with_flags(name: &str, flags: &[&str], expected: &str) {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest}/tests/fixtures/{name}");

    let output = Command::cargo_bin("parlang-driver")
        .expect("driver binary should be built")
        .args(flags)
        .arg(&fixture)
        .output()
        .expect("driver should run");

    assert!(
        output.status.success(),
        "driver failed for {name} {flags:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is valid utf-8");
    assert_eq!(
        stdout.trim(),
        expected,
        "unexpected output for {name} {flags:?}"
    );
}

#[test]
fn dump_typed_text_identity() {
    run_fixture_with_flags(
        "identity.par",
        &["--dump", "--stage", "typed"],
        "Program : Int\n  body: App : Int\n    func: Fun x : Int -> Int\n      body: Var x : Int\n    arg: Int 100 : Int",
    );
}

#[test]
fn dump_parsed_stage_explicit() {
    run_fixture_with_flags(
        "arithmetic.par",
        &["--dump", "--stage", "parsed"],
        "(1 + (2 * 3))",
    );
}

#[test]
fn dump_typed_dot_arithmetic() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest}/tests/fixtures/arithmetic.par");

    let output = Command::cargo_bin("parlang-driver")
        .expect("driver binary should be built")
        .args(["--dump-dot", "--stage", "typed"])
        .arg(&fixture)
        .output()
        .expect("driver should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is valid utf-8");
    assert!(
        stdout.contains("digraph TypedAST {"),
        "stdout was: {stdout}"
    );
    assert!(stdout.contains("BinOp\\n+\\n: Int"), "stdout was: {stdout}");
    assert!(
        stdout.contains("[label=\"Program\\n: Int\"];"),
        "stdout was: {stdout}"
    );
}

/// Absolute path of a driver test fixture.
fn fixture_path(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// Run the driver with `args`, assert it succeeded, and return its stdout.
fn driver_stdout(args: &[&str]) -> String {
    let output = Command::cargo_bin("parlang-driver")
        .expect("driver binary should be built")
        .args(args)
        .output()
        .expect("driver should run");

    assert!(
        output.status.success(),
        "driver failed for {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout is valid utf-8")
}

#[test]
fn mvp_runs_on_surface() {
    let fixture = fixture_path("mvp.par");
    let stdout = driver_stdout(&[fixture.as_str()]);
    assert!(!stdout.trim().is_empty(), "stdout was empty");
}

#[test]
fn mvp_parsed_stage_text_and_dot() {
    let fixture = fixture_path("mvp.par");
    let text = driver_stdout(&["--dump", "--stage", "parsed", fixture.as_str()]);
    assert!(!text.trim().is_empty(), "parsed text dump was empty");
    let dot = driver_stdout(&["--dump-dot", "--stage", "parsed", fixture.as_str()]);
    assert!(dot.starts_with("digraph"), "stdout was: {dot}");
}

#[test]
fn mvp_typed_stage_mentions_tree() {
    let fixture = fixture_path("mvp.par");
    let stdout = driver_stdout(&["--dump", "--stage", "typed", fixture.as_str()]);
    assert!(
        stdout.contains("Tree") || stdout.contains("Node"),
        "stdout was: {stdout}"
    );
}

#[test]
fn mvp_core_stage_shows_constructors() {
    let fixture = fixture_path("mvp.par");
    let stdout = driver_stdout(&["--dump", "--stage", "core", fixture.as_str()]);
    assert!(
        stdout.contains("/\\") || stdout.contains('@'),
        "stdout was: {stdout}"
    );
    assert!(stdout.contains("Node"), "stdout was: {stdout}");
    assert!(stdout.contains("Leaf"), "stdout was: {stdout}");
}

#[test]
fn mvp_lint_stage_text_and_dot() {
    let fixture = fixture_path("mvp.par");
    let text = driver_stdout(&["--dump", "--stage", "lint", fixture.as_str()]);
    assert!(text.starts_with("lint ok: "), "stdout was: {text}");
    let dot = driver_stdout(&["--dump-dot", "--stage", "lint", fixture.as_str()]);
    assert!(dot.starts_with("digraph"), "stdout was: {dot}");
}

#[test]
fn mvp_core_flag_renders_string() {
    let fixture = fixture_path("mvp.par");
    let stdout = driver_stdout(&["--core", fixture.as_str()]);
    assert_eq!(stdout.trim(), "\"aglnpr\"");
}

#[test]
fn trace_small_fixture_ends_with_result() {
    let fixture = fixture_path("let_binding.par");
    let stdout = driver_stdout(&["trace", fixture.as_str()]);
    let last = stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    assert_eq!(last.trim(), "result: 42");
}

#[test]
fn stage_requires_dump() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest}/tests/fixtures/arithmetic.par");

    let output = Command::cargo_bin("parlang-driver")
        .expect("driver binary should be built")
        .args(["--stage", "typed"])
        .arg(&fixture)
        .output()
        .expect("driver should run");

    assert!(!output.status.success());
}

fn run_driver(args: &[&str], name: &str) -> std::process::Output {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest}/tests/fixtures/{name}");

    Command::cargo_bin("parlang-driver")
        .expect("driver binary should be built")
        .args(args)
        .arg(&fixture)
        .output()
        .expect("driver should run")
}

#[test]
fn core_arithmetic() {
    run_fixture_with_flags("arithmetic.par", &["--core"], "7");
}

#[test]
fn core_string() {
    run_fixture_with_flags("core_list.par", &["--core"], "\"ab\"");
}

#[test]
fn core_polymorphic_tuple() {
    run_fixture_with_flags("core_poly.par", &["--core"], "(1, true)");
}

#[test]
fn dump_core_text_identity() {
    let output = run_driver(&["--dump", "--stage", "core"], "identity.par");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is valid utf-8");
    assert!(stdout.contains("100"), "stdout was: {stdout}");
    assert!(stdout.contains('x'), "stdout was: {stdout}");
}

#[test]
fn dump_lint_arithmetic() {
    run_fixture_with_flags(
        "arithmetic.par",
        &["--dump", "--stage", "lint"],
        "lint ok: Int",
    );
}

#[test]
fn core_conflicts_with_dump() {
    let output = run_driver(&["--core", "--dump"], "arithmetic.par");
    assert!(!output.status.success());
}

#[test]
fn trace_arithmetic() {
    let output = run_driver(&["trace"], "arithmetic.par");
    assert!(
        output.status.success(),
        "driver failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout is valid utf-8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(lines[0].starts_with("0: "), "lines were: {lines:?}");
    assert!(
        lines.iter().any(|l| l.starts_with("1: ")),
        "lines were: {lines:?}"
    );
    let last = *lines.last().expect("non-empty trace");
    assert_eq!(last, "result: 7");
    assert!(
        lines[..lines.len() - 1].iter().any(|l| l.contains('7')),
        "lines were: {lines:?}"
    );
}

#[test]
fn trace_raw_arithmetic() {
    let output = run_driver(&["trace", "--raw"], "arithmetic.par");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is valid utf-8");
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(lines[0].starts_with("0: ["), "lines were: {lines:?}");
    assert!(
        lines.iter().any(|l| l.starts_with("1: [")),
        "lines were: {lines:?}"
    );
    let last = *lines.last().expect("non-empty trace");
    assert_eq!(last, "result: 7");
}

#[test]
fn trace_step_limit() {
    let output = run_driver(&["trace", "--fuel", "1"], "arithmetic.par");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr is valid utf-8");
    assert!(stderr.contains("step limit"), "stderr was: {stderr}");
}

#[test]
fn core_unsupported_load() {
    let output = run_driver(&["--core"], "core_load.par");
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr is valid utf-8");
    assert!(
        stderr.contains("elaboration error:"),
        "stderr was: {stderr}"
    );
}
