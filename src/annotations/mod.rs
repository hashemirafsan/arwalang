#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::parser::ast::{
    Annotation, AnnotationArg, ClassDecl, Expr, MethodDecl, Param, SourceFile, Span, TopLevelItem,
};

/// Annotation-related semantic validation errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum AnnotationError {
    #[error("unknown annotation '{name}'")]
    UnknownAnnotation { name: String, span: Span },

    #[error("missing argument for annotation '{name}'")]
    MissingArgument { name: String, span: Span },

    #[error("invalid argument for annotation '{name}': {message}")]
    InvalidArgument {
        name: String,
        message: String,
        span: Span,
    },

    #[error("annotation '{name}' is not valid on {target}")]
    InvalidTarget {
        name: String,
        target: String,
        span: Span,
    },

    #[error("unbound route param '{param}' in route '{route}'")]
    UnboundRouteParam {
        param: String,
        route: String,
        span: Span,
    },

    #[error("duplicate #[body] annotation in method parameters")]
    DuplicateBody { span: Span },
}

/// Valid annotation target categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnnotationTarget {
    Class,
    Method,
    Param,
    TypeDecl,
}

impl AnnotationTarget {
    /// Returns a human-readable target label used in diagnostics.
    ///
    /// Why this is needed:
    /// - invalid-target errors should describe the actual AST context clearly.
    fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Method => "method",
            Self::Param => "parameter",
            Self::TypeDecl => "type declaration",
        }
    }
}

#[derive(Debug, Clone)]
struct AnnotationMetadata {
    name: &'static str,
    targets: Vec<AnnotationTarget>,
}

/// Registry of known built-in annotations.
#[derive(Debug, Default)]
pub struct AnnotationRegistry {
    known: HashMap<String, AnnotationMetadata>,
}

impl AnnotationRegistry {
    /// Creates registry with all v1 built-in annotations.
    pub fn with_builtins() -> Self {
        let mut registry = Self::default();
        registry.register_annotation(AnnotationMetadata {
            name: "injectable",
            targets: vec![AnnotationTarget::Class],
        });
        registry.register_annotation(AnnotationMetadata {
            name: "controller",
            targets: vec![AnnotationTarget::Class],
        });
        for name in ["get", "post", "put", "delete", "patch"] {
            registry.register_annotation(AnnotationMetadata {
                name,
                targets: vec![AnnotationTarget::Method],
            });
        }
        for name in ["param", "query", "body", "header"] {
            registry.register_annotation(AnnotationMetadata {
                name,
                targets: vec![AnnotationTarget::Param],
            });
        }
        for name in ["use_guards", "use_interceptors", "use_pipes"] {
            registry.register_annotation(AnnotationMetadata {
                name,
                targets: vec![AnnotationTarget::Class, AnnotationTarget::Method],
            });
        }
        registry
    }

    /// Registers one annotation definition into the known-annotation map.
    ///
    /// Why this is needed:
    /// - keeps built-in annotation configuration centralized and extensible.
    fn register_annotation(&mut self, metadata: AnnotationMetadata) {
        self.known.insert(metadata.name.to_string(), metadata);
    }

    /// Looks up annotation metadata by annotation name.
    ///
    /// Why this is needed:
    /// - validator must first resolve annotation identity before checking targets/args.
    fn get(&self, name: &str) -> Option<&AnnotationMetadata> {
        self.known.get(name)
    }
}

/// Annotation processing and validation pass.
#[derive(Debug)]
pub struct AnnotationProcessor {
    registry: AnnotationRegistry,
    class_names: HashSet<String>,
    errors: Vec<AnnotationError>,
}

impl Default for AnnotationProcessor {
    /// Creates processor with built-in registry and empty per-run state.
    ///
    /// Why this is needed:
    /// - each analysis run must start from deterministic empty caches/errors.
    fn default() -> Self {
        Self {
            registry: AnnotationRegistry::with_builtins(),
            class_names: HashSet::new(),
            errors: Vec::new(),
        }
    }
}

impl AnnotationProcessor {
    /// Creates processor with built-in annotation registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Processes all annotations in AST and validates targets/arguments.
    pub fn process_source_file(&mut self, ast: &SourceFile) -> Result<(), Vec<AnnotationError>> {
        self.errors.clear();
        self.class_names = ast
            .items
            .iter()
            .filter_map(|item| match item {
                TopLevelItem::Class(class) => Some(class.name.clone()),
                _ => None,
            })
            .collect();

        for item in &ast.items {
            match item {
                TopLevelItem::Class(class) => self.process_class(class),
                TopLevelItem::Struct(struct_decl) => {
                    for ann in &struct_decl.annotations {
                        self.validate_annotation(ann, AnnotationTarget::TypeDecl);
                    }
                }
                TopLevelItem::Interface(interface_decl) => {
                    for ann in &interface_decl.annotations {
                        self.validate_annotation(ann, AnnotationTarget::TypeDecl);
                    }
                }
                TopLevelItem::Enum(enum_decl) => {
                    for ann in &enum_decl.annotations {
                        self.validate_annotation(ann, AnnotationTarget::TypeDecl);
                    }
                }
                TopLevelItem::Module(_) | TopLevelItem::Import(_) => {}
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Processes one class: class annotations first, then method/param annotations.
    ///
    /// Why this is needed:
    /// - class-level metadata (like controller prefix) influences method-level checks.
    fn process_class(&mut self, class: &ClassDecl) {
        for ann in &class.annotations {
            self.validate_annotation(ann, AnnotationTarget::Class);
            self.validate_specific(ann, AnnotationTarget::Class);
        }

        let class_prefix = self.controller_prefix(class);

        for method in &class.methods {
            self.process_method(method, class_prefix.as_deref());
        }
    }

    /// Processes one method and validates route/param/body annotation consistency.
    ///
    /// Why this is needed:
    /// - method-level HTTP routing contracts are enforced at compile time.
    fn process_method(&mut self, method: &MethodDecl, class_prefix: Option<&str>) {
        for ann in &method.annotations {
            self.validate_annotation(ann, AnnotationTarget::Method);
            self.validate_specific(ann, AnnotationTarget::Method);
        }

        let method_path = self.http_method_path(method);
        let full_route = match (class_prefix, method_path) {
            (Some(prefix), Some(path)) => Some(join_route(prefix, &path)),
            (None, Some(path)) => Some(join_route("", &path)),
            _ => None,
        };

        let mut body_count = 0_usize;
        let mut param_annotations = Vec::new();

        for param in &method.params {
            self.process_param(param);
            for ann in &param.annotations {
                if ann.name == "body" {
                    body_count += 1;
                }
                if ann.name == "param" {
                    if let Some(name) = first_string_arg(ann) {
                        param_annotations.push((name, ann.span.clone()));
                    }
                }
            }
        }

        if body_count > 1 {
            self.errors.push(AnnotationError::DuplicateBody {
                span: method.span.clone(),
            });
        }

        if let Some(route) = full_route {
            let route_params = extract_route_params(&route);
            let annotated: HashSet<String> = param_annotations
                .iter()
                .map(|(name, _)| name.clone())
                .collect();

            for (name, span) in &param_annotations {
                if !route_params.contains(name) {
                    self.errors.push(AnnotationError::UnboundRouteParam {
                        param: name.clone(),
                        route: route.clone(),
                        span: span.clone(),
                    });
                }
            }

            for route_param in route_params {
                if !annotated.contains(&route_param) {
                    self.errors.push(AnnotationError::UnboundRouteParam {
                        param: route_param,
                        route: route.clone(),
                        span: method.span.clone(),
                    });
                }
            }
        }
    }

    /// Processes parameter annotations for one method parameter.
    ///
    /// Why this is needed:
    /// - parameter-binding decorators have strict target and argument rules.
    fn process_param(&mut self, param: &Param) {
        for ann in &param.annotations {
            self.validate_annotation(ann, AnnotationTarget::Param);
            self.validate_specific(ann, AnnotationTarget::Param);
        }
    }

    /// Validates generic annotation constraints (known name + allowed target).
    ///
    /// Why this is needed:
    /// - phase 5 must reject unknown/invalidly-placed decorators before semantic passes.
    fn validate_annotation(&mut self, ann: &Annotation, target: AnnotationTarget) {
        let Some(metadata) = self.registry.get(&ann.name) else {
            self.errors.push(AnnotationError::UnknownAnnotation {
                name: ann.name.clone(),
                span: ann.span.clone(),
            });
            return;
        };

        if !metadata.targets.contains(&target) {
            self.errors.push(AnnotationError::InvalidTarget {
                name: ann.name.clone(),
                target: target.as_str().to_string(),
                span: ann.span.clone(),
            });
        }
    }

    /// Dispatches annotation-specific argument semantics.
    ///
    /// Why this is needed:
    /// - each built-in annotation has unique argument and behavior requirements.
    fn validate_specific(&mut self, ann: &Annotation, target: AnnotationTarget) {
        match ann.name.as_str() {
            "injectable" => self.validate_injectable(ann),
            "controller" => self.validate_controller(ann, target),
            "get" | "post" | "put" | "delete" | "patch" => self.validate_http_method(ann),
            "param" | "query" | "header" => self.validate_string_single_arg(ann),
            "body" => self.validate_body(ann),
            "use_guards" | "use_interceptors" | "use_pipes" => self.validate_lifecycle_list(ann),
            _ => {}
        }
    }

    /// Validates `#[injectable(...)]` optional scope argument.
    ///
    /// Why this is needed:
    /// - DI scope semantics are compile-time guarantees and must be constrained.
    fn validate_injectable(&mut self, ann: &Annotation) {
        if ann.args.is_empty() {
            return;
        }

        for arg in &ann.args {
            match arg {
                AnnotationArg::Named { key, value } if key == "scope" => {
                    let Expr::StringLit(scope) = value else {
                        self.errors.push(AnnotationError::InvalidArgument {
                            name: ann.name.clone(),
                            message: "scope must be a string literal".to_string(),
                            span: ann.span.clone(),
                        });
                        continue;
                    };

                    if !matches!(scope.as_str(), "singleton" | "request" | "transient") {
                        self.errors.push(AnnotationError::InvalidArgument {
                            name: ann.name.clone(),
                            message: "scope must be singleton, request, or transient".to_string(),
                            span: ann.span.clone(),
                        });
                    }
                }
                AnnotationArg::Named { .. } => {
                    self.errors.push(AnnotationError::InvalidArgument {
                        name: ann.name.clone(),
                        message: "only named argument 'scope' is supported".to_string(),
                        span: ann.span.clone(),
                    });
                }
                AnnotationArg::Positional(_) => {
                    self.errors.push(AnnotationError::InvalidArgument {
                        name: ann.name.clone(),
                        message: "injectable expects optional named scope argument".to_string(),
                        span: ann.span.clone(),
                    });
                }
            }
        }
    }

    /// Validates `#[controller(path)]` requirements.
    ///
    /// Why this is needed:
    /// - controller prefix path is required to build route registry correctly.
    fn validate_controller(&mut self, ann: &Annotation, target: AnnotationTarget) {
        if target != AnnotationTarget::Class {
            return;
        }
        self.validate_required_string_arg(ann);
    }

    /// Validates HTTP method decorators like `#[get("/x")]`.
    ///
    /// Why this is needed:
    /// - route builder requires explicit method path strings.
    fn validate_http_method(&mut self, ann: &Annotation) {
        self.validate_required_string_arg(ann);
    }

    /// Validates single-string-argument decorators (`param/query/header`).
    ///
    /// Why this is needed:
    /// - binding decorators must name exactly which input key they bind.
    fn validate_string_single_arg(&mut self, ann: &Annotation) {
        self.validate_required_string_arg(ann);
    }

    /// Validates `#[body]` decorator argument shape.
    ///
    /// Why this is needed:
    /// - body binding is marker-only and should not accept extra arguments.
    fn validate_body(&mut self, ann: &Annotation) {
        if !ann.args.is_empty() {
            self.errors.push(AnnotationError::InvalidArgument {
                name: ann.name.clone(),
                message: "body takes no arguments".to_string(),
                span: ann.span.clone(),
            });
        }
    }

    /// Validates lifecycle decorator argument list and class references.
    ///
    /// Why this is needed:
    /// - lifecycle pipelines reference concrete classes and must be resolvable.
    fn validate_lifecycle_list(&mut self, ann: &Annotation) {
        if ann.args.is_empty() {
            self.errors.push(AnnotationError::MissingArgument {
                name: ann.name.clone(),
                span: ann.span.clone(),
            });
            return;
        }

        for arg in &ann.args {
            let AnnotationArg::Positional(expr) = arg else {
                self.errors.push(AnnotationError::InvalidArgument {
                    name: ann.name.clone(),
                    message: "lifecycle annotations accept positional class names only".to_string(),
                    span: ann.span.clone(),
                });
                continue;
            };

            let Expr::Ident(class_name) = expr else {
                self.errors.push(AnnotationError::InvalidArgument {
                    name: ann.name.clone(),
                    message: "lifecycle annotation arguments must be class identifiers".to_string(),
                    span: ann.span.clone(),
                });
                continue;
            };

            if !self.class_names.contains(class_name) {
                self.errors.push(AnnotationError::InvalidArgument {
                    name: ann.name.clone(),
                    message: format!("unknown class '{class_name}' in annotation arguments"),
                    span: ann.span.clone(),
                });
            }
        }
    }

    /// Validates that first annotation argument is a required string literal.
    ///
    /// Why this is needed:
    /// - route/binding decorators rely on predictable string arguments.
    fn validate_required_string_arg(&mut self, ann: &Annotation) {
        if ann.args.is_empty() {
            self.errors.push(AnnotationError::MissingArgument {
                name: ann.name.clone(),
                span: ann.span.clone(),
            });
            return;
        }

        if first_string_arg(ann).is_none() {
            self.errors.push(AnnotationError::InvalidArgument {
                name: ann.name.clone(),
                message: "first argument must be a string literal".to_string(),
                span: ann.span.clone(),
            });
        }
    }

    /// Extracts class-level controller prefix path if present.
    ///
    /// Why this is needed:
    /// - full route path checks require combining class prefix + method path.
    fn controller_prefix(&self, class: &ClassDecl) -> Option<String> {
        class
            .annotations
            .iter()
            .find(|ann| ann.name == "controller")
            .and_then(first_string_arg)
    }

    /// Extracts method-level HTTP route path if method has an HTTP decorator.
    ///
    /// Why this is needed:
    /// - route parameter validation only applies to routed handler methods.
    fn http_method_path(&self, method: &MethodDecl) -> Option<String> {
        method
            .annotations
            .iter()
            .find(|ann| {
                matches!(
                    ann.name.as_str(),
                    "get" | "post" | "put" | "delete" | "patch"
                )
            })
            .and_then(first_string_arg)
    }
}

/// Returns first string argument from annotation positional/named forms.
///
/// Why this is needed:
/// - many decorators use the first string value as primary semantic input.
fn first_string_arg(ann: &Annotation) -> Option<String> {
    ann.args.first().and_then(|arg| match arg {
        AnnotationArg::Positional(Expr::StringLit(v)) => Some(v.clone()),
        AnnotationArg::Named {
            value: Expr::StringLit(v),
            ..
        } => Some(v.clone()),
        _ => None,
    })
}

/// Joins controller prefix and method path into one normalized route path.
///
/// Why this is needed:
/// - parameter-binding validation must work on final effective route strings.
fn join_route(prefix: &str, method_path: &str) -> String {
    let p = normalize_path(prefix);
    let m = normalize_path(method_path);
    if p == "/" {
        return m;
    }
    if m == "/" {
        return p;
    }
    format!("{}{}", p.trim_end_matches('/'), m)
}

/// Normalizes route path to leading-slash canonical form.
///
/// Why this is needed:
/// - consistent path normalization prevents false negatives in param checks.
fn normalize_path(path: &str) -> String {
    let mut value = path.trim().to_string();
    if value.is_empty() {
        return "/".to_string();
    }
    if !value.starts_with('/') {
        value.insert(0, '/');
    }
    value
}

/// Extracts `:param` placeholders from route path segments.
///
/// Why this is needed:
/// - compile-time route/param binding validation compares this set with `#[param]`.
fn extract_route_params(route: &str) -> HashSet<String> {
    let mut params = HashSet::new();
    for segment in route.split('/') {
        if let Some(name) = segment.strip_prefix(':') {
            if !name.is_empty() {
                params.insert(name.to_string());
            }
        }
    }
    params
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::parser::ast::{
        Annotation, AnnotationArg, Block, ClassDecl, Expr, MethodDecl, Param, SourceFile, Span,
        TopLevelItem, TypeExpr,
    };

    use super::{AnnotationError, AnnotationProcessor};

    /// Creates a stable synthetic span for unit tests.
    fn span() -> Span {
        Span {
            file: PathBuf::from("test.rw"),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 1,
        }
    }

    /// Builds a test annotation helper.
    fn ann(name: &str, args: Vec<AnnotationArg>) -> Annotation {
        Annotation {
            name: name.to_string(),
            args,
            span: span(),
        }
    }

    /// Builds positional string argument helper for tests.
    fn str_arg(value: &str) -> AnnotationArg {
        AnnotationArg::Positional(Expr::StringLit(value.to_string()))
    }

    /// Builds positional identifier argument helper for tests.
    fn ident_arg(value: &str) -> AnnotationArg {
        AnnotationArg::Positional(Expr::Ident(value.to_string()))
    }

    /// Builds a method test fixture with provided annotations/params.
    fn method_with_annotations(annotations: Vec<Annotation>, params: Vec<Param>) -> MethodDecl {
        MethodDecl {
            annotations,
            name: "m".to_string(),
            params,
            return_type: TypeExpr::Named("Int".to_string()),
            body: Block { stmts: vec![] },
            span: span(),
        }
    }

    /// Builds a parameter test fixture with provided annotations.
    fn param_with_annotations(annotations: Vec<Annotation>, name: &str) -> Param {
        Param {
            annotations,
            name: name.to_string(),
            ty: TypeExpr::Named("Int".to_string()),
            is_private: false,
            span: span(),
        }
    }

    #[test]
    fn validates_known_annotations_successfully() {
        let class = ClassDecl {
            annotations: vec![
                ann("controller", vec![str_arg("/users")]),
                ann("use_guards", vec![ident_arg("AuthGuard")]),
            ],
            name: "UserController".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![method_with_annotations(
                vec![ann("get", vec![str_arg(":id")])],
                vec![param_with_annotations(
                    vec![ann("param", vec![str_arg("id")])],
                    "id",
                )],
            )],
            fields: vec![],
            span: span(),
        };

        let guard = ClassDecl {
            annotations: vec![ann("injectable", vec![])],
            name: "AuthGuard".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![],
            fields: vec![],
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(guard), TopLevelItem::Class(class)],
        };

        let mut processor = AnnotationProcessor::new();
        assert!(processor.process_source_file(&ast).is_ok());
    }

    #[test]
    fn reports_unknown_annotation() {
        let class = ClassDecl {
            annotations: vec![ann("unknown", vec![])],
            name: "A".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![],
            fields: vec![],
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(class)],
        };

        let mut processor = AnnotationProcessor::new();
        let errors = processor
            .process_source_file(&ast)
            .expect_err("should fail");

        assert!(errors.iter().any(
            |e| matches!(e, AnnotationError::UnknownAnnotation { name, .. } if name == "unknown")
        ));
    }

    #[test]
    fn reports_missing_controller_argument() {
        let class = ClassDecl {
            annotations: vec![ann("controller", vec![])],
            name: "A".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![],
            fields: vec![],
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(class)],
        };

        let mut processor = AnnotationProcessor::new();
        let errors = processor
            .process_source_file(&ast)
            .expect_err("should fail");

        assert!(errors.iter().any(
            |e| matches!(e, AnnotationError::MissingArgument { name, .. } if name == "controller")
        ));
    }

    #[test]
    fn reports_invalid_injectable_scope() {
        let class = ClassDecl {
            annotations: vec![ann(
                "injectable",
                vec![AnnotationArg::Named {
                    key: "scope".to_string(),
                    value: Expr::StringLit("weird".to_string()),
                }],
            )],
            name: "A".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![],
            fields: vec![],
            span: span(),
        };
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(class)],
        };
        let mut processor = AnnotationProcessor::new();
        let errors = processor
            .process_source_file(&ast)
            .expect_err("should fail");
        assert!(errors.iter().any(
            |e| matches!(e, AnnotationError::InvalidArgument { name, .. } if name == "injectable")
        ));
    }

    #[test]
    fn reports_duplicate_body_annotation() {
        let class = ClassDecl {
            annotations: vec![ann("controller", vec![str_arg("/x")])],
            name: "C".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![method_with_annotations(
                vec![ann("post", vec![str_arg("/")])],
                vec![
                    param_with_annotations(vec![ann("body", vec![])], "a"),
                    param_with_annotations(vec![ann("body", vec![])], "b"),
                ],
            )],
            fields: vec![],
            span: span(),
        };

        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(class)],
        };

        let mut processor = AnnotationProcessor::new();
        let errors = processor
            .process_source_file(&ast)
            .expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, AnnotationError::DuplicateBody { .. })));
    }

    #[test]
    fn validates_route_param_binding() {
        let class = ClassDecl {
            annotations: vec![ann("controller", vec![str_arg("/users")])],
            name: "C".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![method_with_annotations(
                vec![ann("get", vec![str_arg("/:id")])],
                vec![param_with_annotations(
                    vec![ann("param", vec![str_arg("id")])],
                    "id",
                )],
            )],
            fields: vec![],
            span: span(),
        };
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(class)],
        };
        let mut processor = AnnotationProcessor::new();
        assert!(processor.process_source_file(&ast).is_ok());
    }

    #[test]
    fn reports_unbound_route_param_annotation() {
        let class = ClassDecl {
            annotations: vec![ann("controller", vec![str_arg("/users")])],
            name: "C".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![method_with_annotations(
                vec![ann("get", vec![str_arg("/:id")])],
                vec![param_with_annotations(
                    vec![ann("param", vec![str_arg("other")])],
                    "id",
                )],
            )],
            fields: vec![],
            span: span(),
        };
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(class)],
        };
        let mut processor = AnnotationProcessor::new();
        let errors = processor
            .process_source_file(&ast)
            .expect_err("should fail");
        assert!(errors.iter().any(
            |e| matches!(e, AnnotationError::UnboundRouteParam { param, .. } if param == "other")
        ));
    }

    #[test]
    fn validates_lifecycle_annotation_class_references() {
        let guard = ClassDecl {
            annotations: vec![],
            name: "AuthGuard".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![],
            fields: vec![],
            span: span(),
        };
        let class = ClassDecl {
            annotations: vec![ann("use_guards", vec![ident_arg("AuthGuard")])],
            name: "C".to_string(),
            implements: vec![],
            constructor: None,
            methods: vec![],
            fields: vec![],
            span: span(),
        };
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(guard), TopLevelItem::Class(class)],
        };
        let mut processor = AnnotationProcessor::new();
        assert!(processor.process_source_file(&ast).is_ok());
    }
}
