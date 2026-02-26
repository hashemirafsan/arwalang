#![allow(dead_code)]

use std::mem;
use std::path::PathBuf;

use thiserror::Error;

use crate::lexer::token::{Token, TokenKind};
use crate::parser::ast::{
    Annotation, AnnotationArg, BinOp, Block, ClassDecl, ConstructorDecl, EnumDecl, EnumVariant,
    Expr, FieldDecl, ImportDecl, InterfaceDecl, MethodDecl, MethodSignature, ModuleDecl, Param,
    ProviderBinding, SourceFile, Span, Stmt, StructDecl, TopLevelItem, TypeExpr, UnaryOp,
};

/// Structured parse failures emitted while building AST from tokens.
///
/// Purpose:
/// - represent grammar violations with precise context
///
/// Why this is needed:
/// - parser must recover and continue to report multiple issues
/// - later error reporting phase relies on rich, typed parse errors
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ParseError {
    #[error("unexpected token: expected {expected}, found {found}")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("unexpected end of file: expected {expected}")]
    UnexpectedEof { expected: String, span: Span },

    #[error("invalid syntax: {message}")]
    InvalidSyntax { message: String, span: Span },
}

/// Recursive-descent parser for ArwaLang grammar.
///
/// Purpose:
/// - convert token stream into strongly-typed AST nodes
///
/// Why this is needed:
/// - all semantic phases (resolver/typechecker/graphs) depend on AST correctness
#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    file: PathBuf,
}

impl Parser {
    /// Creates parser from pre-tokenized input.
    ///
    /// Why this is needed:
    /// - parser runs after lexer and reuses token spans for syntax diagnostics
    pub fn new(tokens: Vec<Token>) -> Self {
        let file = tokens
            .first()
            .map(|t| t.span.file.clone())
            .unwrap_or_else(|| PathBuf::from("<unknown>"));
        Self {
            tokens,
            current: 0,
            file,
        }
    }

    /// Parses full source file and accumulates recoverable parse errors.
    ///
    /// Purpose:
    /// - produce one AST root or a list of syntax problems
    ///
    /// Why this is needed:
    /// - fail-fast behavior is insufficient; users need full syntax feedback per run
    pub fn parse_source_file(&mut self) -> Result<SourceFile, Vec<ParseError>> {
        let mut items = Vec::new();
        let mut errors = Vec::new();

        self.skip_newlines();
        while !self.is_at_end() {
            match self.parse_top_level_item() {
                Ok(item) => items.push(item),
                Err(err) => {
                    errors.push(err);
                    self.synchronize_top_level();
                }
            }
            self.skip_newlines();
        }

        let source = SourceFile {
            path: self.file.clone(),
            items,
        };

        if errors.is_empty() {
            Ok(source)
        } else {
            Err(errors)
        }
    }

    /// Parses one top-level declaration item.
    ///
    /// Why this is needed:
    /// - top-level dispatch drives module/class/interface/struct/enum/import parsing paths
    pub fn parse_top_level_item(&mut self) -> Result<TopLevelItem, ParseError> {
        let annotations = self.parse_annotations()?;

        if self.check_kind(&TokenKind::Import) {
            if !annotations.is_empty() {
                return Err(ParseError::InvalidSyntax {
                    message: "annotations are not allowed on file-level import".into(),
                    span: self.current_span(),
                });
            }
            return Ok(TopLevelItem::Import(self.parse_import()?));
        }

        if self.check_kind(&TokenKind::Module) {
            if !annotations.is_empty() {
                return Err(ParseError::InvalidSyntax {
                    message: "annotations are not allowed on module declarations".into(),
                    span: self.current_span(),
                });
            }
            return Ok(TopLevelItem::Module(self.parse_module()?));
        }

        if self.check_kind(&TokenKind::Class) {
            return Ok(TopLevelItem::Class(self.parse_class(annotations)?));
        }
        if self.check_kind(&TokenKind::Interface) {
            return Ok(TopLevelItem::Interface(self.parse_interface(annotations)?));
        }
        if self.check_kind(&TokenKind::Struct) {
            return Ok(TopLevelItem::Struct(self.parse_struct(annotations)?));
        }
        if self.check_kind(&TokenKind::Enum) {
            return Ok(TopLevelItem::Enum(self.parse_enum(annotations)?));
        }

        Err(ParseError::InvalidSyntax {
            message: "expected top-level declaration".into(),
            span: self.current_span(),
        })
    }

    /// Parses a file-level import declaration.
    ///
    /// Why this is needed:
    /// - imports are required for module/type visibility in later phases.
    fn parse_import(&mut self) -> Result<ImportDecl, ParseError> {
        let start = self.expect_kind(&TokenKind::Import, "import")?.span;
        let name = self.expect_ident("import target")?;
        self.consume_semicolon_or_newline();

        Ok(ImportDecl {
            name,
            span: self.span_from(&start),
        })
    }

    /// Parses a `module { ... }` declaration body.
    ///
    /// Why this is needed:
    /// - DI/module graph phases depend on normalized module metadata.
    fn parse_module(&mut self) -> Result<ModuleDecl, ParseError> {
        let start = self.expect_kind(&TokenKind::Module, "module")?.span;
        let name = self.expect_ident("module name")?;
        self.expect_kind(&TokenKind::LBrace, "{")?;

        let mut imports = Vec::new();
        let mut providers = Vec::new();
        let mut controllers = Vec::new();
        let mut exports = Vec::new();

        while !self.check_kind(&TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&TokenKind::RBrace) {
                break;
            }

            if self.match_kind(&TokenKind::Import) {
                imports.push(self.expect_ident("import target")?);
                self.consume_semicolon_or_newline();
                continue;
            }

            if self.match_kind(&TokenKind::Provide) {
                let token = self.expect_ident("provider token")?;
                if self.match_kind(&TokenKind::Arrow) {
                    let impl_ = self.expect_ident("provider implementation")?;
                    providers.push(ProviderBinding::Aliased { token, impl_ });
                } else {
                    providers.push(ProviderBinding::Simple(token));
                }
                self.consume_semicolon_or_newline();
                continue;
            }

            if self.match_kind(&TokenKind::Control) {
                controllers.push(self.expect_ident("controller name")?);
                self.consume_semicolon_or_newline();
                continue;
            }

            if self.match_kind(&TokenKind::Export) {
                exports.push(self.expect_ident("export symbol")?);
                self.consume_semicolon_or_newline();
                continue;
            }

            return Err(ParseError::InvalidSyntax {
                message: "expected one of import/provide/control/export inside module".into(),
                span: self.current_span(),
            });
        }

        self.expect_kind(&TokenKind::RBrace, "}")?;

        Ok(ModuleDecl {
            name,
            imports,
            providers,
            controllers,
            exports,
            span: self.span_from(&start),
        })
    }

    /// Parses a class declaration including members and optional constructor.
    ///
    /// Why this is needed:
    /// - class definitions provide providers/controllers and method surfaces.
    fn parse_class(&mut self, annotations: Vec<Annotation>) -> Result<ClassDecl, ParseError> {
        let start = self.expect_kind(&TokenKind::Class, "class")?.span;
        let name = self.expect_ident("class name")?;

        let mut implements = Vec::new();
        if self.match_ident_text("implements") {
            loop {
                implements.push(self.expect_ident("interface name")?);
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.expect_kind(&TokenKind::LBrace, "{")?;

        let mut constructor = None;
        let mut methods = Vec::new();
        let mut fields = Vec::new();

        while !self.check_kind(&TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&TokenKind::RBrace) {
                break;
            }

            let member_annotations = self.parse_annotations()?;

            if self.check_kind(&TokenKind::Constructor) {
                if constructor.is_some() {
                    return Err(ParseError::InvalidSyntax {
                        message: "class cannot declare multiple constructors".into(),
                        span: self.current_span(),
                    });
                }
                constructor = Some(self.parse_constructor(member_annotations)?);
                continue;
            }

            if self.check_kind(&TokenKind::Fn) {
                methods.push(self.parse_method(member_annotations)?);
                continue;
            }

            if !member_annotations.is_empty() {
                return Err(ParseError::InvalidSyntax {
                    message: "annotations in class body must target constructor or method".into(),
                    span: self.current_span(),
                });
            }

            fields.push(self.parse_field()?);
        }

        self.expect_kind(&TokenKind::RBrace, "}")?;

        Ok(ClassDecl {
            annotations,
            name,
            implements,
            constructor,
            methods,
            fields,
            span: self.span_from(&start),
        })
    }

    /// Parses an interface declaration and its method signatures.
    ///
    /// Why this is needed:
    /// - lifecycle and DI validations rely on interface contracts.
    fn parse_interface(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<InterfaceDecl, ParseError> {
        let start = self.expect_kind(&TokenKind::Interface, "interface")?.span;
        let name = self.expect_ident("interface name")?;
        self.expect_kind(&TokenKind::LBrace, "{")?;

        let mut methods = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&TokenKind::RBrace) {
                break;
            }
            methods.push(self.parse_method_signature()?);
        }
        self.expect_kind(&TokenKind::RBrace, "}")?;

        Ok(InterfaceDecl {
            annotations,
            name,
            methods,
            span: self.span_from(&start),
        })
    }

    /// Parses a struct declaration.
    ///
    /// Why this is needed:
    /// - struct fields participate in DTO/serializability type checks.
    fn parse_struct(&mut self, annotations: Vec<Annotation>) -> Result<StructDecl, ParseError> {
        let start = self.expect_kind(&TokenKind::Struct, "struct")?.span;
        let name = self.expect_ident("struct name")?;
        self.expect_kind(&TokenKind::LBrace, "{")?;

        let mut fields = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&TokenKind::RBrace) {
                break;
            }
            fields.push(self.parse_field()?);
        }

        self.expect_kind(&TokenKind::RBrace, "}")?;

        Ok(StructDecl {
            annotations,
            name,
            fields,
            span: self.span_from(&start),
        })
    }

    /// Parses an enum declaration with variant list.
    ///
    /// Why this is needed:
    /// - enums are first-class type declarations referenced by resolver/checker.
    fn parse_enum(&mut self, annotations: Vec<Annotation>) -> Result<EnumDecl, ParseError> {
        let start = self.expect_kind(&TokenKind::Enum, "enum")?.span;
        let name = self.expect_ident("enum name")?;
        self.expect_kind(&TokenKind::LBrace, "{")?;

        let mut variants = Vec::new();
        while !self.check_kind(&TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&TokenKind::RBrace) {
                break;
            }
            let span_start = self.peek().span.clone();
            let variant_name = self.expect_ident("enum variant")?;
            variants.push(EnumVariant {
                name: variant_name,
                span: self.span_from(&span_start),
            });
            if !self.match_kind(&TokenKind::Comma) {
                self.consume_semicolon_or_newline();
            }
        }

        self.expect_kind(&TokenKind::RBrace, "}")?;

        Ok(EnumDecl {
            annotations,
            name,
            variants,
            span: self.span_from(&start),
        })
    }

    /// Parses class constructor parameters and optional body.
    ///
    /// Why this is needed:
    /// - constructor params drive DI dependency edges in later phases.
    fn parse_constructor(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<ConstructorDecl, ParseError> {
        if !annotations.is_empty() {
            return Err(ParseError::InvalidSyntax {
                message: "constructor annotations are not supported in v1 parser".into(),
                span: self.current_span(),
            });
        }

        let start = self
            .expect_kind(&TokenKind::Constructor, "constructor")?
            .span;
        let params = self.parse_params(true)?;

        if self.match_kind(&TokenKind::LBrace) {
            self.skip_block_contents()?;
        } else {
            self.consume_semicolon_or_newline();
        }

        Ok(ConstructorDecl {
            params,
            span: self.span_from(&start),
        })
    }

    /// Parses a class method declaration.
    ///
    /// Why this is needed:
    /// - method signatures and bodies are central to type/lifecycle validation.
    fn parse_method(&mut self, annotations: Vec<Annotation>) -> Result<MethodDecl, ParseError> {
        let start = self.expect_kind(&TokenKind::Fn, "fn")?.span;
        let name = self.expect_ident("method name")?;
        let params = self.parse_params(false)?;

        self.expect_kind(&TokenKind::Colon, ":")?;
        let return_type = self.parse_type_expr()?;
        let body = self.parse_block()?;

        Ok(MethodDecl {
            annotations,
            name,
            params,
            return_type,
            body,
            span: self.span_from(&start),
        })
    }

    /// Parses an interface method signature (no executable body).
    fn parse_method_signature(&mut self) -> Result<MethodSignature, ParseError> {
        let start = self.expect_kind(&TokenKind::Fn, "fn")?.span;
        let name = self.expect_ident("method name")?;
        let params = self.parse_params(false)?;
        self.expect_kind(&TokenKind::Colon, ":")?;
        let return_type = self.parse_type_expr()?;
        self.consume_semicolon_or_newline();

        Ok(MethodSignature {
            name,
            params,
            return_type,
            span: self.span_from(&start),
        })
    }

    /// Parses a field declaration (`name: Type`).
    fn parse_field(&mut self) -> Result<FieldDecl, ParseError> {
        let start = self.peek().span.clone();
        let name = self.expect_ident("field name")?;
        self.expect_kind(&TokenKind::Colon, ":")?;
        let ty = self.parse_type_expr()?;
        self.consume_semicolon_or_newline();

        Ok(FieldDecl {
            name,
            ty,
            optional: false,
            span: self.span_from(&start),
        })
    }

    /// Parses parameter list for methods/constructors.
    ///
    /// Why this is needed:
    /// - preserves annotation/type/private metadata used by semantic phases.
    fn parse_params(&mut self, allow_private: bool) -> Result<Vec<Param>, ParseError> {
        self.expect_kind(&TokenKind::LParen, "(")?;
        let mut params = Vec::new();

        self.skip_newlines();
        if self.match_kind(&TokenKind::RParen) {
            return Ok(params);
        }

        loop {
            let start = self.peek().span.clone();
            let annotations = self.parse_annotations()?;

            let is_private = if allow_private {
                self.match_ident_text("private")
            } else {
                false
            };

            let name = self.expect_ident("parameter name")?;
            self.expect_kind(&TokenKind::Colon, ":")?;
            let ty = self.parse_type_expr()?;

            params.push(Param {
                annotations,
                name,
                ty,
                is_private,
                span: self.span_from(&start),
            });

            self.skip_newlines();
            if self.match_kind(&TokenKind::Comma) {
                self.skip_newlines();
                continue;
            }

            self.expect_kind(&TokenKind::RParen, ")")?;
            break;
        }

        Ok(params)
    }

    /// Parses stacked `#[...]` annotations and argument lists.
    ///
    /// Why this is needed:
    /// - annotation processor and route/lifecycle builders require exact annotation AST.
    fn parse_annotations(&mut self) -> Result<Vec<Annotation>, ParseError> {
        let mut annotations = Vec::new();

        loop {
            self.skip_newlines();
            if !self.match_kind(&TokenKind::Hash) {
                break;
            }

            let start = self.peek().span.clone();
            self.expect_kind(&TokenKind::LBracket, "[")?;
            let name = self.expect_ident("annotation name")?;

            let mut args = Vec::new();
            if self.match_kind(&TokenKind::LParen) {
                self.skip_newlines();
                if !self.check_kind(&TokenKind::RParen) {
                    loop {
                        if let Some(key) = self.peek_ident_text() {
                            if self.peek_next_is(&TokenKind::Equals) {
                                self.advance();
                                self.advance();
                                let value = self.parse_expr()?;
                                args.push(AnnotationArg::Named { key, value });
                            } else {
                                args.push(AnnotationArg::Positional(self.parse_expr()?));
                            }
                        } else {
                            args.push(AnnotationArg::Positional(self.parse_expr()?));
                        }

                        self.skip_newlines();
                        if self.match_kind(&TokenKind::Comma) {
                            self.skip_newlines();
                            continue;
                        }
                        break;
                    }
                }
                self.expect_kind(&TokenKind::RParen, ")")?;
            }

            self.expect_kind(&TokenKind::RBracket, "]")?;
            annotations.push(Annotation {
                name,
                args,
                span: self.span_from(&start),
            });
        }

        Ok(annotations)
    }

    /// Parses type expressions including generic, Result, and Option forms.
    ///
    /// Why this is needed:
    /// - resolver/typechecker depend on normalized type expression trees.
    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        let name = self.expect_ident("type name")?;
        if self.match_kind(&TokenKind::Lt) {
            if name == "Result" {
                let ok = self.parse_type_expr()?;
                self.expect_kind(&TokenKind::Comma, ",")?;
                let err = self.parse_type_expr()?;
                self.expect_kind(&TokenKind::Gt, ">")?;
                return Ok(TypeExpr::Result {
                    ok: Box::new(ok),
                    err: Box::new(err),
                });
            }

            if name == "Option" {
                let inner = self.parse_type_expr()?;
                self.expect_kind(&TokenKind::Gt, ">")?;
                return Ok(TypeExpr::Option(Box::new(inner)));
            }

            let mut params = Vec::new();
            loop {
                params.push(self.parse_type_expr()?);
                if self.match_kind(&TokenKind::Comma) {
                    continue;
                }
                break;
            }
            self.expect_kind(&TokenKind::Gt, ">")?;
            return Ok(TypeExpr::Generic { name, params });
        }

        Ok(TypeExpr::Named(name))
    }

    /// Parses a brace-delimited statement block.
    fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.expect_kind(&TokenKind::LBrace, "{")?;
        let mut stmts = Vec::new();

        while !self.check_kind(&TokenKind::RBrace) && !self.is_at_end() {
            self.skip_newlines();
            if self.check_kind(&TokenKind::RBrace) {
                break;
            }
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }

        self.expect_kind(&TokenKind::RBrace, "}")?;
        Ok(Block { stmts })
    }

    /// Parses one statement by leading token dispatch.
    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.check_kind(&TokenKind::Let) {
            return self.parse_let_stmt();
        }
        if self.check_kind(&TokenKind::Return) {
            return self.parse_return_stmt();
        }
        if self.check_kind(&TokenKind::If) {
            return self.parse_if_stmt();
        }

        let expr = self.parse_expr()?;
        self.consume_semicolon_or_newline();
        Ok(Stmt::Expr(expr))
    }

    /// Parses `let` bindings with optional type annotation.
    fn parse_let_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect_kind(&TokenKind::Let, "let")?;
        let name = self.expect_ident("variable name")?;

        let ty = if self.match_kind(&TokenKind::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.expect_kind(&TokenKind::Equals, "=")?;
        let value = self.parse_expr()?;
        self.consume_semicolon_or_newline();
        Ok(Stmt::Let { name, ty, value })
    }

    /// Parses `return` statements with optional expression.
    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect_kind(&TokenKind::Return, "return")?;
        if self.check_kind(&TokenKind::Semicolon)
            || self.check_kind(&TokenKind::Newline)
            || self.check_kind(&TokenKind::RBrace)
        {
            self.consume_semicolon_or_newline();
            return Ok(Stmt::Return(None));
        }

        let expr = self.parse_expr()?;
        self.consume_semicolon_or_newline();
        Ok(Stmt::Return(Some(expr)))
    }

    /// Parses `if` / `else` control-flow statements.
    fn parse_if_stmt(&mut self) -> Result<Stmt, ParseError> {
        self.expect_kind(&TokenKind::If, "if")?;
        let cond = self.parse_expr()?;
        let then = self.parse_block()?;
        let else_ = if self.match_kind(&TokenKind::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Stmt::If { cond, then, else_ })
    }

    /// Entry point for expression parsing (lowest precedence layer).
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    /// Parses logical OR expressions.
    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_and()?;
        while self.match_kind(&TokenKind::Or) {
            let rhs = self.parse_and()?;
            expr = Expr::BinOp {
                op: BinOp::Or,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    /// Parses logical AND expressions.
    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_equality()?;
        while self.match_kind(&TokenKind::And) {
            let rhs = self.parse_equality()?;
            expr = Expr::BinOp {
                op: BinOp::And,
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
            };
        }
        Ok(expr)
    }

    /// Parses equality expressions (`==`, `!=`).
    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;
        loop {
            if self.match_kind(&TokenKind::EqEq) {
                let rhs = self.parse_comparison()?;
                expr = Expr::BinOp {
                    op: BinOp::EqEq,
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                };
                continue;
            }
            if self.match_kind(&TokenKind::BangEq) {
                let rhs = self.parse_comparison()?;
                expr = Expr::BinOp {
                    op: BinOp::BangEq,
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    /// Parses comparison expressions (`<`, `>`, `<=`, `>=`).
    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;
        loop {
            let op = if self.match_kind(&TokenKind::Lt) {
                Some(BinOp::Lt)
            } else if self.match_kind(&TokenKind::Gt) {
                Some(BinOp::Gt)
            } else if self.match_kind(&TokenKind::LtEq) {
                Some(BinOp::LtEq)
            } else if self.match_kind(&TokenKind::GtEq) {
                Some(BinOp::GtEq)
            } else {
                None
            };

            if let Some(op) = op {
                let rhs = self.parse_term()?;
                expr = Expr::BinOp {
                    op,
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Parses additive expressions (`+`, `-`).
    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;
        loop {
            if self.match_kind(&TokenKind::Plus) {
                let rhs = self.parse_factor()?;
                expr = Expr::BinOp {
                    op: BinOp::Add,
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                };
                continue;
            }
            if self.match_kind(&TokenKind::Minus) {
                let rhs = self.parse_factor()?;
                expr = Expr::BinOp {
                    op: BinOp::Sub,
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    /// Parses multiplicative expressions (`*`, `/`).
    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;
        loop {
            if self.match_kind(&TokenKind::Star) {
                let rhs = self.parse_unary()?;
                expr = Expr::BinOp {
                    op: BinOp::Mul,
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                };
                continue;
            }
            if self.match_kind(&TokenKind::Slash) {
                let rhs = self.parse_unary()?;
                expr = Expr::BinOp {
                    op: BinOp::Div,
                    lhs: Box::new(expr),
                    rhs: Box::new(rhs),
                };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    /// Parses unary prefix expressions (`!`, `-`).
    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_kind(&TokenKind::Bang) {
            return Ok(Expr::UnaryOp {
                op: UnaryOp::Not,
                expr: Box::new(self.parse_unary()?),
            });
        }
        if self.match_kind(&TokenKind::Minus) {
            return Ok(Expr::UnaryOp {
                op: UnaryOp::Neg,
                expr: Box::new(self.parse_unary()?),
            });
        }

        self.parse_postfix()
    }

    /// Parses postfix call/field-access chains.
    ///
    /// Why this is needed:
    /// - call and member access must bind tighter than binary operators.
    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_kind(&TokenKind::LParen) {
                let mut args = Vec::new();
                self.skip_newlines();
                if !self.check_kind(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        self.skip_newlines();
                        if self.match_kind(&TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                self.expect_kind(&TokenKind::RParen, ")")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
                continue;
            }

            if self.match_kind(&TokenKind::Dot) {
                let field = self.expect_ident("field name")?;
                expr = Expr::FieldAccess {
                    obj: Box::new(expr),
                    field,
                };
                continue;
            }

            break;
        }

        Ok(expr)
    }

    /// Parses primary expressions (literals, identifiers, parenthesized forms).
    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::IntLiteral(v) => {
                self.advance();
                Ok(Expr::IntLit(v))
            }
            TokenKind::FloatLiteral(v) => {
                self.advance();
                Ok(Expr::FloatLit(v))
            }
            TokenKind::StringLiteral(v) => {
                self.advance();
                Ok(Expr::StringLit(v))
            }
            TokenKind::BoolLiteral(v) => {
                self.advance();
                Ok(Expr::BoolLit(v))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            TokenKind::Ident(v) => {
                self.advance();
                Ok(Expr::Ident(v))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_kind(&TokenKind::RParen, ")")?;
                Ok(expr)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression".into(),
                found: token.kind.to_string(),
                span: self.current_span(),
            }),
        }
    }

    /// Skips over constructor body tokens while preserving brace balance.
    ///
    /// Why this is needed:
    /// - parser keeps constructor body out of current semantic scope but must stay synchronized.
    fn skip_block_contents(&mut self) -> Result<(), ParseError> {
        let mut depth: i32 = 1;
        while depth > 0 {
            if self.is_at_end() {
                return Err(ParseError::UnexpectedEof {
                    expected: "}".into(),
                    span: self.current_span(),
                });
            }

            if self.match_kind(&TokenKind::LBrace) {
                depth += 1;
                continue;
            }
            if self.match_kind(&TokenKind::RBrace) {
                depth -= 1;
                continue;
            }

            self.advance();
        }

        Ok(())
    }

    /// Recovers parser state to the next plausible top-level boundary.
    ///
    /// Why this is needed:
    /// - allows collecting multiple syntax errors in one parse run.
    fn synchronize_top_level(&mut self) {
        while !self.is_at_end() {
            if self.match_kind(&TokenKind::Semicolon) || self.match_kind(&TokenKind::Newline) {
                return;
            }

            match &self.peek().kind {
                TokenKind::Import
                | TokenKind::Module
                | TokenKind::Class
                | TokenKind::Interface
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Hash => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Consumes optional statement terminator and trailing newlines.
    fn consume_semicolon_or_newline(&mut self) {
        if self.match_kind(&TokenKind::Semicolon) {
            self.skip_newlines();
            return;
        }
        if self.match_kind(&TokenKind::Newline) {
            self.skip_newlines();
            return;
        }
        self.skip_newlines();
    }

    /// Consumes contiguous newline tokens.
    fn skip_newlines(&mut self) {
        while self.match_kind(&TokenKind::Newline) {}
    }

    /// Expects an identifier token and returns its text.
    fn expect_ident(&mut self, what: &str) -> Result<String, ParseError> {
        let token = self.peek().clone();
        if let TokenKind::Ident(value) = token.kind {
            self.advance();
            Ok(value)
        } else if matches!(token.kind, TokenKind::Eof) {
            Err(ParseError::UnexpectedEof {
                expected: what.into(),
                span: self.current_span(),
            })
        } else {
            Err(ParseError::UnexpectedToken {
                expected: what.into(),
                found: token.kind.to_string(),
                span: self.current_span(),
            })
        }
    }

    /// Expects a token of specific kind.
    fn expect_kind(&mut self, kind: &TokenKind, expected: &str) -> Result<Token, ParseError> {
        let token = self.peek().clone();
        if self.check_kind(kind) {
            self.advance();
            Ok(token)
        } else if matches!(token.kind, TokenKind::Eof) {
            Err(ParseError::UnexpectedEof {
                expected: expected.into(),
                span: self.current_span(),
            })
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.into(),
                found: token.kind.to_string(),
                span: self.current_span(),
            })
        }
    }

    /// Checks current token kind without consuming.
    fn check_kind(&self, kind: &TokenKind) -> bool {
        mem::discriminant(&self.peek().kind) == mem::discriminant(kind)
    }

    /// Matches and consumes current token if kind matches.
    fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check_kind(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Matches identifier token by exact text.
    fn match_ident_text(&mut self, expected: &str) -> bool {
        if let TokenKind::Ident(value) = &self.peek().kind {
            if value == expected {
                self.advance();
                return true;
            }
        }
        false
    }

    /// Returns current identifier text without consuming.
    fn peek_ident_text(&self) -> Option<String> {
        if let TokenKind::Ident(value) = &self.peek().kind {
            Some(value.clone())
        } else {
            None
        }
    }

    /// Checks next token kind (one-token lookahead).
    fn peek_next_is(&self, kind: &TokenKind) -> bool {
        self.tokens
            .get(self.current + 1)
            .is_some_and(|t| mem::discriminant(&t.kind) == mem::discriminant(kind))
    }

    /// Returns current token reference.
    fn peek(&self) -> &Token {
        let fallback = self.tokens.len().saturating_sub(1);
        &self.tokens[self.current.min(fallback)]
    }

    /// Checks whether parser cursor is at EOF token.
    fn is_at_end(&self) -> bool {
        self.check_kind(&TokenKind::Eof)
    }

    /// Advances cursor to next token and returns current token reference.
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.peek()
    }

    /// Builds AST span from start token span to current parser position.
    fn span_from(&self, start: &crate::lexer::token::Span) -> Span {
        let end = self.peek().span.clone();
        Span {
            file: start.file.clone(),
            line_start: start.line,
            col_start: start.col,
            line_end: end.line,
            col_end: end.col,
        }
    }

    /// Builds single-point span at current parser token.
    fn current_span(&self) -> Span {
        let token = self.peek();
        Span {
            file: token.span.file.clone(),
            line_start: token.span.line,
            col_start: token.span.col,
            line_end: token.span.line,
            col_end: token.span.col,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::lexer::lexer::Lexer;
    use crate::parser::ast::{AnnotationArg, BinOp, Expr, Stmt, TopLevelItem, TypeExpr};

    use super::Parser;

    fn parse(src: &str) -> Result<crate::parser::ast::SourceFile, Vec<super::ParseError>> {
        let mut lexer = Lexer::new(src.to_string(), PathBuf::from("test.rw"));
        let (tokens, lex_errors) = lexer.tokenize_all();
        assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
        Parser::new(tokens).parse_source_file()
    }

    #[test]
    fn parses_simple_module() {
        let src = "module App { import Core provide UserService control UserController export UserService }";
        let ast = parse(src).expect("module should parse");
        assert_eq!(ast.items.len(), 1);
        match &ast.items[0] {
            TopLevelItem::Module(m) => {
                assert_eq!(m.name, "App");
                assert_eq!(m.imports, vec!["Core"]);
                assert_eq!(m.controllers, vec!["UserController"]);
                assert_eq!(m.exports, vec!["UserService"]);
            }
            _ => panic!("expected module"),
        }
    }

    #[test]
    fn parses_class_with_constructor_and_methods() {
        let src = "class UserService implements Service { constructor(private repo: UserRepo, cfg: Config) {} fn getUser(id: Int): Result<User, HttpError> { return repo.find(id); } }";
        let ast = parse(src).expect("class should parse");
        match &ast.items[0] {
            TopLevelItem::Class(c) => {
                assert_eq!(c.name, "UserService");
                assert_eq!(c.implements, vec!["Service"]);
                assert!(c.constructor.is_some());
                let ctor = c.constructor.as_ref().expect("constructor should exist");
                assert_eq!(ctor.params.len(), 2);
                assert!(ctor.params[0].is_private);
                assert_eq!(c.methods.len(), 1);
                assert_eq!(c.methods[0].name, "getUser");
            }
            _ => panic!("expected class"),
        }
    }

    #[test]
    fn parses_interface() {
        let src =
            "interface Guard { fn canActivate(ctx: RequestContext): Result<Bool, HttpError>; }";
        let ast = parse(src).expect("interface should parse");
        match &ast.items[0] {
            TopLevelItem::Interface(i) => {
                assert_eq!(i.name, "Guard");
                assert_eq!(i.methods.len(), 1);
            }
            _ => panic!("expected interface"),
        }
    }

    #[test]
    fn parses_struct() {
        let src = "struct UserDto { id: Int name: String }";
        let ast = parse(src).expect("struct should parse");
        match &ast.items[0] {
            TopLevelItem::Struct(s) => {
                assert_eq!(s.name, "UserDto");
                assert_eq!(s.fields.len(), 2);
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn parses_annotations_with_positional_and_named_args() {
        let src = "#[controller(\"/users\", scope = \"api\")] class UserController { fn list(): Result<List<User>, HttpError> { return users.all(); } }";
        let ast = parse(src).expect("annotations should parse");
        match &ast.items[0] {
            TopLevelItem::Class(c) => {
                assert_eq!(c.annotations.len(), 1);
                assert_eq!(c.annotations[0].name, "controller");
                assert_eq!(c.annotations[0].args.len(), 2);
                assert!(matches!(
                    c.annotations[0].args[0],
                    AnnotationArg::Positional(Expr::StringLit(_))
                ));
                assert!(matches!(
                    c.annotations[0].args[1],
                    AnnotationArg::Named { .. }
                ));
            }
            _ => panic!("expected class"),
        }
    }

    #[test]
    fn parses_generic_types() {
        let src =
            "class X { fn run(): Result<Map<String, List<Int>>, HttpError> { return data; } }";
        let ast = parse(src).expect("generic types should parse");
        let TopLevelItem::Class(c) = &ast.items[0] else {
            panic!("expected class");
        };
        match &c.methods[0].return_type {
            TypeExpr::Result { ok, err } => {
                assert!(matches!(**err, TypeExpr::Named(ref e) if e == "HttpError"));
                assert!(matches!(**ok, TypeExpr::Generic { .. }));
            }
            _ => panic!("expected Result type"),
        }
    }

    #[test]
    fn parses_nested_expressions_with_precedence() {
        let src = "class X { fn run(): Int { return 1 + 2 * 3 == 7 && !false; } }";
        let ast = parse(src).expect("expression should parse");
        let TopLevelItem::Class(c) = &ast.items[0] else {
            panic!("expected class");
        };
        let Stmt::Return(Some(expr)) = &c.methods[0].body.stmts[0] else {
            panic!("expected return expr");
        };
        match expr {
            Expr::BinOp { op: BinOp::And, .. } => {}
            _ => panic!("expected top-level and expression"),
        }
    }

    #[test]
    fn parses_statements() {
        let src =
            "class X { fn run(): Int { let x: Int = 1 if x == 1 { return x } else { return 0 } } }";
        let ast = parse(src).expect("statements should parse");
        let TopLevelItem::Class(c) = &ast.items[0] else {
            panic!("expected class");
        };
        assert_eq!(c.methods[0].body.stmts.len(), 2);
        assert!(matches!(c.methods[0].body.stmts[0], Stmt::Let { .. }));
        assert!(matches!(c.methods[0].body.stmts[1], Stmt::If { .. }));
    }

    #[test]
    fn recovers_from_top_level_errors_and_collects_multiple() {
        let src = "class A { fn x(): Int { return 1 } } class { fn bad(): Int { return 0 } } class B { fn y(): Int { return 2 } }";
        let errors = parse(src).expect_err("expected parse errors");
        assert!(!errors.is_empty());
    }

    #[test]
    fn parses_private_constructor_params() {
        let src =
            "class A { constructor(private repo: Repo, name: String) {} fn x(): Int { return 1 } }";
        let ast = parse(src).expect("constructor should parse");
        let TopLevelItem::Class(c) = &ast.items[0] else {
            panic!("expected class");
        };
        let ctor = c.constructor.as_ref().expect("constructor should exist");
        assert_eq!(ctor.params.len(), 2);
        assert!(ctor.params[0].is_private);
        assert!(!ctor.params[1].is_private);
    }
}
