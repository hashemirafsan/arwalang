# Phase 10 - IR & Codegen (In Progress)

## Objective

Define compiler IR and start lowering validated AST into IR, then wire a Cranelift-first backend entrypoint.

## Delivered So Far

- Added foundational IR model in `src/ir/mod.rs`:
  - `IrType`, `IrValue`, `IrInstruction`, `IrBlock`, `IrFunction`, `IrModule`
  - static table entry structs for DI, routes, and lifecycle pipelines
  - `IrError` and `IrGenerator`
- Implemented initial lowering pass:
  - module name extraction from AST
  - class method lowering into IR functions
  - basic statement lowering (`let`, `return`, expression call)
  - fallback `Return(None)` insertion for non-returning blocks
- Added initial IR tests:
  - generates functions from class methods
  - lowers return literal instruction
  - lowers parameter types
- Added codegen backend contract in `src/codegen/mod.rs`:
  - `CodegenBackend` trait
  - `CraneliftBackend` skeleton
  - `CodegenError`
  - test that confirms backend is wired and explicitly not implemented yet

## Current Gaps

- No full control-flow lowering yet (branching/basic-block graph still minimal).
- Static DI/route/pipeline table population in IR is not connected yet.
- Cranelift translation/object emission/runtime linking remain pending.

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Next Step

Complete static table lowering into IR and start concrete Cranelift module/object generation.
