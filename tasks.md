# ArwaLang Compiler - Development Tasks

This document breaks down the ArwaLang compiler implementation into granular tasks organized by phase.

## Status Legend
- `[ ]` Not started
- `[~]` In progress
- `[x]` Completed
- `[!]` Blocked

## Project Management Directives

- `requirements.md` is the canonical source of truth (brain).
- `tasks.md` is the execution tracker (project manager).
- If any task item conflicts with `requirements.md`, follow `requirements.md` and then update this file.
- Execution mode is strict phase-by-phase progression unless `requirements.md` explicitly requires otherwise.
- Rust toolchain baseline for this project is stable latest.

---

## 0. Project Setup

### 0.1 Initialize Rust Project
- [x] Create Cargo.toml with project metadata
  - [x] Set edition = "2021"
  - [x] Add basic dependencies (clap, serde, serde_json, thiserror)
  - [x] Configure workspace if needed
- [x] Create initial directory structure matching requirements.md
- [x] Set up .gitignore for Rust projects
- [x] Initialize git repository

### 0.2 Configure Development Tools
- [x] Add rustfmt.toml configuration
- [x] Add clippy.toml for linting rules
- [x] Set up CI configuration (GitHub Actions or similar)
  - [x] cargo test
  - [x] cargo clippy --deny warnings
  - [x] cargo fmt --check
- [x] Create CONTRIBUTING.md if needed

---

## 1. Phase 1 - Lexer

### 1.1 Define Token Types
- [x] Create `src/lexer/mod.rs`
- [x] Create `src/lexer/token.rs`
  - [x] Define `Token` enum with all token types
  - [x] Implement keyword tokens (module, import, export, etc.)
  - [x] Implement literal tokens (IntLiteral, FloatLiteral, StringLiteral, BoolLiteral)
  - [x] Implement identifier token
  - [x] Implement decorator tokens (Hash, LBracket, RBracket)
  - [x] Implement delimiter tokens (braces, parens, angles, etc.)
  - [x] Implement operator tokens (arithmetic, comparison, logical)
  - [x] Implement special tokens (EOF, Newline)
  - [x] Add `Span` struct with position tracking (file, line, col)
  - [x] Implement Display trait for Token

### 1.2 Implement Lexer Core
- [x] Create `src/lexer/lexer.rs`
  - [x] Define `Lexer` struct with source and position tracking
  - [x] Implement `new(source: String, file: PathBuf) -> Lexer`
  - [x] Implement `next_token() -> Result<Token, LexError>`
  - [x] Implement character peeking and advancing
  - [x] Track line and column numbers

### 1.3 Implement Token Recognition
- [x] Implement whitespace skipping
- [x] Implement single-line comment handling (`//`)
- [x] Implement block comment handling (`/* */`)
- [x] Implement string literal lexing
  - [x] Handle double quotes
  - [x] Handle escape sequences (\n, \t, \\, \")
  - [x] Error on unterminated strings
- [x] Implement number literal lexing
  - [x] Integer literals (decimal only)
  - [x] Float literals (with decimal point)
- [x] Implement identifier and keyword lexing
  - [x] Distinguish keywords from identifiers
- [x] Implement operator lexing
  - [x] Handle multi-character operators (==, !=, <=, >=, &&, ||)
  - [x] Handle arrow operators (=>, ->)

### 1.4 Lexer Error Handling
- [x] Define `LexError` enum using thiserror
  - [x] UnterminatedString variant
  - [x] UnexpectedChar variant
  - [x] InvalidEscape variant
- [x] Include span information in all errors
- [x] Implement error recovery (continue after error)

### 1.5 Lexer Tests
- [x] Unit test: keyword recognition
- [x] Unit test: identifier recognition
- [x] Unit test: integer literals
- [x] Unit test: float literals
- [x] Unit test: string literals with escapes
- [x] Unit test: all operators
- [x] Unit test: comments (single-line and block)
- [x] Unit test: position tracking accuracy
- [x] Unit test: error cases (unterminated string, invalid char)
- [x] Unit test: decorator sequence `#[`

---

## 2. Phase 2 - Parser & AST

### 2.1 Define AST Node Types
- [x] Create `src/parser/mod.rs`
- [x] Create `src/parser/ast.rs`
  - [x] Define `SourceFile` struct
  - [x] Define `TopLevelItem` enum (Module, Class, Interface, Struct, Enum, Import)
  - [x] Define `ModuleDecl` struct
  - [x] Define `ProviderBinding` enum (Simple, Aliased)
  - [x] Define `ClassDecl` struct
  - [x] Define `InterfaceDecl` struct
  - [x] Define `StructDecl` struct
  - [x] Define `EnumDecl` struct
  - [x] Define `ConstructorDecl` struct
  - [x] Define `MethodDecl` struct
  - [x] Define `FieldDecl` struct
  - [x] Define `Param` struct
  - [x] Define `Annotation` struct
  - [x] Define `AnnotationArg` enum (Positional, Named)
  - [x] Define `TypeExpr` enum (Named, Generic, Result, Option)
  - [x] Define `Expr` enum (all expression types)
  - [x] Define `Stmt` enum (Let, Return, Expr, If)
  - [x] Define `Block` struct
  - [x] Define `BinOp` and `UnaryOp` enums
  - [x] Ensure all nodes have `Span` field
  - [x] Implement Debug traits for all AST nodes

### 2.2 Implement Parser Core
- [x] Create `src/parser/parser.rs`
  - [x] Define `Parser` struct with token stream and position
  - [x] Implement `new(tokens: Vec<Token>) -> Parser`
  - [x] Implement `peek() -> &Token`
  - [x] Implement `advance() -> Token`
  - [x] Implement `expect(TokenType) -> Result<Token, ParseError>`
  - [x] Implement `match_token(TokenType) -> bool`

### 2.3 Implement Top-Level Parsing
- [x] Implement `parse_source_file() -> Result<SourceFile, Vec<ParseError>>`
- [x] Implement `parse_top_level_item() -> Result<TopLevelItem, ParseError>`
- [x] Implement `parse_import() -> Result<ImportDecl, ParseError>`
- [x] Implement `parse_module() -> Result<ModuleDecl, ParseError>`
  - [x] Parse `import` statements within module
  - [x] Parse `provide` bindings (simple and aliased)
  - [x] Parse `control` declarations
  - [x] Parse `export` declarations

### 2.4 Implement Type Parsing
- [x] Implement `parse_type_expr() -> Result<TypeExpr, ParseError>`
  - [x] Parse simple types (Int, String, Bool, etc.)
  - [x] Parse generic types (List<T>, Map<K,V>)
  - [x] Parse Result<T, E>
  - [x] Parse Option<T>
  - [x] Handle nested generics

### 2.5 Implement Annotation Parsing
- [x] Implement `parse_annotations() -> Result<Vec<Annotation>, ParseError>`
  - [x] Parse `#[name]` syntax
  - [x] Parse `#[name(arg1, arg2)]` with positional args
  - [x] Parse `#[name(key1 = value1, key2 = value2)]` with named args
  - [x] Handle multiple annotations on same item

### 2.6 Implement Class/Interface/Struct Parsing
- [x] Implement `parse_class() -> Result<ClassDecl, ParseError>`
  - [x] Parse class annotations
  - [x] Parse class name
  - [x] Parse `implements` clause
  - [x] Parse constructor with `constructor` keyword
  - [x] Parse `private` modifier on constructor params
  - [x] Parse methods
  - [x] Parse fields
- [x] Implement `parse_interface() -> Result<InterfaceDecl, ParseError>`
- [x] Implement `parse_struct() -> Result<StructDecl, ParseError>`
- [x] Implement `parse_enum() -> Result<EnumDecl, ParseError>`

### 2.7 Implement Method/Function Parsing
- [x] Implement `parse_method() -> Result<MethodDecl, ParseError>`
  - [x] Parse method annotations
  - [x] Parse method name
  - [x] Parse parameter list
  - [x] Parse return type
  - [x] Parse method body (block)

### 2.8 Implement Expression Parsing
- [x] Implement `parse_expr() -> Result<Expr, ParseError>`
- [x] Implement `parse_primary() -> Result<Expr, ParseError>`
  - [x] Parse literals (int, float, string, bool, null)
  - [x] Parse identifiers
  - [x] Parse parenthesized expressions
- [x] Implement `parse_call() -> Result<Expr, ParseError>`
- [x] Implement `parse_field_access() -> Result<Expr, ParseError>`
- [x] Implement operator precedence parsing
  - [x] Parse binary operators with correct precedence
  - [x] Parse unary operators
- [x] Implement `parse_block() -> Result<Block, ParseError>`

### 2.9 Implement Statement Parsing
- [x] Implement `parse_stmt() -> Result<Stmt, ParseError>`
- [x] Implement `parse_let_stmt() -> Result<Stmt, ParseError>`
- [x] Implement `parse_return_stmt() -> Result<Stmt, ParseError>`
- [x] Implement `parse_if_stmt() -> Result<Stmt, ParseError>`
- [x] Implement expression statements

### 2.10 Parser Error Handling
- [x] Define `ParseError` enum using thiserror
  - [x] UnexpectedToken variant
  - [x] UnexpectedEof variant
  - [x] InvalidSyntax variant
- [x] Implement error recovery (synchronization points)
- [x] Collect multiple errors before stopping
- [x] Include span information in all errors

### 2.11 Parser Tests
- [x] Unit test: parse simple module
- [x] Unit test: parse class with constructor and methods
- [x] Unit test: parse interface
- [x] Unit test: parse struct
- [x] Unit test: parse annotations with various argument forms
- [x] Unit test: parse generic types
- [x] Unit test: parse nested expressions
- [x] Unit test: parse all statement types
- [x] Unit test: error recovery on invalid syntax
- [x] Unit test: multiple errors collected
- [x] Unit test: `private` constructor params

---

## 3. Phase 3 - Name Resolution

### 3.1 Implement Symbol Table
- [x] Create `src/resolver/mod.rs`
- [x] Define `SymbolTable` struct
  - [x] Implement scope hierarchy (file → class → method)
  - [x] Implement `insert(name: String, kind: SymbolKind, span: Span)`
  - [x] Implement `lookup(name: String) -> Option<&Symbol>`
  - [x] Implement `enter_scope()` and `exit_scope()`
- [x] Define `Symbol` struct (name, kind, type, span)
- [x] Define `SymbolKind` enum (Type, Variable, Class, Interface, etc.)

### 3.2 Implement Name Resolution Pass
- [x] Implement `Resolver` struct
- [x] Implement `resolve_source_file(ast: &SourceFile) -> Result<(), Vec<ResolveError>>`
- [x] Implement `resolve_module(module: &ModuleDecl)`
  - [x] Resolve imported module names
  - [x] Resolve provider class names
  - [x] Resolve controller class names
  - [x] Resolve exported symbol names
- [x] Implement `resolve_class(class: &ClassDecl)`
  - [x] Resolve implemented interface names
  - [x] Resolve constructor parameter types
  - [x] Resolve field types
  - [x] Resolve method parameter and return types
- [x] Implement `resolve_type_expr(ty: &TypeExpr)`
  - [x] Resolve Named types
  - [x] Resolve Generic type parameters
  - [x] Resolve Result and Option inner types
- [x] Implement `resolve_annotation(annotation: &Annotation)`
  - [x] Resolve class references in annotation arguments

### 3.3 Resolution Error Handling
- [x] Define `ResolveError` enum using thiserror
  - [x] UndefinedType variant
  - [x] UndefinedSymbol variant
  - [x] DuplicateSymbol variant
- [x] Include span information in all errors
- [x] Collect all resolution errors

### 3.4 Name Resolution Tests
- [x] Unit test: resolve simple types
- [x] Unit test: resolve generic types
- [x] Unit test: resolve interface implementations
- [x] Unit test: resolve constructor dependencies
- [x] Unit test: error on undefined type
- [x] Unit test: error on undefined symbol
- [x] Unit test: error on duplicate symbols
- [x] Unit test: scope hierarchy works correctly

---

## 4. Phase 4 - Type Checker

### 4.1 Implement Type System Core
- [x] Create `src/typechecker/mod.rs`
- [x] Define `Type` enum (distinct from `TypeExpr`)
  - [x] Primitive types
  - [x] Class/Interface types
  - [x] Generic types with concrete parameters
  - [x] Function types
- [x] Implement type equality checking
- [x] Implement type compatibility checking

### 4.2 Implement Type Inference
- [x] Implement `TypeChecker` struct
- [x] Implement `infer_expr(expr: &Expr) -> Result<Type, TypeError>`
  - [x] Infer literal types
  - [x] Infer identifier types (lookup in symbol table)
  - [x] Infer call expression types
  - [x] Infer field access types
  - [x] Infer binary operation types
  - [x] Infer unary operation types

### 4.3 Implement Type Validation
- [x] Implement `check_source_file(ast: &SourceFile) -> Result<(), Vec<TypeError>>`
- [x] Implement `check_class(class: &ClassDecl)`
  - [x] Check all public methods have explicit return types
  - [x] Check all DTO fields have explicit types
  - [x] Check method bodies match declared return types
- [x] Implement `check_method(method: &MethodDecl)`
  - [x] Validate return type annotation is present
  - [x] Check return statements match declared type
  - [x] Validate parameter types

### 4.4 Implement Controller-Specific Validation
- [x] Check controller handler methods return `Result<T, HttpError>`
- [x] Validate T is serializable
- [x] Check `#[body]` parameter types are deserializable

### 4.5 Implement Serializability Checking
- [x] Implement `is_serializable(ty: &Type) -> bool`
  - [x] Check primitives (Int, Float, Bool, String)
  - [x] Check structs/classes have all-typed fields
  - [x] Check List<T> where T is serializable
  - [x] Check Map<String, T> where T is serializable
- [x] Emit error for non-serializable return types

### 4.6 Type Checker Error Handling
- [x] Define `TypeError` enum using thiserror
  - [x] MissingReturnType variant
  - [x] UntypedDtoField variant
  - [x] NonSerializableReturn variant
  - [x] TypeMismatch variant
  - [x] IncompatibleTypes variant
- [x] Include span information in all errors
- [x] Collect all type errors

### 4.7 Type Checker Tests
- [x] Unit test: infer expression types
- [x] Unit test: check return type annotations required
- [x] Unit test: check DTO fields must be typed
- [x] Unit test: check controller handlers return Result
- [x] Unit test: check serializable types
- [x] Unit test: error on non-serializable return
- [x] Unit test: error on type mismatch
- [x] Unit test: generic type checking

---

## 5. Phase 5 - Annotation Processor

### 5.1 Implement Annotation Registry
- [x] Create `src/annotations/mod.rs`
- [x] Define `AnnotationRegistry` struct
- [x] Define known annotation metadata
  - [x] Name, valid targets, required/optional arguments
- [x] Implement `register_annotation(metadata: AnnotationMetadata)`
- [x] Register all built-in annotations

### 5.2 Implement Annotation Validation
- [x] Implement `AnnotationProcessor` struct
- [x] Implement `process_source_file(ast: &SourceFile) -> Result<(), Vec<AnnotationError>>`
- [x] Implement `validate_annotation(ann: &Annotation, target: AnnotationTarget)`
  - [x] Check annotation name is known
  - [x] Check annotation target is valid
  - [x] Check required arguments are present
  - [x] Check argument types are correct

### 5.3 Validate Specific Annotations
- [x] Validate `#[injectable]`
  - [x] Check scope argument is valid (singleton/request/transient)
  - [x] Default to singleton if omitted
- [x] Validate `#[controller(path)]`
  - [x] Check path argument is a string literal
  - [x] Check applied to class only
- [x] Validate HTTP method annotations (`#[get]`, `#[post]`, etc.)
  - [x] Check path argument is a string literal
  - [x] Check applied to method only
- [x] Validate parameter annotations (`#[param]`, `#[query]`, `#[body]`, `#[header]`)
  - [x] Check applied to method parameters only
  - [x] Validate `#[body]` appears at most once per method
  - [x] Validate `#[param("id")]` matches route path parameter
- [x] Validate lifecycle annotations (`#[use_guards]`, `#[use_interceptors]`, `#[use_pipes]`)
  - [x] Check arguments are class names
  - [x] Validate class references exist

### 5.4 Route Parameter Binding Validation
- [x] Extract route path from controller and method annotations
- [x] Parse route path for parameters (`:paramName`)
- [x] Check each `#[param("name")]` matches a route parameter
- [x] Check each route parameter has a corresponding `#[param]`
- [x] Error on unbound route parameters
- [x] Error on unused param annotations

### 5.5 Annotation Error Handling
- [x] Define `AnnotationError` enum using thiserror
  - [x] UnknownAnnotation variant
  - [x] MissingArgument variant
  - [x] InvalidArgument variant
  - [x] UnboundRouteParam variant
  - [x] DuplicateBody variant
  - [x] InvalidTarget variant
- [x] Include span information in all errors
- [x] Collect all annotation errors

### 5.6 Annotation Processor Tests
- [x] Unit test: validate known annotations
- [x] Unit test: error on unknown annotation
- [x] Unit test: validate required arguments
- [x] Unit test: validate injectable scope values
- [x] Unit test: validate controller path
- [x] Unit test: validate route parameter binding
- [x] Unit test: error on duplicate @body
- [x] Unit test: error on unbound route param
- [x] Unit test: validate lifecycle annotations

---

## 6. Phase 6 - DI Graph Builder & Validator

### 6.1 Implement DI Graph Structure
- [x] Create `src/di/mod.rs`
- [x] Create `src/di/graph.rs`
- [x] Define `DiGraph` struct
  - [x] Nodes: providers (class name, scope, dependencies)
  - [x] Edges: dependency relationships
- [x] Define `Provider` struct (name, scope, dependencies, span)
- [x] Define `Scope` enum (Singleton, Request, Transient)
- [x] Implement `add_provider(provider: Provider)`
- [x] Implement `add_dependency(from: String, to: String)`

### 6.2 Build DI Graph from AST
- [x] Implement `DiGraphBuilder` struct
- [x] Implement `build(ast: &SourceFile) -> Result<DiGraph, Vec<DiError>>`
- [x] Collect all `#[injectable]` classes
- [x] For each injectable class:
  - [x] Extract constructor parameters as dependencies
  - [x] Determine scope from annotation
  - [x] Add as provider node
- [x] Process module `provide` declarations
  - [x] Handle simple bindings (provide X)
  - [x] Handle aliased bindings (provide Interface => Impl)
- [x] Build dependency edges

### 6.3 Validate DI Graph
- [x] Implement `validate_di_graph(graph: &DiGraph) -> Result<(), Vec<DiError>>`
- [x] Check for missing providers
  - [x] For each dependency, ensure provider exists in scope
- [x] Check for circular dependencies
  - [x] Implement cycle detection algorithm (DFS with visited set)
  - [x] Report full cycle path in error
- [x] Check for duplicate providers
  - [x] Detect multiple providers for same token in same module
- [x] Validate scope compatibility
  - [x] singleton can only inject singleton
  - [x] request can inject singleton and request
  - [x] transient can inject all scopes
  - [x] Error on scope violations

### 6.4 Validate Module Provider Rules
- [x] Check provider classes are marked `#[injectable]`
- [x] Check exported symbols are provided in same module
- [x] Validate interface implementations for aliased bindings

### 6.5 DI Graph Error Handling
- [x] Define `DiError` enum using thiserror
  - [x] MissingProvider variant (DI001)
  - [x] CircularDependency variant (DI002)
  - [x] DuplicateProvider variant (DI003)
  - [x] ScopeMismatch variant (DI004)
  - [x] NotInjectable variant (DI005)
  - [x] ExportWithoutProvider variant (DI006)
- [x] Include span information in all errors
- [x] Collect all DI errors

### 6.6 DI Graph Tests
- [x] Unit test: build simple DI graph
- [x] Unit test: detect missing provider
- [x] Unit test: detect circular dependency (A→B→A)
- [x] Unit test: detect longer cycles (A→B→C→A)
- [x] Unit test: detect duplicate provider
- [x] Unit test: validate singleton scope restrictions
- [x] Unit test: validate request scope can inject singleton
- [x] Unit test: validate transient scope permissions
- [x] Unit test: error on non-injectable class
- [x] Unit test: error on export without provider
- [x] Unit test: aliased bindings work correctly

---

## 7. Phase 7 - Module Graph Builder & Validator

### 7.1 Implement Module Graph Structure
- [x] Create `src/modules/mod.rs`
- [x] Create `src/modules/graph.rs`
- [x] Define `ModuleGraph` struct
  - [x] Nodes: modules (name, imports, exports, providers, controllers)
  - [x] Edges: import relationships
- [x] Define `Module` struct
- [x] Implement `add_module(module: Module)`
- [x] Implement `add_import(from: String, to: String)`

### 7.2 Build Module Graph from AST
- [x] Implement `ModuleGraphBuilder` struct
- [x] Implement `build(ast: &SourceFile) -> Result<ModuleGraph, Vec<ModuleError>>`
- [x] Collect all module declarations
- [x] For each module:
  - [x] Record imports
  - [x] Record providers
  - [x] Record controllers
  - [x] Record exports
  - [x] Add module node
- [x] Build import edges between modules

### 7.3 Validate Module Graph
- [x] Implement `validate_module_graph(graph: &ModuleGraph) -> Result<(), Vec<ModuleError>>`
- [x] Check for unknown imports
  - [x] Verify imported module exists
- [x] Check for circular imports
  - [x] Implement cycle detection
  - [x] Report full import cycle
- [x] Validate controller dependencies
  - [x] Check controller can access all required services
  - [x] Services must be provided in same module or imported

### 7.4 Validate Module Provider Visibility
- [x] Check symbols are exported from source module
- [x] Error when consuming private provider from another module
- [x] Validate export declarations match provided symbols

### 7.5 Module Graph Error Handling
- [x] Define `ModuleError` enum using thiserror
  - [x] UnknownImport variant (MOD001)
  - [x] CircularImport variant (MOD002)
  - [x] UnsatisfiedController variant (MOD003)
  - [x] PrivateProvider variant (MOD004)
- [x] Include span information in all errors
- [x] Collect all module errors

### 7.6 Module Graph Tests
- [x] Unit test: build simple module graph
- [x] Unit test: detect unknown import
- [x] Unit test: detect circular import (A→B→A)
- [x] Unit test: detect longer import cycles
- [x] Unit test: validate controller dependencies satisfied
- [x] Unit test: error on private provider access
- [x] Unit test: validate exports match providers
- [x] Unit test: multi-module app with proper imports

---

## 8. Phase 8 - Route Table Builder & Validator

### 8.1 Implement Route Table Structure
- [x] Create `src/routes/mod.rs`
- [x] Create `src/routes/registry.rs`
- [x] Define `RouteTable` struct
- [x] Define `Route` struct
  - [x] HTTP method (GET, POST, PUT, DELETE, PATCH)
  - [x] Full path (prefix + method path)
  - [x] Handler (fully qualified method name)
  - [x] Path parameters
  - [x] Span
- [x] Define `HttpMethod` enum
- [x] Implement `add_route(route: Route)`
- [x] Implement `get_routes() -> &[Route]`

### 8.2 Build Route Table from AST
- [x] Implement `RouteTableBuilder` struct
- [x] Implement `build(ast: &SourceFile) -> Result<RouteTable, Vec<RouteError>>`
- [x] Find all classes with `#[controller(prefix)]`
- [x] For each controller:
  - [x] Extract controller path prefix
  - [x] Find all methods with HTTP method annotations
  - [x] For each handler method:
    - [x] Extract HTTP method (get, post, put, delete, patch)
    - [x] Extract method path
    - [x] Construct full path (prefix + method path)
    - [x] Record route entry

### 8.3 Parse Route Paths
- [x] Implement route path parser
- [x] Extract path parameters (`:paramName` syntax)
- [x] Validate path syntax
- [x] Store parameter names for validation

### 8.4 Validate Route Table
- [x] Implement `validate_route_table(table: &RouteTable) -> Result<(), Vec<RouteError>>`
- [x] Check for duplicate routes
  - [x] Same HTTP method + exact path → error
  - [x] Report both conflicting handlers
- [x] Validate path parameters
  - [x] Each `:param` in path must have `#[param("name")]` on method
  - [x] Each `#[param("name")]` must have corresponding `:name` in path
  - [x] Error on unbound path parameters
  - [x] Error on unused param annotations

### 8.5 Route Error Handling
- [x] Define `RouteError` enum using thiserror
  - [x] DuplicateRoute variant (ROUTE001)
  - [x] UnboundPathParam variant (ROUTE002)
  - [x] UnusedParamAnnotation variant (ROUTE003)
- [x] Include span information in all errors
- [x] Collect all route errors

### 8.6 Route Table Tests
- [x] Unit test: build simple route table
- [x] Unit test: construct full paths from prefix + method path
- [x] Unit test: detect duplicate routes
- [x] Unit test: validate path parameters
- [x] Unit test: error on unbound path param
- [x] Unit test: error on unused param annotation
- [x] Unit test: multiple controllers with different prefixes
- [x] Unit test: all HTTP methods (GET, POST, PUT, DELETE, PATCH)

---

## 9. Phase 9 - Lifecycle Pipeline Builder

### 9.1 Implement Pipeline Structure
- [x] Create `src/lifecycle/mod.rs`
- [x] Create `src/lifecycle/pipeline.rs`
- [x] Define `Pipeline` struct
  - [x] Middleware list
  - [x] Guards list
  - [x] Pipes list
  - [x] Interceptors list
  - [x] Exception filters list
- [x] Define `PipelineStage` enum
- [x] Define `LifecycleComponent` struct (name, type, span)

### 9.2 Build Pipelines from Annotations
- [x] Implement `PipelineBuilder` struct
- [x] Implement `build(route_table: &RouteTable, ast: &SourceFile) -> Result<HashMap<Route, Pipeline>, Vec<LifecycleError>>`
- [x] For each route:
  - [x] Find corresponding controller class
  - [x] Find corresponding handler method
  - [x] Collect class-level lifecycle annotations
  - [x] Collect method-level lifecycle annotations
  - [x] Build ordered pipeline

### 9.3 Assemble Pipeline Order
- [x] Extract `#[use_guards(...)]` from class level
- [x] Extract `#[use_guards(...)]` from method level
- [x] Combine: class-level first, then method-level
- [x] Repeat for interceptors and pipes
- [x] Maintain fixed order: Middleware → Guards → Pipes → Handler → Interceptors → Filters

### 9.4 Validate Pipeline Components
- [x] Implement `validate_pipelines(pipelines: &HashMap<Route, Pipeline>) -> Result<(), Vec<LifecycleError>>`
- [x] For each guard class:
  - [x] Check implements `Guard` interface
  - [x] Check is `#[injectable]`
- [x] For each interceptor class:
  - [x] Check implements `Interceptor` interface
  - [x] Check is `#[injectable]`
- [x] For each pipe class:
  - [x] Check implements `Pipe` interface
  - [x] Check is `#[injectable]`

### 9.5 Lifecycle Error Handling
- [x] Define `LifecycleError` enum using thiserror
  - [x] InvalidGuard variant (LC001)
  - [x] InvalidInterceptor variant (LC002)
  - [x] InvalidPipe variant (LC003)
  - [x] NotInjectable variant (LC004)
- [x] Include span information in all errors
- [x] Collect all lifecycle errors

### 9.6 Lifecycle Pipeline Tests
- [x] Unit test: build simple pipeline
- [x] Unit test: combine class and method level annotations
- [x] Unit test: maintain correct order
- [x] Unit test: error on invalid guard (wrong interface)
- [x] Unit test: error on invalid interceptor
- [x] Unit test: error on invalid pipe
- [x] Unit test: error on non-injectable component
- [x] Unit test: multiple guards applied correctly
- [x] Unit test: empty pipeline (no lifecycle components)

---

## 10. Phase 10 - IR & Code Generation

Execution policy for this phase:
- [ ] Complete Cranelift end-to-end path first (IR lowering -> object generation -> runtime linking -> executable output).
- [ ] Treat LLVM work as optional and non-blocking for v1 delivery.

### 10.1 Define Intermediate Representation
- [x] Create `src/ir/mod.rs`
- [x] Define `IrModule` struct
- [x] Define `IrFunction` struct
  - [x] Name, parameters, return type, body
- [x] Define `IrBlock` struct (basic block)
- [x] Define `IrInstruction` enum
  - [x] Load, Store, Call, Return, Branch, etc.
- [x] Define `IrValue` enum (constants, variables, etc.)
- [x] Define `IrType` enum

### 10.2 Lower AST to IR
- [x] Implement `IrGenerator` struct
- [x] Implement `generate_ir(ast: &SourceFile) -> Result<IrModule, IrError>`
- [x] Lower module declarations to IR
- [ ] Lower class declarations to struct definitions
- [x] Lower methods to IR functions
- [x] Lower expressions to IR instructions
- [x] Lower statements to IR instructions

### 10.3 Generate Static Tables in IR
- [ ] Generate DI registry as static data
  - [ ] Map of token string → factory function pointer
- [ ] Generate route table as static data
  - [ ] Array of (method, path, handler_fn_ptr)
- [ ] Generate lifecycle pipelines as static data
  - [ ] Per-route array of lifecycle function pointers

### 10.4 Implement Cranelift Backend
- [x] Create `src/codegen/mod.rs`
- [ ] Create `src/codegen/cranelift.rs`
- [ ] Add Cranelift dependencies to Cargo.toml
  - [ ] cranelift-codegen
  - [ ] cranelift-frontend
  - [ ] cranelift-module
  - [ ] cranelift-object
- [x] Implement `CraneliftBackend` struct
- [~] Implement `compile(ir: &IrModule) -> Result<Vec<u8>, CodegenError>`
  - [ ] Initialize Cranelift ISA and module
  - [ ] Translate IR functions to Cranelift IR
  - [ ] Compile to machine code
  - [ ] Generate object file
- [ ] Phase gate: do not start LLVM tasks until Cranelift-based executable generation succeeds for minimal app.

### 10.5 Link Runtime
- [ ] Create minimal Rust runtime crate
  - [ ] HTTP server (TCP listener, HTTP/1.1 parser)
  - [ ] Route dispatcher (reads static route table)
  - [ ] DI container (reads static DI registry)
  - [ ] JSON serializer/deserializer
  - [ ] Lifecycle pipeline executor
- [ ] Link generated code with runtime
- [ ] Produce final executable binary

### 10.6 Output Binary
- [ ] Implement binary output to `./dist/<name>`
- [ ] Make binary executable
- [ ] Handle cross-platform concerns (if needed)

### 10.7 Optional LLVM Backend
- [ ] Create `src/codegen/llvm.rs` (behind feature flag)
- [ ] Add LLVM dependencies (feature-gated)
- [ ] Implement LLVM IR generation
- [ ] Add `--features llvm` build option

### 10.8 Codegen Tests
- [ ] Unit test: generate IR from simple class
- [ ] Unit test: generate IR from method with expressions
- [ ] Unit test: generate static DI registry
- [ ] Unit test: generate static route table
- [ ] Integration test: compile minimal program to binary
- [ ] Integration test: run generated binary
- [ ] Integration test: verify HTTP server responds

---

## 11. Phase 11 - Error Reporting

### 11.1 Design Error Format
- [ ] Create `src/errors/mod.rs`
- [ ] Define `Diagnostic` struct
  - [ ] Error code (e.g., DI001)
  - [ ] Severity (Error, Warning)
  - [ ] Message
  - [ ] Span
  - [ ] Note (optional explanation)
  - [ ] Help (optional suggestion)
- [ ] Define `DiagnosticLevel` enum

### 11.2 Implement Error Formatter
- [ ] Implement `ErrorFormatter` struct
- [ ] Implement `format_diagnostic(diag: &Diagnostic) -> String`
  - [ ] Format error code and severity
  - [ ] Show file path and line:col
  - [ ] Display source code snippet
  - [ ] Highlight error span with carets (^)
  - [ ] Show note and help messages

### 11.3 Implement Error Collection
- [ ] Implement `DiagnosticCollector` struct
- [ ] Implement `add(diagnostic: Diagnostic)`
- [ ] Implement `has_errors() -> bool`
- [ ] Implement `get_errors() -> &[Diagnostic]`
- [ ] Implement `get_warnings() -> &[Diagnostic]`
- [ ] Sort diagnostics by file, line, column

### 11.4 Convert Phase Errors to Diagnostics
- [ ] Convert `LexError` to `Diagnostic`
- [ ] Convert `ParseError` to `Diagnostic`
- [ ] Convert `ResolveError` to `Diagnostic`
- [ ] Convert `TypeError` to `Diagnostic`
- [ ] Convert `AnnotationError` to `Diagnostic`
- [ ] Convert `DiError` to `Diagnostic`
- [ ] Convert `ModuleError` to `Diagnostic`
- [ ] Convert `RouteError` to `Diagnostic`
- [ ] Convert `LifecycleError` to `Diagnostic`

### 11.5 Implement Output Formats
- [ ] Implement human-readable output (default)
  - [ ] Colored output for terminals
  - [ ] Plain text for non-TTY
- [ ] Implement JSON output (`--format json`)
  - [ ] Machine-readable format for tooling
  - [ ] Include all diagnostic fields

### 11.6 Error Reporting Tests
- [ ] Unit test: format diagnostic with all fields
- [ ] Unit test: format diagnostic with source snippet
- [ ] Unit test: collect multiple diagnostics
- [ ] Unit test: sort diagnostics by position
- [ ] Unit test: JSON output format
- [ ] Unit test: convert each error type to diagnostic
- [ ] Integration test: error output matches expected format

---

## 12. Phase 12 - CLI

### 12.1 Setup CLI Framework
- [ ] Create `src/cli/mod.rs`
- [ ] Create `src/main.rs`
- [ ] Add `clap` dependency with derive feature
- [ ] Define main CLI struct with clap
- [ ] Define subcommands enum

### 12.2 Implement `arwa new` Command
- [ ] Create `src/cli/new.rs`
- [ ] Define command arguments (project name, optional template)
- [ ] Implement `run(args: NewArgs) -> Result<(), Error>`
  - [ ] Validate project name
  - [ ] Create project directory
  - [ ] Copy template files from bundled templates
  - [ ] Generate `arwa.blueprint.json`
  - [ ] Initialize basic project structure
  - [ ] Print success message with next steps

### 12.3 Implement `arwa build` Command
- [ ] Create `src/cli/build.rs`
- [ ] Define command arguments (optional: output path, optimization level)
- [ ] Implement `run(args: BuildArgs) -> Result<(), Error>`
  - [ ] Read all `.rw` source files
  - [ ] Run lexer phase
  - [ ] Run parser phase
  - [ ] Run resolver phase
  - [ ] Run type checker phase
  - [ ] Run annotation processor phase
  - [ ] Run DI graph validator phase
  - [ ] Run module graph validator phase
  - [ ] Run route table builder phase
  - [ ] Run lifecycle pipeline builder phase
  - [ ] Generate IR
  - [ ] Run codegen
  - [ ] Output binary to `./dist/<name>`
  - [ ] Print compilation summary

### 12.4 Implement `arwa check` Command
- [ ] Create `src/cli/check.rs`
- [ ] Define command arguments (optional: format)
- [ ] Implement `run(args: CheckArgs) -> Result<(), Error>`
  - [ ] Run phases 1-9 (all validation, no codegen)
  - [ ] Report errors and warnings
  - [ ] Exit with appropriate code

### 12.5 Implement `arwa run` Command
- [ ] Create `src/cli/run.rs`
- [ ] Define command arguments (optional: port, env)
- [ ] Implement `run(args: RunArgs) -> Result<(), Error>`
  - [ ] Run `arwa build`
  - [ ] If build succeeds, execute output binary
  - [ ] Pass through command-line arguments
  - [ ] Stream output to stdout/stderr

### 12.6 Implement `arwa add` Command
- [ ] Create `src/cli/add.rs`
- [ ] Define command arguments (feature name)
- [ ] Implement `run(args: AddArgs) -> Result<(), Error>`
  - [ ] Read `arwa.blueprint.json`
  - [ ] Validate feature exists in registry
  - [ ] Copy feature files from templates
  - [ ] Update `arwa.blueprint.json`
  - [ ] Print instructions for using the feature

### 12.7 Implement `arwa fmt` Command
- [ ] Create `src/cli/fmt.rs`
- [ ] Define command arguments (optional: check mode)
- [ ] Implement `run(args: FmtArgs) -> Result<(), Error>`
  - [ ] Find all `.rw` files
  - [ ] Parse each file
  - [ ] Reformat according to style rules
    - [ ] 2-space indentation
    - [ ] Consistent blank lines
    - [ ] Sort imports
  - [ ] Write back to file (or check mode)

### 12.8 CLI Error Handling
- [ ] Implement user-friendly error messages
- [ ] Use appropriate exit codes
  - [ ] 0 for success
  - [ ] 1 for compilation errors
  - [ ] 2 for CLI usage errors
- [ ] Handle file system errors gracefully
- [ ] Handle missing dependencies

### 12.9 CLI Tests
- [ ] Integration test: `arwa new` creates project
- [ ] Integration test: `arwa build` compiles minimal project
- [ ] Integration test: `arwa check` validates without codegen
- [ ] Integration test: `arwa run` executes binary
- [ ] Integration test: `arwa add` adds feature
- [ ] Integration test: `arwa fmt` formats files
- [ ] Unit test: CLI argument parsing
- [ ] Unit test: error handling and exit codes

---

## 13. Phase 13 - Project Scaffolding & Blueprint

### 13.1 Design Blueprint Schema
- [ ] Define `arwa.blueprint.json` structure
  - [ ] name: String
  - [ ] version: String
  - [ ] starter: String
  - [ ] features: Vec<String>
- [ ] Create JSON schema validation

### 13.2 Create Starter Templates
- [ ] Create `templates/starters/minimal/`
  - [ ] Basic module with main function
  - [ ] Minimal project structure
- [ ] Create `templates/starters/api/`
  - [ ] Module with controller
  - [ ] Service with DI
  - [ ] DTO examples
  - [ ] Example routes
- [ ] Create `templates/starters/full/`
  - [ ] API starter
  - [ ] Auth module
  - [ ] Database module
  - [ ] Logger module
  - [ ] Complete example app

### 13.3 Create Feature Templates
- [ ] Create `templates/features/http/`
  - [ ] HTTP utilities
  - [ ] Custom decorators
- [ ] Create `templates/features/di/`
  - [ ] Advanced DI examples
  - [ ] Custom scopes
- [ ] Create `templates/features/logger/`
  - [ ] Logger service
  - [ ] Logger module
  - [ ] Usage examples
- [ ] Create `templates/features/auth-jwt/`
  - [ ] JWT auth guard
  - [ ] Auth module
  - [ ] JWT utilities
  - [ ] Login/register examples
- [ ] Create `templates/features/db-postgres/`
  - [ ] Postgres connection
  - [ ] Repository pattern
  - [ ] Migration utilities

### 13.4 Create Template Registry
- [ ] Create `templates/registry.json`
- [ ] Document each feature
  - [ ] Name
  - [ ] Description
  - [ ] File list
  - [ ] Dependencies
  - [ ] Usage instructions
- [ ] Implement registry parser

### 13.5 Bundle Templates in Binary
- [ ] Use `include_str!` or `include_bytes!` macros
- [ ] Embed all template files in compiler binary
- [ ] Implement template extraction at runtime

### 13.6 Template Tests
- [ ] Unit test: parse blueprint.json
- [ ] Unit test: validate blueprint schema
- [ ] Unit test: parse registry.json
- [ ] Integration test: create project from minimal template
- [ ] Integration test: create project from api template
- [ ] Integration test: create project from full template
- [ ] Integration test: add each feature to project

---

## 14. Phase 14 - Standard Library

### 14.1 Implement `stdlib/http.rw`
- [ ] Create `stdlib/http.rw`
- [ ] Define `HttpError` class
  - [ ] status: Int field
  - [ ] message: String field
  - [ ] Constructor
- [ ] Define `RequestContext` struct
  - [ ] method: String
  - [ ] path: String
  - [ ] headers: Map<String, String>
  - [ ] body: String
- [ ] Define `Response` struct
  - [ ] status: Int
  - [ ] headers: Map<String, String>
  - [ ] body: String
- [ ] Define `Guard` interface
  - [ ] canActivate method signature
- [ ] Define `Interceptor` interface
  - [ ] intercept method signature
- [ ] Define `Pipe` interface
  - [ ] transform method signature
- [ ] Define `PipeMetadata` struct

### 14.2 Implement `stdlib/result.rw`
- [ ] Create `stdlib/result.rw`
- [ ] Define `Result<T, E>` enum (if not built-in)
  - [ ] Ok(T) variant
  - [ ] Err(E) variant
- [ ] Implement methods:
  - [ ] unwrap() -> T
  - [ ] unwrap_or(default: T) -> T
  - [ ] map<U>(fn: (T) -> U) -> Result<U, E>
  - [ ] is_ok() -> Bool
  - [ ] is_err() -> Bool
- [ ] Define `Option<T>` enum (if not built-in)
  - [ ] Some(T) variant
  - [ ] None variant
- [ ] Implement Option methods

### 14.3 Implement `stdlib/json.rw`
- [ ] Create `stdlib/json.rw`
- [ ] Define `Json` class with static methods
  - [ ] encode<T>(value: T) -> String
  - [ ] decode<T>(raw: String) -> Result<T, JsonError>
- [ ] Define `JsonError` class
  - [ ] message: String
  - [ ] position: Int (optional)

### 14.4 Standard Library Integration
- [ ] Make stdlib available to all ArwaLang programs
- [ ] Implement automatic imports for common types
- [ ] Handle stdlib types in type checker
- [ ] Ensure stdlib types are available in codegen

### 14.5 Standard Library Tests
- [ ] Unit test: HttpError construction
- [ ] Unit test: RequestContext usage
- [ ] Unit test: Guard interface implementation
- [ ] Unit test: Interceptor interface implementation
- [ ] Unit test: Pipe interface implementation
- [ ] Unit test: Result methods
- [ ] Unit test: Option methods
- [ ] Unit test: Json encode/decode
- [ ] Integration test: use stdlib in example program

---

## 15. Testing Infrastructure

### 15.1 Setup Test Framework
- [ ] Configure cargo test
- [ ] Add test utilities module
- [ ] Add test fixtures directory
- [ ] Create test helper functions

### 15.2 Create E2E Test Harness
- [ ] Create `tests/e2e/` directory
- [ ] Implement test runner
  - [ ] Parse `@expect` annotations
  - [ ] Compile test programs
  - [ ] Assert expected outcome (compile_success or compile_error CODE)
- [ ] Implement test result reporting

### 15.3 Create Required E2E Tests
- [ ] `tests/e2e/minimal_app.rw`
  - [ ] Single module, single controller, no DI
  - [ ] `@expect: compile_success`
- [ ] `tests/e2e/di_basic.rw`
  - [ ] Service injected into controller
  - [ ] `@expect: compile_success`
- [ ] `tests/e2e/di_scope_violation.rw`
  - [ ] Request-scoped into singleton
  - [ ] `@expect: compile_error DI004`
- [ ] `tests/e2e/circular_di.rw`
  - [ ] A depends on B depends on A
  - [ ] `@expect: compile_error DI002`
- [ ] `tests/e2e/duplicate_route.rw`
  - [ ] Two handlers on same path
  - [ ] `@expect: compile_error ROUTE001`
- [ ] `tests/e2e/missing_provider.rw`
  - [ ] Controller needs service not in module
  - [ ] `@expect: compile_error DI001`
- [ ] `tests/e2e/full_app.rw`
  - [ ] Multi-module with guards, interceptors, DTO validation
  - [ ] `@expect: compile_success`

### 15.4 Additional E2E Tests
- [ ] Test circular module imports
- [ ] Test private provider access error
- [ ] Test unbound path parameter error
- [ ] Test missing return type error
- [ ] Test non-serializable return error
- [ ] Test unknown annotation error
- [ ] Test duplicate body parameter error
- [ ] Test invalid guard error
- [ ] Test complex multi-module app

### 15.5 Performance Tests
- [ ] Benchmark lexer performance
- [ ] Benchmark parser performance
- [ ] Benchmark compilation time for large projects
- [ ] Memory usage profiling

---

## 16. Documentation

### 16.1 Code Documentation
- [ ] Add doc comments to all public functions
- [ ] Add module-level documentation
- [ ] Document error codes and their meanings
- [ ] Document AST node structures
- [ ] Generate rustdoc documentation

### 16.2 User Documentation
- [ ] Create README.md for the project
  - [ ] Project overview
  - [ ] Installation instructions
  - [ ] Quick start guide
  - [ ] Link to full documentation
- [ ] Create language guide
  - [ ] Syntax reference
  - [ ] Type system
  - [ ] Decorators
  - [ ] DI system
  - [ ] Module system
- [ ] Create CLI reference
  - [ ] Document all commands
  - [ ] Document command-line options
  - [ ] Provide examples

### 16.3 Example Programs
- [ ] Create hello world example
- [ ] Create REST API example
- [ ] Create multi-module example
- [ ] Create authentication example
- [ ] Create database integration example

### 16.4 Error Code Reference
- [ ] Create error code documentation
- [ ] For each error code:
  - [ ] Description
  - [ ] Common causes
  - [ ] How to fix
  - [ ] Example code that triggers it

---

## 17. Polish & Release

### 17.1 Error Message Quality
- [ ] Review all error messages for clarity
- [ ] Ensure helpful suggestions are provided
- [ ] Test error messages with real users
- [ ] Add color coding for terminal output

### 17.2 Performance Optimization
- [ ] Profile compilation performance
- [ ] Optimize hot paths
- [ ] Reduce memory allocations
- [ ] Parallelize independent phases (if beneficial)

### 17.3 Code Quality
- [ ] Run cargo clippy on entire codebase
- [ ] Fix all warnings
- [ ] Run cargo fmt on entire codebase
- [ ] Review code for improvements
- [ ] Remove dead code
- [ ] Remove debug print statements

### 17.4 Final Testing
- [ ] Run full test suite
- [ ] Test on different platforms (Linux, macOS, Windows)
- [ ] Test with various example programs
- [ ] Stress test with large codebases
- [ ] Verify all error codes are tested

### 17.5 Release Preparation
- [ ] Create changelog
- [ ] Write release notes
- [ ] Create binary distribution
- [ ] Setup installation method
- [ ] Create GitHub release
- [ ] Tag version

---

## Dependencies Between Phases

- Phase 2 depends on Phase 1 (Parser needs Lexer)
- Phase 3 depends on Phase 2 (Resolver needs AST)
- Phase 4 depends on Phase 3 (Type checker needs resolved symbols)
- Phase 5 depends on Phase 2 (Annotation processor needs AST)
- Phase 6 depends on Phases 3, 4, 5 (DI graph needs resolved, typed, annotated AST)
- Phase 7 depends on Phase 6 (Module graph needs DI graph)
- Phase 8 depends on Phases 3, 5 (Route table needs resolved, annotated AST)
- Phase 9 depends on Phases 6, 8 (Lifecycle needs DI and route table)
- Phase 10 depends on Phases 1-9 (Codegen needs fully validated AST)
- Phase 11 follows Phase 10 in strict phase-by-phase execution mode
- Phase 12 depends on Phases 1-10 (CLI orchestrates all phases)
- Phase 13 follows Phase 12 in strict phase-by-phase execution mode
- Phase 14 follows Phase 13 in strict phase-by-phase execution mode
- Phase 15 follows Phase 14 in strict phase-by-phase execution mode

## Recommended Development Order

1. Start with Phase 0 (Project Setup)
2. Implement Phase 1 (Lexer)
3. Implement Phase 2 (Parser & AST)
4. Implement Phase 3 (Name Resolution)
5. Implement Phase 4 (Type Checker)
6. Implement Phase 5 (Annotation Processor)
7. Implement Phase 6 (DI Graph)
8. Implement Phase 7 (Module Graph)
9. Implement Phase 8 (Route Table)
10. Implement Phase 9 (Lifecycle Pipeline)
11. Implement Phase 10 (IR & Codegen, Cranelift-first)
12. Implement Phase 11 (Error Reporting)
13. Implement Phase 12 (CLI)
14. Implement Phase 13 (Scaffolding & Blueprint)
15. Implement Phase 14 (Standard Library)
16. Implement Phase 15 (Testing Infrastructure)
17. Implement Phase 16 (Documentation)
18. Complete Phase 17 (Polish & Release)
