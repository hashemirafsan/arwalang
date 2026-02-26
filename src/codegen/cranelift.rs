use std::collections::HashMap;

use cranelift_codegen::ir::{types, InstBuilder};
use cranelift_codegen::settings;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{default_libcall_names, DataDescription, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use serde::Serialize;

use crate::codegen::{CodegenBackend, CodegenError};
use crate::ir::{IrFunction, IrInstruction, IrModule, IrType, IrValue};

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
    fn compile(&self, ir: &IrModule) -> Result<Vec<u8>, CodegenError> {
        let flag_builder = settings::builder();
        let flags = settings::Flags::new(flag_builder);

        let isa_builder = cranelift_native::builder().map_err(|msg| CodegenError::Backend {
            message: format!("native isa setup failed: {msg}"),
        })?;
        let isa = isa_builder
            .finish(flags)
            .map_err(|msg| CodegenError::Backend {
                message: format!("isa finalize failed: {msg}"),
            })?;

        let object_builder = ObjectBuilder::new(isa, ir.name.clone(), default_libcall_names())
            .map_err(|msg| CodegenError::Backend {
                message: format!("object builder failed: {msg}"),
            })?;
        let mut module = ObjectModule::new(object_builder);

        for function in &ir.functions {
            compile_function(&mut module, function)?;
        }
        emit_runtime_table_metadata(&mut module, ir)?;

        let object = module.finish();
        object.emit().map_err(|err| CodegenError::Backend {
            message: format!("object emission failed: {err}"),
        })
    }
}

fn emit_runtime_table_metadata(
    module: &mut ObjectModule,
    ir: &IrModule,
) -> Result<(), CodegenError> {
    define_u64_data_symbol(module, "__arwa_route_count", ir.route_table.len() as u64)?;
    define_u64_data_symbol(module, "__arwa_di_count", ir.di_registry.len() as u64)?;
    define_u64_data_symbol(module, "__arwa_pipeline_count", ir.pipelines.len() as u64)?;

    define_json_table_symbols(module, "__arwa_routes_json", &ir.route_table)?;
    define_json_table_symbols(module, "__arwa_di_json", &ir.di_registry)?;
    define_json_table_symbols(module, "__arwa_pipelines_json", &ir.pipelines)?;

    Ok(())
}

fn define_json_table_symbols<T: Serialize>(
    module: &mut ObjectModule,
    data_symbol: &str,
    payload: &T,
) -> Result<(), CodegenError> {
    let json = serde_json::to_vec(payload).map_err(|err| CodegenError::Backend {
        message: format!("serialize linked table '{data_symbol}' failed: {err}"),
    })?;
    let len = json.len() as u64;

    define_blob_data_symbol(module, data_symbol, json)?;
    define_u64_data_symbol(module, &format!("{data_symbol}_len"), len)?;
    Ok(())
}

fn define_u64_data_symbol(
    module: &mut ObjectModule,
    name: &str,
    value: u64,
) -> Result<(), CodegenError> {
    let data_id = module
        .declare_data(name, Linkage::Export, true, false)
        .map_err(|err| CodegenError::Backend {
            message: format!("declare data '{name}' failed: {err}"),
        })?;

    let mut data = DataDescription::new();
    data.define(value.to_le_bytes().to_vec().into_boxed_slice());

    module
        .define_data(data_id, &data)
        .map_err(|err| CodegenError::Backend {
            message: format!("define data '{name}' failed: {err}"),
        })?;

    Ok(())
}

fn define_blob_data_symbol(
    module: &mut ObjectModule,
    name: &str,
    payload: Vec<u8>,
) -> Result<(), CodegenError> {
    let data_id = module
        .declare_data(name, Linkage::Export, false, false)
        .map_err(|err| CodegenError::Backend {
            message: format!("declare data '{name}' failed: {err}"),
        })?;

    let mut data = DataDescription::new();
    data.define(payload.into_boxed_slice());

    module
        .define_data(data_id, &data)
        .map_err(|err| CodegenError::Backend {
            message: format!("define data '{name}' failed: {err}"),
        })?;

    Ok(())
}

fn compile_function(module: &mut ObjectModule, ir_fn: &IrFunction) -> Result<(), CodegenError> {
    let sig = build_signature(module, ir_fn)?;
    let fn_id = module
        .declare_function(&ir_fn.name, Linkage::Export, &sig)
        .map_err(|err| CodegenError::Backend {
            message: format!("declare function '{}' failed: {err}", ir_fn.name),
        })?;

    let mut ctx = module.make_context();
    ctx.func.signature = sig;

    let mut fn_builder_ctx = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fn_builder_ctx);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let mut locals = HashMap::new();
        for (idx, (param_name, _)) in ir_fn.params.iter().enumerate() {
            let value = builder.block_params(entry)[idx];
            locals.insert(param_name.clone(), value);
        }

        let mut returned = false;
        for block in &ir_fn.blocks {
            for ins in &block.instructions {
                if lower_instruction(&mut builder, ins, &mut locals)? {
                    returned = true;
                    break;
                }
            }
            if returned {
                break;
            }
        }

        if !returned {
            emit_default_return(&mut builder, &ir_fn.return_type)?;
        }

        builder.finalize();
    }

    module
        .define_function(fn_id, &mut ctx)
        .map_err(|err| CodegenError::Backend {
            message: format!("define function '{}' failed: {err}", ir_fn.name),
        })?;
    module.clear_context(&mut ctx);
    Ok(())
}

fn build_signature(
    module: &ObjectModule,
    ir_fn: &IrFunction,
) -> Result<cranelift_codegen::ir::Signature, CodegenError> {
    let mut sig = module.make_signature();
    for (_, ty) in &ir_fn.params {
        sig.params
            .push(cranelift_codegen::ir::AbiParam::new(map_type(ty)?));
    }
    if !matches!(ir_fn.return_type, IrType::Void) {
        sig.returns
            .push(cranelift_codegen::ir::AbiParam::new(map_type(
                &ir_fn.return_type,
            )?));
    }
    Ok(sig)
}

fn lower_instruction(
    builder: &mut FunctionBuilder,
    ins: &IrInstruction,
    locals: &mut HashMap<String, cranelift_codegen::ir::Value>,
) -> Result<bool, CodegenError> {
    match ins {
        IrInstruction::Store { dst, value } => {
            let value = lower_value(builder, value, locals)?;
            locals.insert(dst.clone(), value);
            Ok(false)
        }
        IrInstruction::Call { .. } => Ok(false),
        IrInstruction::Return(value) => {
            if let Some(v) = value {
                let value = lower_value(builder, v, locals)?;
                builder.ins().return_(&[value]);
            } else {
                builder.ins().return_(&[]);
            }
            Ok(true)
        }
        IrInstruction::Nop => Ok(false),
    }
}

fn emit_default_return(
    builder: &mut FunctionBuilder,
    return_type: &IrType,
) -> Result<(), CodegenError> {
    match return_type {
        IrType::Void => {
            builder.ins().return_(&[]);
        }
        IrType::Int
        | IrType::Bool
        | IrType::String
        | IrType::Any
        | IrType::Named(_)
        | IrType::List(_)
        | IrType::Map(_, _)
        | IrType::Result { .. }
        | IrType::Option(_) => {
            let value = builder.ins().iconst(types::I64, 0);
            builder.ins().return_(&[value]);
        }
        IrType::Float => {
            let value = builder.ins().f64const(0.0);
            builder.ins().return_(&[value]);
        }
    }
    Ok(())
}

fn lower_value(
    builder: &mut FunctionBuilder,
    value: &IrValue,
    locals: &HashMap<String, cranelift_codegen::ir::Value>,
) -> Result<cranelift_codegen::ir::Value, CodegenError> {
    match value {
        IrValue::Int(v) => Ok(builder.ins().iconst(types::I64, *v)),
        IrValue::Float(v) => Ok(builder.ins().f64const(*v)),
        IrValue::Bool(v) => Ok(builder.ins().iconst(types::I64, i64::from(*v))),
        IrValue::Null => Ok(builder.ins().iconst(types::I64, 0)),
        IrValue::Local(name) | IrValue::Temporary(name) | IrValue::FunctionRef(name) => locals
            .get(name)
            .copied()
            .ok_or_else(|| CodegenError::Backend {
                message: format!("unknown local value '{name}'"),
            }),
        IrValue::String(_) => Ok(builder.ins().iconst(types::I64, 0)),
    }
}

fn map_type(ty: &IrType) -> Result<cranelift_codegen::ir::Type, CodegenError> {
    match ty {
        IrType::Void => Err(CodegenError::UnsupportedType { ty: IrType::Void }),
        IrType::Int => Ok(types::I64),
        IrType::Float => Ok(types::F64),
        IrType::Bool => Ok(types::I64),
        IrType::String
        | IrType::Any
        | IrType::Named(_)
        | IrType::List(_)
        | IrType::Map(_, _)
        | IrType::Result { .. }
        | IrType::Option(_) => Ok(types::I64),
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::CodegenBackend;
    use crate::ir::{IrBlock, IrFunction, IrInstruction, IrModule, IrType, IrValue};

    use super::CraneliftBackend;

    #[test]
    fn compiles_empty_module_to_object_bytes() {
        let ir = IrModule {
            name: "App".to_string(),
            ..IrModule::default()
        };

        let bytes = CraneliftBackend::new()
            .compile(&ir)
            .expect("empty object should compile");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn compiles_simple_function_to_object_bytes() {
        let ir = IrModule {
            name: "App".to_string(),
            functions: vec![IrFunction {
                name: "Math.one".to_string(),
                params: vec![],
                return_type: IrType::Int,
                blocks: vec![IrBlock {
                    label: "entry".to_string(),
                    instructions: vec![IrInstruction::Return(Some(IrValue::Int(1)))],
                }],
            }],
            ..IrModule::default()
        };

        let bytes = CraneliftBackend::new()
            .compile(&ir)
            .expect("simple object should compile");
        assert!(!bytes.is_empty());
    }
}
