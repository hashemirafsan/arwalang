# Phase 5 - Annotation Processor

## Objective

Validate all built-in annotations for name, target, argument shape, and route-parameter consistency before DI/module/route/lifecycle phases consume annotation metadata.

## Delivered Scope

### Registry and Targets

- Implemented `AnnotationRegistry` with built-in metadata in `src/annotations/mod.rs`.
- Added explicit target model via `AnnotationTarget`:
  - class
  - method
  - param
  - generic type declaration (invalid for framework decorators)

### Processor Core

- Implemented `AnnotationProcessor` and `process_source_file(ast)`.
- Added class-name preloading for validating lifecycle annotation references.
- Implemented generic validation (`validate_annotation`) for:
  - known annotation name
  - valid target

### Specific Annotation Rules

- `#[injectable]`:
  - validates optional named `scope`
  - validates values: `singleton`, `request`, `transient`
- `#[controller(path)]` and HTTP method annotations:
  - require first arg string path
- Parameter annotations:
  - `#[param]`, `#[query]`, `#[header]` require string arg
  - `#[body]` takes no args
  - duplicate `#[body]` in same method params reports `DuplicateBody`
- Lifecycle annotations:
  - `#[use_guards(...)]`, `#[use_interceptors(...)]`, `#[use_pipes(...)]`
  - positional identifier args only
  - referenced classes must exist

### Route Parameter Binding

- Implemented route reconstruction from controller prefix + method path.
- Implemented route param extraction from `:param` segments.
- Validations:
  - `#[param("x")]` must exist in route path
  - each route `:x` must have corresponding `#[param("x")]`

### Error Model

Implemented `AnnotationError` variants:

- `UnknownAnnotation`
- `MissingArgument`
- `InvalidArgument`
- `InvalidTarget`
- `UnboundRouteParam`
- `DuplicateBody`

All include source spans.

## Tests

Added unit tests for:

- known annotation happy path
- unknown annotation
- missing required argument
- invalid injectable scope
- duplicate `#[body]`
- route parameter binding valid/invalid cases
- lifecycle class-reference validation

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

Result: all checks passing.

## Next Phase

Implement Phase 6 (DI Graph): provider collection, dependency edges, alias bindings, and DI001-DI006 validations.
