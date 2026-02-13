/// ParLang: A small ML-alike functional language interpreter
/// 
/// This executable provides:
/// - REPL mode for interactive evaluation
/// - File execution mode for running .par files

use parlang::{parse, eval, Environment, Value};
use std::env;
use std::fs;
use std::io::{self, Write};

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
    let env = Environment::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let input = input.trim();
                if input.is_empty() {
                    continue;
                }

                match parse(input) {
                    Ok(expr) => match eval(&expr, &env) {
                        Ok(value) => {
                            println!("{}", value);
                            // In a real REPL, we might want to bind the result to a special variable
                        }
                        Err(e) => eprintln!("Evaluation error: {}", e),
                    },
                    Err(e) => eprintln!("Parse error: {}", e),
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                break;
            }
        }
    }

    println!("\nGoodbye!");
}
