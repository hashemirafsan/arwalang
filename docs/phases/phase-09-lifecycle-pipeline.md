# Phase 09 - Lifecycle Pipeline Builder

## Objective

Statically assemble per-route lifecycle pipelines and validate that referenced lifecycle classes are injectable and implement required interfaces.

## Delivered Scope

- Implemented lifecycle structures in `src/lifecycle/pipeline.rs`:
  - `PipelineStage`
  - `LifecycleComponent`
  - `Pipeline`
- Implemented `PipelineBuilder::build` returning `HashMap<Route, Pipeline>`.
- Implemented class-level + method-level annotation merging for:
  - `#[use_guards(...)]`
  - `#[use_pipes(...)]`
  - `#[use_interceptors(...)]`
- Enforced ordering semantics:
  - class-level components first
  - method-level components appended
  - fixed stage order represented in final `Pipeline`
- Added lifecycle validation errors:
  - `LC001` invalid guard
  - `LC002` invalid interceptor
  - `LC003` invalid pipe
  - `LC004` lifecycle class not injectable
- Added unit tests for happy path, merge/order behavior, and all error modes.

## Key Implementation Details

- Route handlers are mapped back to AST methods using the `Class.method` handler format.
- Lifecycle component arguments are extracted from positional identifier args in annotation lists.
- Component validation checks both contracts:
  - interface implementation (`Guard`, `Interceptor`, `Pipe`), and
  - injectable marker (`#[injectable]`).

## Tests

- `builds_simple_pipeline`
- `combines_class_and_method_annotations`
- `maintains_fixed_stage_order`
- `errors_on_invalid_guard_interface`
- `errors_on_invalid_interceptor_interface`
- `errors_on_invalid_pipe_interface`
- `errors_on_non_injectable_component`
- `applies_multiple_guards_in_order`
- `supports_empty_pipeline`

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Known Limitations

- Middleware and exception filters are modeled in `Pipeline` but not yet populated from module-level registrations.

## Next Phase

Phase 10: IR definition and Cranelift-first code generation.
