/// `ParLang`: A small ML-alike functional language interpreter
///
/// This executable provides:
/// - REPL mode for interactive evaluation
/// - File execution mode for running .par files
/// - AST dumping to stdout as text IR (--dump) or Graphviz DOT (--dump-dot)
use clap::{Parser, Subcommand};
use parlang::{
    eval_program, eval_program_with_env, parse_program, program_to_dot, run_on_evaluator_stack,
    typecheck_program, typecheck_program_with_env, Environment, TypeEnv, EVALUATOR_STACK_SIZE,
};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fs;
use std::process;
use std::sync::mpsc;

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
        if !repl() {
            process::exit(1);
        }
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

                        match run_on_evaluator_stack(move || {
                            let env = Environment::new();
                            eval_program(&program, &env)
                                .map(|value| format!("{value}"))
                                .map_err(|error| format!("Error: {error}"))
                        }) {
                            Ok(Ok(value)) => println!("{value}"),
                            Ok(Err(error)) => {
                                eprintln!("{error}");
                                process::exit(1);
                            }
                            Err(error) => {
                                eprintln!("Error: {error}");
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

fn repl() -> bool {
    let mut rl = DefaultEditor::new().expect("Failed to initialize line editor");
    let (input_sender, input_receiver) = mpsc::channel::<String>();
    let (response_sender, response_receiver) = mpsc::channel::<Result<String, String>>();
    let worker = match std::thread::Builder::new()
        .stack_size(EVALUATOR_STACK_SIZE)
        .spawn(move || {
            let mut env = Environment::new();
            let mut type_env = TypeEnv::new();

            while let Ok(input) = input_receiver.recv() {
                let response = match parse_program(&input) {
                    Ok(program) => {
                        let mut next_type_env = type_env.clone();
                        match typecheck_program_with_env(&program, &mut next_type_env) {
                            Ok(ty) => match eval_program_with_env(&program, &env) {
                                Ok((value, new_env)) => {
                                    env = new_env;
                                    type_env = next_type_env;
                                    if program.body.is_some() {
                                        Ok(format!("Type: {ty}\n{value}"))
                                    } else {
                                        Ok("Declarations added.".to_string())
                                    }
                                }
                                Err(error) => Err(format!("Evaluation error: {error}")),
                            },
                            Err(error) => Err(format!("Type error: {error}")),
                        }
                    }
                    Err(error) => Err(format!("Parse error: {error}")),
                };

                if response_sender.send(response).is_err() {
                    return;
                }
            }
        }) {
        Ok(worker) => worker,
        Err(error) => {
            eprintln!("Error starting evaluator worker: {error}");
            return false;
        }
    };
    let mut should_continue = true;

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
                    should_continue = false;
                    break;
                }
                Err(err) => {
                    eprintln!("Error reading input: {err}");
                    should_continue = false;
                    break;
                }
            }
        }

        if !should_continue {
            break;
        }

        // Input is parsed again by the worker so persistent non-Send evaluator state never
        // crosses the channel. Ctrl-C only interrupts `readline`, as before.
        if !lines.is_empty() {
            if input_sender.send(lines.concat()).is_err() {
                eprintln!("Error: evaluator worker stopped unexpectedly");
                should_continue = false;
                break;
            }

            match response_receiver.recv() {
                Ok(Ok(response)) => println!("{response}"),
                Ok(Err(error)) => eprintln!("{error}"),
                Err(_) => {
                    eprintln!("Error: evaluator worker stopped unexpectedly");
                    should_continue = false;
                    break;
                }
            }
        }
    }

    drop(input_sender);
    if worker.join().is_err() {
        eprintln!("Error: evaluator worker panicked");
        return false;
    }

    should_continue
}
