use clap::Args;

use super::build::{load_and_validate_source, resolve_input_path};

/// CLI options for `arwa check`.
#[derive(Debug, Clone, Args)]
pub struct CheckArgs {
    /// Input source file.
    #[arg(value_name = "INPUT")]
    pub input: Option<std::path::PathBuf>,
}

/// Executes validation-only compiler pipeline (phases 1-9).
pub fn execute_check(args: &CheckArgs) -> Result<(), String> {
    let input = resolve_input_path(args.input.clone())?;
    let _ = load_and_validate_source(&input)?;
    Ok(())
}
