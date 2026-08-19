/// `ParLang`: A small ML-alike functional language interpreter
///
/// This executable provides:
/// - REPL mode for interactive evaluation
/// - File execution mode for running .par files
/// - AST dumping to stdout as text IR (--dump) or Graphviz DOT (--dump-dot)
use clap::{Parser, Subcommand};
use parlang::{
    eval_program, eval_program_with_env, parse_program, program_to_dot, typecheck_program,
    typecheck_program_with_env, Environment, TypeEnv,
};
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

    /// Dump the parsed AST as text IR to stdout, then exit (skips evaluation).
    #[arg(long, conflicts_with = "dump_dot", requires = "file")]
    dump: bool,

    /// Dump the parsed AST as Graphviz DOT to stdout, then exit (skips evaluation).
    #[arg(long, requires = "file")]
    dump_dot: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive REPL
    Repl,
}

fn main() {
    let cli = Cli::parse();

    // Handle REPL command or no arguments
    if cli.command.is_some() || cli.file.is_none() {
        // REPL mode
        println!(
            "ParLang v{} - A small ML-alike functional language",
            env!("CARGO_PKG_VERSION")
        );
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
                match parse_program(&contents) {
                    Ok(program) => {
                        // Terminal dump modes: print AST then skip evaluation.
                        if cli.dump {
                            println!("{program}");
                            return;
                        }
                        if cli.dump_dot {
                            println!("{}", program_to_dot(&program));
                            return;
                        }

                        // Type check before evaluating
                        if let Err(e) = typecheck_program(&program) {
                            eprintln!("type error: {e}");
                            process::exit(1);
                        }

                        // Execute the program
                        let env = Environment::new();
                        match eval_program(&program, &env).map_err(|e| e.to_string()) {
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
    }
}

fn repl() {
    let mut env = Environment::new();
    let mut type_env = TypeEnv::new();
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

                    if parse_program(accumulated_trimmed).is_ok() {
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
            let input = lines.concat(); // Preserves newlines
            let input = input.trim();

            match parse_program(input) {
                Ok(program) => {
                    // Type check before evaluating (mandatory)
                    let mut next_type_env = type_env.clone();
                    match typecheck_program_with_env(&program, &mut next_type_env) {
                        Ok(ty) => println!("Type: {ty}"),
                        Err(e) => {
                            eprintln!("Type error: {e}");
                            continue;
                        }
                    }

                    match eval_program_with_env(&program, &env) {
                        Ok((value, new_env)) => {
                            println!("{value}");
                            // Persist declarations evaluated as part of this REPL input.
                            env = new_env;
                            type_env = next_type_env;
                        }
                        Err(e) => eprintln!("Evaluation error: {e}"),
                    }
                }
                Err(e) => eprintln!("Parse error: {e}"),
            }
        }
    }
}
