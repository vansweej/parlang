/// `ParLang`: A small ML-alike functional language interpreter
///
/// This executable provides:
/// - REPL mode for interactive evaluation
/// - File execution mode for running .par files
/// - AST dumping to stdout as text IR (--dump) or DOT (--dump-dot), parsed or typed (--stage)
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
#[command(group(clap::ArgGroup::new("dump_mode").args(["dump", "dump_dot"])))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Input file to execute (.par file)
    file: Option<String>,

    /// Dump the AST as text IR to stdout, then exit (skips evaluation).
    #[arg(long, conflicts_with = "dump_dot", requires = "file")]
    dump: bool,

    /// Dump the AST as Graphviz DOT to stdout, then exit (skips evaluation).
    #[arg(long, requires = "file")]
    dump_dot: bool,

    /// Pipeline stage to dump: `parsed` (default) or `typed` (requires --dump or --dump-dot).
    #[arg(long, value_enum, requires = "dump_mode")]
    stage: Option<Stage>,
}

struct ReplResponse {
    type_line: Option<String>,
    outcome: Result<String, String>,
}

/// Which pipeline stage to dump.
#[derive(Copy, Clone, clap::ValueEnum)]
enum Stage {
    /// The AST straight from the parser.
    Parsed,
    /// The type-annotated AST after Hindley-Milner inference.
    Typed,
}

/// Dump `program` at the chosen `stage`, as DOT when `dot` is set.
///
/// Runs `typecheck_program_typed` for the `typed` stage and returns the type
/// error text on failure.
fn run_dump(program: &parlang::Program, stage: Stage, dot: bool) -> Result<(), String> {
    match stage {
        Stage::Parsed => {
            if dot {
                println!("{}", program_to_dot(program));
            } else {
                println!("{program}");
            }
        }
        Stage::Typed => {
            let typed = parlang::typecheck_program_typed(program)
                .map_err(|e| format!("type error: {e}"))?;
            if dot {
                println!("{}", parlang::typed_program_to_dot(&typed));
            } else {
                print!("{}", parlang::typed_program_to_text(&typed));
            }
        }
    }
    Ok(())
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
                        if cli.dump || cli.dump_dot {
                            let stage = cli.stage.unwrap_or(Stage::Parsed);
                            if let Err(e) = run_dump(&program, stage, cli.dump_dot) {
                                eprintln!("{e}");
                                process::exit(1);
                            }
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
    let (response_sender, response_receiver) = mpsc::channel::<ReplResponse>();
    let worker = match start_repl_worker(input_receiver, response_sender) {
        Ok(worker) => worker,
        Err(error) => {
            eprintln!("Error starting evaluator worker: {error}");
            return false;
        }
    };
    let mut failed = false;

    'session: loop {
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
                    break 'session;
                }
                Err(err) => {
                    eprintln!("Error reading input: {err}");
                    failed = true;
                    break 'session;
                }
            }
        }

        // Input is parsed again by the worker so persistent non-Send evaluator state never
        // crosses the channel. Ctrl-C only interrupts `readline`, as before.
        if !lines.is_empty() {
            if input_sender
                .send(lines.concat().trim().to_string())
                .is_err()
            {
                eprintln!("Error: evaluator worker stopped unexpectedly");
                failed = true;
                break;
            }

            if let Ok(response) = response_receiver.recv() {
                if let Some(type_line) = response.type_line {
                    println!("{type_line}");
                }
                match response.outcome {
                    Ok(value) => println!("{value}"),
                    Err(error) => eprintln!("{error}"),
                }
            } else {
                eprintln!("Error: evaluator worker stopped unexpectedly");
                failed = true;
                break 'session;
            }
        }
    }

    drop(input_sender);
    if worker.join().is_err() {
        eprintln!("Error: evaluator worker panicked");
        return false;
    }

    !failed
}

fn start_repl_worker(
    input_receiver: mpsc::Receiver<String>,
    response_sender: mpsc::Sender<ReplResponse>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .stack_size(EVALUATOR_STACK_SIZE)
        .spawn(move || {
            let mut env = Environment::new();
            let mut type_env = TypeEnv::new();

            while let Ok(input) = input_receiver.recv() {
                let response = evaluate_repl_input(&input, &mut env, &mut type_env);
                if response_sender.send(response).is_err() {
                    return;
                }
            }
        })
}

fn evaluate_repl_input(input: &str, env: &mut Environment, type_env: &mut TypeEnv) -> ReplResponse {
    let program = match parse_program(input) {
        Ok(program) => program,
        Err(error) => {
            return ReplResponse {
                type_line: None,
                outcome: Err(format!("Parse error: {error}")),
            };
        }
    };
    let mut next_type_env = type_env.clone();
    let ty = match typecheck_program_with_env(&program, &mut next_type_env) {
        Ok(ty) => ty,
        Err(error) => {
            return ReplResponse {
                type_line: None,
                outcome: Err(format!("Type error: {error}")),
            };
        }
    };
    let type_line = program.body.is_some().then(|| format!("Type: {ty}"));
    let (value, new_env) = match eval_program_with_env(&program, env) {
        Ok(result) => result,
        Err(error) => {
            return ReplResponse {
                type_line,
                outcome: Err(format!("Evaluation error: {error}")),
            };
        }
    };

    *env = new_env;
    *type_env = next_type_env;
    let outcome = if program.body.is_some() {
        Ok(format!("{value}"))
    } else {
        Ok("Declarations added.".to_string())
    };

    ReplResponse { type_line, outcome }
}
