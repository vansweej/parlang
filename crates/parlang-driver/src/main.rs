use std::fs;
use std::process::ExitCode;

use clap::Parser;

/// Run a ParLang `.par` file through parse + eval and print the resulting value.
#[derive(Parser)]
#[command(name = "parlang-driver")]
struct Cli {
    /// Path to the `.par` source file to evaluate.
    path: std::path::PathBuf,

    /// Dump the parsed AST as text IR to stdout, then exit (skips evaluation).
    #[arg(long, conflicts_with = "dump_dot")]
    dump: bool,

    /// Dump the parsed AST as Graphviz DOT to stdout, then exit (skips evaluation).
    #[arg(long)]
    dump_dot: bool,
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

    if cli.dump {
        println!("{program}");
        return ExitCode::SUCCESS;
    }
    if cli.dump_dot {
        println!("{}", parlang::program_to_dot(&program));
        return ExitCode::SUCCESS;
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
