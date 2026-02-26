#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use thiserror::Error;

use crate::parser::ast::{
    Annotation, AnnotationArg, ClassDecl, ConstructorDecl, ModuleDecl, Param, ProviderBinding,
    SourceFile, Span, TopLevelItem,
};

/// Provider lifetime scope for DI compatibility checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Singleton,
    Request,
    Transient,
}

impl Scope {
    /// Derives DI scope from `#[injectable(scope = ...)]` annotation.
    ///
    /// Why this is needed:
    /// - scope compatibility validation depends on each provider's effective lifetime.
    fn from_annotation(annotations: &[Annotation]) -> Self {
        let Some(injectable) = annotations.iter().find(|ann| ann.name == "injectable") else {
            return Self::Singleton;
        };

        for arg in &injectable.args {
            if let AnnotationArg::Named { key, value } = arg {
                if key == "scope" {
                    if let crate::parser::ast::Expr::StringLit(scope) = value {
                        return match scope.as_str() {
                            "request" => Self::Request,
                            "transient" => Self::Transient,
                            _ => Self::Singleton,
                        };
                    }
                }
            }
        }

        Self::Singleton
    }

    /// Returns whether `self` can inject a dependency with `dependency` scope.
    ///
    /// Why this is needed:
    /// - enforces compile-time scope safety matrix for DI004 checks.
    fn allows(self, dependency: Scope) -> bool {
        matches!(
            (self, dependency),
            (Self::Singleton, Self::Singleton)
                | (Self::Request, Self::Singleton)
                | (Self::Request, Self::Request)
                | (Self::Transient, Self::Singleton)
                | (Self::Transient, Self::Request)
                | (Self::Transient, Self::Transient)
        )
    }
}

impl std::fmt::Display for Scope {
    /// Formats scope names for diagnostics.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Singleton => write!(f, "singleton"),
            Self::Request => write!(f, "request"),
            Self::Transient => write!(f, "transient"),
        }
    }
}

/// DI provider node.
#[derive(Debug, Clone)]
pub struct Provider {
    pub token: String,
    pub implementation: String,
    pub scope: Scope,
    pub dependencies: Vec<Dependency>,
    pub module: String,
    pub span: Span,
}

/// Provider constructor dependency.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub span: Span,
}

/// Directed DI graph for one source file.
#[derive(Debug, Clone, Default)]
pub struct DiGraph {
    pub providers: Vec<Arc<Provider>>,
    token_index: HashMap<(String, String), Arc<Provider>>, // (module, token)
    implementation_index: HashMap<(String, String), Arc<Provider>>, // (module, class)
    edges: Vec<(String, String)>,                          // consumer impl -> dependency impl
}

impl DiGraph {
    /// Adds provider to graph and updates indexes.
    pub fn add_provider(&mut self, provider: Provider) {
        let arc = Arc::new(provider);
        self.token_index
            .insert((arc.module.clone(), arc.token.clone()), Arc::clone(&arc));
        self.implementation_index.insert(
            (arc.module.clone(), arc.implementation.clone()),
            Arc::clone(&arc),
        );
        self.providers.push(arc);
    }

    /// Adds a dependency edge between provider implementations.
    ///
    /// Why this is needed:
    /// - preserves explicit graph connectivity for cycle/introspection workflows.
    pub fn add_dependency(&mut self, from: String, to: String) {
        self.edges.push((from, to));
    }

    /// Looks up provider by module-local token.
    fn find_token(&self, module: &str, token: &str) -> Option<Arc<Provider>> {
        self.token_index
            .get(&(module.to_string(), token.to_string()))
            .cloned()
    }

    /// Looks up provider by module-local implementation class.
    fn find_impl(&self, module: &str, implementation: &str) -> Option<Arc<Provider>> {
        self.implementation_index
            .get(&(module.to_string(), implementation.to_string()))
            .cloned()
    }
}

/// DI graph validation errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DiError {
    #[error("DI001 missing provider: '{dependency}' for '{consumer}'")]
    MissingProvider {
        consumer: String,
        dependency: String,
        span: Span,
    },

    #[error("DI002 circular dependency: {cycle:?}")]
    CircularDependency { cycle: Vec<String>, span: Span },

    #[error("DI003 duplicate provider token '{token}'")]
    DuplicateProvider { token: String, span: Span },

    #[error(
        "DI004 scope mismatch: '{consumer}' ({consumer_scope}) cannot inject '{dependency}' ({dependency_scope})"
    )]
    ScopeMismatch {
        consumer: String,
        dependency: String,
        consumer_scope: Scope,
        dependency_scope: Scope,
        span: Span,
    },

    #[error("DI005 not injectable: '{class}'")]
    NotInjectable { class: String, span: Span },

    #[error("DI006 export without provider: '{symbol}' in module '{module}'")]
    ExportWithoutProvider {
        symbol: String,
        module: String,
        span: Span,
    },
}

/// Builder/validator for DI graph.
#[derive(Debug, Default)]
pub struct DiGraphBuilder {
    classes: HashMap<String, ClassDecl>,
    modules: HashMap<String, ModuleDecl>,
}

impl DiGraphBuilder {
    /// Creates a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds and validates DI graph.
    ///
    /// Why this is needed:
    /// - phase 6 must provide one deterministic entrypoint returning all DI errors.
    pub fn build(&mut self, ast: &SourceFile) -> Result<DiGraph, Vec<DiError>> {
        self.collect_declarations(ast);
        let mut graph = DiGraph::default();
        let mut errors = Vec::new();

        for module in self.modules.values() {
            self.build_module_providers(module, &mut graph, &mut errors);
        }

        self.validate_exports(&graph, &mut errors);
        self.validate_missing_providers(&graph, &mut errors);
        self.validate_scope_compatibility(&graph, &mut errors);
        self.validate_cycles(&graph, &mut errors);

        if errors.is_empty() {
            Ok(graph)
        } else {
            Err(errors)
        }
    }

    /// Collects class/module declarations used by builder passes.
    ///
    /// Why this is needed:
    /// - provider assembly/validation requires fast lookup of declarations by name.
    fn collect_declarations(&mut self, ast: &SourceFile) {
        self.classes.clear();
        self.modules.clear();

        for item in &ast.items {
            match item {
                TopLevelItem::Class(class) => {
                    self.classes.insert(class.name.clone(), class.clone());
                }
                TopLevelItem::Module(module) => {
                    self.modules.insert(module.name.clone(), module.clone());
                }
                _ => {}
            }
        }
    }

    /// Builds module-local providers from `provide` bindings.
    ///
    /// Why this is needed:
    /// - transforms syntax-level bindings into normalized provider graph nodes.
    fn build_module_providers(
        &self,
        module: &ModuleDecl,
        graph: &mut DiGraph,
        errors: &mut Vec<DiError>,
    ) {
        let mut seen_tokens = HashSet::new();

        for binding in &module.providers {
            let (token, implementation) = match binding {
                ProviderBinding::Simple(name) => (name.clone(), name.clone()),
                ProviderBinding::Aliased { token, impl_ } => (token.clone(), impl_.clone()),
            };

            if !seen_tokens.insert(token.clone()) {
                errors.push(DiError::DuplicateProvider {
                    token,
                    span: module.span.clone(),
                });
                continue;
            }

            let Some(class) = self.classes.get(&implementation) else {
                errors.push(DiError::NotInjectable {
                    class: implementation,
                    span: module.span.clone(),
                });
                continue;
            };

            if !is_injectable(class) {
                errors.push(DiError::NotInjectable {
                    class: class.name.clone(),
                    span: class.span.clone(),
                });
                continue;
            }

            let dependencies = dependencies_from_constructor(class.constructor.as_ref());
            graph.add_provider(Provider {
                token,
                implementation: class.name.clone(),
                scope: Scope::from_annotation(&class.annotations),
                dependencies,
                module: module.name.clone(),
                span: class.span.clone(),
            });

            if let Some(added) = graph.find_impl(&module.name, &class.name) {
                for dep in &added.dependencies {
                    if let Some(dep_provider) = graph.find_token(&module.name, &dep.name) {
                        graph.add_dependency(
                            added.implementation.clone(),
                            dep_provider.implementation.clone(),
                        );
                    }
                }
            }
        }
    }

    /// Validates module export list references against provided tokens.
    fn validate_exports(&self, graph: &DiGraph, errors: &mut Vec<DiError>) {
        for module in self.modules.values() {
            for export in &module.exports {
                if graph.find_token(&module.name, export).is_none() {
                    errors.push(DiError::ExportWithoutProvider {
                        symbol: export.clone(),
                        module: module.name.clone(),
                        span: module.span.clone(),
                    });
                }
            }
        }
    }

    /// Validates that each constructor dependency has a provider in scope.
    ///
    /// Why this is needed:
    /// - unresolved dependencies break container bootstrap (DI001).
    fn validate_missing_providers(&self, graph: &DiGraph, errors: &mut Vec<DiError>) {
        for module in self.modules.values() {
            for consumer_name in module
                .providers
                .iter()
                .map(provider_impl_name)
                .chain(module.controllers.iter().cloned())
            {
                let Some(consumer_class) = self.classes.get(&consumer_name) else {
                    continue;
                };

                for dep in dependencies_from_constructor(consumer_class.constructor.as_ref()) {
                    if is_builtin_type(&dep.name) {
                        continue;
                    }
                    if graph.find_token(&module.name, &dep.name).is_none() {
                        errors.push(DiError::MissingProvider {
                            consumer: consumer_name.clone(),
                            dependency: dep.name,
                            span: dep.span,
                        });
                    }
                }
            }
        }
    }

    /// Validates scope compatibility for all provider dependency edges.
    fn validate_scope_compatibility(&self, graph: &DiGraph, errors: &mut Vec<DiError>) {
        for provider in &graph.providers {
            for dep in &provider.dependencies {
                let Some(dep_provider) = graph.find_token(&provider.module, &dep.name) else {
                    continue;
                };
                if !provider.scope.allows(dep_provider.scope) {
                    errors.push(DiError::ScopeMismatch {
                        consumer: provider.implementation.clone(),
                        dependency: dep_provider.implementation.clone(),
                        consumer_scope: provider.scope,
                        dependency_scope: dep_provider.scope,
                        span: dep.span.clone(),
                    });
                }
            }
        }
    }

    /// Detects circular dependencies per module using DFS.
    ///
    /// Why this is needed:
    /// - cycle-free dependency graph is required for deterministic DI initialization.
    fn validate_cycles(&self, graph: &DiGraph, errors: &mut Vec<DiError>) {
        for module in self.modules.values() {
            let providers: Vec<Arc<Provider>> = graph
                .providers
                .iter()
                .filter(|p| p.module == module.name)
                .cloned()
                .collect();

            let mut visiting = HashSet::new();
            let mut visited = HashSet::new();
            let mut stack: Vec<String> = Vec::new();

            for provider in providers {
                if visited.contains(&provider.implementation) {
                    continue;
                }
                if let Some(cycle) = dfs_cycle(
                    graph,
                    &module.name,
                    &provider.implementation,
                    &mut visiting,
                    &mut visited,
                    &mut stack,
                ) {
                    errors.push(DiError::CircularDependency {
                        cycle,
                        span: provider.span.clone(),
                    });
                    return;
                }
            }
        }
    }
}

/// Depth-first cycle detection on provider dependency graph.
///
/// Why this is needed:
/// - captures full cycle path for DI002 diagnostics.
fn dfs_cycle(
    graph: &DiGraph,
    module: &str,
    current_impl: &str,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    visiting.insert(current_impl.to_string());
    stack.push(current_impl.to_string());

    let provider = graph.find_impl(module, current_impl)?;
    for dep in &provider.dependencies {
        let Some(dep_provider) = graph.find_token(module, &dep.name) else {
            continue;
        };
        let next = dep_provider.implementation.clone();

        if visiting.contains(&next) {
            if let Some(idx) = stack.iter().position(|name| name == &next) {
                let mut cycle = stack[idx..].to_vec();
                cycle.push(next);
                return Some(cycle);
            }
        }

        if !visited.contains(&next) {
            if let Some(cycle) = dfs_cycle(graph, module, &next, visiting, visited, stack) {
                return Some(cycle);
            }
        }
    }

    visiting.remove(current_impl);
    visited.insert(current_impl.to_string());
    let _ = stack.pop();
    None
}

/// Extracts implementation class name from a provider binding.
fn provider_impl_name(binding: &ProviderBinding) -> String {
    match binding {
        ProviderBinding::Simple(name) => name.clone(),
        ProviderBinding::Aliased { impl_, .. } => impl_.clone(),
    }
}

/// Extracts constructor dependencies into DI dependency records.
///
/// Why this is needed:
/// - constructor injection is the source of directed DI edges.
fn dependencies_from_constructor(constructor: Option<&ConstructorDecl>) -> Vec<Dependency> {
    constructor
        .map(|ctor| {
            ctor.params
                .iter()
                .map(|param| Dependency {
                    name: param_type_name(param),
                    span: param.span.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Converts parameter type expression into dependency token name.
fn param_type_name(param: &Param) -> String {
    match &param.ty {
        crate::parser::ast::TypeExpr::Named(name) => name.clone(),
        crate::parser::ast::TypeExpr::Generic { name, .. } => name.clone(),
        crate::parser::ast::TypeExpr::Result { .. } => "Result".to_string(),
        crate::parser::ast::TypeExpr::Option(_) => "Option".to_string(),
    }
}

/// Checks whether a class is marked as injectable provider.
fn is_injectable(class: &ClassDecl) -> bool {
    class.annotations.iter().any(|ann| ann.name == "injectable")
}

/// Returns whether token refers to built-in type not requiring DI provider.
fn is_builtin_type(name: &str) -> bool {
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
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::parser::ast::{
        Annotation, AnnotationArg, ClassDecl, ConstructorDecl, ModuleDecl, Param, ProviderBinding,
        SourceFile, Span, TopLevelItem, TypeExpr,
    };

    use super::{DiError, DiGraphBuilder, Scope};

    /// Creates stable synthetic span for unit tests.
    fn span() -> Span {
        Span {
            file: PathBuf::from("test.rw"),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 1,
        }
    }

    /// Builds injectable annotation fixture with optional scope.
    fn injectable(scope: Option<&str>) -> Annotation {
        let args = scope
            .map(|s| {
                vec![AnnotationArg::Named {
                    key: "scope".into(),
                    value: crate::parser::ast::Expr::StringLit(s.into()),
                }]
            })
            .unwrap_or_default();
        Annotation {
            name: "injectable".into(),
            args,
            span: span(),
        }
    }

    /// Builds injectable class fixture with constructor dependencies.
    fn class(name: &str, deps: Vec<&str>, scope: Option<&str>) -> ClassDecl {
        ClassDecl {
            annotations: vec![injectable(scope)],
            name: name.into(),
            implements: vec![],
            constructor: Some(ConstructorDecl {
                params: deps
                    .into_iter()
                    .map(|d| Param {
                        annotations: vec![],
                        name: format!("{}_dep", d.to_lowercase()),
                        ty: TypeExpr::Named(d.into()),
                        is_private: false,
                        span: span(),
                    })
                    .collect(),
                span: span(),
            }),
            methods: vec![],
            fields: vec![],
            span: span(),
        }
    }

    /// Builds non-injectable class fixture.
    fn plain_class(name: &str) -> ClassDecl {
        ClassDecl {
            annotations: vec![],
            name: name.into(),
            implements: vec![],
            constructor: None,
            methods: vec![],
            fields: vec![],
            span: span(),
        }
    }

    /// Builds module fixture with providers/exports.
    fn module(name: &str, providers: Vec<ProviderBinding>, exports: Vec<&str>) -> ModuleDecl {
        ModuleDecl {
            name: name.into(),
            imports: vec![],
            providers,
            controllers: vec![],
            exports: exports.into_iter().map(|v| v.into()).collect(),
            span: span(),
        }
    }

    #[test]
    fn builds_simple_di_graph() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("Repo", vec![], None)),
                TopLevelItem::Class(class("Service", vec!["Repo"], None)),
                TopLevelItem::Module(module(
                    "App",
                    vec![
                        ProviderBinding::Simple("Repo".into()),
                        ProviderBinding::Simple("Service".into()),
                    ],
                    vec![],
                )),
            ],
        };

        let mut builder = DiGraphBuilder::new();
        let graph = builder.build(&ast).expect("graph should build");
        assert_eq!(graph.providers.len(), 2);
    }

    #[test]
    fn detects_missing_provider() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("Service", vec!["Repo"], None)),
                TopLevelItem::Module(module(
                    "App",
                    vec![ProviderBinding::Simple("Service".into())],
                    vec![],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors.iter().any(
            |e| matches!(e, DiError::MissingProvider { dependency, .. } if dependency == "Repo")
        ));
    }

    #[test]
    fn detects_circular_dependency() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("A", vec!["B"], None)),
                TopLevelItem::Class(class("B", vec!["A"], None)),
                TopLevelItem::Module(module(
                    "App",
                    vec![
                        ProviderBinding::Simple("A".into()),
                        ProviderBinding::Simple("B".into()),
                    ],
                    vec![],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, DiError::CircularDependency { .. })));
    }

    #[test]
    fn detects_longer_circular_dependency() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("A", vec!["B"], None)),
                TopLevelItem::Class(class("B", vec!["C"], None)),
                TopLevelItem::Class(class("C", vec!["A"], None)),
                TopLevelItem::Module(module(
                    "App",
                    vec![
                        ProviderBinding::Simple("A".into()),
                        ProviderBinding::Simple("B".into()),
                        ProviderBinding::Simple("C".into()),
                    ],
                    vec![],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, DiError::CircularDependency { .. })));
    }

    #[test]
    fn detects_duplicate_provider_token() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("Repo", vec![], None)),
                TopLevelItem::Class(class("OtherRepo", vec![], None)),
                TopLevelItem::Module(module(
                    "App",
                    vec![
                        ProviderBinding::Simple("Repo".into()),
                        ProviderBinding::Aliased {
                            token: "Repo".into(),
                            impl_: "OtherRepo".into(),
                        },
                    ],
                    vec![],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, DiError::DuplicateProvider { token, .. } if token == "Repo")));
    }

    #[test]
    fn detects_scope_mismatch_singleton_to_request() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("ReqDep", vec![], Some("request"))),
                TopLevelItem::Class(class("Service", vec!["ReqDep"], Some("singleton"))),
                TopLevelItem::Module(module(
                    "App",
                    vec![
                        ProviderBinding::Simple("ReqDep".into()),
                        ProviderBinding::Simple("Service".into()),
                    ],
                    vec![],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors.iter().any(|e| matches!(
            e,
            DiError::ScopeMismatch {
                consumer_scope: Scope::Singleton,
                dependency_scope: Scope::Request,
                ..
            }
        )));
    }

    #[test]
    fn allows_request_to_inject_singleton() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("SingletonDep", vec![], Some("singleton"))),
                TopLevelItem::Class(class("ReqSvc", vec!["SingletonDep"], Some("request"))),
                TopLevelItem::Module(module(
                    "App",
                    vec![
                        ProviderBinding::Simple("SingletonDep".into()),
                        ProviderBinding::Simple("ReqSvc".into()),
                    ],
                    vec![],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        assert!(builder.build(&ast).is_ok());
    }

    #[test]
    fn allows_transient_to_inject_request() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("ReqDep", vec![], Some("request"))),
                TopLevelItem::Class(class("TransientSvc", vec!["ReqDep"], Some("transient"))),
                TopLevelItem::Module(module(
                    "App",
                    vec![
                        ProviderBinding::Simple("ReqDep".into()),
                        ProviderBinding::Simple("TransientSvc".into()),
                    ],
                    vec![],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        assert!(builder.build(&ast).is_ok());
    }

    #[test]
    fn detects_not_injectable_provider() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(plain_class("Repo")),
                TopLevelItem::Module(module(
                    "App",
                    vec![ProviderBinding::Simple("Repo".into())],
                    vec![],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, DiError::NotInjectable { class, .. } if class == "Repo")));
    }

    #[test]
    fn detects_export_without_provider() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Module(module("App", vec![], vec!["Repo"]))],
        };
        let mut builder = DiGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors.iter().any(
            |e| matches!(e, DiError::ExportWithoutProvider { symbol, .. } if symbol == "Repo")
        ));
    }

    #[test]
    fn supports_aliased_bindings() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("PostgresRepo", vec![], None)),
                TopLevelItem::Class(class("Service", vec!["UserRepo"], None)),
                TopLevelItem::Module(module(
                    "App",
                    vec![
                        ProviderBinding::Aliased {
                            token: "UserRepo".into(),
                            impl_: "PostgresRepo".into(),
                        },
                        ProviderBinding::Simple("Service".into()),
                    ],
                    vec!["UserRepo"],
                )),
            ],
        };
        let mut builder = DiGraphBuilder::new();
        assert!(builder.build(&ast).is_ok());
    }
}
