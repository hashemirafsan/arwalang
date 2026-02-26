# Phase 12 - CLI (In Progress)

## Objective

Implement `arwa` command entrypoints for build/check/run workflows.

## Delivered So Far

- Implemented command parsing in `src/cli/mod.rs` using `clap` subcommands:
  - `build`
  - `check`
  - `run`
  - placeholders for `new`, `add`, `fmt`
- Implemented `arwa build` flow in `src/cli/build.rs`:
  - source loading
  - phases 1-9 validation pipeline
  - IR generation
  - codegen/link invocation
  - executable output reporting
- Implemented `arwa check` flow in `src/cli/check.rs`:
  - runs validation-only pipeline (phases 1-9)
- Implemented `arwa run` flow in `src/cli/run.rs`:
  - runs build
  - executes produced binary
- Added CLI tests:
  - command argument parsing tests
  - build integration-like test (minimal app -> executable artifact)
  - check integration-like tests (valid + invalid source)
  - run integration-like test (minimal app -> execute binary)
- Added CLI error typing + exit-code mapping:
  - compilation/runtime errors -> exit code `1`
  - usage/unsupported command errors -> exit code `2`

## Current Gaps

- `new`, `add`, and `fmt` are not implemented yet.
- Build/check currently operate on a single source file, not full project file discovery.

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Next Step

Implement full project discovery for `build`/`check` and complete `new` command scaffolding.
