use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::codegen::CodegenError;

/// Writes generated object bytes into dist output directory.
pub fn write_object_file(
    module_name: &str,
    object_bytes: &[u8],
    dist_dir: &Path,
) -> Result<PathBuf, CodegenError> {
    fs::create_dir_all(dist_dir).map_err(|err| CodegenError::Backend {
        message: format!("failed to create dist dir '{}': {err}", dist_dir.display()),
    })?;

    let object_path = dist_dir.join(format!("{module_name}.o"));
    fs::write(&object_path, object_bytes).map_err(|err| CodegenError::Backend {
        message: format!(
            "failed to write object file '{}': {err}",
            object_path.display()
        ),
    })?;

    Ok(object_path)
}

/// Links one object file into executable using system C toolchain.
pub fn link_executable(object_file: &Path, output_binary: &Path) -> Result<(), CodegenError> {
    let runtime_lib = build_runtime_staticlib()?;

    let status = Command::new("cc")
        .arg(object_file)
        .arg(&runtime_lib)
        .arg("-o")
        .arg(output_binary)
        .status()
        .map_err(|err| CodegenError::Backend {
            message: format!("failed to invoke linker: {err}"),
        })?;

    if !status.success() {
        return Err(CodegenError::Backend {
            message: format!(
                "linker exited with status {status} for output '{}' using runtime '{}'",
                output_binary.display(),
                runtime_lib.display()
            ),
        });
    }

    Ok(())
}

fn build_runtime_staticlib() -> Result<PathBuf, CodegenError> {
    let manifest = runtime_manifest_path();
    let status = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--release")
        .status()
        .map_err(|err| CodegenError::Backend {
            message: format!(
                "failed to build runtime crate from '{}': {err}",
                manifest.display()
            ),
        })?;

    if !status.success() {
        return Err(CodegenError::Backend {
            message: format!("runtime build failed with status {status}"),
        });
    }

    let runtime_lib = runtime_staticlib_path();
    if !runtime_lib.exists() {
        return Err(CodegenError::Backend {
            message: format!(
                "runtime static library missing after build: '{}'",
                runtime_lib.display()
            ),
        });
    }

    Ok(runtime_lib)
}

fn runtime_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime/Cargo.toml")
}

fn runtime_staticlib_path() -> PathBuf {
    let file_name = if cfg!(target_os = "windows") {
        "arwa_runtime.lib"
    } else {
        "libarwa_runtime.a"
    };
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("runtime/target/release")
        .join(file_name)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::write_object_file;

    #[test]
    fn writes_object_file_into_dist_directory() {
        let unique = format!(
            "arwa-codegen-linker-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let dist = std::env::temp_dir().join(unique);

        let output = write_object_file("app", &[1_u8, 2_u8, 3_u8], &dist).expect("must write");
        assert!(output.exists());

        let bytes = fs::read(&output).expect("must read object back");
        assert_eq!(bytes, vec![1_u8, 2_u8, 3_u8]);

        fs::remove_file(&output).expect("cleanup file");
        fs::remove_dir(&dist).expect("cleanup dir");
    }
}
