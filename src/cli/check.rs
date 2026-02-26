use clap::Args;

use super::build::{load_and_validate_sources, resolve_input_paths};

/// CLI options for `arwa check`.
#[derive(Debug, Clone, Args)]
pub struct CheckArgs {
    /// Input source file.
    #[arg(value_name = "INPUT")]
    pub input: Option<std::path::PathBuf>,
}

/// Executes validation-only compiler pipeline (phases 1-9).
pub fn execute_check(args: &CheckArgs) -> Result<(), String> {
    let inputs = resolve_input_paths(args.input.clone())?;
    let _ = load_and_validate_sources(&inputs)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;

    use crate::cli::cwd_test_lock;

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

    #[test]
    fn check_discovers_sources_from_src_directory() {
        let _guard = cwd_test_lock().lock().expect("acquire cwd lock");

        let unique = format!(
            "arwa-cli-check-discovery-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        let src = base.join("src");
        fs::create_dir_all(&src).expect("create src dir");

        fs::write(
            src.join("main.rw"),
            r#"
module App {
  provide UserController
  control UserController
}
"#,
        )
        .expect("write main module");

        fs::write(
            src.join("user_controller.rw"),
            r#"
#[injectable]
#[controller("/users")]
class UserController {
  #[get("/")]
  fn list(res: Result<Int, HttpError>): Result<Int, HttpError> {
    return res
  }
}
"#,
        )
        .expect("write controller");

        let old_cwd = env::current_dir().expect("read cwd");
        env::set_current_dir(&base).expect("set cwd");

        let args = CheckArgs { input: None };
        execute_check(&args).expect("check should succeed from discovered sources");

        env::set_current_dir(old_cwd).expect("restore cwd");
        fs::remove_file(src.join("main.rw")).expect("cleanup main source");
        fs::remove_file(src.join("user_controller.rw")).expect("cleanup controller source");
        fs::remove_dir(src).expect("cleanup src dir");
        fs::remove_dir(base).expect("cleanup base");
    }
}
