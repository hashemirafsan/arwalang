use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde::Serialize;

/// CLI options for `arwa new`.
#[derive(Debug, Clone, Args)]
pub struct NewArgs {
    /// Project directory name.
    #[arg(value_name = "NAME")]
    pub name: String,

    /// Starter template name.
    #[arg(long, default_value = "api")]
    pub starter: String,
}

#[derive(Debug, Serialize)]
struct Blueprint {
    name: String,
    version: String,
    starter: String,
    features: Vec<String>,
}

/// Creates a new ArwaLang project from bundled starter template.
pub fn execute_new(args: &NewArgs) -> Result<PathBuf, String> {
    validate_project_name(&args.name)?;
    validate_starter(&args.starter)?;

    let project_dir = PathBuf::from(&args.name);
    if project_dir.exists() {
        return Err(format!(
            "new: target directory '{}' already exists",
            project_dir.display()
        ));
    }

    fs::create_dir_all(project_dir.join("src"))
        .map_err(|err| format!("new: failed creating project directories: {err}"))?;

    let (module_source, controller_source) = starter_sources(&args.name, &args.starter);
    write_file(&project_dir.join("src/main.rw"), module_source)?;
    write_file(
        &project_dir.join("src/app.controller.rw"),
        &controller_source,
    )?;

    let blueprint = Blueprint {
        name: args.name.clone(),
        version: "0.1.0".to_string(),
        starter: args.starter.clone(),
        features: vec!["http".to_string(), "di".to_string()],
    };
    let blueprint_text = serde_json::to_string_pretty(&blueprint)
        .map_err(|err| format!("new: failed serializing blueprint: {err}"))?;
    write_file(&project_dir.join("arwa.blueprint.json"), &blueprint_text)?;

    Ok(project_dir)
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents)
        .map_err(|err| format!("new: failed writing '{}': {err}", path.display()))
}

fn validate_project_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    if !valid {
        return Err("new: invalid project name; use only letters, digits, '-' and '_'".to_string());
    }
    Ok(())
}

fn validate_starter(starter: &str) -> Result<(), String> {
    if starter == "api" || starter == "minimal" {
        Ok(())
    } else {
        Err(format!(
            "new: unsupported starter '{}'; supported starters: api, minimal",
            starter
        ))
    }
}

fn starter_sources(name: &str, starter: &str) -> (&'static str, String) {
    let module = if starter == "minimal" {
        "module App {\n}\n"
    } else {
        "module App {\n  provide AppController\n  control AppController\n}\n"
    };

    let controller = if starter == "minimal" {
        "#[injectable]\nclass AppController {\n}\n".to_string()
    } else {
        format!(
            "#[injectable]\n#[controller(\"/\")]\nclass AppController {{\n  #[get(\"/\")]\n  fn hello(res: Result<String, HttpError>): Result<String, HttpError> {{\n    return res\n  }}\n}}\n// project: {name}\n"
        )
    };

    (module, controller)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::cli::cwd_test_lock;

    use super::{execute_new, NewArgs};

    #[test]
    fn new_creates_project_structure_and_blueprint() {
        let _guard = cwd_test_lock().lock().expect("acquire cwd lock");

        let unique = format!(
            "arwa-new-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create base dir");
        let old_cwd = std::env::current_dir().expect("read cwd");
        std::env::set_current_dir(&base).expect("set cwd");

        let project_name = "sample_api".to_string();
        let project_path = base.join(&project_name);
        let created = execute_new(&NewArgs {
            name: project_name.clone(),
            starter: "api".to_string(),
        })
        .expect("new command should succeed");

        assert!(created.exists());
        assert!(project_path.join("src/main.rw").exists());
        assert!(project_path.join("src/app.controller.rw").exists());
        assert!(project_path.join("arwa.blueprint.json").exists());

        std::env::set_current_dir(old_cwd).expect("restore cwd");
        fs::remove_file(project_path.join("src/main.rw")).expect("cleanup main source");
        fs::remove_file(project_path.join("src/app.controller.rw")).expect("cleanup controller");
        fs::remove_file(project_path.join("arwa.blueprint.json")).expect("cleanup blueprint");
        fs::remove_dir(project_path.join("src")).expect("cleanup src");
        fs::remove_dir(project_path).expect("cleanup project dir");
        fs::remove_dir(base).expect("cleanup base");
    }
}
