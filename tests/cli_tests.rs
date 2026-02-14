/// CLI integration tests
/// These tests verify the command-line interface functionality
use std::fs;
use std::process::Command;

#[test]
fn test_cli_file_execution() {
    // Create a temporary test file
    let test_file = "/tmp/test_program.par";
    fs::write(test_file, "1 + 2 + 3").unwrap();

    // Execute the file with the CLI
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", test_file])
        .output()
        .expect("Failed to execute command");

    // Clean up
    let _ = fs::remove_file(test_file);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "6");
}

#[test]
fn test_cli_file_not_found() {
    // Try to execute a non-existent file
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", "/tmp/nonexistent_file.par"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read file"));
}

#[test]
fn test_cli_parse_error() {
    // Create a temporary test file with invalid syntax
    let test_file = "/tmp/test_parse_error.par";
    fs::write(test_file, "let x = in y").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", test_file])
        .output()
        .expect("Failed to execute command");

    // Clean up
    let _ = fs::remove_file(test_file);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Parse error"));
}

#[test]
fn test_cli_eval_error() {
    // Create a temporary test file with runtime error
    let test_file = "/tmp/test_eval_error.par";
    fs::write(test_file, "1 / 0").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", test_file])
        .output()
        .expect("Failed to execute command");

    // Clean up
    let _ = fs::remove_file(test_file);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error") || stderr.contains("Division by zero"));
}

#[test]
fn test_cli_dump_ast() {
    // Create a temporary test file
    let test_file = "/tmp/test_ast.par";
    let dot_file = "/tmp/test_ast.dot";
    fs::write(test_file, "1 + 2").unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", test_file, "--dump-ast", dot_file])
        .output()
        .expect("Failed to execute command");

    // Check that DOT file was created
    let dot_exists = fs::metadata(dot_file).is_ok();

    // Clean up
    let _ = fs::remove_file(test_file);
    let _ = fs::remove_file(dot_file);

    assert!(output.status.success());
    assert!(dot_exists, "DOT file should have been created");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("AST dumped to"));
}

#[test]
fn test_cli_dump_ast_without_file() {
    // Try to dump AST without providing a source file
    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", "--dump-ast", "/tmp/test.dot"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("requires a file argument") || stderr.contains("REPL") || output.status.code() != Some(0));
}

#[test]
fn test_cli_complex_program() {
    // Test a more complex program
    let test_file = "/tmp/test_complex.par";
    let program = r"
        let factorial = rec f -> fun n ->
            if n == 0
            then 1
            else n * f (n - 1)
        in factorial 5
    ";
    fs::write(test_file, program).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", test_file])
        .output()
        .expect("Failed to execute command");

    // Clean up
    let _ = fs::remove_file(test_file);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "120");
}

#[test]
fn test_cli_multiline_program() {
    // Test a multiline program
    let test_file = "/tmp/test_multiline.par";
    let program = r"
let x = 10 in
let y = 20 in
let z = 30 in
x + y + z
    ";
    fs::write(test_file, program).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--quiet", "--", test_file])
        .output()
        .expect("Failed to execute command");

    // Clean up
    let _ = fs::remove_file(test_file);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim(), "60");
}
