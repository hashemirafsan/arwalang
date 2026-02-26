#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::parser::ast::{
    Annotation, AnnotationArg, ClassDecl, Expr, MethodDecl, ModuleDecl, Param, ProviderBinding,
    SourceFile, Span, TopLevelItem, TypeExpr,
};

/// Resolution errors emitted by the name-resolution phase.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ResolveError {
    #[error("undefined type '{name}'")]
    UndefinedType { name: String, span: Span },

    #[error("undefined symbol '{name}'")]
    UndefinedSymbol { name: String, span: Span },

    #[error("duplicate symbol '{name}'")]
    DuplicateSymbol { name: String, span: Span },
}

/// Symbol categories tracked in lexical scopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Module,
    Type,
    Class,
    Interface,
    Struct,
    Enum,
    Method,
    Field,
    Parameter,
    Variable,
}

/// A named symbol with its source declaration span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

/// Hierarchical symbol table (file -> class -> method scopes).
#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolTable {
    /// Creates a symbol table with one root scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    /// Enters a nested scope.
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Exits the current scope, preserving at least the root scope.
    pub fn exit_scope(&mut self) {
        if self.scopes.len() > 1 {
            let _ = self.scopes.pop();
        }
    }

    /// Inserts a symbol into the current scope.
    pub fn insert(&mut self, symbol: Symbol) -> Result<(), Symbol> {
        if self.scopes.is_empty() {
            self.scopes.push(HashMap::new());
        }

        let idx = self.scopes.len() - 1;
        let current = &mut self.scopes[idx];
        if current.contains_key(&symbol.name) {
            return Err(symbol);
        }
        current.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    /// Looks up a symbol from innermost to outermost scope.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }
}

/// Name resolver for ArwaLang AST.
#[derive(Debug, Default)]
pub struct Resolver {
    symbols: SymbolTable,
    type_names: HashSet<String>,
    module_names: HashSet<String>,
    errors: Vec<ResolveError>,
}

impl Resolver {
    /// Creates a new resolver instance.
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            type_names: HashSet::new(),
            module_names: HashSet::new(),
            errors: Vec::new(),
        }
    }

    /// Resolves all names in a source file and collects resolution errors.
    pub fn resolve_source_file(&mut self, ast: &SourceFile) -> Result<(), Vec<ResolveError>> {
        self.collect_top_level_symbols(ast);

        for item in &ast.items {
            self.resolve_top_level_item(item);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn collect_top_level_symbols(&mut self, ast: &SourceFile) {
        for item in &ast.items {
            match item {
                TopLevelItem::Module(module) => {
                    self.insert_symbol(
                        module.name.clone(),
                        SymbolKind::Module,
                        module.span.clone(),
                    );
                    self.module_names.insert(module.name.clone());
                }
                TopLevelItem::Class(class) => {
                    self.insert_symbol(class.name.clone(), SymbolKind::Class, class.span.clone());
                    self.type_names.insert(class.name.clone());
                }
                TopLevelItem::Interface(interface) => {
                    self.insert_symbol(
                        interface.name.clone(),
                        SymbolKind::Interface,
                        interface.span.clone(),
                    );
                    self.type_names.insert(interface.name.clone());
                }
                TopLevelItem::Struct(struct_decl) => {
                    self.insert_symbol(
                        struct_decl.name.clone(),
                        SymbolKind::Struct,
                        struct_decl.span.clone(),
                    );
                    self.type_names.insert(struct_decl.name.clone());
                }
                TopLevelItem::Enum(enum_decl) => {
                    self.insert_symbol(
                        enum_decl.name.clone(),
                        SymbolKind::Enum,
                        enum_decl.span.clone(),
                    );
                    self.type_names.insert(enum_decl.name.clone());
                }
                TopLevelItem::Import(_) => {}
            }
        }
    }

    fn resolve_top_level_item(&mut self, item: &TopLevelItem) {
        match item {
            TopLevelItem::Module(module) => self.resolve_module(module),
            TopLevelItem::Class(class) => self.resolve_class(class),
            TopLevelItem::Interface(interface) => {
                self.symbols.enter_scope();
                for method in &interface.methods {
                    self.resolve_type_expr(&method.return_type, &method.span);
                    self.symbols.enter_scope();
                    for param in &method.params {
                        self.resolve_param(param);
                    }
                    self.symbols.exit_scope();
                }
                self.symbols.exit_scope();
            }
            TopLevelItem::Struct(struct_decl) => {
                for field in &struct_decl.fields {
                    self.resolve_type_expr(&field.ty, &field.span);
                }
            }
            TopLevelItem::Enum(_) => {}
            TopLevelItem::Import(_) => {}
        }
    }

    fn resolve_module(&mut self, module: &ModuleDecl) {
        for import in &module.imports {
            if !self.module_names.contains(import) {
                self.errors.push(ResolveError::UndefinedSymbol {
                    name: import.clone(),
                    span: module.span.clone(),
                });
            }
        }

        for provider in &module.providers {
            match provider {
                ProviderBinding::Simple(name) => self.ensure_symbol_exists(name, &module.span),
                ProviderBinding::Aliased { token, impl_ } => {
                    self.ensure_symbol_exists(token, &module.span);
                    self.ensure_symbol_exists(impl_, &module.span);
                }
            }
        }

        for controller in &module.controllers {
            self.ensure_symbol_exists(controller, &module.span);
        }

        for export in &module.exports {
            self.ensure_symbol_exists(export, &module.span);
        }
    }

    fn resolve_class(&mut self, class: &ClassDecl) {
        for interface_name in &class.implements {
            self.resolve_type_name(interface_name, &class.span);
        }

        self.resolve_annotations(&class.annotations);

        self.symbols.enter_scope();

        for field in &class.fields {
            self.insert_symbol(field.name.clone(), SymbolKind::Field, field.span.clone());
            self.resolve_type_expr(&field.ty, &field.span);
        }

        if let Some(constructor) = &class.constructor {
            self.symbols.enter_scope();
            for param in &constructor.params {
                self.resolve_param(param);
            }
            self.symbols.exit_scope();
        }

        for method in &class.methods {
            self.resolve_method(method);
        }

        self.symbols.exit_scope();
    }

    fn resolve_method(&mut self, method: &MethodDecl) {
        self.insert_symbol(method.name.clone(), SymbolKind::Method, method.span.clone());
        self.resolve_annotations(&method.annotations);
        self.resolve_type_expr(&method.return_type, &method.span);

        self.symbols.enter_scope();
        for param in &method.params {
            self.resolve_param(param);
        }
        self.symbols.exit_scope();
    }

    fn resolve_param(&mut self, param: &Param) {
        self.insert_symbol(
            param.name.clone(),
            SymbolKind::Parameter,
            param.span.clone(),
        );
        self.resolve_type_expr(&param.ty, &param.span);
        self.resolve_annotations(&param.annotations);
    }

    fn resolve_annotations(&mut self, annotations: &[Annotation]) {
        for annotation in annotations {
            for arg in &annotation.args {
                match arg {
                    AnnotationArg::Positional(expr) => {
                        self.resolve_annotation_expr(expr, &annotation.span)
                    }
                    AnnotationArg::Named { value, .. } => {
                        self.resolve_annotation_expr(value, &annotation.span)
                    }
                }
            }
        }
    }

    fn resolve_annotation_expr(&mut self, expr: &Expr, span: &Span) {
        match expr {
            Expr::Ident(name) => {
                if self.symbols.lookup(name).is_none() && !self.is_builtin_type_name(name) {
                    self.errors.push(ResolveError::UndefinedSymbol {
                        name: name.clone(),
                        span: span.clone(),
                    });
                }
            }
            Expr::Call { callee, args } => {
                self.resolve_annotation_expr(callee, span);
                for arg in args {
                    self.resolve_annotation_expr(arg, span);
                }
            }
            Expr::FieldAccess { obj, .. } => self.resolve_annotation_expr(obj, span),
            Expr::BinOp { lhs, rhs, .. } => {
                self.resolve_annotation_expr(lhs, span);
                self.resolve_annotation_expr(rhs, span);
            }
            Expr::UnaryOp { expr, .. } => self.resolve_annotation_expr(expr, span),
            Expr::IntLit(_)
            | Expr::FloatLit(_)
            | Expr::StringLit(_)
            | Expr::BoolLit(_)
            | Expr::Null => {}
        }
    }

    fn resolve_type_expr(&mut self, ty: &TypeExpr, span: &Span) {
        match ty {
            TypeExpr::Named(name) => self.resolve_type_name(name, span),
            TypeExpr::Generic { name, params } => {
                self.resolve_type_name(name, span);
                for param in params {
                    self.resolve_type_expr(param, span);
                }
            }
            TypeExpr::Result { ok, err } => {
                self.resolve_type_expr(ok, span);
                self.resolve_type_expr(err, span);
            }
            TypeExpr::Option(inner) => self.resolve_type_expr(inner, span),
        }
    }

    fn resolve_type_name(&mut self, name: &str, span: &Span) {
        if !self.type_names.contains(name) && !self.is_builtin_type_name(name) {
            self.errors.push(ResolveError::UndefinedType {
                name: name.to_string(),
                span: span.clone(),
            });
        }
    }

    fn ensure_symbol_exists(&mut self, name: &str, span: &Span) {
        if self.symbols.lookup(name).is_none() && !self.is_builtin_type_name(name) {
            self.errors.push(ResolveError::UndefinedSymbol {
                name: name.to_string(),
                span: span.clone(),
            });
        }
    }

    fn insert_symbol(&mut self, name: String, kind: SymbolKind, span: Span) {
        let result = self.symbols.insert(Symbol {
            name: name.clone(),
            kind,
            span: span.clone(),
        });

        if result.is_err() {
            self.errors
                .push(ResolveError::DuplicateSymbol { name, span });
        }
    }

    fn is_builtin_type_name(&self, name: &str) -> bool {
        matches!(
            name,
            "Int"
                | "Float"
                | "Bool"
                | "String"
                | "Any"
                | "Result"
                | "Option"
                | "List"
                | "Map"
                | "HttpError"
                | "RequestContext"
                | "Response"
                | "PipeMetadata"
        )
    }

    fn is_builtin_annotation_name(&self, name: &str) -> bool {
        matches!(
            name,
            "injectable"
                | "controller"
                | "get"
                | "post"
                | "put"
                | "delete"
                | "patch"
                | "param"
                | "query"
                | "body"
                | "header"
                | "use_guards"
                | "use_interceptors"
                | "use_pipes"
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::parser::ast::{
        Annotation, AnnotationArg, Block, ClassDecl, ConstructorDecl, Expr, InterfaceDecl,
        MethodDecl, MethodSignature, ModuleDecl, Param, ProviderBinding, SourceFile, Span,
        TopLevelItem, TypeExpr,
    };

    use super::{ResolveError, Resolver, Symbol, SymbolKind, SymbolTable};

    fn span() -> Span {
        Span {
            file: PathBuf::from("test.rw"),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 1,
        }
    }

    fn empty_method() -> MethodDecl {
        MethodDecl {
            annotations: vec![],
            name: "m".to_string(),
            params: vec![],
            return_type: TypeExpr::Named("Int".to_string()),
            body: Block { stmts: vec![] },
            span: span(),
        }
    }

    #[test]
    fn symbol_table_scope_lookup_works() {
        let mut table = SymbolTable::new();
        let root = Symbol {
            name: "Root".to_string(),
            kind: SymbolKind::Type,
            span: span(),
        };
        assert!(table.insert(root).is_ok());
        assert!(table.lookup("Root").is_some());

        table.enter_scope();
        let inner = Symbol {
            name: "Inner".to_string(),
            kind: SymbolKind::Variable,
            span: span(),
        };
        assert!(table.insert(inner).is_ok());
        assert!(table.lookup("Inner").is_some());
        assert!(table.lookup("Root").is_some());

        table.exit_scope();
        assert!(table.lookup("Inner").is_none());
        assert!(table.lookup("Root").is_some());
    }

    #[test]
    fn resolves_constructor_and_method_types() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(ClassDecl {
                    annotations: vec![],
                    name: "Repo".to_string(),
                    implements: vec![],
                    constructor: None,
                    methods: vec![empty_method()],
                    fields: vec![],
                    span: span(),
                }),
                TopLevelItem::Class(ClassDecl {
                    annotations: vec![],
                    name: "UserService".to_string(),
                    implements: vec![],
                    constructor: Some(ConstructorDecl {
                        params: vec![Param {
                            annotations: vec![],
                            name: "repo".to_string(),
                            ty: TypeExpr::Named("Repo".to_string()),
                            is_private: true,
                            span: span(),
                        }],
                        span: span(),
                    }),
                    methods: vec![MethodDecl {
                        annotations: vec![],
                        name: "getUser".to_string(),
                        params: vec![],
                        return_type: TypeExpr::Named("Int".to_string()),
                        body: Block { stmts: vec![] },
                        span: span(),
                    }],
                    fields: vec![],
                    span: span(),
                }),
            ],
        };

        let mut resolver = Resolver::new();
        let result = resolver.resolve_source_file(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn errors_on_undefined_type() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(ClassDecl {
                annotations: vec![],
                name: "UserService".to_string(),
                implements: vec![],
                constructor: None,
                methods: vec![MethodDecl {
                    annotations: vec![],
                    name: "x".to_string(),
                    params: vec![],
                    return_type: TypeExpr::Named("MissingType".to_string()),
                    body: Block { stmts: vec![] },
                    span: span(),
                }],
                fields: vec![],
                span: span(),
            })],
        };

        let mut resolver = Resolver::new();
        let errors = resolver
            .resolve_source_file(&ast)
            .expect_err("resolver should fail");

        assert!(errors.iter().any(
            |e| matches!(e, ResolveError::UndefinedType { name, .. } if name == "MissingType")
        ));
    }

    #[test]
    fn resolves_generic_types() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(ClassDecl {
                    annotations: vec![],
                    name: "User".to_string(),
                    implements: vec![],
                    constructor: None,
                    methods: vec![empty_method()],
                    fields: vec![],
                    span: span(),
                }),
                TopLevelItem::Class(ClassDecl {
                    annotations: vec![],
                    name: "UserService".to_string(),
                    implements: vec![],
                    constructor: None,
                    methods: vec![MethodDecl {
                        annotations: vec![],
                        name: "list".to_string(),
                        params: vec![],
                        return_type: TypeExpr::Generic {
                            name: "List".to_string(),
                            params: vec![TypeExpr::Named("User".to_string())],
                        },
                        body: Block { stmts: vec![] },
                        span: span(),
                    }],
                    fields: vec![],
                    span: span(),
                }),
            ],
        };

        let mut resolver = Resolver::new();
        assert!(resolver.resolve_source_file(&ast).is_ok());
    }

    #[test]
    fn resolves_interface_implementations() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Interface(InterfaceDecl {
                    annotations: vec![],
                    name: "IService".to_string(),
                    methods: vec![MethodSignature {
                        name: "run".to_string(),
                        params: vec![],
                        return_type: TypeExpr::Named("Int".to_string()),
                        span: span(),
                    }],
                    span: span(),
                }),
                TopLevelItem::Class(ClassDecl {
                    annotations: vec![],
                    name: "Service".to_string(),
                    implements: vec!["IService".to_string()],
                    constructor: None,
                    methods: vec![empty_method()],
                    fields: vec![],
                    span: span(),
                }),
            ],
        };

        let mut resolver = Resolver::new();
        assert!(resolver.resolve_source_file(&ast).is_ok());
    }

    #[test]
    fn errors_on_undefined_symbol_in_module() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Module(ModuleDecl {
                name: "App".to_string(),
                imports: vec!["UnknownModule".to_string()],
                providers: vec![ProviderBinding::Simple("MissingProvider".to_string())],
                controllers: vec!["MissingController".to_string()],
                exports: vec!["MissingExport".to_string()],
                span: span(),
            })],
        };

        let mut resolver = Resolver::new();
        let errors = resolver
            .resolve_source_file(&ast)
            .expect_err("resolver should fail");

        assert!(errors.iter().any(
            |e| matches!(e, ResolveError::UndefinedSymbol { name, .. } if name == "UnknownModule")
        ));
    }

    #[test]
    fn resolves_annotation_identifier_references() {
        let guard_class = TopLevelItem::Class(ClassDecl {
            annotations: vec![],
            name: "AuthGuard".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![empty_method()],
            fields: vec![],
            span: span(),
        });

        let controller = TopLevelItem::Class(ClassDecl {
            annotations: vec![Annotation {
                name: "use_guards".to_string(),
                args: vec![AnnotationArg::Positional(Expr::Ident(
                    "AuthGuard".to_string(),
                ))],
                span: span(),
            }],
            name: "UserController".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![empty_method()],
            fields: vec![],
            span: span(),
        });

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![guard_class, controller],
        };

        let mut resolver = Resolver::new();
        assert!(resolver.resolve_source_file(&ast).is_ok());
    }

    #[test]
    fn errors_on_unknown_annotation_identifier_reference() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(ClassDecl {
                annotations: vec![Annotation {
                    name: "use_guards".to_string(),
                    args: vec![AnnotationArg::Positional(Expr::Ident(
                        "MissingGuard".to_string(),
                    ))],
                    span: span(),
                }],
                name: "UserController".to_string(),
                implements: vec![],
                constructor: None,
                methods: vec![empty_method()],
                fields: vec![],
                span: span(),
            })],
        };

        let mut resolver = Resolver::new();
        let errors = resolver
            .resolve_source_file(&ast)
            .expect_err("resolver should fail");

        assert!(errors.iter().any(
            |e| matches!(e, ResolveError::UndefinedSymbol { name, .. } if name == "MissingGuard")
        ));
    }

    #[test]
    fn errors_on_duplicate_symbols() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(ClassDecl {
                    annotations: vec![],
                    name: "UserService".to_string(),
                    implements: vec![],
                    constructor: None,
                    methods: vec![empty_method()],
                    fields: vec![],
                    span: span(),
                }),
                TopLevelItem::Class(ClassDecl {
                    annotations: vec![],
                    name: "UserService".to_string(),
                    implements: vec![],
                    constructor: None,
                    methods: vec![empty_method()],
                    fields: vec![],
                    span: span(),
                }),
            ],
        };

        let mut resolver = Resolver::new();
        let errors = resolver
            .resolve_source_file(&ast)
            .expect_err("resolver should fail");

        assert!(errors.iter().any(
            |e| matches!(e, ResolveError::DuplicateSymbol { name, .. } if name == "UserService")
        ));
    }
}
