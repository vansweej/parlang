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
