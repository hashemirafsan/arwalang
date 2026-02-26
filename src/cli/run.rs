use std::process::Command;

use clap::Args;

use super::build::{execute_build, BuildArgs};

/// CLI options for `arwa run`.
#[derive(Debug, Clone, Args)]
pub struct RunArgs {
    #[command(flatten)]
    pub build: BuildArgs,
}

/// Builds project and runs produced executable.
pub fn execute_run(args: &RunArgs) -> Result<(), String> {
    let output = execute_build(&args.build)?;

    let status = Command::new(&output)
        .status()
        .map_err(|err| format!("run: failed to execute '{}': {err}", output.display()))?;

    if !status.success() {
        return Err(format!("run: process exited with status {status}"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::cli::build::BuildArgs;

    use super::{execute_run, RunArgs};

    fn minimal_app_source() -> &'static str {
        r#"
module App {
  provide UserController
  control UserController
}

#[injectable]
#[controller("/users")]
class UserController {
  #[get("/")]
  fn list(res: Result<Int, HttpError>): Result<Int, HttpError> {
    return res
  }
}
"#
    }

    #[test]
    fn run_builds_and_executes_binary() {
        let unique = format!(
            "arwa-cli-run-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create temp dir");

        let input = base.join("main.rw");
        fs::write(&input, minimal_app_source()).expect("write source");

        let dist = base.join("dist");
        let args = RunArgs {
            build: BuildArgs {
                input: Some(input.clone()),
                dist: dist.clone(),
                name: Some("runapp".to_string()),
            },
        };

        execute_run(&args).expect("run should succeed");

        let output = dist.join("runapp");
        let object = dist.join("runapp.o");
        if object.exists() {
            fs::remove_file(object).expect("cleanup object");
        }
        if output.exists() {
            fs::remove_file(output).expect("cleanup output");
        }
        fs::remove_file(input).expect("cleanup input");
        fs::remove_dir(dist).expect("cleanup dist");
        fs::remove_dir(base).expect("cleanup base");
    }
}
