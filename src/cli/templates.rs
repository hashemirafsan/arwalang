use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Project blueprint persisted in `arwa.blueprint.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Blueprint {
    pub name: String,
    pub version: String,
    pub starter: String,
    #[serde(default)]
    pub features: Vec<String>,
}

/// Feature template metadata registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TemplateRegistry {
    #[serde(default)]
    pub features: Vec<RegistryFeature>,
}

/// One feature descriptor inside registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryFeature {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub usage: Vec<String>,
}

/// Reads and validates blueprint file from disk.
pub fn read_blueprint(path: &Path) -> Result<Blueprint, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("template: failed reading '{}': {err}", path.display()))?;
    let blueprint: Blueprint = serde_json::from_str(&raw)
        .map_err(|err| format!("template: invalid blueprint json: {err}"))?;
    validate_blueprint(&blueprint)?;
    Ok(blueprint)
}

/// Writes and validates blueprint file to disk.
pub fn write_blueprint(path: &Path, blueprint: &Blueprint) -> Result<(), String> {
    validate_blueprint(blueprint)?;
    let serialized = serde_json::to_string_pretty(blueprint)
        .map_err(|err| format!("template: failed serializing blueprint: {err}"))?;
    fs::write(path, serialized)
        .map_err(|err| format!("template: failed writing '{}': {err}", path.display()))
}

/// Validates blueprint schema constraints.
pub fn validate_blueprint(blueprint: &Blueprint) -> Result<(), String> {
    if blueprint.name.trim().is_empty() {
        return Err("template: blueprint.name must not be empty".to_string());
    }
    if blueprint.version.trim().is_empty() {
        return Err("template: blueprint.version must not be empty".to_string());
    }
    if blueprint.starter.trim().is_empty() {
        return Err("template: blueprint.starter must not be empty".to_string());
    }
    Ok(())
}

/// Reads and validates feature registry.
pub fn read_registry(path: &Path) -> Result<TemplateRegistry, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("template: failed reading '{}': {err}", path.display()))?;
    let registry: TemplateRegistry = serde_json::from_str(&raw)
        .map_err(|err| format!("template: invalid registry json: {err}"))?;
    validate_registry(&registry)?;
    Ok(registry)
}

/// Validates registry schema constraints.
pub fn validate_registry(registry: &TemplateRegistry) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for feature in &registry.features {
        if feature.name.trim().is_empty() {
            return Err("template: registry feature name must not be empty".to_string());
        }
        if !seen.insert(feature.name.clone()) {
            return Err(format!(
                "template: duplicate feature '{}' in registry",
                feature.name
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        read_blueprint, read_registry, validate_blueprint, validate_registry, Blueprint,
        TemplateRegistry,
    };

    #[test]
    fn validates_blueprint_schema_success() {
        let bp = Blueprint {
            name: "demo".to_string(),
            version: "0.1.0".to_string(),
            starter: "api".to_string(),
            features: vec![],
        };
        validate_blueprint(&bp).expect("blueprint should validate");
    }

    #[test]
    fn validates_blueprint_schema_failure() {
        let bp = Blueprint {
            name: "".to_string(),
            version: "0.1.0".to_string(),
            starter: "api".to_string(),
            features: vec![],
        };
        let err = validate_blueprint(&bp).expect_err("blueprint should fail validation");
        assert!(err.contains("blueprint.name"));
    }

    #[test]
    fn validates_registry_schema_duplicate_feature_failure() {
        let reg = TemplateRegistry {
            features: vec![
                super::RegistryFeature {
                    name: "logger".to_string(),
                    description: "x".to_string(),
                    files: vec![],
                    dependencies: vec![],
                    usage: vec![],
                },
                super::RegistryFeature {
                    name: "logger".to_string(),
                    description: "y".to_string(),
                    files: vec![],
                    dependencies: vec![],
                    usage: vec![],
                },
            ],
        };
        let err = validate_registry(&reg).expect_err("registry should fail validation");
        assert!(err.contains("duplicate feature"));
    }

    #[test]
    fn parses_blueprint_json_file() {
        let unique = format!(
            "arwa-blueprint-parse-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(format!("{unique}.json"));
        fs::write(
            &path,
            r#"{"name":"demo","version":"0.1.0","starter":"api","features":["logger"]}"#,
        )
        .expect("write blueprint file");

        let parsed = read_blueprint(&path).expect("blueprint should parse");
        assert_eq!(parsed.name, "demo");
        assert_eq!(parsed.starter, "api");
        assert_eq!(parsed.features, vec!["logger".to_string()]);

        fs::remove_file(path).expect("cleanup blueprint file");
    }

    #[test]
    fn parses_registry_json_file() {
        let unique = format!(
            "arwa-registry-parse-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(format!("{unique}.json"));
        fs::write(
            &path,
            r#"{"features":[{"name":"logger","description":"log","files":["src/features/logger.rw"],"dependencies":[],"usage":["use logger"]}]}"#,
        )
        .expect("write registry file");

        let parsed = read_registry(&path).expect("registry should parse");
        assert_eq!(parsed.features.len(), 1);
        assert_eq!(parsed.features[0].name, "logger");

        fs::remove_file(path).expect("cleanup registry file");
    }
}
