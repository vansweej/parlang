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

    /// Evaluate on the Core CEK machine instead of the surface evaluator (file mode or REPL); redundant with the trace subcommand, which always runs on Core.
    #[arg(long, conflicts_with = "dump_mode")]
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
    /// Committed declarations when evaluating on Core (`--core`); `None`
    /// selects the surface evaluator.
    core_decls: Option<Vec<parlang::Decl>>,
}

impl ReplState {
    /// Creates an empty surface-evaluator REPL state.
    fn new() -> Self {
        Self {
            env: Environment::new(),
            type_env: TypeEnv::new(),
            core_decls: None,
        }
    }

    /// Creates an empty REPL state that evaluates inputs on the Core machine.
    fn new_core() -> Self {
        Self {
            core_decls: Some(Vec::new()),
            ..Self::new()
        }
    }

    /// Evaluates `input` on the selected evaluator, committing state only
    /// when every stage succeeds.
    fn eval_input(&mut self, input: &str) -> ReplResponse {
        if let Some(decls) = self.core_decls.as_mut() {
            return evaluate_core_repl_input(input, decls);
        }
        evaluate_repl_input(input, &mut self.env, &mut self.type_env)
    }
}

/// Evaluates `input` on the Core CEK machine against the committed Core
/// declarations `decls`.
///
/// There is no incremental Core API, so the committed declarations plus the
/// new input are elaborated, linted and run as one whole program; the input's
/// declarations are appended to `decls` only when that succeeds. A
/// declaration-only input is evaluated too (with the default body), so a
/// declaration whose evaluation fails, such as `let x = 1 / 0;`, is rejected
/// and not committed.
fn evaluate_core_repl_input(input: &str, decls: &mut Vec<parlang::Decl>) -> ReplResponse {
    let program = match parse_program(input) {
        Ok(program) => program,
        Err(error) => {
            return ReplResponse {
                type_line: None,
                outcome: Err(format!("Parse error: {error}")),
            };
        }
    };
    let has_body = program.body.is_some();
    let candidate = parlang::Program {
        decls: decls.iter().cloned().chain(program.decls).collect(),
        body: program.body,
    };
    let outcome = parlang::pipeline::run_core(&candidate).map(|text| {
        if has_body {
            text.trim_end().to_string()
        } else {
            "Declarations added.".to_string()
        }
    });
    if outcome.is_ok() {
        *decls = candidate.decls;
    }
    ReplResponse {
        type_line: None,
        outcome,
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
    /// Check whether a top-level function is kernel-eligible (slice 8 predicate).
    KernelCheck {
        /// Input file (.par file)
        file: String,
        /// Name of the top-level function to check.
        #[arg(long = "fn")]
        name: String,
    },
}

/// Kernel-check function `name` in `file`, returning the verdict or the error text.
fn run_kernel_check(file: &str, name: &str) -> Result<String, String> {
    fs::read_to_string(file)
        .map_err(|e| format!("Failed to read file '{file}': {e}"))
        .and_then(|contents| parse_program(&contents).map_err(|e| format!("Parse error: {e}")))
        .and_then(|program| parlang::pipeline::kernel_check(&program, name))
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
        Some(Commands::Trace { .. } | Commands::KernelCheck { .. }) => false,
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
    if let Some(Commands::KernelCheck { file, name }) = &cli.command {
        return run_kernel_check(file, name);
    }
    let Some(filename) = &cli.file else {
        return Err("Error: no input file given".to_string());
    };
    run_file(cli, filename)
}

/// Returns the exact REPL start-up banner text, ending with an empty line;
/// the `--core` notice line is included when `core` is set.
fn banner(core: bool) -> String {
    let hint = "Type expressions to evaluate them. Press Ctrl+C to exit.\n";
    let mut text = format!(
        "ParLang v{} - A small ML-alike functional language\n",
        env!("CARGO_PKG_VERSION")
    );
    text.push_str(hint);
    if core {
        text.push_str("Evaluating on the Core CEK machine (--core).\n");
    }
    text.push('\n');
    text
}

fn main() {
    let cli = Cli::parse();

    if wants_repl(&cli) {
        print!("{}", banner(cli.core));
        if !repl(cli.core) {
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

fn repl(core: bool) -> bool {
    let mut rl = DefaultEditor::new().expect("Failed to initialize line editor");
    let (input_sender, input_receiver) = mpsc::channel::<String>();
    let (response_sender, response_receiver) = mpsc::channel::<ReplResponse>();
    let worker = match start_repl_worker(input_receiver, response_sender, core) {
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
    core: bool,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .stack_size(EVALUATOR_STACK_SIZE)
        .spawn(move || {
            let mut state = if core {
                ReplState::new_core()
            } else {
                ReplState::new()
            };

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

#[cfg(test)]
mod tests {
    use super::{banner, run, wants_repl, Cli, ReplState};
    use clap::{CommandFactory, Parser};
    use std::path::PathBuf;

    /// Writes `src` to a pid-unique temp file tagged `name` and returns its path.
    fn temp_program(name: &str, src: &str) -> PathBuf {
        let file = format!("parlang_main_{}_{name}.par", std::process::id());
        let path = std::env::temp_dir().join(file);
        std::fs::write(&path, src).expect("write temp program");
        path
    }

    /// Parses `args` (without the binary name) as `Cli` and calls `run`.
    fn run_args(args: &[&str]) -> Result<String, String> {
        let full_args = std::iter::once("parlang").chain(args.iter().copied());
        let cli = Cli::try_parse_from(full_args).expect("valid arguments");
        run(&cli)
    }

    /// Runs `src` from a temp file with extra `flags`, then removes the file.
    fn run_src(name: &str, src: &str, flags: &[&str]) -> Result<String, String> {
        let path = temp_program(name, src);
        let path_text = path.to_string_lossy().into_owned();
        let mut args = vec![path_text.as_str()];
        args.extend_from_slice(flags);
        let result = run_args(&args);
        let _ = std::fs::remove_file(&path);
        result
    }

    #[test]
    fn run_example_file_prints_value() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/simple.par");
        assert_eq!(run_args(&[path]), Ok("20\n".to_string()));
    }

    #[test]
    fn dump_stage_typed_mentions_int() {
        let out = run_src("typed", "1 + 2", &["--dump", "--stage", "typed"]);
        let text = out.expect("typed dump succeeds");
        assert!(text.contains("Int"), "{text}");
    }

    #[test]
    fn dump_stage_core_is_not_empty() {
        let out = run_src("core_dump", "1 + 2", &["--dump", "--stage", "core"]);
        let text = out.expect("core dump succeeds");
        assert!(!text.trim().is_empty());
    }

    #[test]
    fn dump_stage_lint_reports_type() {
        let out = run_src("lint", "1 + 2", &["--dump", "--stage", "lint"]);
        assert_eq!(out, Ok("lint ok: Int\n".to_string()));
    }

    #[test]
    fn core_flag_evaluates_on_core() {
        let out = run_src("core_eval", "1 + 2 * 3", &["--core"]);
        assert_eq!(out, Ok("7\n".to_string()));
    }

    #[test]
    fn trace_subcommand_lists_steps() {
        let path = temp_program("trace", "1 + 2 * 3");
        let path_text = path.to_string_lossy().into_owned();
        let out = run_args(&["trace", path_text.as_str()]);
        let _ = std::fs::remove_file(&path);
        let text = out.expect("trace succeeds");
        let first = text.lines().find(|l| !l.trim().is_empty());
        assert!(first.is_some_and(|l| l.starts_with("0: ")), "{text}");
        assert!(text.contains("result: 7"), "{text}");
    }

    #[test]
    fn kernel_check_reports_eligible() {
        let path = temp_program("kc_ok", "let inc = fun x -> x + 1;\n0");
        let path_text = path.to_string_lossy().into_owned();
        let out = run_args(&["kernel-check", path_text.as_str(), "--fn", "inc"]);
        let _ = std::fs::remove_file(&path);
        assert_eq!(out, Ok("eligible: inc\n".to_string()));
    }

    #[test]
    fn kernel_check_reports_rejection() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/factorial.par");
        let out = run_args(&["kernel-check", path, "--fn", "factorial"]);
        let err = out.expect_err("factorial is recursive");
        assert!(err.starts_with("rejected: factorial\n"), "{err}");
        assert!(err.contains("  R3 recursion: factorial: "), "{err}");
    }

    #[test]
    fn kernel_check_unknown_function() {
        let path = temp_program("kc_unknown", "1");
        let path_text = path.to_string_lossy().into_owned();
        let out = run_args(&["kernel-check", path_text.as_str(), "--fn", "nope"]);
        let _ = std::fs::remove_file(&path);
        let expected = "kernel-check error: unknown function: nope";
        assert_eq!(out, Err(expected.to_string()));
    }

    #[test]
    fn kernel_check_requires_fn_flag() {
        let parsed = Cli::try_parse_from(["parlang", "kernel-check", "x.par"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn parse_error_is_reported() {
        let out = run_src("parse_err", "let x = in y", &[]);
        assert!(
            matches!(&out, Err(e) if e.starts_with("Parse error")),
            "{out:?}"
        );
    }

    #[test]
    fn type_error_is_reported() {
        let out = run_src("type_err", "1 + true", &[]);
        assert!(
            matches!(&out, Err(e) if e.starts_with("type error")),
            "{out:?}"
        );
    }

    #[test]
    fn eval_error_is_reported() {
        let out = run_src("eval_err", "1 / 0", &[]);
        assert!(matches!(&out, Err(e) if e.starts_with("Error")), "{out:?}");
    }

    #[test]
    fn missing_file_is_reported() {
        let out = run_args(&["/nonexistent/parlang_missing_file.par"]);
        assert!(
            matches!(&out, Err(e) if e.starts_with("Failed to read file")),
            "{out:?}"
        );
    }

    #[test]
    fn repl_declaration_persists_across_inputs() {
        let mut state = ReplState::new();
        let first = state.eval_input("let x = 41;");
        assert_eq!(first.outcome, Ok("Declarations added.".to_string()));
        let second = state.eval_input("x + 1");
        assert_eq!(second.type_line.as_deref(), Some("Type: Int"));
        assert_eq!(second.outcome, Ok("42".to_string()));
    }

    #[test]
    fn repl_failed_input_does_not_commit() {
        let mut state = ReplState::new();
        let failed = state.eval_input("let y = true + 1;");
        let bad = &failed.outcome;
        assert!(
            matches!(bad, Err(e) if e.starts_with("Type error")),
            "{bad:?}"
        );
        let later = state.eval_input("y");
        let unbound = &later.outcome;
        assert!(
            matches!(unbound, Err(e) if e.starts_with("Type error")),
            "{unbound:?}"
        );
    }

    #[test]
    fn core_flag_without_file_starts_repl() {
        let cli = Cli::try_parse_from(["parlang", "--core"]).expect("valid arguments");
        assert!(cli.core);
        assert!(wants_repl(&cli));
    }

    #[test]
    fn core_flag_still_conflicts_with_dump() {
        let parsed = Cli::try_parse_from(["parlang", "f.par", "--core", "--dump"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn core_flag_with_trace_is_accepted() {
        let parsed = Cli::try_parse_from(["parlang", "--core", "trace", "f.par"]);
        let cli = parsed.expect("--core with trace is accepted");
        assert!(!wants_repl(&cli));
    }

    #[test]
    fn core_repl_evaluates_arithmetic() {
        let mut state = ReplState::new_core();
        let response = state.eval_input("1 + 2");
        assert_eq!(response.type_line, None);
        assert_eq!(response.outcome, Ok("3".to_string()));
    }

    #[test]
    fn core_repl_let_persists_across_inputs() {
        let mut state = ReplState::new_core();
        let first = state.eval_input("let x = 41;");
        assert_eq!(first.outcome, Ok("Declarations added.".to_string()));
        let second = state.eval_input("x + 1");
        assert_eq!(second.outcome, Ok("42".to_string()));
    }

    #[test]
    fn core_repl_failing_declaration_does_not_commit() {
        let mut state = ReplState::new_core();
        let failed = state.eval_input("let x = 1 / 0;");
        assert_eq!(failed.type_line, None);
        assert_eq!(
            failed.outcome,
            Err("eval error: integer division by zero".to_string())
        );
        let unbound = state.eval_input("x");
        assert_eq!(
            unbound.outcome,
            Err("type error: Unbound variable: x".to_string())
        );
        let next = state.eval_input("let y = 2;");
        assert_eq!(next.outcome, Ok("Declarations added.".to_string()));
        let used = state.eval_input("y + 1");
        assert_eq!(used.outcome, Ok("3".to_string()));
    }

    #[test]
    fn core_repl_declaration_only_input_is_evaluated() {
        let mut state = ReplState::new_core();
        let first = state.eval_input("let z = 6 * 7;");
        assert_eq!(first.type_line, None);
        assert_eq!(first.outcome, Ok("Declarations added.".to_string()));
        let second = state.eval_input("z");
        assert_eq!(second.outcome, Ok("42".to_string()));
    }

    #[test]
    fn core_repl_data_then_match() {
        let mut state = ReplState::new_core();
        let first = state.eval_input("data Color = Red | Green;");
        assert_eq!(first.outcome, Ok("Declarations added.".to_string()));
        let second = state.eval_input("match Green with | Red -> 1 | Green -> 2");
        assert_eq!(second.outcome, Ok("2".to_string()));
    }

    #[test]
    fn core_repl_rejects_duplicate_data_declaration() {
        let mut state = ReplState::new_core();
        let first = state.eval_input("data Color = Red | Green;");
        assert_eq!(first.outcome, Ok("Declarations added.".to_string()));
        let duplicate = state.eval_input("data Color = Blue;");
        assert_eq!(duplicate.type_line, None);
        assert_eq!(
            duplicate.outcome,
            Err("type error: Duplicate type name: Color".to_string())
        );
        let kept = state.eval_input("match Green with | Red -> 1 | Green -> 2");
        assert_eq!(kept.outcome, Ok("2".to_string()));
        let rejected = state.eval_input("Blue");
        assert_eq!(
            rejected.outcome,
            Err("type error: Unknown constructor: Blue".to_string())
        );
    }

    #[test]
    fn core_repl_rejects_constructor_reused_by_new_type() {
        let mut state = ReplState::new_core();
        let first = state.eval_input("data Color = Red | Green;");
        assert_eq!(first.outcome, Ok("Declarations added.".to_string()));
        let reused = state.eval_input("data Shade = Red;");
        assert_eq!(reused.type_line, None);
        assert_eq!(
            reused.outcome,
            Err("type error: Duplicate constructor: Red".to_string())
        );
        let kept = state.eval_input("match Red with | Red -> 1 | Green -> 2");
        assert_eq!(kept.outcome, Ok("1".to_string()));
        let fresh = state.eval_input("data Shade = Dark;");
        assert_eq!(fresh.outcome, Ok("Declarations added.".to_string()));
    }

    #[test]
    fn core_repl_elaboration_error_does_not_commit() {
        let mut state = ReplState::new_core();
        let failed = state.eval_input("let t = fun x -> fun y -> x == y;");
        let bad = &failed.outcome;
        assert!(
            matches!(bad, Err(e) if e.starts_with("elaboration error")),
            "{bad:?}"
        );
        let later = state.eval_input("t");
        let unbound = &later.outcome;
        assert!(
            matches!(unbound, Err(e) if e.starts_with("type error")),
            "{unbound:?}"
        );
    }

    #[test]
    fn surface_repl_unchanged_by_core_mode() {
        let mut state = ReplState::new();
        let response = state.eval_input("1 + 2");
        assert_eq!(response.type_line.as_deref(), Some("Type: Int"));
        assert_eq!(response.outcome, Ok("3".to_string()));
    }

    #[test]
    fn banner_lines_match_main_output() {
        let version = env!("CARGO_PKG_VERSION");
        let head = format!("ParLang v{version} - A small ML-alike functional language\n");
        let hint = "Type expressions to evaluate them. Press Ctrl+C to exit.\n";
        let core_line = "Evaluating on the Core CEK machine (--core).\n";
        assert_eq!(banner(false), format!("{head}{hint}\n"));
        assert_eq!(banner(true), format!("{head}{hint}{core_line}\n"));
    }

    #[test]
    fn module_main_doc_shows_current_help() {
        let doc = include_str!("../../../docs/MODULE_MAIN.md");
        let help = Cli::command().render_long_help().to_string();
        let expected = help.trim_end();
        assert!(
            doc.contains(expected),
            "docs/MODULE_MAIN.md Help Output is stale; rendered help:\n{expected}"
        );
    }

    #[test]
    fn docs_show_current_banners() {
        let module_doc = include_str!("../../../docs/MODULE_MAIN.md");
        let readme = include_str!("../../../README.md");
        for text in [banner(false), banner(true)] {
            assert!(
                module_doc.contains(&text),
                "docs/MODULE_MAIN.md lacks banner:\n{text}"
            );
            assert!(readme.contains(&text), "README.md lacks banner:\n{text}");
        }
    }
}
