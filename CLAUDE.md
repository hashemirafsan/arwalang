# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ArwaLang is a compiled, strictly-typed, framework-oriented backend language inspired by NestJS. The compiler is written in Rust and produces native binaries with zero runtime reflection. File extension: `.rw` | CLI binary: `arwa`

**Core guarantee:** If `arwa build` succeeds, the server boots correctly.

## Build & Run Commands

Since the project structure doesn't exist yet, these commands will become available once implementation begins:

```bash
# Build the compiler
cargo build --release

# Run compiler tests
cargo test

# Run with clippy (required in CI)
cargo clippy --deny warnings

# Format code
cargo fmt

# Run the ArwaLang CLI (after building)
./target/release/arwa new <project-name>
./target/release/arwa build
./target/release/arwa check    # Validation only, no codegen
./target/release/arwa run       # Build + execute
./target/release/arwa add <feature>
./target/release/arwa fmt
```

## Architecture Overview

The compiler is structured as a multi-phase pipeline that progressively validates and transforms ArwaLang source code into native executables:

### Compilation Pipeline (14 Phases)

1. **Lexer** (`src/lexer/`) - Tokenizes `.rw` files with position tracking
2. **Parser** (`src/parser/`) - Builds AST via recursive descent
3. **Name Resolution** (`src/resolver/`) - Resolves all type and symbol references
4. **Type Checker** (`src/typechecker/`) - Enforces strict typing, validates DTOs and return types
5. **Annotation Processor** (`src/annotations/`) - Validates decorators (`#[injectable]`, `#[controller]`, etc.)
6. **DI Graph** (`src/di/`) - Validates dependency injection (detects cycles, scope violations, missing providers)
7. **Module Graph** (`src/modules/`) - Validates module imports/exports (detects circular imports, private access)
8. **Route Table** (`src/routes/`) - Builds static route registry from controllers
9. **Lifecycle Pipeline** (`src/lifecycle/`) - Assembles middleware → guards → pipes → handlers → interceptors
10. **IR** (`src/ir/`) - Intermediate representation
11. **Codegen** (`src/codegen/`) - Cranelift-based native code generation
12. **Error Reporting** (`src/errors/`) - Structured diagnostics with error codes and spans
13. **CLI** (`src/cli/`) - Command-line interface
14. **Scaffolding** (`templates/`) - Project templates bundled in binary

### Key Architectural Principles

**Fail-Fast Validation:** All semantic errors must be caught before codegen. The compiler collects errors across all phases rather than stopping at the first error.

**Static Everything:** DI registry, route table, and lifecycle pipelines are all statically constructed at compile time and embedded as read-only data in the output binary.

**Scope Compatibility Rules:**
- `singleton` can inject: `singleton`
- `request` can inject: `singleton`, `request`
- `transient` can inject: `singleton`, `request`, `transient`

**Pipeline Execution Order (fixed):**
```
Middleware → Guard(s) → Pipe(s) → Handler → Interceptor(s) → Exception Filter(s)
```

**Annotation Inheritance:**
- Class-level decorators apply to all methods
- Method-level decorators append to class-level
- Order: class-level first, then method-level

## Critical Implementation Details

### Error Handling Strategy
- Use `thiserror` for all error types
- Never `unwrap()` in compiler code - always propagate with `?`
- Compiler must not panic on malformed input
- Every error must include: error code (e.g., `DI001`), source span (file, line, col), and helpful message
- Support both human and JSON output formats (`--format json`)

### Error Code Reference
- `DI001-006`: Dependency injection errors
- `MOD001-004`: Module graph errors
- `ROUTE001-003`: Route validation errors
- `TYPE001-003`: Type checking errors
- `ANN001-004`: Annotation errors
- `LC001-004`: Lifecycle errors
- `NAME001-002`: Name resolution errors

### Parser Implementation Notes
- Annotations (`#[...]`) appear immediately before the item they decorate
- Multiple annotations on one item are allowed
- `#[` is lexed as two tokens: `Hash` then `LBracket`
- `private` on constructor params creates private fields (record in AST, do not desugar)
- Generic types use `<>`: `List<T>`, `Map<K,V>`, `Result<T,E>`
- Implement error recovery - collect all errors before halting

### Type System Rules
- All public methods must have explicit return types
- Controller handlers must return `Result<T, HttpError>` where `T` is serializable
- DTO fields must have explicit type annotations
- Serializable types (v1): primitives, structs/classes with typed fields, `List<T>`, `Map<String, T>`

### Standard Library Interfaces
The compiler must recognize these built-in interfaces (defined in `stdlib/http.rw`):
```
interface Guard {
  fn canActivate(ctx: RequestContext): Result<Bool, HttpError>
}

interface Interceptor {
  fn intercept(ctx: RequestContext, next: () -> Response): Result<Response, HttpError>
}

interface Pipe {
  fn transform(value: Any, metadata: PipeMetadata): Result<Any, HttpError>
}
```

## Testing Strategy

### Unit Tests
Each compiler phase requires unit tests covering:
- Happy path
- All documented error cases
- Edge cases (empty modules, single-class apps, etc.)

### Integration Tests (`tests/e2e/`)
Required test programs with expected outcomes:
1. `minimal_app.rw` - single module, single controller, no DI → compile success
2. `di_basic.rw` - service injected into controller → compile success
3. `di_scope_violation.rw` - request-scoped into singleton → must fail with `DI004`
4. `circular_di.rw` - circular dependency → must fail with `DI002`
5. `duplicate_route.rw` - same path twice → must fail with `ROUTE001`
6. `missing_provider.rw` - unsatisfied dependency → must fail with `DI001`
7. `full_app.rw` - multi-module with guards/interceptors → compile success

Test files use annotations: `// @expect: compile_error DI004` or `// @expect: compile_success`

## Dependencies
- `clap` - CLI argument parsing
- `serde` + `serde_json` - JSON handling (blueprints, error output)
- `thiserror` - Error types
- `cranelift-*` - Code generation backend (v1)
- LLVM backend is optional feature flag (out of scope for v1)

## v1 Scope Boundaries

**In scope:** Constructor-based DI, all HTTP method decorators, route/query/body/header bindings, JSON serialization, singleton/request/transient scopes, basic middleware, bundled templates, Cranelift codegen.

**Out of scope:** Async/await (all synchronous), WebSockets, microservices, OpenAPI generation, advanced generics, network-based templates, LSP, `arwa test` command, field decorators.

## Code Standards
- All public functions must have doc comments
- Use `Arc<T>` for shared ownership in graph structures
- Run `cargo clippy --deny warnings` before commits
- Format with `cargo fmt`
- Propagate errors, never panic on user input
