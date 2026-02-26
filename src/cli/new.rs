use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;

use super::templates::{write_blueprint, Blueprint};

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

/// Creates a new ArwaLang project from bundled starter template.
pub fn execute_new(args: &NewArgs) -> Result<PathBuf, String> {
    validate_project_name(&args.name)?;

    let project_dir = PathBuf::from(&args.name);
    if project_dir.exists() {
        return Err(format!(
            "new: target directory '{}' already exists",
            project_dir.display()
        ));
    }

    let starter_root = PathBuf::from("templates/starters").join(&args.starter);
    validate_starter(&starter_root, &args.starter)?;

    copy_dir_recursive(&starter_root, &project_dir)?;

    let blueprint = Blueprint {
        name: args.name.clone(),
        version: "0.1.0".to_string(),
        starter: args.starter.clone(),
        features: vec!["http".to_string(), "di".to_string()],
    };
    write_blueprint(&project_dir.join("arwa.blueprint.json"), &blueprint)?;

    Ok(project_dir)
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

fn validate_starter(starter_root: &Path, starter: &str) -> Result<(), String> {
    if starter_root.exists() {
        return Ok(());
    }

    let starters_dir = PathBuf::from("templates/starters");
    let supported = if starters_dir.exists() {
        let mut names = Vec::new();
        for entry in fs::read_dir(&starters_dir)
            .map_err(|err| format!("new: failed reading '{}': {err}", starters_dir.display()))?
        {
            let entry = entry.map_err(|err| format!("new: failed reading starter entry: {err}"))?;
            if entry.path().is_dir() {
                names.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        names.sort();
        names.join(", ")
    } else {
        "<none>".to_string()
    };

    Err(format!(
        "new: unsupported starter '{}'; supported starters: {}",
        starter, supported
    ))
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to)
        .map_err(|err| format!("new: failed creating '{}': {err}", to.display()))?;

    for entry in fs::read_dir(from)
        .map_err(|err| format!("new: failed reading '{}': {err}", from.display()))?
    {
        let entry = entry.map_err(|err| format!("new: failed reading directory entry: {err}"))?;
        let src_path = entry.path();
        let dst_path = to.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|err| {
                format!(
                    "new: failed copying '{}' to '{}': {err}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }

    Ok(())
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
        fs::create_dir_all(base.join("templates/starters/api/src"))
            .expect("create starter template dir");
        fs::write(
            base.join("templates/starters/api/src/main.rw"),
            "module App {\n  provide AppController\n  control AppController\n}\n",
        )
        .expect("write starter main");
        fs::write(
            base.join("templates/starters/api/src/app.controller.rw"),
            "#[injectable]\nclass AppController {\n}\n",
        )
        .expect("write starter controller");
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
        fs::remove_file(base.join("templates/starters/api/src/main.rw"))
            .expect("cleanup starter main");
        fs::remove_file(base.join("templates/starters/api/src/app.controller.rw"))
            .expect("cleanup starter controller");
        fs::remove_dir(base.join("templates/starters/api/src")).expect("cleanup starter src");
        fs::remove_dir(base.join("templates/starters/api")).expect("cleanup starter api");
        fs::remove_dir(base.join("templates/starters")).expect("cleanup starters");
        fs::remove_dir(base.join("templates")).expect("cleanup templates");
        fs::remove_dir(base).expect("cleanup base");
    }
}
