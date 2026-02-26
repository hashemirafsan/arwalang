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
  - class declaration lowering into nominal IR struct definitions
  - class method lowering into IR functions
  - basic statement lowering (`let`, `return`, expression call)
  - fallback `Return(None)` insertion for non-returning blocks
- Connected static-table lowering into IR generation:
  - DI registry from phase-6 graph (`token -> factory function`)
  - route table from phase-8 registry (`method, path, handler`)
  - lifecycle pipeline registry from phase-9 output (`guards/pipes/interceptors`)
- Added initial IR tests:
  - generates functions from class methods
  - lowers return literal instruction
  - lowers parameter types
  - generates static DI registry
  - generates static route table
- Added codegen backend contract in `src/codegen/mod.rs`:
  - `CodegenBackend` trait
  - `CodegenError`
- Implemented Cranelift object backend in `src/codegen/cranelift.rs`:
  - host ISA initialization via Cranelift native builder
  - IR function signature mapping and function declaration/definition
  - instruction lowering for `store` and `return` subset
  - object emission (`Vec<u8>`) for compiled module
  - backend tests for empty module and simple function object generation
- Added runtime crate scaffold in `runtime/`:
  - static route/DI/pipeline table reader structures
  - minimal request dispatch facade
  - runtime unit tests for route resolution, DI lookup, and dispatch behavior
  - runtime staticlib export for executable entry (`arwa_runtime_start`)
  - linked payload readers for compiler-emitted JSON table blobs
  - runtime-owned bootstrap model that builds dispatchable state from linked payloads
  - minimal HTTP server helpers (TCP listener + HTTP/1.1 request parsing/response formatting)
  - bounded serve-loop execution path used by generated binaries
- Added linker utilities in `src/codegen/linker.rs`:
  - write object output into dist paths
  - build runtime static library via `cargo build --manifest-path runtime/Cargo.toml --release`
  - invoke system linker (`cc`) for object + runtime staticlib -> executable step
- Extended Cranelift metadata emission in `src/codegen/cranelift.rs`:
  - exports table count symbols
  - exports JSON payload symbols + payload-length symbols for route/DI/lifecycle tables
- Added codegen orchestration helpers in `src/codegen/mod.rs`:
  - `compile_to_object` (IR -> Cranelift object -> `dist/<name>.o`)
  - `compile_to_executable` (object -> linked `dist/<name>`)
  - unit test that verifies object artifact output path creation
- Added generated-binary HTTP integration coverage in `src/cli/run.rs`:
  - builds executable, starts env-gated runtime server, issues request, validates HTTP 200 response

## Current Gaps

- No full control-flow lowering yet (branching/basic-block graph still minimal).
- Runtime startup now supports unbounded serving by default and bounded loops via `ARWA_RUNTIME_MAX_REQUESTS`; production-grade lifecycle management is still pending.
- `call` instruction lowering is currently placeholder-only.
- Cross-platform linker behavior is not finalized.

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Next Step

Build end-to-end generated-binary HTTP response tests and finalize runtime serving lifecycle defaults.
