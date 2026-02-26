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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{execute_check, CheckArgs};

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
    fn check_validates_minimal_project_without_codegen() {
        let unique = format!(
            "arwa-cli-check-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create temp dir");

        let input = base.join("main.rw");
        fs::write(&input, minimal_app_source()).expect("write source");

        let args = CheckArgs {
            input: Some(input.clone()),
        };
        execute_check(&args).expect("check should succeed");

        fs::remove_file(input).expect("cleanup input");
        fs::remove_dir(base).expect("cleanup base");
    }

    #[test]
    fn check_reports_validation_error_for_invalid_source() {
        let unique = format!(
            "arwa-cli-check-invalid-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create temp dir");

        let input = base.join("main.rw");
        fs::write(
            &input,
            "module App { provide Missing control Missing }\nclass X { fn m(): Int { return 1 } }\n",
        )
        .expect("write invalid source");

        let args = CheckArgs {
            input: Some(input.clone()),
        };
        let err = execute_check(&args).expect_err("check should fail");
        assert!(
            err.contains("parser:") || err.contains("resolver:") || err.contains("annotations:")
        );

        fs::remove_file(input).expect("cleanup input");
        fs::remove_dir(base).expect("cleanup base");
    }
}
