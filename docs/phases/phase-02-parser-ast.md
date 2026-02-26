# Phase 2 - Parser & AST

## Objective

Implement a recursive-descent parser that consumes lexer tokens and produces a typed AST with structured parse errors and recovery behavior.

## Delivered Scope

### AST Definitions (`src/parser/ast.rs`)

- Added complete AST model for phase 2:
  - `SourceFile`, `TopLevelItem`, `ImportDecl`
  - `ModuleDecl`, `ProviderBinding`
  - `ClassDecl`, `InterfaceDecl`, `StructDecl`, `EnumDecl`, `EnumVariant`
  - `ConstructorDecl`, `MethodDecl`, `MethodSignature`, `FieldDecl`, `Param`
  - `Annotation`, `AnnotationArg`
  - `TypeExpr`
  - `Expr`, `BinOp`, `UnaryOp`
  - `Stmt`, `Block`
  - `Span`
- Added AST-level support for `private` constructor parameters via `Param.is_private`.

### Parser Core (`src/parser/parser.rs`)

- Added `Parser` with token cursor operations and helper methods.
- Added `ParseError` (`thiserror`) variants:
  - `UnexpectedToken`
  - `UnexpectedEof`
  - `InvalidSyntax`
- Implemented `parse_source_file()` with recoverable top-level parsing.

### Top-Level Parsing

- Implemented parsing for:
  - file-level `import`
  - `module`
  - `class`
  - `interface`
  - `struct`
  - `enum`
- Implemented module body parsing for:
  - `import`
  - `provide` (simple and aliased `=>`)
  - `control`
  - `export`

### Type, Annotation, and Member Parsing

- Implemented type parsing:
  - named types
  - generic types (`List<T>`, `Map<K,V>`, nested generics)
  - specialized `Result<T,E>` and `Option<T>` variants
- Implemented annotation parsing:
  - `#[name]`
  - positional args
  - named args (`key = value`)
  - multiple stacked annotations
- Implemented class member parsing:
  - constructor
  - methods
  - fields
  - `implements` clause

### Statement and Expression Parsing

- Implemented statement parsing:
  - `let`
  - `return`
  - `if/else`
  - expression statements
- Implemented expression parsing with precedence:
  - logical (`||`, `&&`)
  - equality and comparison
  - arithmetic (`+`, `-`, `*`, `/`)
  - unary (`!`, `-`)
  - calls and field access
  - literals, identifiers, parenthesized expressions

### Error Recovery

- Implemented top-level synchronization to continue parsing after invalid declarations.
- `parse_source_file()` returns `Result<SourceFile, Vec<ParseError>>` and preserves multiple errors.

## Tests

Added parser unit tests covering:

- simple module parsing
- class parsing with constructor/methods
- interface parsing
- struct parsing
- annotation argument forms
- generic type parsing
- nested expression precedence
- statement parsing
- recovery + multi-error collection
- `private` constructor param behavior

## Validation Performed

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

Result: all checks passing.

## Notes

- Parser currently uses strict function syntax for methods (`fn name(...): ReturnType { ... }`).
- Constructor body content is currently skipped syntactically (kept out of phase-2 semantic scope).

## Next Phase

Implement Phase 3 (Name Resolution): scoped symbol tables and type/symbol binding validation.
