#![allow(dead_code)]

use std::fmt;
use std::path::PathBuf;

/// Source position attached to tokens and lexer errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub file: PathBuf,
    pub line: u32,
    pub col: u32,
}

/// Token kinds recognized by the ArwaLang lexer.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    // Keywords
    Module,
    Import,
    Export,
    Provide,
    Control,
    Constructor,
    Class,
    Interface,
    Struct,
    Enum,
    Fn,
    Return,
    Let,
    Const,
    If,
    Else,
    Match,
    For,
    While,
    Null,

    // Literals / identifiers
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    Ident(String),

    // Decorators
    Hash,
    LBracket,
    RBracket,

    // Delimiters / punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LAngle,
    RAngle,
    Comma,
    Colon,
    Semicolon,
    Dot,
    Arrow,    // =>
    FatArrow, // ->
    Equals,
    Slash,
    At,

    // Operators
    Plus,
    Minus,
    Star,
    BangEq,
    EqEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Bang,

    // Special
    Eof,
    Newline,
}

/// A lexical token with source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Module => write!(f, "module"),
            Self::Import => write!(f, "import"),
            Self::Export => write!(f, "export"),
            Self::Provide => write!(f, "provide"),
            Self::Control => write!(f, "control"),
            Self::Constructor => write!(f, "constructor"),
            Self::Class => write!(f, "class"),
            Self::Interface => write!(f, "interface"),
            Self::Struct => write!(f, "struct"),
            Self::Enum => write!(f, "enum"),
            Self::Fn => write!(f, "fn"),
            Self::Return => write!(f, "return"),
            Self::Let => write!(f, "let"),
            Self::Const => write!(f, "const"),
            Self::If => write!(f, "if"),
            Self::Else => write!(f, "else"),
            Self::Match => write!(f, "match"),
            Self::For => write!(f, "for"),
            Self::While => write!(f, "while"),
            Self::Null => write!(f, "null"),
            Self::IntLiteral(v) => write!(f, "{v}"),
            Self::FloatLiteral(v) => write!(f, "{v}"),
            Self::StringLiteral(v) => write!(f, "\"{v}\""),
            Self::BoolLiteral(v) => write!(f, "{v}"),
            Self::Ident(v) => write!(f, "{v}"),
            Self::Hash => write!(f, "#"),
            Self::LBracket => write!(f, "["),
            Self::RBracket => write!(f, "]"),
            Self::LBrace => write!(f, "{{"),
            Self::RBrace => write!(f, "}}"),
            Self::LParen => write!(f, "("),
            Self::RParen => write!(f, ")"),
            Self::LAngle => write!(f, "<"),
            Self::RAngle => write!(f, ">"),
            Self::Comma => write!(f, ","),
            Self::Colon => write!(f, ":"),
            Self::Semicolon => write!(f, ";"),
            Self::Dot => write!(f, "."),
            Self::Arrow => write!(f, "=>"),
            Self::FatArrow => write!(f, "->"),
            Self::Equals => write!(f, "="),
            Self::Slash => write!(f, "/"),
            Self::At => write!(f, "@"),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::BangEq => write!(f, "!="),
            Self::EqEq => write!(f, "=="),
            Self::Lt => write!(f, "<"),
            Self::Gt => write!(f, ">"),
            Self::LtEq => write!(f, "<="),
            Self::GtEq => write!(f, ">="),
            Self::And => write!(f, "&&"),
            Self::Or => write!(f, "||"),
            Self::Bang => write!(f, "!"),
            Self::Eof => write!(f, "<eof>"),
            Self::Newline => write!(f, "<newline>"),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}:{}", self.kind, self.span.line, self.span.col)
    }
}
