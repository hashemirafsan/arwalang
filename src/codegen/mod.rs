#![allow(dead_code)]

use thiserror::Error;

use crate::ir::IrType;
use std::path::{Path, PathBuf};

pub mod cranelift;
pub mod linker;

use crate::ir::IrModule;

/// Native code generation errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodegenError {
    #[error("codegen backend error: {message}")]
    Backend { message: String },

    #[error("unsupported IR type in codegen: {ty:?}")]
    UnsupportedType { ty: IrType },
}

/// Backend-agnostic code generation contract.
pub trait CodegenBackend {
    /// Compiles one IR module into target binary/object bytes.
    fn compile(&self, ir: &IrModule) -> Result<Vec<u8>, CodegenError>;
}

/// Compiles IR with Cranelift and writes object file to dist path.
pub fn compile_to_object(ir: &IrModule, dist_dir: &Path) -> Result<PathBuf, CodegenError> {
    let object = cranelift::CraneliftBackend::new().compile(ir)?;
    linker::write_object_file(&ir.name, &object, dist_dir)
}

/// Compiles IR and links executable binary in dist path.
pub fn compile_to_executable(ir: &IrModule, dist_dir: &Path) -> Result<PathBuf, CodegenError> {
    let object_path = compile_to_object(ir, dist_dir)?;
    let output = dist_dir.join(&ir.name);
    linker::link_executable(&object_path, &output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::ir::IrModule;

    use super::{compile_to_executable, compile_to_object};

    #[test]
    fn compiles_ir_to_object_in_dist() {
        let unique = format!(
            "arwa-codegen-object-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let dist = std::env::temp_dir().join(unique);

        let ir = IrModule {
            name: "App".to_string(),
            ..IrModule::default()
        };

        let output = compile_to_object(&ir, &dist).expect("object should compile");
        assert!(output.exists());

        fs::remove_file(&output).expect("cleanup file");
        fs::remove_dir(&dist).expect("cleanup dir");
    }

    #[test]
    fn compiles_ir_to_executable_in_dist() {
        let unique = format!(
            "arwa-codegen-exe-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let dist = std::env::temp_dir().join(unique);

        let ir = IrModule {
            name: "App".to_string(),
            ..IrModule::default()
        };

        let output = compile_to_executable(&ir, &dist).expect("executable should compile");
        assert!(output.exists());

        let object = dist.join("App.o");
        if object.exists() {
            fs::remove_file(&object).expect("cleanup object file");
        }
        fs::remove_file(&output).expect("cleanup executable file");
        fs::remove_dir(&dist).expect("cleanup dir");
    }
}
