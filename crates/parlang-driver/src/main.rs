use std::fs;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Run a `ParLang` `.par` file through parse + eval and print the resulting value.
#[derive(Parser)]
#[command(name = "parlang-driver")]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
#[command(group(clap::ArgGroup::new("dump_mode").args(["dump", "dump_dot"])))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to the `.par` source file to evaluate.
    #[arg(required = true)]
    path: Option<std::path::PathBuf>,

    /// Dump the AST as text IR to stdout, then exit (skips evaluation).
    #[arg(long, conflicts_with = "dump_dot")]
    dump: bool,

    /// Dump the AST as Graphviz DOT to stdout, then exit (skips evaluation).
    #[arg(long)]
    dump_dot: bool,

    /// Pipeline stage to dump: `parsed` (default), `typed`, `core` or `lint` (requires --dump or --dump-dot).
    #[arg(long, value_enum, requires = "dump_mode")]
    stage: Option<Stage>,

    /// Evaluate the program on the Core CEK machine instead of the surface evaluator.
    #[arg(long, conflicts_with = "dump_mode")]
    core: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Trace a program's evaluation on the Core CEK machine.
    Trace {
        /// Path to the `.par` source file to trace.
        path: std::path::PathBuf,
        /// Maximum number of machine steps before giving up.
        #[arg(long, default_value_t = 10_000)]
        fuel: usize,
        /// Print every raw machine step instead of the reduction view.
        #[arg(long)]
        raw: bool,
    },
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

/// Dump `program` at the chosen `stage`, as DOT when `dot` is set.
///
/// Runs `typecheck_program_typed` for the `typed` stage; a type error prints
/// `type error: <e>` to stderr and returns a failing `ExitCode`.
fn run_dump(program: &parlang::Program, stage: Stage, dot: bool) -> ExitCode {
    match stage {
        Stage::Parsed => {
            if dot {
                println!("{}", parlang::program_to_dot(program));
            } else {
                println!("{program}");
            }
            ExitCode::SUCCESS
        }
        Stage::Typed => match parlang::typecheck_program_typed(program) {
            Ok(typed) => {
                if dot {
                    println!("{}", parlang::typed_program_to_dot(&typed));
                } else {
                    print!("{}", parlang::typed_program_to_text(&typed));
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("type error: {e}");
                ExitCode::FAILURE
            }
        },
        Stage::Core => print_outcome(parlang::pipeline::dump_core(program, dot)),
        Stage::Lint => print_outcome(parlang::pipeline::dump_lint(program, dot)),
    }
}

/// Print a pipeline `outcome`: `Ok` text to stdout, `Err` message to stderr with a failing code.
fn print_outcome(outcome: Result<String, String>) -> ExitCode {
    match outcome {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Read and parse the `.par` file at `path`, returning the user-facing error on failure.
fn load_program(path: &std::path::Path) -> Result<parlang::Program, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("error: cannot read {}: {err}", path.display()))?;
    parlang::parse_program(&source).map_err(|err| format!("parse error: {err}"))
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Some(Commands::Trace { path, fuel, raw }) = &cli.command {
        return print_outcome(
            load_program(path)
                .and_then(|program| parlang::pipeline::trace_core(&program, *fuel, *raw)),
        );
    }

    // Defensive: clap's `required = true` on `path` (negated only by a subcommand)
    // makes this branch unreachable when no subcommand is given.
    let Some(path) = cli.path.as_deref() else {
        eprintln!("error: missing <PATH>");
        return ExitCode::FAILURE;
    };

    let program = match load_program(path) {
        Ok(program) => program,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::FAILURE;
        }
    };

    if cli.dump || cli.dump_dot {
        let stage = cli.stage.unwrap_or(Stage::Parsed);
        return run_dump(&program, stage, cli.dump_dot);
    }

    if cli.core {
        return print_outcome(parlang::pipeline::run_core(&program));
    }

    if let Err(err) = parlang::typecheck_program(&program) {
        eprintln!("type error: {err}");
        return ExitCode::FAILURE;
    }

    match parlang::run_on_evaluator_stack(move || {
        let env = parlang::Environment::new();
        parlang::eval_program(&program, &env)
            .map(|value| format!("{value}"))
            .map_err(|error| format!("eval error: {error}"))
    }) {
        Ok(Ok(value)) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Ok(Err(error)) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("eval error: {error}");
            ExitCode::FAILURE
        }
    }
}
