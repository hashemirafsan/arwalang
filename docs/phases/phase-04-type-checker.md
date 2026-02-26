# Phase 4 - Type Checker

## Objective

Implement strict type validation for ArwaLang methods and controller handlers, including return contracts, DTO field checks, expression inference, and serializability validation.

## Delivered Scope

### Type System (`src/typechecker/mod.rs`)

- Added dedicated semantic `Type` enum distinct from parser `TypeExpr`:
  - primitives (`Int`, `Float`, `Bool`, `String`)
  - named types
  - container/generic forms (`List`, `Map`, `Result`, `Option`)
  - function type and unknown/any/null support
- Added type conversion from `TypeExpr` to semantic `Type`.
- Added compatibility checking for nested generic structures.

### Type Checker Core

- Added `TypeChecker` with declaration preload and class-level checks.
- Implemented `check_source_file(ast)` with aggregated errors.
- Implemented method validation:
  - return type presence validation
  - statement/block traversal
  - return expression compatibility checks
  - `let` type compatibility checks

### Type Inference

- Implemented expression inference for:
  - literals and identifiers
  - unary/binary operators
  - field access (`obj.field`)
  - method calls (`obj.method(...)`) with argument compatibility checks

### Controller Rules

- Enforced controller handler return contract:
  - must return `Result<T, HttpError>`
- Validated serializability of `T`.
- Added DTO usage scan from:
  - `#[body]` params
  - controller return payload types

### DTO and Serializability Validation

- Implemented serializable type checks for:
  - primitives
  - `List<T>`
  - `Map<String, T>`
  - named class/struct fields recursively
- Added `UntypedDtoField` detection for DTO fields missing type information.

### Error Model

- Added `TypeError` variants:
  - `MissingReturnType`
  - `UntypedDtoField`
  - `NonSerializableReturn`
  - `TypeMismatch`
  - `IncompatibleTypes`

## Tests

Added type-checker tests for:

- expression inference
- missing return type detection
- controller result signature enforcement
- non-serializable return detection
- DTO typed-field enforcement for `#[body]`
- return type mismatch detection
- generic compatibility mismatch handling

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

Result: all checks passing.

## Notes

- Current AST always carries a return type node; missing return type is represented as an empty named type in synthetic tests.
- Statement span tracking inside type-checker currently uses placeholder spans for statement-level mismatch reports and will be improved when statement spans are added to AST.

## Next Phase

Implement Phase 5 (Annotation Processor): known annotation registry, target validation, argument checks, and route-parameter binding validation.
