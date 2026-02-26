#![allow(dead_code)]

use std::path::PathBuf;

/// Top-level AST for a parsed source file.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub items: Vec<TopLevelItem>,
}

/// Top-level declarations supported by ArwaLang.
#[derive(Clone, Debug, PartialEq)]
pub enum TopLevelItem {
    Module(ModuleDecl),
    Class(ClassDecl),
    Interface(InterfaceDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Import(ImportDecl),
}

/// File-level import declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportDecl {
    pub name: String,
    pub span: Span,
}

/// Module declaration and module-scoped metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct ModuleDecl {
    pub name: String,
    pub imports: Vec<String>,
    pub providers: Vec<ProviderBinding>,
    pub controllers: Vec<String>,
    pub exports: Vec<String>,
    pub span: Span,
}

/// Provider binding inside a module block.
#[derive(Clone, Debug, PartialEq)]
pub enum ProviderBinding {
    Simple(String),
    Aliased { token: String, impl_: String },
}

/// Class declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassDecl {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub implements: Vec<String>,
    pub constructor: Option<ConstructorDecl>,
    pub methods: Vec<MethodDecl>,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

/// Interface declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct InterfaceDecl {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub methods: Vec<MethodSignature>,
    pub span: Span,
}

/// Struct declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct StructDecl {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub fields: Vec<FieldDecl>,
    pub span: Span,
}

/// Enum declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumDecl {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub span: Span,
}

/// Enum variant.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub span: Span,
}

/// Class constructor declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstructorDecl {
    pub params: Vec<Param>,
    pub span: Span,
}

/// Method declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct MethodDecl {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub body: Block,
    pub span: Span,
}

/// Interface method signature.
#[derive(Clone, Debug, PartialEq)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    pub span: Span,
}

/// Class/struct field declaration.
#[derive(Clone, Debug, PartialEq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeExpr,
    pub optional: bool,
    pub span: Span,
}

/// Function or constructor parameter.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub ty: TypeExpr,
    pub is_private: bool,
    pub span: Span,
}

/// Attribute-like declaration metadata (`#[...]`).
#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    pub name: String,
    pub args: Vec<AnnotationArg>,
    pub span: Span,
}

/// Annotation arguments.
#[derive(Clone, Debug, PartialEq)]
pub enum AnnotationArg {
    Positional(Expr),
    Named { key: String, value: Expr },
}

/// Type expression syntax.
#[derive(Clone, Debug, PartialEq)]
pub enum TypeExpr {
    Named(String),
    Generic {
        name: String,
        params: Vec<TypeExpr>,
    },
    Result {
        ok: Box<TypeExpr>,
        err: Box<TypeExpr>,
    },
    Option(Box<TypeExpr>),
}

/// Expression syntax (v1 minimal set).
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    Null,
    Ident(String),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    FieldAccess {
        obj: Box<Expr>,
        field: String,
    },
    BinOp {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        expr: Box<Expr>,
    },
}

/// Binary operators.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Add,
    Sub,
    Mul,
    Div,
}

/// Unary operators.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// Statement syntax (v1 minimal set).
#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<TypeExpr>,
        value: Expr,
    },
    Return(Option<Expr>),
    Expr(Expr),
    If {
        cond: Expr,
        then: Block,
        else_: Option<Block>,
    },
}

/// Block of statements.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

/// Source span for AST and diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub file: PathBuf,
    pub line_start: u32,
    pub col_start: u32,
    pub line_end: u32,
    pub col_end: u32,
}
