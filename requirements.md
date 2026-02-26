# ArwaLang — Compiler Requirements

## Overview

ArwaLang is a compiled, strictly-typed, framework-oriented backend language.  
File extension: `.rw` | CLI binary: `arwa` | Compiler implementation language: **Rust**

The language takes heavy inspiration from NestJS but enforces all framework contracts at **compile time**, producing a native binary with no runtime reflection.

**Core guarantee:** If `arwa build` succeeds, the server boots correctly.

## Execution Authority

- `requirements.md` is the canonical source of truth ("brain") for architecture, behavior, and acceptance criteria.
- `tasks.md` is the execution and tracking layer ("project manager") and must follow this document.
- If a conflict appears, implementation must follow `requirements.md` and then update `tasks.md` to reflect the resolved direction.

## Toolchain Policy

- Rust toolchain target: stable latest.

---

## Repository Structure

```
arwalang/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── requirements.md
├── src/
│   ├── main.rs                  # CLI entry point
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── new.rs               # arwa new <name>
│   │   ├── add.rs               # arwa add <feature>
│   │   ├── build.rs             # arwa build
│   │   ├── run.rs               # arwa run
│   │   ├── check.rs             # arwa check (no codegen)
│   │   └── fmt.rs               # arwa fmt
│   ├── lexer/
│   │   ├── mod.rs
│   │   ├── token.rs             # Token enum
│   │   └── lexer.rs             # Lexer impl
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── ast.rs               # AST node definitions
│   │   └── parser.rs            # Recursive descent parser
│   ├── resolver/
│   │   └── mod.rs               # Name resolution pass
│   ├── typechecker/
│   │   └── mod.rs               # Type inference + strict checks
│   ├── annotations/
│   │   └── mod.rs               # Annotation/decorator processor
│   ├── di/
│   │   └── graph.rs             # DI graph builder + validator
│   ├── modules/
│   │   └── graph.rs             # Module graph builder + validator
│   ├── routes/
│   │   └── registry.rs          # Route table builder + validator
│   ├── lifecycle/
│   │   └── pipeline.rs          # Middleware/Guard/Pipe/Interceptor/Filter chain builder
│   ├── ir/
│   │   └── mod.rs               # Intermediate representation
│   ├── codegen/
│   │   └── mod.rs               # Native code generation (Cranelift or LLVM)
│   └── errors/
│       └── mod.rs               # Structured compiler diagnostics
├── stdlib/                      # ArwaLang standard library (.rw files)
│   ├── http.rw
│   ├── result.rw
│   └── json.rw
├── templates/                   # Scaffolding templates
│   ├── registry.json
│   ├── starters/
│   │   ├── minimal/
│   │   ├── api/
│   │   └── full/
│   └── features/
│       ├── http/
│       ├── di/
│       ├── logger/
│       ├── auth-jwt/
│       └── db-postgres/
└── tests/
    ├── lexer/
    ├── parser/
    ├── typechecker/
    ├── di/
    ├── routes/
    └── e2e/
```

---

## Phase 1 — Lexer

### Task
Implement a hand-written lexer in `src/lexer/lexer.rs` that tokenizes `.rw` source files.

### Token Types (`src/lexer/token.rs`)

```
Keywords:
  module, import, export, provide, control, constructor
  class, interface, struct, enum
  fn, return, let, const, if, else, match, for, while
  true, false, null

Literals:
  IntLiteral(i64)
  FloatLiteral(f64)
  StringLiteral(String)
  BoolLiteral(bool)

Identifiers:
  Ident(String)

Decorator Tokens:
  Hash       (#)
  LBracket   ([)
  RBracket   (])

Delimiters:
  LBrace ({)   RBrace (})
  LParen (()   RParen ())
  LAngle (<)   RAngle (>)
  Comma (,)    Colon (:)   Semicolon (;)
  Dot (.)      Arrow (=>)  FatArrow (->)
  Equals (=)   Slash (/)   At (@)

Operators:
  Plus (+)   Minus (-)   Star (*)   Slash (/)
  BangEq (!=)   EqEq (==)   Lt (<)   Gt (>)   LtEq (<=)   GtEq (>=)
  And (&&)   Or (||)   Bang (!)

Special:
  EOF
  Newline (tracked for error reporting, not significant syntactically)
```

### Lexer Behaviour
- Track `line` and `column` for every token (used in error messages).
- Skip whitespace and comments (`//` single-line, `/* */` block).
- Strings are double-quoted, support `\n`, `\t`, `\\`, `\"` escapes.
- Integer literals: decimal only for v1. Float literals contain a `.`.
- Decorator sequence `#[` is lexed as two tokens: `Hash` then `LBracket`.

### Error Cases
- Unterminated string literal → emit `LexError::UnterminatedString { line, col }`.
- Unknown character → emit `LexError::UnexpectedChar { char, line, col }`.

---

## Phase 2 — Parser & AST

### Task
Implement a recursive-descent parser in `src/parser/parser.rs` producing an AST defined in `src/parser/ast.rs`.

### AST Node Definitions (`src/parser/ast.rs`)

```rust
// Top-level
pub struct SourceFile {
    pub path: PathBuf,
    pub items: Vec<TopLevelItem>,
}

pub enum TopLevelItem {
    Module(ModuleDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Import(ImportDecl),     // file-level imports (stdlib etc.)
}

// Module
pub struct ModuleDecl {
    pub name: String,
    pub imports: Vec<String>,       // import AuthModule
    pub providers: Vec<ProviderBinding>,
    pub controllers: Vec<String>,
    pub exports: Vec<String>,
    pub span: Span,
}

pub enum ProviderBinding {
    Simple(String),                         // provide UserService
    Aliased { token: String, impl_: String }, // provide UserRepo => PostgresUserRepo
}

// Class
pub struct ClassDecl {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub implements: Vec<String>,
    pub constructor: Option<ConstructorDecl>,
    pub methods: Vec<MethodDecl>,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

pub struct ConstructorDecl {
    pub params: Vec<Param>,
    pub span: Span,
}

pub struct MethodDecl {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub body: Block,
    pub span: Span,
}

pub struct FieldDecl {
    pub name: String,
    pub ty: TypeExpr,
    pub optional: bool,
    pub span: Span,
}

pub struct Param {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

// Annotation
pub struct Annotation {
    pub name: String,
    pub args: Vec<AnnotationArg>,
    pub span: Span,
}

pub enum AnnotationArg {
    Positional(Expr),
    Named { key: String, value: Expr },
}

// Types
pub enum TypeExpr {
    Named(String),
    Generic { name: String, params: Vec<TypeExpr> },  // List<T>, Map<K,V>
    Result { ok: Box<TypeExpr>, err: Box<TypeExpr> },  // Result<T, E>
    Option(Box<TypeExpr>),
}

// Expressions (minimal v1)
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    Null,
    Ident(String),
    Call { callee: Box<Expr>, args: Vec<Expr> },
    FieldAccess { obj: Box<Expr>, field: String },
    BinOp { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    UnaryOp { op: UnaryOp, expr: Box<Expr> },
}

// Statements
pub enum Stmt {
    Let { name: String, ty: Option<TypeExpr>, value: Expr },
    Return(Option<Expr>),
    Expr(Expr),
    If { cond: Expr, then: Block, else_: Option<Block> },
}

pub struct Block {
    pub stmts: Vec<Stmt>,
}

pub struct Span {
    pub file: PathBuf,
    pub line_start: u32,
    pub col_start: u32,
    pub line_end: u32,
    pub col_end: u32,
}
```

### Parser Rules
- Annotations appear immediately before the item they decorate.
- Multiple annotations on one item are allowed: `#[get("/")]` then `#[use_pipes(Validate)]` on next line.
- `module` blocks use a brace-delimited body with specific keywords: `import`, `provide`, `control`, `export`.
- `constructor` keyword inside a class body introduces the constructor.
- `private` modifier on constructor params is syntactic sugar for a private field + assignment (record for type checker, do not desugar in parser).
- Generic types use `<>`: `List<UserDto>`, `Map<String, Int>`, `Result<UserDto, HttpError>`.

### Error Recovery
- On parse error, emit a structured `ParseError` with span and continue to collect further errors (do not stop at first error).
- Minimum: collect all top-level item errors before halting.

---

## Phase 3 — Name Resolution

### Task
Implement `src/resolver/mod.rs`. Walk the AST and:

1. Build a symbol table per scope (file → class → method).
2. Resolve all `TypeExpr::Named` references to their declaration.
3. Resolve all constructor parameter types.
4. Resolve method return types.
5. Resolve annotation argument identifiers (e.g., `AuthGuard` in `#[use_guards(AuthGuard)]`).

### Error Cases
- Undefined type reference → `ResolveError::UndefinedType { name, span }`.
- Undefined class/interface referenced in `module` block → `ResolveError::UndefinedSymbol { name, span }`.

---

## Phase 4 — Type Checker

### Task
Implement `src/typechecker/mod.rs`. Enforce strict typing rules.

### Rules

**Return Types**
- All public methods on classes must have an explicit return type annotation.
- Controller handler methods must return `Result<T, HttpError>` where `T` is a serializable type (class, struct, List<T>, or primitive).
- Emit `TypeError::MissingReturnType { method, span }` if absent.

**DTO Fields**
- All fields in a class/struct used as `#[body]` parameter or as a controller return type must have explicit type annotations.
- Emit `TypeError::UntypedDtoField { class, field, span }`.

**Serializable Types (v1)**
- Primitives: `Int`, `Float`, `Bool`, `String`
- Structs and classes with all-typed fields
- `List<T>` where `T` is serializable
- `Map<String, T>` where `T` is serializable
- Non-serializable type used as return: `TypeError::NonSerializableReturn { ty, span }`.

**Type Compatibility**
- Method call argument types must match parameter types.
- `Result<T, E>` and `Option<T>` are generic; type-check inner types.

---

## Phase 5 — Annotation Processor

### Task
Implement `src/annotations/mod.rs`. Process all `#[...]` annotations and attach semantic meaning.

### Known Annotations

| Annotation | Target | Arguments |
|---|---|---|
| `#[injectable]` | class | optional: `scope = "singleton" \| "request" \| "transient"` |
| `#[controller(path)]` | class | route prefix string |
| `#[get(path)]` | method | route path string |
| `#[post(path)]` | method | route path string |
| `#[put(path)]` | method | route path string |
| `#[delete(path)]` | method | route path string |
| `#[patch(path)]` | method | route path string |
| `#[param(name)]` | method param | path param name string |
| `#[query(name)]` | method param | query param name string |
| `#[body]` | method param | none |
| `#[header(name)]` | method param | header name string |
| `#[use_guards(...)]` | class or method | one or more class names |
| `#[use_interceptors(...)]` | class or method | one or more class names |
| `#[use_pipes(...)]` | class or method | one or more class names |

### Validation
- Unknown annotation name → `AnnotationError::UnknownAnnotation { name, span }`.
- `#[controller]` without a string path argument → `AnnotationError::MissingArgument`.
- `#[param("id")]` where `"id"` does not appear in the route string of the enclosing method or class → `AnnotationError::UnboundRouteParam { param, route, span }`.
- `#[body]` appearing more than once in the same method parameter list → `AnnotationError::DuplicateBody { span }`.
- `#[injectable(scope="request")]` default scope is `"singleton"` if argument is absent.

---

## Phase 6 — DI Graph Builder & Validator

### Task
Implement `src/di/graph.rs`. Build a directed graph of all providers and validate it.

### Graph Construction
1. Collect all classes annotated with `#[injectable]`.
2. For each, inspect constructor parameters — each is an edge from consumer → dependency.
3. Collect all `provide X => Y` bindings from module declarations.
4. Resolve interface tokens to their concrete implementations.

### Validation Rules

| Rule | Error |
|---|---|
| Constructor param type has no provider in scope | `DiError::MissingProvider { consumer, dependency, span }` |
| Circular dependency A → B → A | `DiError::CircularDependency { cycle: Vec<String>, span }` |
| Two providers bound to same token in same module | `DiError::DuplicateProvider { token, span }` |
| `request`-scoped provider injected into `singleton` | `DiError::ScopeMismatch { consumer, dependency, consumer_scope, dependency_scope, span }` |
| `transient`-scoped provider injected into `singleton` | Same as above |
| Provider declared in module but class is not `#[injectable]` | `DiError::NotInjectable { class, span }` |
| Exported symbol not provided within same module | `DiError::ExportWithoutProvider { symbol, module, span }` |

### Scope Compatibility Matrix
```
singleton  can inject: singleton
request    can inject: singleton, request
transient  can inject: singleton, request, transient
```

---

## Phase 7 — Module Graph Builder & Validator

### Task
Implement `src/modules/graph.rs`. Build and validate the module dependency graph.

### Validation Rules

| Rule | Error |
|---|---|
| Module imports a module that doesn't exist | `ModuleError::UnknownImport { module, import, span }` |
| Circular module imports A → B → A | `ModuleError::CircularImport { cycle: Vec<String>, span }` |
| Controller references a service not provided (or imported) in the same module | `ModuleError::UnsatisfiedController { controller, dependency, module, span }` |
| Symbol exported but not provided in module | `ModuleError::ExportWithoutProvider { symbol, module, span }` |
| Consuming a provider from another module that is not exported | `ModuleError::PrivateProvider { provider, source_module, consuming_module, span }` |

---

## Phase 8 — Route Table Builder & Validator

### Task
Implement `src/routes/registry.rs`. Build a static route table from all controllers.

### Construction
1. Collect all classes with `#[controller(prefix)]`.
2. For each, collect methods with `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`.
3. Construct full route: `prefix + method_path`.
4. Record: `{ method: HttpMethod, path: String, handler: FullyQualifiedMethodName }`.

### Validation Rules

| Rule | Error |
|---|---|
| Two handlers with identical method + path | `RouteError::DuplicateRoute { method, path, handlers: [String; 2], span }` |
| Path param in route string (`:id`) has no corresponding `#[param("id")]` parameter | `RouteError::UnboundPathParam { param, handler, span }` |
| `#[param("id")]` declared but `:id` not in route string | `RouteError::UnusedParamAnnotation { param, handler, span }` |
| Handler return type not serializable | Delegate to type checker (already caught in Phase 4) |

---

## Phase 9 — Lifecycle Pipeline Builder

### Task
Implement `src/lifecycle/pipeline.rs`. Statically assemble the execution pipeline for each route handler.

### Pipeline Order (fixed)
```
Middleware → Guard(s) → Pipe(s) → Handler → Interceptor(s) → Exception Filter(s)
```

### Construction
- Class-level `#[use_guards(...)]` applies to all handlers in that controller.
- Method-level `#[use_guards(...)]` appends to the class-level list.
- Same for `#[use_interceptors(...)]` and `#[use_pipes(...)]`.
- Order within each stage: class-level first, method-level second.

### Validation Rules

| Rule | Error |
|---|---|
| Guard class referenced in `#[use_guards]` does not implement `Guard` interface | `LifecycleError::InvalidGuard { class, span }` |
| Interceptor class does not implement `Interceptor` interface | `LifecycleError::InvalidInterceptor { class, span }` |
| Pipe class does not implement `Pipe` interface | `LifecycleError::InvalidPipe { class, span }` |
| Guard/Interceptor/Pipe class is not injectable | `LifecycleError::NotInjectable { class, span }` |

### Standard Library Interfaces (defined in `stdlib/http.rw`)
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

---

## Phase 10 — IR & Code Generation

### Task
Lower the validated AST + all static tables (DI registry, route table, pipeline chains) into IR and generate a native executable.

### IR (`src/ir/mod.rs`)
Define a simple IR that represents:
- Function definitions with typed parameters and return types
- Static tables as read-only data structures
- The DI registry as a static map of token → factory function pointer
- The route table as a static array of `(method, path_pattern, handler_fn_ptr)`
- Each handler's lifecycle pipeline as a static array of function pointers

### Code Generation (`src/codegen/mod.rs`)
- Use **Cranelift** as the default and primary backend (pure Rust, no LLVM dependency for v1).
- Phase 10 delivery policy: finish end-to-end Cranelift flow first (IR lowering, object generation, runtime linking, executable output).
- LLVM backend remains optional via feature flag `--features llvm` and is non-blocking for v1 completion.
- Emit a single self-contained native binary.
- The runtime loop (HTTP server) is embedded via a minimal Rust runtime crate linked at compile time (not generated ArwaLang code).

### Runtime Responsibilities (embedded Rust, not generated)
- TCP listener
- HTTP/1.1 request parsing
- Route dispatch (reads the static route table)
- DI container bootstrap (reads the static DI registry)
- Lifecycle pipeline execution
- JSON serialization/deserialization of DTOs

---

## Phase 11 — Error Reporting

### Task
Implement `src/errors/mod.rs`. Produce human-readable, structured diagnostics.

### Format
```
error[DI001]: missing provider
  --> src/user/user.module.rw:12:3
   |
12 |   provide UserController
   |   ^^^^^^^^^^^^^^^^^^^^^^
   = note: UserController requires `UserRepo` but no provider is bound in UserModule
   = help: add `provide UserRepo` or `provide UserRepo => PostgresUserRepo` to UserModule
```

### Requirements
- Every error has a code (e.g., `DI001`, `ROUTE002`, `TYPE003`).
- Every error references the source span (file, line, column).
- Errors are collected across all phases before printing — do not stop at first error per phase.
- Warnings are separate from errors and do not block compilation.
- Output format: human (default), JSON (`--format json` flag for tooling integration).

---

## Phase 12 — CLI

### Task
Implement the `arwa` CLI in `src/cli/`.

### Commands

**`arwa new <name>`**
- Creates a new project directory.
- Fetches template from `templates/starters/api/` (bundled in binary for v1, no network fetch yet).
- Generates `arwa.blueprint.json`.
- Initialises the project structure.

**`arwa build`**
- Runs the full compiler pipeline (phases 1–10).
- Outputs binary to `./dist/<name>`.

**`arwa check`**
- Runs phases 1–9 (validation only, no code generation).
- Faster feedback loop.

**`arwa run`**
- Runs `arwa build` then executes the output binary.

**`arwa add <feature>`**
- Reads `arwa.blueprint.json`.
- Scaffolds feature files from `templates/features/<feature>/`.
- Updates `arwa.blueprint.json`.

**`arwa fmt`**
- Formats `.rw` source files in place.
- v1: enforce 2-space indentation, consistent blank lines, sorted imports.

---

## Phase 13 — Project Scaffolding & Blueprint

### `arwa.blueprint.json` Schema
```json
{
  "name": "myapp",
  "version": "0.1.0",
  "starter": "api",
  "features": ["http", "di", "logger"]
}
```

### Template Directory (bundled in binary)
```
templates/
  registry.json              <- feature metadata
  starters/
    minimal/                 <- bare module + main
    api/                     <- module + controller + service + dto
    full/                    <- api + auth + db + logger
  features/
    http/
    di/
    logger/
    auth-jwt/
    db-postgres/
```

### `registry.json` Schema
```json
{
  "features": [
    {
      "name": "auth-jwt",
      "description": "JWT-based authentication guard and decorator",
      "files": ["src/auth/auth.module.rw", "src/auth/auth.guard.rw"]
    }
  ]
}
```

---

## Phase 14 — Standard Library

### `stdlib/http.rw`
Defines:
- `HttpError` class with `status: Int` and `message: String`
- `RequestContext` struct
- `Guard`, `Interceptor`, `Pipe` interfaces
- `Response` struct

### `stdlib/result.rw`
Defines:
- `Result<T, E>` (built-in, but documented here)
- `Option<T>` (built-in)
- Helper methods: `.unwrap()`, `.unwrap_or(default)`, `.map(fn)`

### `stdlib/json.rw`
Defines:
- `Json.encode<T>(value: T): String`
- `Json.decode<T>(raw: String): Result<T, JsonError>`

---

## Compile-Time Validation Summary

The following table lists every compile-time guarantee ArwaLang makes. All must be checked before code generation begins.

| Check | Phase | Error Code |
|---|---|---|
| Missing DI provider | DI Graph | DI001 |
| Circular DI dependency | DI Graph | DI002 |
| Duplicate DI provider | DI Graph | DI003 |
| Scope mismatch | DI Graph | DI004 |
| Not injectable | DI Graph | DI005 |
| Export without provider | DI Graph | DI006 |
| Unknown module import | Module Graph | MOD001 |
| Circular module import | Module Graph | MOD002 |
| Unsatisfied controller dependency | Module Graph | MOD003 |
| Private provider consumed | Module Graph | MOD004 |
| Duplicate route | Route Table | ROUTE001 |
| Unbound path param | Route Table | ROUTE002 |
| Unused param annotation | Route Table | ROUTE003 |
| Missing return type | Type Checker | TYPE001 |
| Untyped DTO field | Type Checker | TYPE002 |
| Non-serializable return type | Type Checker | TYPE003 |
| Unknown annotation | Annotation Processor | ANN001 |
| Missing annotation argument | Annotation Processor | ANN002 |
| Unbound route param in annotation | Annotation Processor | ANN003 |
| Duplicate @body | Annotation Processor | ANN004 |
| Invalid guard (wrong interface) | Lifecycle | LC001 |
| Invalid interceptor | Lifecycle | LC002 |
| Invalid pipe | Lifecycle | LC003 |
| Lifecycle class not injectable | Lifecycle | LC004 |
| Undefined type | Name Resolution | NAME001 |
| Undefined symbol | Name Resolution | NAME002 |

---

## v1 Scope Boundaries

### In Scope for v1
- All phases described above (1–14)
- `module {}` declarations
- `#[injectable]` with singleton/request/transient scopes
- `#[controller]`, `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`
- Constructor-based DI only
- Route param, query param, body, header bindings
- JSON serialization/deserialization
- `Result<T, HttpError>` return type enforcement
- Basic middleware (class-based, no annotation, registered in module)
- `arwa new`, `arwa build`, `arwa check`, `arwa run`, `arwa add`, `arwa fmt`
- Bundled templates (no network fetch in v1)
- Cranelift codegen backend

### Out of Scope for v1
- Microservices / message brokers
- OpenAPI / Swagger generation
- Advanced generics (generic classes, type constraints)
- Websockets
- LLVM backend (feature-flagged, not required to work)
- Network-based template registry
- Plugin/extension system
- Decorators on fields (only on classes, methods, and params)
- Async/await (all handlers are synchronous in v1)
- `arwa test` command
- IDE language server (LSP)

---

## Testing Requirements

### Unit Tests
Each module must have unit tests covering:
- Happy path
- All documented error cases
- Edge cases (empty module, no providers, single-class app)

### Integration Tests (`tests/e2e/`)
Provide at least the following `.rw` test programs:

1. `minimal_app.rw` — single module, single controller, no DI
2. `di_basic.rw` — one service injected into one controller
3. `di_scope_violation.rw` — request-scoped into singleton → must fail with `DI004`
4. `circular_di.rw` — A depends on B depends on A → must fail with `DI002`
5. `duplicate_route.rw` — two handlers on same path → must fail with `ROUTE001`
6. `missing_provider.rw` — controller needs service not in module → must fail with `DI001`
7. `full_app.rw` — multi-module app with guards, interceptors, DTO validation

### Test Harness
- Each e2e test file annotates its expected outcome:
  ```
  // @expect: compile_error DI004
  ```
  or
  ```
  // @expect: compile_success
  ```
- The test runner compiles each file and asserts the outcome.

---

## Coding Conventions (for Claude Code)

- Use `thiserror` for all error types.
- Use `clap` for CLI argument parsing.
- Use `serde` + `serde_json` for JSON (blueprint files, error JSON output).
- Avoid `unwrap()` in compiler code — propagate errors with `?`.
- All public functions must have doc comments.
- Prefer `Arc<T>` for shared ownership in graph structures.
- The compiler must not panic on malformed input — all user-facing errors must be caught and reported gracefully.
- Run `cargo clippy --deny warnings` in CI.
- Format with `cargo fmt`.

---

## Deliverables Checklist

- [ ] Lexer with full token set and error recovery
- [ ] Parser producing typed AST
- [ ] Name resolver
- [ ] Type checker with all v1 rules
- [ ] Annotation processor
- [ ] DI graph builder and validator (all 6 error types)
- [ ] Module graph builder and validator (all 4 error types)
- [ ] Route table builder and validator (all 3 error types)
- [ ] Lifecycle pipeline builder and validator
- [ ] IR definition
- [ ] Cranelift code generation
- [ ] Structured error reporting with codes and source spans
- [ ] CLI (`new`, `build`, `check`, `run`, `add`, `fmt`)
- [ ] Bundled templates (minimal, api, full starters + 5 features)
- [ ] Standard library (http.rw, result.rw, json.rw)
- [ ] Unit tests for each phase
- [ ] E2E test harness with 7 test programs
