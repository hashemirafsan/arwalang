pub mod add;
pub mod build;
pub mod check;
pub mod fmt;
pub mod new;
pub mod run;

use clap::{Parser, Subcommand};

use self::build::{execute_build, BuildArgs};
use self::check::{execute_check, CheckArgs};
use self::run::{execute_run, RunArgs};

#[derive(Debug, Parser)]
#[command(name = "arwa", version, about = "ArwaLang compiler")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Build(BuildArgs),
    Check(CheckArgs),
    Run(RunArgs),
    New,
    Add,
    Fmt,
}

pub fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Build(args)) => {
            let output = execute_build(&args)?;
            println!("build: wrote executable {}", output.display());
            Ok(())
        }
        Some(Commands::Check(args)) => {
            execute_check(&args)?;
            println!("check: validation passed");
            Ok(())
        }
        Some(Commands::Run(args)) => execute_run(&args),
        Some(Commands::New) => Err("new: not implemented yet".to_string()),
        Some(Commands::Add) => Err("add: not implemented yet".to_string()),
        Some(Commands::Fmt) => Err("fmt: not implemented yet".to_string()),
        None => Err("usage: arwa <build|check|run|new|add|fmt>".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Commands};

    #[test]
    fn parses_build_command() {
        let cli =
            Cli::try_parse_from(["arwa", "build", "main.rw"]).expect("cli parsing should succeed");
        assert!(matches!(cli.command, Some(Commands::Build(_))));
    }

    #[test]
    fn parses_check_command() {
        let cli =
            Cli::try_parse_from(["arwa", "check", "main.rw"]).expect("cli parsing should succeed");
        assert!(matches!(cli.command, Some(Commands::Check(_))));
    }

    #[test]
    fn parses_run_command() {
        let cli =
            Cli::try_parse_from(["arwa", "run", "main.rw"]).expect("cli parsing should succeed");
        assert!(matches!(cli.command, Some(Commands::Run(_))));
    }
}
