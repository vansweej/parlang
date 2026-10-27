use std::fs;
use std::process::ExitCode;

use clap::Parser;

/// Run a ParLang `.par` file through parse + eval and print the resulting value.
#[derive(Parser)]
#[command(name = "parlang-driver")]
#[command(group(clap::ArgGroup::new("dump_mode").args(["dump", "dump_dot"])))]
struct Cli {
    /// Path to the `.par` source file to evaluate.
    path: std::path::PathBuf,

    /// Dump the AST as text IR to stdout, then exit (skips evaluation).
    #[arg(long, conflicts_with = "dump_dot")]
    dump: bool,

    /// Dump the AST as Graphviz DOT to stdout, then exit (skips evaluation).
    #[arg(long)]
    dump_dot: bool,

    /// Pipeline stage to dump: `parsed` (default) or `typed` (requires --dump or --dump-dot).
    #[arg(long, value_enum, requires = "dump_mode")]
    stage: Option<Stage>,
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
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let source = match fs::read_to_string(&cli.path) {
        Ok(src) => src,
        Err(err) => {
            let path = cli.path.display();
            eprintln!("error: cannot read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };

    let program = match parlang::parse_program(&source) {
        Ok(program) => program,
        Err(err) => {
            eprintln!("parse error: {err}");
            return ExitCode::FAILURE;
        }
    };

    if cli.dump || cli.dump_dot {
        let stage = cli.stage.unwrap_or(Stage::Parsed);
        return run_dump(&program, stage, cli.dump_dot);
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
