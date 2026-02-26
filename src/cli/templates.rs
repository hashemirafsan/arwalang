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

const EMBEDDED_TEMPLATE_FILES: &[(&str, &str)] = &[
    (
        "templates/registry.json",
        include_str!("../../templates/registry.json"),
    ),
    (
        "templates/starters/minimal/src/main.rw",
        include_str!("../../templates/starters/minimal/src/main.rw"),
    ),
    (
        "templates/starters/api/src/main.rw",
        include_str!("../../templates/starters/api/src/main.rw"),
    ),
    (
        "templates/starters/api/src/app.controller.rw",
        include_str!("../../templates/starters/api/src/app.controller.rw"),
    ),
    (
        "templates/starters/api/src/app.service.rw",
        include_str!("../../templates/starters/api/src/app.service.rw"),
    ),
    (
        "templates/starters/api/src/create-app.dto.rw",
        include_str!("../../templates/starters/api/src/create-app.dto.rw"),
    ),
    (
        "templates/starters/full/src/main.rw",
        include_str!("../../templates/starters/full/src/main.rw"),
    ),
    (
        "templates/starters/full/src/app.controller.rw",
        include_str!("../../templates/starters/full/src/app.controller.rw"),
    ),
    (
        "templates/starters/full/src/auth.service.rw",
        include_str!("../../templates/starters/full/src/auth.service.rw"),
    ),
    (
        "templates/starters/full/src/database.service.rw",
        include_str!("../../templates/starters/full/src/database.service.rw"),
    ),
    (
        "templates/starters/full/src/logger.service.rw",
        include_str!("../../templates/starters/full/src/logger.service.rw"),
    ),
    (
        "templates/starters/full/src/auth.guard.rw",
        include_str!("../../templates/starters/full/src/auth.guard.rw"),
    ),
    (
        "templates/starters/full/src/create-user.dto.rw",
        include_str!("../../templates/starters/full/src/create-user.dto.rw"),
    ),
    (
        "templates/features/http/src/features/http.rw",
        include_str!("../../templates/features/http/src/features/http.rw"),
    ),
    (
        "templates/features/http/src/features/http.utils.rw",
        include_str!("../../templates/features/http/src/features/http.utils.rw"),
    ),
    (
        "templates/features/http/src/features/http.decorators.rw",
        include_str!("../../templates/features/http/src/features/http.decorators.rw"),
    ),
    (
        "templates/features/di/src/features/di.rw",
        include_str!("../../templates/features/di/src/features/di.rw"),
    ),
    (
        "templates/features/di/src/features/di.advanced.rw",
        include_str!("../../templates/features/di/src/features/di.advanced.rw"),
    ),
    (
        "templates/features/di/src/features/di.scopes.rw",
        include_str!("../../templates/features/di/src/features/di.scopes.rw"),
    ),
    (
        "templates/features/logger/src/features/logger.rw",
        include_str!("../../templates/features/logger/src/features/logger.rw"),
    ),
    (
        "templates/features/logger/src/features/logger.service.rw",
        include_str!("../../templates/features/logger/src/features/logger.service.rw"),
    ),
    (
        "templates/features/logger/src/features/logger.usage.rw",
        include_str!("../../templates/features/logger/src/features/logger.usage.rw"),
    ),
    (
        "templates/features/auth-jwt/src/features/auth-jwt.rw",
        include_str!("../../templates/features/auth-jwt/src/features/auth-jwt.rw"),
    ),
    (
        "templates/features/auth-jwt/src/features/jwt.guard.rw",
        include_str!("../../templates/features/auth-jwt/src/features/jwt.guard.rw"),
    ),
    (
        "templates/features/auth-jwt/src/features/jwt.utils.rw",
        include_str!("../../templates/features/auth-jwt/src/features/jwt.utils.rw"),
    ),
    (
        "templates/features/auth-jwt/src/features/auth.example.rw",
        include_str!("../../templates/features/auth-jwt/src/features/auth.example.rw"),
    ),
    (
        "templates/features/db-postgres/src/features/db-postgres.rw",
        include_str!("../../templates/features/db-postgres/src/features/db-postgres.rw"),
    ),
    (
        "templates/features/db-postgres/src/features/db.repository.rw",
        include_str!("../../templates/features/db-postgres/src/features/db.repository.rw"),
    ),
    (
        "templates/features/db-postgres/src/features/db.migrations.rw",
        include_str!("../../templates/features/db-postgres/src/features/db.migrations.rw"),
    ),
];

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

/// Ensures template tree exists on disk by extracting embedded templates when missing.
pub fn ensure_templates_on_disk(project_root: &Path) -> Result<(), String> {
    let registry = project_root.join("templates/registry.json");
    if registry.exists() {
        return Ok(());
    }

    extract_embedded_templates(project_root)
}

/// Extracts all embedded templates into a target project root.
pub fn extract_embedded_templates(project_root: &Path) -> Result<(), String> {
    for (relative, contents) in EMBEDDED_TEMPLATE_FILES {
        let out = project_root.join(relative);
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("template: failed creating '{}': {err}", parent.display())
            })?;
        }
        fs::write(&out, contents)
            .map_err(|err| format!("template: failed writing '{}': {err}", out.display()))?;
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
