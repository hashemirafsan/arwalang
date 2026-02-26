# Phase 07 - Module Graph Builder & Validator

## Objective

Build a module dependency graph and enforce module import/export visibility rules at compile time.

## Delivered Scope

- Implemented `ModuleGraph` and `Module` models in `src/modules/graph.rs`.
- Implemented `ModuleGraphBuilder::build` to construct graph nodes/edges from AST modules.
- Added validation for unknown imports (`MOD001`) and circular imports (`MOD002`).
- Added validation for unsatisfied controller dependencies (`MOD003`).
- Added validation for private provider consumption (`MOD004`) and export-without-provider.
- Added full unit-test coverage for success and error paths.

## Key Implementation Details

- Controller dependencies are resolved from constructor parameter types and validated against:
  - providers in the same module, or
  - exported providers from imported modules.
- Private-provider detection distinguishes between:
  - dependency present in imported module but not exported (`MOD004`), and
  - dependency missing entirely (`MOD003`).
- Circular import detection uses DFS with visiting/visited tracking and emits cycle paths.

## Tests

- `builds_simple_module_graph`
- `detects_unknown_import`
- `detects_circular_import`
- `validates_controller_dependencies_satisfied`
- `detects_private_provider_access`
- `validates_exports_match_providers`
- `multi_module_with_proper_imports_passes`

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Known Limitations

- Module dependency checks only inspect constructor injection dependencies, matching current DI model.
- Error variant naming for export-without-provider is currently local to module graph implementation.

## Next Phase

Phase 08: route table construction and static route validation.
