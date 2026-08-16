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

    let expr = match parlang::parse(&source) {
        Ok(expr) => expr,
        Err(err) => {
            eprintln!("parse error: {err}");
            return ExitCode::FAILURE;
        }
    };

    if cli.dump {
        println!("{expr}");
        return ExitCode::SUCCESS;
    }
    if cli.dump_dot {
        println!("{}", parlang::dot::ast_to_dot(&expr));
        return ExitCode::SUCCESS;
    }

    let env = parlang::Environment::new();
    match parlang::eval(&expr, &env) {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("eval error: {err}");
            ExitCode::FAILURE
        }
    }
}
