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
    let bootstrap_source = output_binary.with_extension("bootstrap.c");
    fs::write(&bootstrap_source, "int main(void) { return 0; }\n").map_err(|err| {
        CodegenError::Backend {
            message: format!(
                "failed to write bootstrap source '{}': {err}",
                bootstrap_source.display()
            ),
        }
    })?;

    let status = Command::new("cc")
        .arg(object_file)
        .arg(&bootstrap_source)
        .arg("-o")
        .arg(output_binary)
        .status()
        .map_err(|err| CodegenError::Backend {
            message: format!("failed to invoke linker: {err}"),
        })?;

    if !status.success() {
        let _ = fs::remove_file(&bootstrap_source);
        return Err(CodegenError::Backend {
            message: format!(
                "linker exited with status {status} for output '{}'",
                output_binary.display()
            ),
        });
    }

    let _ = fs::remove_file(&bootstrap_source);

    Ok(())
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
