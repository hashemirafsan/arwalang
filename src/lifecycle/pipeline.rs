#![allow(dead_code)]

use std::collections::HashMap;

use thiserror::Error;

use crate::parser::ast::{
    Annotation, AnnotationArg, ClassDecl, Expr, SourceFile, Span, TopLevelItem,
};
use crate::routes::registry::{Route, RouteTable};

/// Lifecycle stages used in per-route execution pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineStage {
    Middleware,
    Guard,
    Pipe,
    Handler,
    Interceptor,
    ExceptionFilter,
}

/// One lifecycle component entry in a stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleComponent {
    pub name: String,
    pub stage: PipelineStage,
    pub span: Span,
}

/// Fully assembled lifecycle pipeline for one route handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pipeline {
    pub middleware: Vec<LifecycleComponent>,
    pub guards: Vec<LifecycleComponent>,
    pub pipes: Vec<LifecycleComponent>,
    pub handler: LifecycleComponent,
    pub interceptors: Vec<LifecycleComponent>,
    pub exception_filters: Vec<LifecycleComponent>,
}

/// Lifecycle assembly and validation errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum LifecycleError {
    #[error("LC001 invalid guard class '{class}'")]
    InvalidGuard { class: String, span: Span },

    #[error("LC002 invalid interceptor class '{class}'")]
    InvalidInterceptor { class: String, span: Span },

    #[error("LC003 invalid pipe class '{class}'")]
    InvalidPipe { class: String, span: Span },

    #[error("LC004 lifecycle class '{class}' is not injectable")]
    NotInjectable { class: String, span: Span },
}

/// Builder for per-route lifecycle pipelines.
#[derive(Debug, Default)]
pub struct PipelineBuilder;

impl PipelineBuilder {
    /// Creates a pipeline builder.
    pub fn new() -> Self {
        Self
    }

    /// Builds pipelines for every route and validates lifecycle class contracts.
    pub fn build(
        &self,
        route_table: &RouteTable,
        ast: &SourceFile,
    ) -> Result<HashMap<Route, Pipeline>, Vec<LifecycleError>> {
        let classes = collect_classes(ast);
        let mut pipelines = HashMap::new();
        let mut errors = Vec::new();

        for route in route_table.get_routes() {
            let Some((class_name, method_name)) = parse_handler_name(&route.handler) else {
                continue;
            };
            let Some(class) = classes.get(class_name) else {
                continue;
            };
            let Some(method) = class.methods.iter().find(|m| m.name == method_name) else {
                continue;
            };

            let guards = collect_lifecycle_components(
                &class.annotations,
                &method.annotations,
                "use_guards",
                PipelineStage::Guard,
            );
            let pipes = collect_lifecycle_components(
                &class.annotations,
                &method.annotations,
                "use_pipes",
                PipelineStage::Pipe,
            );
            let interceptors = collect_lifecycle_components(
                &class.annotations,
                &method.annotations,
                "use_interceptors",
                PipelineStage::Interceptor,
            );

            let pipeline = Pipeline {
                middleware: Vec::new(),
                guards,
                pipes,
                handler: LifecycleComponent {
                    name: route.handler.clone(),
                    stage: PipelineStage::Handler,
                    span: route.span.clone(),
                },
                interceptors,
                exception_filters: Vec::new(),
            };

            validate_pipeline(&pipeline, &classes, &mut errors);
            pipelines.insert(route.clone(), pipeline);
        }

        if errors.is_empty() {
            Ok(pipelines)
        } else {
            Err(errors)
        }
    }
}

fn collect_classes(ast: &SourceFile) -> HashMap<String, ClassDecl> {
    let mut classes = HashMap::new();
    for item in &ast.items {
        if let TopLevelItem::Class(class) = item {
            classes.insert(class.name.clone(), class.clone());
        }
    }
    classes
}

fn parse_handler_name(handler: &str) -> Option<(&str, &str)> {
    handler.split_once('.')
}

fn collect_lifecycle_components(
    class_annotations: &[Annotation],
    method_annotations: &[Annotation],
    annotation_name: &str,
    stage: PipelineStage,
) -> Vec<LifecycleComponent> {
    let mut out = Vec::new();
    out.extend(components_from_annotations(
        class_annotations,
        annotation_name,
        stage,
    ));
    out.extend(components_from_annotations(
        method_annotations,
        annotation_name,
        stage,
    ));
    out
}

fn components_from_annotations(
    annotations: &[Annotation],
    annotation_name: &str,
    stage: PipelineStage,
) -> Vec<LifecycleComponent> {
    let mut out = Vec::new();
    for ann in annotations {
        if ann.name != annotation_name {
            continue;
        }
        for arg in &ann.args {
            if let AnnotationArg::Positional(Expr::Ident(name)) = arg {
                out.push(LifecycleComponent {
                    name: name.clone(),
                    stage,
                    span: ann.span.clone(),
                });
            }
        }
    }
    out
}

fn validate_pipeline(
    pipeline: &Pipeline,
    classes: &HashMap<String, ClassDecl>,
    errors: &mut Vec<LifecycleError>,
) {
    for guard in &pipeline.guards {
        validate_component(guard, classes, "Guard", LifecycleKind::Guard, errors);
    }
    for interceptor in &pipeline.interceptors {
        validate_component(
            interceptor,
            classes,
            "Interceptor",
            LifecycleKind::Interceptor,
            errors,
        );
    }
    for pipe in &pipeline.pipes {
        validate_component(pipe, classes, "Pipe", LifecycleKind::Pipe, errors);
    }
}

#[derive(Debug, Clone, Copy)]
enum LifecycleKind {
    Guard,
    Interceptor,
    Pipe,
}

fn validate_component(
    component: &LifecycleComponent,
    classes: &HashMap<String, ClassDecl>,
    required_interface: &str,
    kind: LifecycleKind,
    errors: &mut Vec<LifecycleError>,
) {
    let Some(class) = classes.get(&component.name) else {
        push_invalid_error(kind, component.name.clone(), component.span.clone(), errors);
        return;
    };

    if !class
        .implements
        .iter()
        .any(|name| name == required_interface)
    {
        push_invalid_error(kind, class.name.clone(), component.span.clone(), errors);
    }
    if !is_injectable(class) {
        errors.push(LifecycleError::NotInjectable {
            class: class.name.clone(),
            span: component.span.clone(),
        });
    }
}

fn push_invalid_error(
    kind: LifecycleKind,
    class: String,
    span: Span,
    errors: &mut Vec<LifecycleError>,
) {
    match kind {
        LifecycleKind::Guard => errors.push(LifecycleError::InvalidGuard { class, span }),
        LifecycleKind::Interceptor => {
            errors.push(LifecycleError::InvalidInterceptor { class, span })
        }
        LifecycleKind::Pipe => errors.push(LifecycleError::InvalidPipe { class, span }),
    }
}

fn is_injectable(class: &ClassDecl) -> bool {
    class.annotations.iter().any(|ann| ann.name == "injectable")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{LifecycleError, PipelineBuilder, PipelineStage};
    use crate::parser::ast::{
        Annotation, AnnotationArg, Block, ClassDecl, Expr, MethodDecl, SourceFile, Span,
        TopLevelItem, TypeExpr,
    };
    use crate::routes::registry::{HttpMethod, Route, RouteTable};

    fn span() -> Span {
        Span {
            file: PathBuf::from("test.rw"),
            line_start: 1,
            col_start: 1,
            line_end: 1,
            col_end: 1,
        }
    }

    fn ann(name: &str, args: Vec<AnnotationArg>) -> Annotation {
        Annotation {
            name: name.into(),
            args,
            span: span(),
        }
    }

    fn ident_arg(name: &str) -> AnnotationArg {
        AnnotationArg::Positional(Expr::Ident(name.into()))
    }

    fn method(name: &str, annotations: Vec<Annotation>) -> MethodDecl {
        MethodDecl {
            annotations,
            name: name.into(),
            params: vec![],
            return_type: TypeExpr::Named("Int".into()),
            body: Block { stmts: vec![] },
            span: span(),
        }
    }

    fn class(
        name: &str,
        annotations: Vec<Annotation>,
        implements: Vec<&str>,
        methods: Vec<MethodDecl>,
    ) -> ClassDecl {
        ClassDecl {
            annotations,
            name: name.into(),
            implements: implements.into_iter().map(|s| s.to_string()).collect(),
            constructor: None,
            methods,
            fields: vec![],
            span: span(),
        }
    }

    fn route(handler: &str) -> Route {
        Route {
            method: HttpMethod::Get,
            path: "/users".into(),
            handler: handler.into(),
            path_params: vec![],
            span: span(),
        }
    }

    fn table_with_handlers(handlers: &[&str]) -> RouteTable {
        let mut table = RouteTable::default();
        for handler in handlers {
            table.add_route(route(handler));
        }
        table
    }

    fn injectable() -> Annotation {
        ann("injectable", vec![])
    }

    #[test]
    fn builds_simple_pipeline() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class(
                    "AuthGuard",
                    vec![injectable()],
                    vec!["Guard"],
                    vec![],
                )),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![ann("use_guards", vec![ident_arg("AuthGuard")])],
                    vec![],
                    vec![method("list", vec![])],
                )),
            ],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let pipelines = PipelineBuilder::new()
            .build(&table, &ast)
            .expect("must pass");
        let pipeline = pipelines.values().next().expect("pipeline exists");
        assert_eq!(pipeline.guards.len(), 1);
        assert_eq!(pipeline.guards[0].name, "AuthGuard");
    }

    #[test]
    fn combines_class_and_method_annotations() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("G1", vec![injectable()], vec!["Guard"], vec![])),
                TopLevelItem::Class(class("G2", vec![injectable()], vec!["Guard"], vec![])),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![ann("use_guards", vec![ident_arg("G1")])],
                    vec![],
                    vec![method(
                        "list",
                        vec![ann("use_guards", vec![ident_arg("G2")])],
                    )],
                )),
            ],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let pipelines = PipelineBuilder::new()
            .build(&table, &ast)
            .expect("must pass");
        let pipeline = pipelines.values().next().expect("pipeline exists");
        assert_eq!(pipeline.guards.len(), 2);
        assert_eq!(pipeline.guards[0].name, "G1");
        assert_eq!(pipeline.guards[1].name, "G2");
    }

    #[test]
    fn maintains_fixed_stage_order() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("G", vec![injectable()], vec!["Guard"], vec![])),
                TopLevelItem::Class(class("P", vec![injectable()], vec!["Pipe"], vec![])),
                TopLevelItem::Class(class("I", vec![injectable()], vec!["Interceptor"], vec![])),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![],
                    vec![],
                    vec![method(
                        "list",
                        vec![
                            ann("use_interceptors", vec![ident_arg("I")]),
                            ann("use_guards", vec![ident_arg("G")]),
                            ann("use_pipes", vec![ident_arg("P")]),
                        ],
                    )],
                )),
            ],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let pipelines = PipelineBuilder::new()
            .build(&table, &ast)
            .expect("must pass");
        let pipeline = pipelines.values().next().expect("pipeline exists");

        assert_eq!(pipeline.guards[0].stage, PipelineStage::Guard);
        assert_eq!(pipeline.pipes[0].stage, PipelineStage::Pipe);
        assert_eq!(pipeline.handler.stage, PipelineStage::Handler);
        assert_eq!(pipeline.interceptors[0].stage, PipelineStage::Interceptor);
    }

    #[test]
    fn errors_on_invalid_guard_interface() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("BadGuard", vec![injectable()], vec![], vec![])),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![ann("use_guards", vec![ident_arg("BadGuard")])],
                    vec![],
                    vec![method("list", vec![])],
                )),
            ],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let errors = PipelineBuilder::new()
            .build(&table, &ast)
            .expect_err("must fail");
        assert!(errors.iter().any(
            |e| matches!(e, LifecycleError::InvalidGuard { class, .. } if class == "BadGuard")
        ));
    }

    #[test]
    fn errors_on_invalid_interceptor_interface() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("BadInterceptor", vec![injectable()], vec![], vec![])),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![],
                    vec![],
                    vec![method(
                        "list",
                        vec![ann("use_interceptors", vec![ident_arg("BadInterceptor")])],
                    )],
                )),
            ],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let errors = PipelineBuilder::new()
            .build(&table, &ast)
            .expect_err("must fail");
        assert!(errors.iter().any(|e| matches!(
            e,
            LifecycleError::InvalidInterceptor { class, .. } if class == "BadInterceptor"
        )));
    }

    #[test]
    fn errors_on_invalid_pipe_interface() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("BadPipe", vec![injectable()], vec![], vec![])),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![],
                    vec![],
                    vec![method(
                        "list",
                        vec![ann("use_pipes", vec![ident_arg("BadPipe")])],
                    )],
                )),
            ],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let errors = PipelineBuilder::new()
            .build(&table, &ast)
            .expect_err("must fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, LifecycleError::InvalidPipe { class, .. } if class == "BadPipe")));
    }

    #[test]
    fn errors_on_non_injectable_component() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("AuthGuard", vec![], vec!["Guard"], vec![])),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![ann("use_guards", vec![ident_arg("AuthGuard")])],
                    vec![],
                    vec![method("list", vec![])],
                )),
            ],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let errors = PipelineBuilder::new()
            .build(&table, &ast)
            .expect_err("must fail");
        assert!(errors.iter().any(
            |e| matches!(e, LifecycleError::NotInjectable { class, .. } if class == "AuthGuard")
        ));
    }

    #[test]
    fn applies_multiple_guards_in_order() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(class("G1", vec![injectable()], vec!["Guard"], vec![])),
                TopLevelItem::Class(class("G2", vec![injectable()], vec!["Guard"], vec![])),
                TopLevelItem::Class(class(
                    "UserController",
                    vec![ann("use_guards", vec![ident_arg("G1"), ident_arg("G2")])],
                    vec![],
                    vec![method("list", vec![])],
                )),
            ],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let pipelines = PipelineBuilder::new()
            .build(&table, &ast)
            .expect("must pass");
        let pipeline = pipelines.values().next().expect("pipeline exists");
        assert_eq!(pipeline.guards.len(), 2);
        assert_eq!(pipeline.guards[0].name, "G1");
        assert_eq!(pipeline.guards[1].name, "G2");
    }

    #[test]
    fn supports_empty_pipeline() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(class(
                "UserController",
                vec![],
                vec![],
                vec![method("list", vec![])],
            ))],
        };
        let table = table_with_handlers(&["UserController.list"]);

        let pipelines = PipelineBuilder::new()
            .build(&table, &ast)
            .expect("must pass");
        let pipeline = pipelines.values().next().expect("pipeline exists");
        assert!(pipeline.guards.is_empty());
        assert!(pipeline.pipes.is_empty());
        assert!(pipeline.interceptors.is_empty());
    }
}
