# Phase 13 - Scaffolding & Blueprint (In Progress)

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

## Current Gaps

- Full starter content depth (API/full app examples) is still scaffold-level.
- Template bundling into binary (`include_str!/include_bytes!`) is not implemented yet.
- `arwa add` currently overlays template files without merge/conflict policy.

## Validation Performed

```bash
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```

## Next Step

Implement template bundling and extraction, then add end-to-end starter creation coverage for minimal/api/full templates.
