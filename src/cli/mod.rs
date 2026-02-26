pub mod add;
pub mod build;
pub mod check;
pub mod fmt;
pub mod new;
pub mod run;

use std::fmt::{Display, Formatter};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    Usage(String),
    Compilation(String),
    Runtime(String),
    Unsupported(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Compilation(_) => 1,
            Self::Runtime(_) => 1,
            Self::Unsupported(_) => 2,
        }
    }
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(msg)
            | Self::Compilation(msg)
            | Self::Runtime(msg)
            | Self::Unsupported(msg) => write!(f, "{msg}"),
        }
    }
}

pub fn run() -> Result<(), CliError> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Build(args)) => {
            let output = execute_build(&args).map_err(CliError::Compilation)?;
            println!("build: wrote executable {}", output.display());
            Ok(())
        }
        Some(Commands::Check(args)) => {
            execute_check(&args).map_err(CliError::Compilation)?;
            println!("check: validation passed");
            Ok(())
        }
        Some(Commands::Run(args)) => execute_run(&args).map_err(CliError::Runtime),
        Some(Commands::New) => Err(CliError::Unsupported(
            "new: not implemented yet".to_string(),
        )),
        Some(Commands::Add) => Err(CliError::Unsupported(
            "add: not implemented yet".to_string(),
        )),
        Some(Commands::Fmt) => Err(CliError::Unsupported(
            "fmt: not implemented yet".to_string(),
        )),
        None => Err(CliError::Usage(
            "usage: arwa <build|check|run|new|add|fmt>".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, CliError, Commands};

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

    #[test]
    fn maps_cli_error_exit_codes() {
        assert_eq!(CliError::Usage("x".to_string()).exit_code(), 2);
        assert_eq!(CliError::Compilation("x".to_string()).exit_code(), 1);
        assert_eq!(CliError::Runtime("x".to_string()).exit_code(), 1);
        assert_eq!(CliError::Unsupported("x".to_string()).exit_code(), 2);
    }
}
