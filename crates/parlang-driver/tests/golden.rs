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
        stdout.contains("node0 [label=\"BinOp\\n+\"]"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("node2 [label=\"BinOp\\n*\"]"),
        "stdout was: {stdout}"
    );
    assert!(
        stdout.contains("node0 -> node2 [label=\"right\"]"),
        "stdout was: {stdout}"
    );
}
