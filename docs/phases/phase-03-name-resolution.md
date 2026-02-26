# Phase 3 - Name Resolution

## Objective

Resolve symbols and type names across the parsed AST using scoped symbol tables, and report unresolved references with structured resolver errors.

## Delivered Scope

### Symbol Table (`src/resolver/mod.rs`)

- Added `SymbolTable` with explicit scope stack:
  - file scope
  - class scope
  - method scope
- Added `Symbol` and `SymbolKind` metadata for tracked declarations.
- Added symbol operations:
  - `enter_scope()` / `exit_scope()`
  - `insert(...)` with duplicate detection
  - `lookup(...)` with inner-to-outer resolution

### Resolver Core

- Added `Resolver` with collection-first resolution workflow:
  - collect top-level symbols/types/modules
  - resolve each top-level item
- Implemented `resolve_source_file(ast)` returning `Result<(), Vec<ResolveError>>`.

### Resolution Coverage

- Resolved module references:
  - imports
  - providers
  - controllers
  - exports
- Resolved class/interface references:
  - `implements` clauses
  - constructor parameter types
  - field types
  - method parameter and return types
- Resolved type expressions:
  - `Named`
  - `Generic`
  - `Result`
  - `Option`
- Resolved annotation identifier references inside annotation arguments.

### Error Model

- Added `ResolveError` with source spans:
  - `UndefinedType { name, span }`
  - `UndefinedSymbol { name, span }`
  - `DuplicateSymbol { name, span }`
- Resolver accumulates all discovered errors before returning.

## Tests

Added resolver tests for:

- scope hierarchy and symbol lookup behavior
- constructor and method type resolution
- generic type resolution
- interface implementation resolution
- undefined type error
- undefined symbol error in module references
- unknown identifier in annotation args
- duplicate symbol error

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

Result: all checks passing.

## Notes

- Built-in core type names (`Int`, `String`, `List`, `Map`, etc.) are treated as predefined.
- Resolver is currently validation-oriented and does not yet rewrite AST nodes to resolved IDs.

## Next Phase

Implement Phase 4 (Type Checker): strict type validation, controller return constraints, DTO typing rules, and serializability checks.
