# Phase 13 - Scaffolding & Blueprint

## Objective

Provide starter/feature template infrastructure and validated project blueprint parsing for scaffold workflows.

## Delivered So Far

- Added template schema module in `src/cli/templates.rs`:
  - `Blueprint` model + validation
  - `TemplateRegistry` model + validation
  - parser helpers for blueprint and registry files
- Added schema-focused unit tests:
  - parse `arwa.blueprint.json`
  - validate blueprint fields
  - parse `templates/registry.json`
  - detect duplicate registry feature names
- Added starter template directories:
  - `templates/starters/minimal/`
  - `templates/starters/api/`
  - `templates/starters/full/`
- Added feature template directories:
  - `templates/features/http/`
  - `templates/features/di/`
  - `templates/features/logger/`
  - `templates/features/auth-jwt/`
  - `templates/features/db-postgres/`
- Expanded `templates/registry.json` with feature metadata fields:
  - `name`, `description`, `files`, `dependencies`, `usage`
- Integrated template schema into CLI flows:
  - `arwa new` now copies from `templates/starters/<starter>/`
  - `arwa add` now validates features from registry and updates blueprint through schema helpers
- Added template bundling/extraction in `src/cli/templates.rs`:
  - compile-time template embedding via `include_str!`
  - runtime extraction helper when `templates/` is missing in current project
  - `arwa new` and `arwa add` now ensure embedded templates are available before operation
- Added integration coverage for scaffold workflows:
  - create project from minimal starter
  - create project from api starter
  - create project from full starter
  - add all registry features into a project blueprint

## Current Gaps

- Full starter content depth (API/full app examples) is still scaffold-level.
- `arwa add` currently overlays template files without merge/conflict policy.

## Validation Performed

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```

## Next Step

Move to Phase 14 and implement standard library files (`stdlib/http.rw`, `stdlib/result.rw`, `stdlib/json.rw`).
