/// `ParLang`: A small ML-alike functional language interpreter
///
/// This executable provides:
/// - REPL mode for interactive evaluation
/// - File execution mode for running .par files
/// - AST dumping to stdout as text IR (--dump) or DOT (--dump-dot), parsed, typed, core or lint (--stage)
/// - Core evaluation (--core) and Core machine tracing (trace subcommand)
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

    /// Pipeline stage to dump: `parsed` (default), `typed`, `core` or `lint` (requires --dump or --dump-dot).
    #[arg(long, value_enum, requires = "dump_mode")]
    stage: Option<Stage>,

    /// Evaluate the program on the Core CEK machine instead of the surface evaluator.
    #[arg(long, conflicts_with = "dump_mode", requires = "file")]
    core: bool,
}

struct ReplResponse {
    type_line: Option<String>,
    outcome: Result<String, String>,
}

/// Persistent REPL evaluator state: the value and type environments that
/// successfully evaluated inputs commit to.
struct ReplState {
    env: Environment,
    type_env: TypeEnv,
}

impl ReplState {
    /// Creates an empty surface-evaluator REPL state.
    fn new() -> Self {
        Self {
            env: Environment::new(),
            type_env: TypeEnv::new(),
        }
    }

    /// Parses, type-checks and evaluates `input`, committing the new
    /// environments only when every stage succeeds.
    fn eval_input(&mut self, input: &str) -> ReplResponse {
        evaluate_repl_input(input, &mut self.env, &mut self.type_env)
    }
}

/// Which pipeline stage to dump.
#[derive(Copy, Clone, clap::ValueEnum)]
enum Stage {
    /// The AST straight from the parser.
    Parsed,
    /// The type-annotated AST after Hindley-Milner inference.
    Typed,
    /// The elaborated Core term (elaboration only, no Core lint).
    Core,
    /// The elaborated Core term checked by Core lint.
    Lint,
}

/// Dump `program` at the chosen `stage`, as DOT when `dot` is set, returning
/// the exact text to write to stdout.
///
/// Runs `typecheck_program_typed` for the `typed` stage and returns the type
/// error text on failure.
fn run_dump(program: &parlang::Program, stage: Stage, dot: bool) -> Result<String, String> {
    match stage {
        Stage::Parsed => {
            if dot {
                Ok(format!("{}\n", program_to_dot(program)))
            } else {
                Ok(format!("{program}\n"))
            }
        }
        Stage::Typed => {
            let typed = parlang::typecheck_program_typed(program)
                .map_err(|e| format!("type error: {e}"))?;
            if dot {
                Ok(format!("{}\n", parlang::typed_program_to_dot(&typed)))
            } else {
                Ok(parlang::typed_program_to_text(&typed))
            }
        }
        Stage::Core => parlang::pipeline::dump_core(program, dot),
        Stage::Lint => parlang::pipeline::dump_lint(program, dot),
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive REPL
    Repl,
    /// Trace a program's evaluation on the Core CEK machine.
    Trace {
        /// Input file to trace (.par file)
        file: String,
        /// Maximum number of machine steps before giving up.
        #[arg(long, default_value_t = 10_000)]
        fuel: usize,
        /// Print every raw machine step instead of the reduction view.
        #[arg(long)]
        raw: bool,
    },
}

/// Trace `file` on the Core machine, returning the trace text or the error line.
fn run_trace(file: &str, fuel: usize, raw: bool) -> Result<String, String> {
    fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file '{file}': {e}"))
        .and_then(|contents| parse_program(&contents).map_err(|e| format!("Parse error: {e}")))
        .and_then(|program| parlang::pipeline::trace_core(&program, fuel, raw))
}

/// Whether `cli` selects the interactive REPL: the `repl` subcommand, or no
/// subcommand and no input file.
fn wants_repl(cli: &Cli) -> bool {
    match &cli.command {
        Some(Commands::Repl) => true,
        Some(Commands::Trace { .. }) => false,
        None => cli.file.is_none(),
    }
}

/// Run file mode for `filename`: dump, Core evaluation, or type check plus
/// surface evaluation, returning the exact stdout text or the stderr line.
fn run_file(cli: &Cli, filename: &str) -> Result<String, String> {
    let contents = fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read file '{filename}': {e}"))?;
    let program = parse_program(&contents).map_err(|e| format!("Parse error: {e}"))?;

    // Terminal dump modes: print AST then skip evaluation.
    if cli.dump || cli.dump_dot {
        return run_dump(&program, cli.stage.unwrap_or(Stage::Parsed), cli.dump_dot);
    }

    if cli.core {
        return parlang::pipeline::run_core(&program);
    }

    // Type check before evaluating
    typecheck_program(&program).map_err(|e| format!("type error: {e}"))?;

    let value = run_on_evaluator_stack(move || {
        let env = Environment::new();
        eval_program(&program, &env)
            .map(|value| format!("{value}"))
            .map_err(|error| format!("Error: {error}"))
    })
    .map_err(|error| format!("Error: {error}"))??;
    Ok(format!("{value}\n"))
}

/// Run the non-interactive CLI (trace subcommand or file mode).
///
/// Returns the exact text for stdout on success, or the single line for
/// stderr on failure; the caller prints it and exits with status 1 on `Err`.
fn run(cli: &Cli) -> Result<String, String> {
    if let Some(Commands::Trace { file, fuel, raw }) = &cli.command {
        return run_trace(file, *fuel, *raw);
    }
    let Some(filename) = &cli.file else {
        return Err("Error: no input file given".to_string());
    };
    run_file(cli, filename)
}

fn main() {
    let cli = Cli::parse();

    if wants_repl(&cli) {
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

    match run(&cli) {
        Ok(text) => print!("{text}"),
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
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
            let mut state = ReplState::new();

            while let Ok(input) = input_receiver.recv() {
                let response = state.eval_input(&input);
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
