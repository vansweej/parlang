/// `ParLang`: A small ML-alike functional language interpreter
/// 
/// This executable provides:
/// - REPL mode for interactive evaluation
/// - File execution mode for running .par files
/// - AST dumping to DOT format for visualization
use clap::{Parser, Subcommand};
use parlang::{parse, eval, extract_bindings, dot, Environment};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fs;
use std::process;

#[derive(Parser)]
#[command(name = "parlang")]
#[command(author, version, about = "A small ML-alike functional language", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Input file to execute (.par file)
    file: Option<String>,

    /// Dump AST to DOT file (Graphviz format)
    #[arg(short, long, value_name = "FILE")]
    dump_ast: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive REPL
    Repl,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let cli = Cli::parse();

    // Handle REPL command or no arguments
    if cli.command.is_some() || (cli.file.is_none() && cli.dump_ast.is_none()) {
        // REPL mode
        println!("ParLang v{} - A small ML-alike functional language", env!("CARGO_PKG_VERSION"));
        println!("Type expressions to evaluate them. Press Ctrl+C to exit.");
        println!();
        repl();
        return;
    }

    // File execution mode
    if let Some(filename) = &cli.file {
        match fs::read_to_string(filename) {
            Ok(contents) => {
                // Parse the file
                match parse(&contents) {
                    Ok(expr) => {
                        // Dump AST if requested
                        if let Some(dot_file) = &cli.dump_ast {
                            match dot::write_ast_to_dot_file(&expr, dot_file) {
                                Ok(()) => {
                                    eprintln!("AST dumped to: {dot_file}");
                                }
                                Err(e) => {
                                    eprintln!("Failed to write DOT file '{dot_file}': {e}");
                                    process::exit(1);
                                }
                            }
                        }

                        // Execute the program
                        let env = Environment::new();
                        match eval(&expr, &env).map_err(|e| e.to_string()) {
                            Ok(value) => println!("{value}"),
                            Err(e) => {
                                eprintln!("Error: {e}");
                                process::exit(1);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Parse error: {e}");
                        process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read file '{filename}': {e}");
                process::exit(1);
            }
        }
    } else if cli.dump_ast.is_some() {
        eprintln!("Error: --dump-ast requires a file argument");
        process::exit(1);
    }
}

fn repl() {
    let mut env = Environment::new();
    let mut rl = DefaultEditor::new().expect("Failed to initialize line editor");

    loop {
        // Accumulate multiline input
        let mut lines = Vec::new();
        let mut is_first_line = true;

        loop {
            // Read line with history support
            let prompt = if is_first_line { "> " } else { "... " };
            
            let readline = rl.readline(prompt);
            match readline {
                Ok(line) => {
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
                    
                    // Add the line to history if it's the first line
                    if is_first_line {
                        if let Err(e) = rl.add_history_entry(line.as_str()) {
                            eprintln!("Warning: Failed to add entry to history: {e}");
                        }
                    }
                    
                    // Add the line to our accumulator (with newline to match old behavior)
                    lines.push(line + "\n");
                    is_first_line = false;
                    
                    // Try to parse the accumulated input after each line
                    // If it's parseable, auto-submit without requiring a blank line
                    let accumulated = lines.concat();
                    let accumulated_trimmed = accumulated.trim();
                    
                    if parse(accumulated_trimmed).is_ok() {
                        // Input is complete and parseable, submit it
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl+C - reset the multiline input state and start fresh
                    println!("^C");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl+D
                    println!("\nGoodbye!");
                    return;
                }
                Err(err) => {
                    eprintln!("Error reading input: {err}");
                    return;
                }
            }
        }

        // Join all lines and try to parse/evaluate
        if !lines.is_empty() {
            let input = lines.concat();  // Preserves newlines
            let input = input.trim();

            match parse(input) {
                Ok(expr) => {
                    match eval(&expr, &env) {
                        Ok(value) => {
                            println!("{value}");
                            // Extract bindings from the expression and merge into environment
                            match extract_bindings(&expr, &env) {
                                Ok(new_env) => {
                                    env = new_env;
                                }
                                Err(e) => {
                                    // If binding extraction fails, report it but continue with the old environment
                                    eprintln!("Warning: Failed to persist bindings: {e}");
                                }
                            }
                        }
                        Err(e) => eprintln!("Evaluation error: {e}"),
                    }
                },
                Err(e) => eprintln!("Parse error: {e}"),
            }
        }
    }
}
