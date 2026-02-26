use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;

use super::templates::{read_blueprint, read_registry, write_blueprint};

/// CLI options for `arwa add`.
#[derive(Debug, Clone, Args)]
pub struct AddArgs {
    /// Feature name to add.
    #[arg(value_name = "FEATURE")]
    pub feature: String,
}

/// Adds a feature scaffold to the current Arwa project.
pub fn execute_add(args: &AddArgs) -> Result<(), String> {
    let mut blueprint =
        read_blueprint(Path::new("arwa.blueprint.json")).map_err(|err| format!("add: {err}"))?;
    validate_feature_exists(&args.feature)?;

    if blueprint.features.iter().any(|f| f == &args.feature) {
        return Ok(());
    }

    copy_feature_template_or_stub(&args.feature)?;
    blueprint.features.push(args.feature.clone());
    blueprint.features.sort();

    write_blueprint(Path::new("arwa.blueprint.json"), &blueprint)
        .map_err(|err| format!("add: {err}"))?;

    Ok(())
}

fn validate_feature_exists(feature: &str) -> Result<(), String> {
    let registry_path = Path::new("templates/registry.json");
    let reg = read_registry(registry_path).map_err(|err| format!("add: {err}"))?;
    let mut known = reg
        .features
        .iter()
        .map(|f| f.name.clone())
        .collect::<Vec<_>>();
    known.sort();

    if known.iter().any(|name| name == feature) {
        Ok(())
    } else {
        Err(format!(
            "add: unknown feature '{feature}'; available features: {}",
            known.join(", ")
        ))
    }
}

fn copy_feature_template_or_stub(feature: &str) -> Result<(), String> {
    let template_root = PathBuf::from("templates/features").join(feature);
    if template_root.exists() {
        copy_dir_recursive(&template_root, Path::new("."))?;
        return Ok(());
    }

    let feature_dir = PathBuf::from("src/features");
    fs::create_dir_all(&feature_dir)
        .map_err(|err| format!("add: create feature dir failed: {err}"))?;
    let feature_file = feature_dir.join(format!("{feature}.rw"));
    fs::write(
        &feature_file,
        format!(
            "// feature scaffold: {feature}\nmodule {}Feature {{\n}}\n",
            sanitize_ident(feature)
        ),
    )
    .map_err(|err| format!("add: write feature scaffold failed: {err}"))?;
    Ok(())
}

fn sanitize_ident(feature: &str) -> String {
    feature
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
    for entry in
        fs::read_dir(from).map_err(|err| format!("add: read template dir failed: {err}"))?
    {
        let entry = entry.map_err(|err| format!("add: read template entry failed: {err}"))?;
        let path = entry.path();
        let rel = path
            .strip_prefix(from)
            .map_err(|err| format!("add: template path error: {err}"))?;
        let dst = to.join(rel);

        if path.is_dir() {
            fs::create_dir_all(&dst).map_err(|err| format!("add: create dir failed: {err}"))?;
            copy_dir_recursive(&path, &dst)?;
        } else {
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("add: create dir failed: {err}"))?;
            }
            fs::copy(&path, &dst).map_err(|err| format!("add: copy file failed: {err}"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::cli::cwd_test_lock;

    use super::{execute_add, AddArgs};

    #[test]
    fn add_updates_blueprint_and_creates_feature_scaffold() {
        let _guard = cwd_test_lock().lock().expect("acquire cwd lock");

        let unique = format!(
            "arwa-add-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(base.join("templates")).expect("create templates dir");
        fs::write(
            base.join("templates/registry.json"),
            r#"{"features":[{"name":"logger","description":"x","files":[],"dependencies":[],"usage":[]}]}"#,
        )
        .expect("write registry");
        fs::write(
            base.join("arwa.blueprint.json"),
            r#"{"name":"demo","version":"0.1.0","starter":"api","features":[]}"#,
        )
        .expect("write blueprint");

        let old_cwd = std::env::current_dir().expect("read cwd");
        std::env::set_current_dir(&base).expect("set cwd");

        execute_add(&AddArgs {
            feature: "logger".to_string(),
        })
        .expect("add should succeed");

        assert!(base.join("src/features/logger.rw").exists());
        let blueprint =
            fs::read_to_string(base.join("arwa.blueprint.json")).expect("read blueprint");
        assert!(blueprint.contains("logger"));

        std::env::set_current_dir(old_cwd).expect("restore cwd");
        fs::remove_file(base.join("src/features/logger.rw")).expect("cleanup feature file");
        fs::remove_dir(base.join("src/features")).expect("cleanup features dir");
        fs::remove_dir(base.join("src")).expect("cleanup src dir");
        fs::remove_file(base.join("arwa.blueprint.json")).expect("cleanup blueprint");
        fs::remove_file(base.join("templates/registry.json")).expect("cleanup registry");
        fs::remove_dir(base.join("templates")).expect("cleanup templates dir");
        fs::remove_dir(base).expect("cleanup base");
    }
}
