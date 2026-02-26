#![allow(dead_code)]

use thiserror::Error;

use crate::ir::IrModule;

/// Native code generation errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CodegenError {
    #[error("codegen backend not implemented")]
    NotImplemented,
}

/// Backend-agnostic code generation contract.
pub trait CodegenBackend {
    /// Compiles one IR module into target binary/object bytes.
    fn compile(&self, ir: &IrModule) -> Result<Vec<u8>, CodegenError>;
}

/// Cranelift-first backend entrypoint for v1.
#[derive(Debug, Default)]
pub struct CraneliftBackend;

impl CraneliftBackend {
    /// Creates a Cranelift backend handle.
    pub fn new() -> Self {
        Self
    }
}

impl CodegenBackend for CraneliftBackend {
    fn compile(&self, _ir: &IrModule) -> Result<Vec<u8>, CodegenError> {
        Err(CodegenError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use crate::ir::IrModule;

    use super::{CodegenBackend, CodegenError, CraneliftBackend};

    #[test]
    fn cranelift_backend_is_wired_but_not_implemented_yet() {
        let backend = CraneliftBackend::new();
        let ir = IrModule {
            name: "App".to_string(),
            ..IrModule::default()
        };

        let err = backend.compile(&ir).expect_err("must fail for now");
        assert_eq!(err, CodegenError::NotImplemented);
    }
}
