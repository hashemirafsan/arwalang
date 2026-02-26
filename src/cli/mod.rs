pub mod add;
pub mod build;
pub mod check;
pub mod fmt;
pub mod new;
pub mod run;

use std::fmt::{Display, Formatter};

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use clap::{Parser, Subcommand};

use self::add::{execute_add, AddArgs};
use self::build::{execute_build, BuildArgs};
use self::check::{execute_check, CheckArgs};
use self::fmt::{execute_fmt, FmtArgs};
use self::new::{execute_new, NewArgs};
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
    Version,
    New(NewArgs),
    Add(AddArgs),
    Fmt(FmtArgs),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    Usage(String),
    Compilation(String),
    Runtime(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Usage(_) => 2,
            Self::Compilation(_) => 1,
            Self::Runtime(_) => 1,
        }
    }
}

impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(msg) | Self::Compilation(msg) | Self::Runtime(msg) => write!(f, "{msg}"),
        }
    }
}

#[cfg(test)]
pub(crate) fn cwd_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub fn run() -> Result<(), CliError> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Build(args)) => {
            let output = execute_build(&args)
                .map_err(|err| CliError::Compilation(enrich_error_message(&err)))?;
            println!("build: wrote executable {}", output.display());
            Ok(())
        }
        Some(Commands::Check(args)) => {
            execute_check(&args)
                .map_err(|err| CliError::Compilation(enrich_error_message(&err)))?;
            println!("check: validation passed");
            Ok(())
        }
        Some(Commands::Run(args)) => {
            execute_run(&args).map_err(|err| CliError::Runtime(enrich_error_message(&err)))
        }
        Some(Commands::Version) => {
            println!("arwa {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(Commands::New(args)) => {
            let created =
                execute_new(&args).map_err(|err| CliError::Runtime(enrich_error_message(&err)))?;
            println!("new: created project {}", created.display());
            Ok(())
        }
        Some(Commands::Add(args)) => {
            execute_add(&args).map_err(|err| CliError::Runtime(enrich_error_message(&err)))?;
            println!("add: feature '{}' applied", args.feature);
            Ok(())
        }
        Some(Commands::Fmt(args)) => {
            let changed =
                execute_fmt(&args).map_err(|err| CliError::Runtime(enrich_error_message(&err)))?;
            if args.check {
                println!("fmt: all files are properly formatted");
            } else {
                println!("fmt: formatted {changed} file(s)");
            }
            Ok(())
        }
        None => Err(CliError::Usage(
            "usage: arwa <build|check|run|version|new|add|fmt>".to_string(),
        )),
    }
}

fn enrich_error_message(message: &str) -> String {
    if message.contains("io: no input file provided") {
        return format!(
            "{message}\nhelp: provide an input path or run inside a project with .rw files under src/"
        );
    }

    if message.contains("failed to invoke linker")
        || message.contains("failed to build runtime crate")
        || message.contains("native isa setup failed")
    {
        return format!(
            "{message}\nhelp: ensure Rust toolchain and a system C linker are installed and available in PATH"
        );
    }

    if message.contains("No such file or directory") {
        return format!("{message}\nhelp: verify file paths and current working directory");
    }

    message.to_string()
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{enrich_error_message, Cli, CliError, Commands};

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
    fn parses_version_command() {
        let cli = Cli::try_parse_from(["arwa", "version"]).expect("parse version");
        assert!(matches!(cli.command, Some(Commands::Version)));
    }

    #[test]
    fn parses_new_command_with_name() {
        let cli =
            Cli::try_parse_from(["arwa", "new", "myapp"]).expect("cli parsing should succeed");
        assert!(matches!(cli.command, Some(Commands::New(_))));
    }

    #[test]
    fn maps_cli_error_exit_codes() {
        assert_eq!(CliError::Usage("x".to_string()).exit_code(), 2);
        assert_eq!(CliError::Compilation("x".to_string()).exit_code(), 1);
        assert_eq!(CliError::Runtime("x".to_string()).exit_code(), 1);
    }

    #[test]
    fn parses_add_command() {
        let cli = Cli::try_parse_from(["arwa", "add", "logger"]).expect("parse add");
        assert!(matches!(cli.command, Some(Commands::Add(_))));
    }

    #[test]
    fn parses_fmt_command() {
        let cli = Cli::try_parse_from(["arwa", "fmt", "--check"]).expect("parse fmt");
        assert!(matches!(cli.command, Some(Commands::Fmt(_))));
    }

    #[test]
    fn enriches_missing_input_error_with_help() {
        let msg = enrich_error_message("io: no input file provided and none found");
        assert!(msg.contains("help:"));
    }
}
