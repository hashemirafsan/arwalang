use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use serde::{Deserialize, Serialize};

/// CLI options for `arwa add`.
#[derive(Debug, Clone, Args)]
pub struct AddArgs {
    /// Feature name to add.
    #[arg(value_name = "FEATURE")]
    pub feature: String,
}

#[derive(Debug, Deserialize)]
struct Registry {
    #[serde(default)]
    features: Vec<RegistryFeature>,
}

#[derive(Debug, Deserialize)]
struct RegistryFeature {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Blueprint {
    name: String,
    version: String,
    starter: String,
    #[serde(default)]
    features: Vec<String>,
}

/// Adds a feature scaffold to the current Arwa project.
pub fn execute_add(args: &AddArgs) -> Result<(), String> {
    let mut blueprint = read_blueprint(Path::new("arwa.blueprint.json"))?;
    validate_feature_exists(&args.feature)?;

    if blueprint.features.iter().any(|f| f == &args.feature) {
        return Ok(());
    }

    copy_feature_template_or_stub(&args.feature)?;
    blueprint.features.push(args.feature.clone());
    blueprint.features.sort();

    let serialized = serde_json::to_string_pretty(&blueprint)
        .map_err(|err| format!("add: failed serializing blueprint: {err}"))?;
    fs::write("arwa.blueprint.json", serialized)
        .map_err(|err| format!("add: failed writing blueprint: {err}"))?;

    Ok(())
}

fn read_blueprint(path: &Path) -> Result<Blueprint, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("add: failed reading '{}': {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("add: invalid blueprint json: {err}"))
}

fn validate_feature_exists(feature: &str) -> Result<(), String> {
    let mut known = vec![
        "http".to_string(),
        "di".to_string(),
        "logger".to_string(),
        "auth-jwt".to_string(),
        "db-postgres".to_string(),
    ];

    let registry_path = Path::new("templates/registry.json");
    if registry_path.exists() {
        let raw = fs::read_to_string(registry_path)
            .map_err(|err| format!("add: failed reading registry: {err}"))?;
        let reg: Registry = serde_json::from_str(&raw)
            .map_err(|err| format!("add: invalid registry json: {err}"))?;
        known.extend(reg.features.into_iter().map(|f| f.name));
    }

    if known.iter().any(|name| name == feature) {
        Ok(())
    } else {
        Err(format!("add: unknown feature '{feature}'"))
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
        fs::write(base.join("templates/registry.json"), "{\"features\":[]}")
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
