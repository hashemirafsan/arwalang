# Phase 12 - CLI (In Progress)

## Objective

Implement `arwa` command entrypoints for build/check/run workflows.

## Delivered So Far

- Implemented command parsing in `src/cli/mod.rs` using `clap` subcommands:
  - `build`
  - `check`
  - `run`
  - implemented `new`
  - implemented `add`
  - implemented `fmt`
- Implemented `arwa new` flow in `src/cli/new.rs`:
  - project name validation
  - starter validation (`api`, `minimal`)
  - creates project directory and `src/` layout
  - writes starter `.rw` files
  - generates `arwa.blueprint.json`
- Implemented `arwa build` flow in `src/cli/build.rs`:
  - source loading (explicit input or auto-discovered `src/**/*.rw`)
  - phases 1-9 validation pipeline
  - IR generation
  - codegen/link invocation
  - executable output reporting
- Implemented `arwa check` flow in `src/cli/check.rs`:
  - runs validation-only pipeline (phases 1-9)
  - supports project source auto-discovery
- Implemented `arwa run` flow in `src/cli/run.rs`:
  - runs build
  - executes produced binary
  - forwards args to generated executable
- Implemented `arwa add` flow in `src/cli/add.rs`:
  - reads and updates `arwa.blueprint.json`
  - validates features from registry + built-in v1 set
  - copies template files when available (fallback scaffold when missing)
- Implemented `arwa fmt` flow in `src/cli/fmt.rs`:
  - recursively discovers `.rw` files
  - supports `--check` mode
  - applies baseline formatting (2-space indent, import sorting, trailing cleanup)
- Added CLI tests:
  - command argument parsing tests
  - build integration-like test (minimal app -> executable artifact)
  - check integration-like tests (valid + invalid source)
  - run integration-like test (minimal app -> execute binary)
  - build/check project-discovery tests from `src/` layouts
  - new command project-generation test
  - run forwarded-arg behavior test
  - add command feature-application test
  - fmt command format/check tests
- Added CLI error typing + exit-code mapping:
  - compilation/runtime errors -> exit code `1`
  - usage/unsupported command errors -> exit code `2`

## Current Gaps

- `fmt` currently uses lightweight line-based formatting (not AST-driven).
- `add` template copy path is limited by currently bundled template coverage.

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Next Step

Harden `add`/`fmt` behaviors with richer template bundles and stricter formatting semantics.
