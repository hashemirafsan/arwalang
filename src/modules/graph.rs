#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::parser::ast::{
    ClassDecl, ConstructorDecl, Param, ProviderBinding, SourceFile, Span, TopLevelItem,
};

/// Module graph node model used for validation.
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub imports: Vec<String>,
    pub providers: Vec<ProviderBinding>,
    pub controllers: Vec<String>,
    pub exports: Vec<String>,
    pub span: Span,
}

/// Directed module import graph.
#[derive(Debug, Clone, Default)]
pub struct ModuleGraph {
    pub modules: HashMap<String, Module>,
    pub edges: Vec<(String, String)>,
}

impl ModuleGraph {
    /// Adds a module node.
    pub fn add_module(&mut self, module: Module) {
        self.modules.insert(module.name.clone(), module);
    }

    /// Adds an import edge from one module to another.
    pub fn add_import(&mut self, from: String, to: String) {
        self.edges.push((from, to));
    }
}

/// Module graph validation errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ModuleError {
    #[error("MOD001 unknown import '{import}' in module '{module}'")]
    UnknownImport {
        module: String,
        import: String,
        span: Span,
    },

    #[error("MOD002 circular import: {cycle:?}")]
    CircularImport { cycle: Vec<String>, span: Span },

    #[error(
        "MOD003 unsatisfied controller dependency '{dependency}' for '{controller}' in module '{module}'"
    )]
    UnsatisfiedController {
        controller: String,
        dependency: String,
        module: String,
        span: Span,
    },

    #[error(
        "MOD004 private provider '{provider}' from '{source_module}' used in '{consuming_module}'"
    )]
    PrivateProvider {
        provider: String,
        source_module: String,
        consuming_module: String,
        span: Span,
    },

    #[error("module export '{symbol}' is not provided in '{module}'")]
    ExportWithoutProvider {
        symbol: String,
        module: String,
        span: Span,
    },
}

/// Module graph builder and validator.
#[derive(Debug, Default)]
pub struct ModuleGraphBuilder {
    classes: HashMap<String, ClassDecl>,
}

impl ModuleGraphBuilder {
    /// Creates new module graph builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds and validates module graph.
    pub fn build(&mut self, ast: &SourceFile) -> Result<ModuleGraph, Vec<ModuleError>> {
        self.collect_classes(ast);
        let mut graph = ModuleGraph::default();
        let mut errors = Vec::new();

        for item in &ast.items {
            if let TopLevelItem::Module(module) = item {
                graph.add_module(Module {
                    name: module.name.clone(),
                    imports: module.imports.clone(),
                    providers: module.providers.clone(),
                    controllers: module.controllers.clone(),
                    exports: module.exports.clone(),
                    span: module.span.clone(),
                });
            }
        }

        let edges: Vec<(String, String)> = graph
            .modules
            .values()
            .flat_map(|module| {
                module
                    .imports
                    .iter()
                    .map(|import| (module.name.clone(), import.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (from, to) in edges {
            graph.add_import(from, to);
        }

        self.validate_unknown_imports(&graph, &mut errors);
        self.validate_circular_imports(&graph, &mut errors);
        self.validate_exports(&graph, &mut errors);
        self.validate_controller_dependencies(&graph, &mut errors);

        if errors.is_empty() {
            Ok(graph)
        } else {
            Err(errors)
        }
    }

    fn collect_classes(&mut self, ast: &SourceFile) {
        self.classes.clear();
        for item in &ast.items {
            if let TopLevelItem::Class(class) = item {
                self.classes.insert(class.name.clone(), class.clone());
            }
        }
    }

    fn validate_unknown_imports(&self, graph: &ModuleGraph, errors: &mut Vec<ModuleError>) {
        for module in graph.modules.values() {
            for import in &module.imports {
                if !graph.modules.contains_key(import) {
                    errors.push(ModuleError::UnknownImport {
                        module: module.name.clone(),
                        import: import.clone(),
                        span: module.span.clone(),
                    });
                }
            }
        }
    }

    fn validate_circular_imports(&self, graph: &ModuleGraph, errors: &mut Vec<ModuleError>) {
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::<String>::new();

        for name in graph.modules.keys() {
            if visited.contains(name) {
                continue;
            }
            if let Some(cycle) =
                dfs_module_cycle(graph, name, &mut visiting, &mut visited, &mut stack)
            {
                let span = graph
                    .modules
                    .get(&cycle[0])
                    .map(|m| m.span.clone())
                    .unwrap_or_else(default_span);
                errors.push(ModuleError::CircularImport { cycle, span });
                return;
            }
        }
    }

    fn validate_exports(&self, graph: &ModuleGraph, errors: &mut Vec<ModuleError>) {
        for module in graph.modules.values() {
            for export in &module.exports {
                if !module_provides_token(module, export) {
                    errors.push(ModuleError::ExportWithoutProvider {
                        symbol: export.clone(),
                        module: module.name.clone(),
                        span: module.span.clone(),
                    });
                }
            }
        }
    }

    fn validate_controller_dependencies(&self, graph: &ModuleGraph, errors: &mut Vec<ModuleError>) {
        for module in graph.modules.values() {
            for controller_name in &module.controllers {
                let Some(controller) = self.classes.get(controller_name) else {
                    continue;
                };

                for dep in ctor_dependencies(controller.constructor.as_ref()) {
                    if is_builtin_type(&dep.name) {
                        continue;
                    }

                    if module_provides_token(module, &dep.name) {
                        continue;
                    }

                    let mut found_private = None::<String>;
                    let mut found_exported = false;
                    for import in &module.imports {
                        let Some(source_module) = graph.modules.get(import) else {
                            continue;
                        };

                        if module_provides_token(source_module, &dep.name) {
                            if source_module.exports.contains(&dep.name) {
                                found_exported = true;
                                break;
                            }
                            found_private = Some(source_module.name.clone());
                        }
                    }

                    if found_exported {
                        continue;
                    }

                    if let Some(source_module) = found_private {
                        errors.push(ModuleError::PrivateProvider {
                            provider: dep.name,
                            source_module,
                            consuming_module: module.name.clone(),
                            span: dep.span,
                        });
                    } else {
                        errors.push(ModuleError::UnsatisfiedController {
                            controller: controller_name.clone(),
                            dependency: dep.name,
                            module: module.name.clone(),
                            span: dep.span,
                        });
                    }
                }
            }
        }
    }
}

fn dfs_module_cycle(
    graph: &ModuleGraph,
    current: &str,
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    visiting.insert(current.to_string());
    stack.push(current.to_string());

    let node = graph.modules.get(current)?;
    for next in &node.imports {
        if visiting.contains(next) {
            if let Some(idx) = stack.iter().position(|n| n == next) {
                let mut cycle = stack[idx..].to_vec();
                cycle.push(next.clone());
                return Some(cycle);
            }
        }

        if !visited.contains(next) {
            if let Some(cycle) = dfs_module_cycle(graph, next, visiting, visited, stack) {
                return Some(cycle);
            }
        }
    }

    visiting.remove(current);
    visited.insert(current.to_string());
    let _ = stack.pop();
    None
}

fn module_provides_token(module: &Module, token: &str) -> bool {
    module.providers.iter().any(|binding| match binding {
        ProviderBinding::Simple(name) => name == token,
        ProviderBinding::Aliased {
            token: provider_token,
            ..
        } => provider_token == token,
    })
}

#[derive(Debug, Clone)]
struct DepRef {
    name: String,
    span: Span,
}

fn ctor_dependencies(constructor: Option<&ConstructorDecl>) -> Vec<DepRef> {
    constructor
        .map(|ctor| {
            ctor.params
                .iter()
                .map(|p| DepRef {
                    name: param_type_name(p),
                    span: p.span.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn param_type_name(param: &Param) -> String {
    match &param.ty {
        crate::parser::ast::TypeExpr::Named(name) => name.clone(),
        crate::parser::ast::TypeExpr::Generic { name, .. } => name.clone(),
        crate::parser::ast::TypeExpr::Result { .. } => "Result".to_string(),
        crate::parser::ast::TypeExpr::Option(_) => "Option".to_string(),
    }
}

fn is_builtin_type(name: &str) -> bool {
    matches!(
        name,
        "Int" | "Float" | "Bool" | "String" | "Any" | "Result" | "Option" | "List" | "Map"
    )
}

fn default_span() -> Span {
    Span {
        file: std::path::PathBuf::from("<module>"),
        line_start: 0,
        col_start: 0,
        line_end: 0,
        col_end: 0,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::parser::ast::{
        Annotation, ClassDecl, ConstructorDecl, ModuleDecl, Param, ProviderBinding, SourceFile,
        Span, TopLevelItem, TypeExpr,
    };

    use super::{ModuleError, ModuleGraphBuilder};

    fn span() -> Span {
        Span {
            file: PathBuf::from("test.rw"),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 1,
        }
    }

    fn class(name: &str, deps: Vec<&str>) -> ClassDecl {
        ClassDecl {
            annotations: vec![Annotation {
                name: "injectable".into(),
                args: vec![],
                span: span(),
            }],
            name: name.into(),
            implements: vec![],
            constructor: Some(ConstructorDecl {
                params: deps
                    .into_iter()
                    .map(|dep| Param {
                        annotations: vec![],
                        name: format!("{}_dep", dep),
                        ty: TypeExpr::Named(dep.into()),
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

    fn module(
        name: &str,
        imports: Vec<&str>,
        providers: Vec<ProviderBinding>,
        controllers: Vec<&str>,
        exports: Vec<&str>,
    ) -> ModuleDecl {
        ModuleDecl {
            name: name.into(),
            imports: imports.into_iter().map(|v| v.into()).collect(),
            providers,
            controllers: controllers.into_iter().map(|v| v.into()).collect(),
            exports: exports.into_iter().map(|v| v.into()).collect(),
            span: span(),
        }
    }

    #[test]
    fn builds_simple_module_graph() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Module(module(
                "A",
                vec![],
                vec![],
                vec![],
                vec![],
            ))],
        };
        let mut builder = ModuleGraphBuilder::new();
        let graph = builder.build(&ast).expect("should build");
        assert!(graph.modules.contains_key("A"));
    }

    #[test]
    fn detects_unknown_import() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Module(module(
                "A",
                vec!["B"],
                vec![],
                vec![],
                vec![],
            ))],
        };
        let mut builder = ModuleGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, ModuleError::UnknownImport { import, .. } if import == "B")));
    }

    #[test]
    fn detects_circular_import() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Module(module("A", vec!["B"], vec![], vec![], vec![])),
                TopLevelItem::Module(module("B", vec!["A"], vec![], vec![], vec![])),
            ],
        };
        let mut builder = ModuleGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, ModuleError::CircularImport { .. })));
    }

    #[test]
    fn validates_controller_dependencies_satisfied() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("UserService", vec![])),
                TopLevelItem::Class(class("UserController", vec!["UserService"])),
                TopLevelItem::Module(module(
                    "App",
                    vec![],
                    vec![ProviderBinding::Simple("UserService".into())],
                    vec!["UserController"],
                    vec![],
                )),
            ],
        };
        let mut builder = ModuleGraphBuilder::new();
        assert!(builder.build(&ast).is_ok());
    }

    #[test]
    fn detects_private_provider_access() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("UserService", vec![])),
                TopLevelItem::Class(class("UserController", vec!["UserService"])),
                TopLevelItem::Module(module(
                    "Core",
                    vec![],
                    vec![ProviderBinding::Simple("UserService".into())],
                    vec![],
                    vec![],
                )),
                TopLevelItem::Module(module(
                    "App",
                    vec!["Core"],
                    vec![],
                    vec!["UserController"],
                    vec![],
                )),
            ],
        };
        let mut builder = ModuleGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, ModuleError::PrivateProvider { provider, .. } if provider == "UserService")));
    }

    #[test]
    fn validates_exports_match_providers() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Module(module(
                "App",
                vec![],
                vec![],
                vec![],
                vec!["Missing"],
            ))],
        };
        let mut builder = ModuleGraphBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, ModuleError::ExportWithoutProvider { symbol, .. } if symbol == "Missing")));
    }

    #[test]
    fn multi_module_with_proper_imports_passes() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("UserService", vec![])),
                TopLevelItem::Class(class("UserController", vec!["UserService"])),
                TopLevelItem::Module(module(
                    "Core",
                    vec![],
                    vec![ProviderBinding::Simple("UserService".into())],
                    vec![],
                    vec!["UserService"],
                )),
                TopLevelItem::Module(module(
                    "App",
                    vec!["Core"],
                    vec![],
                    vec!["UserController"],
                    vec![],
                )),
            ],
        };
        let mut builder = ModuleGraphBuilder::new();
        assert!(builder.build(&ast).is_ok());
    }
}
