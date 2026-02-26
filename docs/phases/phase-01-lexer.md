# Phase 1 - Lexer

## Objective

Implement a hand-written lexer for `.rw` files with source position tracking, complete token coverage for v1 syntax, and structured lexer errors.

## Delivered Scope

### Token System (`src/lexer/token.rs`)

- Added `Span` with per-token file/line/column tracking.
- Added `TokenKind` enum covering:
  - keywords
  - literals (`IntLiteral`, `FloatLiteral`, `StringLiteral`, `BoolLiteral`)
  - identifiers (`Ident`)
  - decorators (`Hash`, `LBracket`, `RBracket`)
  - punctuation and delimiters
  - operators
  - special markers (`Newline`, `Eof`)
- Added `Token` struct carrying `TokenKind` + `Span`.
- Implemented `Display` for `TokenKind` and `Token`.

### Lexer Engine (`src/lexer/lexer.rs`)

- Implemented `Lexer` with:
  - source buffer
  - file path tracking
  - index, line, and column cursors
- Implemented `Lexer::new(source, file)`.
- Implemented `next_token()` for incremental lexing.
- Implemented `tokenize_all()` to recover from errors and continue lexing.

### Lexing Features

- Skips non-newline whitespace.
- Handles comments:
  - single-line comments (`// ...`)
  - block comments (`/* ... */`)
- Emits newline tokens (`Newline`) for line-aware downstream diagnostics.
- Parses string literals with escapes:
  - `\n`, `\t`, `\\`, `\"`
- Parses numbers:
  - integers (`i64`)
  - floats (`f64`) with decimal point
- Parses identifiers and keyword forms.
- Parses multi-char operators:
  - `==`, `!=`, `<=`, `>=`, `&&`, `||`, `=>`, `->`
- Parses decorator sequence as independent tokens (`#` then `[`), as required.

### Error Model

- Added `LexError` using `thiserror`:
  - `UnterminatedString { line, col }`
  - `UnexpectedChar { char, line, col }`
  - `InvalidEscape { escape, line, col }`
- Error recovery behavior:
  - `tokenize_all()` collects errors and continues scanning until EOF.

## Tests

Implemented 11 lexer unit tests covering:

- keyword recognition
- identifier recognition
- integer literals
- float literals
- string escapes
- operator and punctuation coverage
- single-line and block comments
- position tracking
- error cases (unterminated string, unexpected char)
- decorator sequence (`#[`)

## Validation Performed

Phase 1 passed with:

```bash
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Notes

- `true` and `false` lex to `BoolLiteral(true/false)` to align with literal representation.
- `Newline` token emission is intentional for better parser/error-reporting control.

## Next Phase

Implement Phase 2 (Parser & AST): define AST in `src/parser/ast.rs` and build recursive-descent parser with error recovery.
