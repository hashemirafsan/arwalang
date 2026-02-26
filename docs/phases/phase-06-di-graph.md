# Phase 6 - DI Graph Builder and Validator

## Objective

Build a module-aware DI provider graph from AST declarations and enforce all v1 DI correctness rules before runtime/codegen phases.

## Delivered Scope

### Graph Model (`src/di/graph.rs`)

- Implemented `Scope` enum with compatibility matrix:
  - singleton -> singleton only
  - request -> singleton, request
  - transient -> singleton, request, transient
- Implemented `Provider` and `Dependency` records.
- Implemented `DiGraph` with:
  - provider storage
  - module+token index
  - module+implementation index
  - dependency edges (`add_provider`, `add_dependency`)

### Builder (`DiGraphBuilder`)

- Implemented `build(ast)` pipeline:
  - collect classes/modules
  - build module provider set from `provide` bindings
  - extract constructor dependencies
  - infer scope from `#[injectable(scope = ...)]`
  - add graph edges for known module dependencies

### Validations

Implemented DI validations and errors:

- DI001 `MissingProvider`
- DI002 `CircularDependency`
- DI003 `DuplicateProvider`
- DI004 `ScopeMismatch`
- DI005 `NotInjectable`
- DI006 `ExportWithoutProvider`

Covered checks include:

- provider token duplicates inside module
- non-injectable or missing implementation classes
- export list references that are not provided
- unresolved constructor dependencies in module scope
- scope compatibility violations
- cycle detection (including long cycles)

## Tests

Added DI unit tests for:

- simple graph build
- missing provider
- short and long circular dependencies
- duplicate provider token
- singleton/request scope mismatch
- request->singleton allowed
- transient->request allowed
- not-injectable provider rejection
- export-without-provider rejection
- aliased binding support

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

Result: all checks passing.

## Notes

- Current DI scope resolution is module-local and token-driven.
- Built-in primitive/core types are ignored for missing-provider checks.

## Next Phase

Implement Phase 7 (Module Graph): import graph construction, visibility checks, circular import detection, and module-level dependency validation.
