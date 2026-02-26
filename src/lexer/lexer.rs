#![allow(dead_code)]

use std::path::PathBuf;

use thiserror::Error;

use crate::lexer::token::{Span, Token, TokenKind};

/// Lexing failures produced while scanning source text.
///
/// Purpose:
/// - capture recoverable tokenization problems with precise location
///
/// Why this is needed:
/// - parser/type phases depend on stable token stream boundaries
/// - diagnostics need exact line/column for actionable feedback
#[derive(Debug, Error, PartialEq)]
pub enum LexError {
    #[error("unterminated string literal at {line}:{col}")]
    UnterminatedString { line: u32, col: u32 },

    #[error("unexpected character '{char}' at {line}:{col}")]
    UnexpectedChar { char: char, line: u32, col: u32 },

    #[error("invalid escape sequence '\\{escape}' at {line}:{col}")]
    InvalidEscape { escape: char, line: u32, col: u32 },
}

/// Hand-written lexer that converts raw `.rw` text into tokens.
///
/// Purpose:
/// - provide deterministic tokenization for the parser
///
/// Why this is needed:
/// - ArwaLang grammar has custom symbols/annotations requiring explicit control
/// - compile-time guarantees start with correct lexical boundaries
#[derive(Debug)]
pub struct Lexer {
    source: Vec<char>,
    file: PathBuf,
    index: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    /// Creates a lexer from source text and source file path.
    ///
    /// Why this is needed:
    /// - file path is embedded into token spans for downstream diagnostics
    pub fn new(source: String, file: PathBuf) -> Self {
        Self {
            source: source.chars().collect(),
            file,
            index: 0,
            line: 1,
            col: 1,
        }
    }

    /// Produces the next token from input or a lex error.
    ///
    /// Purpose:
    /// - incremental scanner API for parser-driven consumption
    ///
    /// Why this is needed:
    /// - enables streaming tokenization and localized recovery behavior
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_non_newline_whitespace_and_comments()?;

        let span = self.current_span();
        let Some(ch) = self.peek() else {
            return Ok(Token {
                kind: TokenKind::Eof,
                span,
            });
        };

        if ch == '\n' {
            self.advance();
            return Ok(Token {
                kind: TokenKind::Newline,
                span,
            });
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            return Ok(Token {
                kind: self.lex_ident_or_keyword(),
                span,
            });
        }

        if ch.is_ascii_digit() {
            return Ok(Token {
                kind: self.lex_number(),
                span,
            });
        }

        if ch == '"' {
            let kind = self.lex_string()?;
            return Ok(Token { kind, span });
        }

        let token = match ch {
            '#' => {
                self.advance();
                TokenKind::Hash
            }
            '[' => {
                self.advance();
                TokenKind::LBracket
            }
            ']' => {
                self.advance();
                TokenKind::RBracket
            }
            '{' => {
                self.advance();
                TokenKind::LBrace
            }
            '}' => {
                self.advance();
                TokenKind::RBrace
            }
            '(' => {
                self.advance();
                TokenKind::LParen
            }
            ')' => {
                self.advance();
                TokenKind::RParen
            }
            ',' => {
                self.advance();
                TokenKind::Comma
            }
            ':' => {
                self.advance();
                TokenKind::Colon
            }
            ';' => {
                self.advance();
                TokenKind::Semicolon
            }
            '.' => {
                self.advance();
                TokenKind::Dot
            }
            '@' => {
                self.advance();
                TokenKind::At
            }
            '+' => {
                self.advance();
                TokenKind::Plus
            }
            '*' => {
                self.advance();
                TokenKind::Star
            }
            '/' => {
                self.advance();
                TokenKind::Slash
            }
            '=' => {
                self.advance();
                if self.consume_if('=') {
                    TokenKind::EqEq
                } else if self.consume_if('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Equals
                }
            }
            '-' => {
                self.advance();
                if self.consume_if('>') {
                    TokenKind::FatArrow
                } else {
                    TokenKind::Minus
                }
            }
            '!' => {
                self.advance();
                if self.consume_if('=') {
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            '&' => {
                self.advance();
                if self.consume_if('&') {
                    TokenKind::And
                } else {
                    return Err(LexError::UnexpectedChar {
                        char: '&',
                        line: span.line,
                        col: span.col,
                    });
                }
            }
            '|' => {
                self.advance();
                if self.consume_if('|') {
                    TokenKind::Or
                } else {
                    return Err(LexError::UnexpectedChar {
                        char: '|',
                        line: span.line,
                        col: span.col,
                    });
                }
            }
            '<' => {
                self.advance();
                if self.consume_if('=') {
                    TokenKind::LtEq
                } else {
                    TokenKind::Lt
                }
            }
            '>' => {
                self.advance();
                if self.consume_if('=') {
                    TokenKind::GtEq
                } else {
                    TokenKind::Gt
                }
            }
            _ => {
                self.advance();
                return Err(LexError::UnexpectedChar {
                    char: ch,
                    line: span.line,
                    col: span.col,
                });
            }
        };

        Ok(Token { kind: token, span })
    }

    /// Tokenizes the full file, collecting errors without stopping at first failure.
    ///
    /// Why this is needed:
    /// - improves developer feedback by reporting multiple lexical issues per run
    pub fn tokenize_all(&mut self) -> (Vec<Token>, Vec<LexError>) {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        loop {
            match self.next_token() {
                Ok(token) => {
                    let is_eof = token.kind == TokenKind::Eof;
                    tokens.push(token);
                    if is_eof {
                        break;
                    }
                }
                Err(err) => {
                    errors.push(err);
                    if self.peek().is_none() {
                        tokens.push(Token {
                            kind: TokenKind::Eof,
                            span: self.current_span(),
                        });
                        break;
                    }
                }
            }
        }

        (tokens, errors)
    }

    /// Skips spaces/tabs/comments while preserving newline tokens.
    ///
    /// Why this is needed:
    /// - newlines are significant for diagnostics and parser recovery points.
    fn skip_non_newline_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            while let Some(ch) = self.peek() {
                if ch == '\n' || !ch.is_whitespace() {
                    break;
                }
                self.advance();
            }

            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                self.advance();
                self.advance();
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue;
            }

            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                let start = self.current_span();
                self.advance();
                self.advance();

                let mut closed = false;
                while let Some(ch) = self.peek() {
                    if ch == '*' && self.peek_next() == Some('/') {
                        self.advance();
                        self.advance();
                        closed = true;
                        break;
                    }
                    self.advance();
                }

                if !closed {
                    return Err(LexError::UnexpectedChar {
                        char: '/',
                        line: start.line,
                        col: start.col,
                    });
                }
                continue;
            }

            break;
        }

        Ok(())
    }

    /// Lexes an identifier and upgrades it to a keyword/bool literal when applicable.
    ///
    /// Why this is needed:
    /// - parser expects keyword token kinds instead of raw identifier text.
    fn lex_ident_or_keyword(&mut self) -> TokenKind {
        let mut ident = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        match ident.as_str() {
            "module" => TokenKind::Module,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "provide" => TokenKind::Provide,
            "control" => TokenKind::Control,
            "constructor" => TokenKind::Constructor,
            "class" => TokenKind::Class,
            "interface" => TokenKind::Interface,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "match" => TokenKind::Match,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "true" => TokenKind::BoolLiteral(true),
            "false" => TokenKind::BoolLiteral(false),
            "null" => TokenKind::Null,
            _ => TokenKind::Ident(ident),
        }
    }

    /// Lexes decimal integer and float literals.
    ///
    /// Why this is needed:
    /// - numeric token boundaries must be established before parser precedence logic.
    fn lex_number(&mut self) -> TokenKind {
        let mut number = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                number.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if self.peek() == Some('.') && self.peek_next().is_some_and(|ch| ch.is_ascii_digit()) {
            number.push('.');
            self.advance();

            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    number.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }

            let value = number.parse::<f64>().unwrap_or(0.0);
            return TokenKind::FloatLiteral(value);
        }

        let value = number.parse::<i64>().unwrap_or(0);
        TokenKind::IntLiteral(value)
    }

    /// Lexes a double-quoted string with supported escape sequences.
    ///
    /// Why this is needed:
    /// - invalid or unterminated strings must be caught at lexical phase.
    fn lex_string(&mut self) -> Result<TokenKind, LexError> {
        let start = self.current_span();
        self.advance();

        let mut result = String::new();
        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.advance();
                    return Ok(TokenKind::StringLiteral(result));
                }
                '\\' => {
                    self.advance();
                    let Some(esc) = self.peek() else {
                        return Err(LexError::UnterminatedString {
                            line: start.line,
                            col: start.col,
                        });
                    };

                    let mapped = match esc {
                        'n' => '\n',
                        't' => '\t',
                        '\\' => '\\',
                        '"' => '"',
                        _ => {
                            self.advance();
                            return Err(LexError::InvalidEscape {
                                escape: esc,
                                line: self.line,
                                col: self.col.saturating_sub(1),
                            });
                        }
                    };

                    self.advance();
                    result.push(mapped);
                }
                '\n' => {
                    return Err(LexError::UnterminatedString {
                        line: start.line,
                        col: start.col,
                    });
                }
                _ => {
                    result.push(ch);
                    self.advance();
                }
            }
        }

        Err(LexError::UnterminatedString {
            line: start.line,
            col: start.col,
        })
    }

    /// Builds span at the current cursor position.
    ///
    /// Why this is needed:
    /// - every emitted token/error must carry source location metadata.
    fn current_span(&self) -> Span {
        Span {
            file: self.file.clone(),
            line: self.line,
            col: self.col,
        }
    }

    /// Returns current character without consuming it.
    fn peek(&self) -> Option<char> {
        self.source.get(self.index).copied()
    }

    /// Returns next character without consuming it.
    fn peek_next(&self) -> Option<char> {
        self.source.get(self.index + 1).copied()
    }

    /// Consumes current char if it matches `expected`.
    ///
    /// Why this is needed:
    /// - multi-char operator parsing (`==`, `!=`, `->`, `=>`, etc.) depends on lookahead.
    fn consume_if(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consumes one character and updates line/column counters.
    ///
    /// Why this is needed:
    /// - precise position tracking is foundational for all diagnostics.
    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.index += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{LexError, Lexer};
    use crate::lexer::token::{Token, TokenKind};

    fn lex(src: &str) -> (Vec<Token>, Vec<LexError>) {
        let mut lexer = Lexer::new(src.to_string(), PathBuf::from("test.rw"));
        lexer.tokenize_all()
    }

    fn kinds(src: &str) -> Vec<TokenKind> {
        let (tokens, errors) = lex(src);
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        tokens.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn lexes_keywords() {
        let src = "module import export provide control constructor class interface struct enum fn return let const if else match for while true false null";
        let got = kinds(src);
        assert_eq!(
            got,
            vec![
                TokenKind::Module,
                TokenKind::Import,
                TokenKind::Export,
                TokenKind::Provide,
                TokenKind::Control,
                TokenKind::Constructor,
                TokenKind::Class,
                TokenKind::Interface,
                TokenKind::Struct,
                TokenKind::Enum,
                TokenKind::Fn,
                TokenKind::Return,
                TokenKind::Let,
                TokenKind::Const,
                TokenKind::If,
                TokenKind::Else,
                TokenKind::Match,
                TokenKind::For,
                TokenKind::While,
                TokenKind::BoolLiteral(true),
                TokenKind::BoolLiteral(false),
                TokenKind::Null,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_identifiers() {
        let got = kinds("foo _bar Baz123");
        assert_eq!(
            got,
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::Ident("_bar".into()),
                TokenKind::Ident("Baz123".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_integer_literals() {
        let got = kinds("0 1 42");
        assert_eq!(
            got,
            vec![
                TokenKind::IntLiteral(0),
                TokenKind::IntLiteral(1),
                TokenKind::IntLiteral(42),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_float_literals() {
        let got = kinds("1.5 0.25 10.0");
        assert_eq!(
            got,
            vec![
                TokenKind::FloatLiteral(1.5),
                TokenKind::FloatLiteral(0.25),
                TokenKind::FloatLiteral(10.0),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_string_literals_with_escapes() {
        let got = kinds("\"line\\ncol\\tquote:\\\" slash:\\\\\" ");
        assert_eq!(
            got,
            vec![
                TokenKind::StringLiteral("line\ncol\tquote:\" slash:\\".into()),
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn lexes_operators_and_punctuation() {
        let got = kinds("+ - * / == != <= >= < > && || ! = => -> # [ ] { } ( ) , : ; . @");
        assert_eq!(
            got,
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::LtEq,
                TokenKind::GtEq,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::And,
                TokenKind::Or,
                TokenKind::Bang,
                TokenKind::Equals,
                TokenKind::Arrow,
                TokenKind::FatArrow,
                TokenKind::Hash,
                TokenKind::LBracket,
                TokenKind::RBracket,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Semicolon,
                TokenKind::Dot,
                TokenKind::At,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn skips_comments() {
        let got = kinds("let x = 1; // tail\n/* block */let y = 2;");
        assert_eq!(
            got,
            vec![
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Equals,
                TokenKind::IntLiteral(1),
                TokenKind::Semicolon,
                TokenKind::Newline,
                TokenKind::Let,
                TokenKind::Ident("y".into()),
                TokenKind::Equals,
                TokenKind::IntLiteral(2),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn tracks_positions() {
        let (tokens, errors) = lex("let\n  x");
        assert!(errors.is_empty());

        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.col, 1);
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].span.line, 2);
        assert_eq!(tokens[2].span.col, 3);
    }

    #[test]
    fn reports_unterminated_string() {
        let (_tokens, errors) = lex("\"oops");
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], LexError::UnterminatedString { .. }));
    }

    #[test]
    fn reports_unexpected_char() {
        let (_tokens, errors) = lex("^");
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            LexError::UnexpectedChar { char: '^', .. }
        ));
    }

    #[test]
    fn lexes_decorator_hash_bracket_sequence() {
        let got = kinds("#[injectable]");
        assert_eq!(
            got,
            vec![
                TokenKind::Hash,
                TokenKind::LBracket,
                TokenKind::Ident("injectable".into()),
                TokenKind::RBracket,
                TokenKind::Eof,
            ]
        );
    }
}
