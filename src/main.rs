/// ParLang: A small ML-alike functional language interpreter
/// 
/// This executable provides:
/// - REPL mode for interactive evaluation
/// - File execution mode for running .par files

use parlang::{parse, eval, extract_bindings, Environment, Value};
use std::env;
use std::fs;
use std::io::{self, Write};

#[cfg(not(tarpaulin_include))]
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        // File execution mode
        let filename = &args[1];
        match fs::read_to_string(filename) {
            Ok(contents) => {
                match execute(&contents) {
                    Ok(value) => println!("{}", value),
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read file '{}': {}", filename, e);
                std::process::exit(1);
            }
        }
    } else {
        // REPL mode
        println!("ParLang v0.1.0 - A small ML-alike functional language");
        println!("Type expressions to evaluate them. Press Ctrl+C to exit.");
        println!();
        repl();
    }
}

fn execute(source: &str) -> Result<Value, String> {
    let expr = parse(source)?;
    let env = Environment::new();
    eval(&expr, &env).map_err(|e| e.to_string())
}

fn repl() {
    let mut env = Environment::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Accumulate multiline input
        let mut lines = Vec::new();
        let mut is_first_line = true;

        loop {
            // Print appropriate prompt
            if is_first_line {
                print!("> ");
            } else {
                print!("... ");
            }
            stdout.flush().unwrap();

            let mut line = String::new();
            match stdin.read_line(&mut line) {
                Ok(0) => {
                    // EOF - exit the REPL
                    println!("\nGoodbye!");
                    return;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    
                    // Empty line signals end of input (if we have at least one line)
                    if trimmed.is_empty() {
                        if !is_first_line {
                            // We have accumulated lines, try to parse and evaluate
                            break;
                        }
                        // First line is empty, just continue to next prompt
                        continue;
                    }

                    // Add the line to our accumulator
                    lines.push(line);
                    is_first_line = false;
                }
                Err(e) => {
                    eprintln!("Error reading input: {}", e);
                    return;
                }
            }
        }

        // Join all lines and try to parse/evaluate
        if !lines.is_empty() {
            let input = lines.concat();  // Preserves newlines from read_line()
            let input = input.trim();

            match parse(input) {
                Ok(expr) => {
                    match eval(&expr, &env) {
                        Ok(value) => {
                            println!("{}", value);
                            // Extract bindings from the expression and merge into environment
                            match extract_bindings(&expr, &env) {
                                Ok(new_env) => {
                                    env = new_env;
                                }
                                Err(e) => {
                                    // If binding extraction fails, report it but continue with the old environment
                                    eprintln!("Warning: Failed to persist bindings: {}", e);
                                }
                            }
                        }
                        Err(e) => eprintln!("Evaluation error: {}", e),
                    }
                },
                Err(e) => eprintln!("Parse error: {}", e),
            }
        }
    }
}
