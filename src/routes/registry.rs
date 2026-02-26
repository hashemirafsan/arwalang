#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::parser::ast::{
    Annotation, AnnotationArg, ClassDecl, MethodDecl, SourceFile, Span, TopLevelItem,
};

/// HTTP methods supported by route registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Get => write!(f, "GET"),
            Self::Post => write!(f, "POST"),
            Self::Put => write!(f, "PUT"),
            Self::Delete => write!(f, "DELETE"),
            Self::Patch => write!(f, "PATCH"),
        }
    }
}

/// One route table entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Route {
    pub method: HttpMethod,
    pub path: String,
    pub handler: String,
    pub path_params: Vec<String>,
    pub span: Span,
}

/// Full route table.
#[derive(Debug, Clone, Default)]
pub struct RouteTable {
    routes: Vec<Route>,
}

impl RouteTable {
    /// Adds route entry.
    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    /// Returns immutable route slice.
    pub fn get_routes(&self) -> &[Route] {
        &self.routes
    }
}

/// Route-building validation errors.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum RouteError {
    #[error("ROUTE001 duplicate route {method} {path}")]
    DuplicateRoute {
        method: HttpMethod,
        path: String,
        handlers: [String; 2],
        span: Span,
    },

    #[error("ROUTE002 unbound path param '{param}' in handler '{handler}'")]
    UnboundPathParam {
        param: String,
        handler: String,
        span: Span,
    },

    #[error("ROUTE003 unused #[param] '{param}' in handler '{handler}'")]
    UnusedParamAnnotation {
        param: String,
        handler: String,
        span: Span,
    },
}

/// Builder for static route table from controller annotations.
#[derive(Debug, Default)]
pub struct RouteTableBuilder;

impl RouteTableBuilder {
    /// Creates route-table builder.
    pub fn new() -> Self {
        Self
    }

    /// Builds and validates route table from AST.
    pub fn build(&self, ast: &SourceFile) -> Result<RouteTable, Vec<RouteError>> {
        let mut table = RouteTable::default();
        let mut errors = Vec::new();

        for item in &ast.items {
            let TopLevelItem::Class(class) = item else {
                continue;
            };
            let Some(prefix) = controller_prefix(class) else {
                continue;
            };

            for method in &class.methods {
                let Some((http_method, method_path)) = http_method_path(method) else {
                    continue;
                };

                let full_path = join_route(&prefix, &method_path);
                let path_params = parse_route_params(&full_path);
                let handler = format!("{}.{}", class.name, method.name);

                table.add_route(Route {
                    method: http_method,
                    path: full_path,
                    handler,
                    path_params,
                    span: method.span.clone(),
                });
            }
        }

        validate_route_table(&table, ast, &mut errors);

        if errors.is_empty() {
            Ok(table)
        } else {
            Err(errors)
        }
    }
}

fn validate_route_table(table: &RouteTable, ast: &SourceFile, errors: &mut Vec<RouteError>) {
    let mut seen: HashMap<(HttpMethod, String), (String, Span)> = HashMap::new();
    let mut method_map: HashMap<String, &MethodDecl> = HashMap::new();
    let mut class_map: HashMap<String, &ClassDecl> = HashMap::new();

    for item in &ast.items {
        if let TopLevelItem::Class(class) = item {
            class_map.insert(class.name.clone(), class);
            for method in &class.methods {
                method_map.insert(format!("{}.{}", class.name, method.name), method);
            }
        }
    }

    for route in table.get_routes() {
        let key = (route.method, route.path.clone());
        if let Some((handler, span)) = seen.get(&key) {
            errors.push(RouteError::DuplicateRoute {
                method: route.method,
                path: route.path.clone(),
                handlers: [handler.clone(), route.handler.clone()],
                span: span.clone(),
            });
        } else {
            seen.insert(key, (route.handler.clone(), route.span.clone()));
        }

        let Some(method_decl) = method_map.get(&route.handler) else {
            continue;
        };

        let annotated = method_param_annotations(method_decl);
        let path_set: HashSet<String> = route.path_params.iter().cloned().collect();

        for path_param in &route.path_params {
            if !annotated.iter().any(|(n, _)| n == path_param) {
                errors.push(RouteError::UnboundPathParam {
                    param: path_param.clone(),
                    handler: route.handler.clone(),
                    span: route.span.clone(),
                });
            }
        }

        for (param_name, span) in annotated {
            if !path_set.contains(&param_name) {
                errors.push(RouteError::UnusedParamAnnotation {
                    param: param_name,
                    handler: route.handler.clone(),
                    span,
                });
            }
        }
    }
}

fn method_param_annotations(method: &MethodDecl) -> Vec<(String, Span)> {
    let mut out = Vec::new();
    for param in &method.params {
        for ann in &param.annotations {
            if ann.name == "param" {
                if let Some(name) = first_string_arg(ann) {
                    out.push((name, ann.span.clone()));
                }
            }
        }
    }
    out
}

fn controller_prefix(class: &ClassDecl) -> Option<String> {
    class
        .annotations
        .iter()
        .find(|ann| ann.name == "controller")
        .and_then(first_string_arg)
}

fn http_method_path(method: &MethodDecl) -> Option<(HttpMethod, String)> {
    for ann in &method.annotations {
        let method_kind = match ann.name.as_str() {
            "get" => Some(HttpMethod::Get),
            "post" => Some(HttpMethod::Post),
            "put" => Some(HttpMethod::Put),
            "delete" => Some(HttpMethod::Delete),
            "patch" => Some(HttpMethod::Patch),
            _ => None,
        };
        if let Some(kind) = method_kind {
            if let Some(path) = first_string_arg(ann) {
                return Some((kind, path));
            }
        }
    }
    None
}

fn first_string_arg(ann: &Annotation) -> Option<String> {
    ann.args.first().and_then(|arg| match arg {
        AnnotationArg::Positional(crate::parser::ast::Expr::StringLit(v)) => Some(v.clone()),
        AnnotationArg::Named {
            value: crate::parser::ast::Expr::StringLit(v),
            ..
        } => Some(v.clone()),
        _ => None,
    })
}

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

fn parse_route_params(path: &str) -> Vec<String> {
    path.split('/')
        .filter_map(|seg| seg.strip_prefix(':'))
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::parser::ast::{
        Annotation, AnnotationArg, Block, ClassDecl, MethodDecl, Param, SourceFile, Span,
        TopLevelItem, TypeExpr,
    };

    use super::{HttpMethod, RouteError, RouteTableBuilder};

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

    fn str_arg(value: &str) -> AnnotationArg {
        AnnotationArg::Positional(crate::parser::ast::Expr::StringLit(value.into()))
    }

    fn method(name: &str, anns: Vec<Annotation>, params: Vec<Param>) -> MethodDecl {
        MethodDecl {
            annotations: anns,
            name: name.into(),
            params,
            return_type: TypeExpr::Named("Int".into()),
            body: Block { stmts: vec![] },
            span: span(),
        }
    }

    fn param_with_param(name: &str, route_name: &str) -> Param {
        Param {
            annotations: vec![ann("param", vec![str_arg(route_name)])],
            name: name.into(),
            ty: TypeExpr::Named("Int".into()),
            is_private: false,
            span: span(),
        }
    }

    fn controller(name: &str, prefix: &str, methods: Vec<MethodDecl>) -> ClassDecl {
        ClassDecl {
            annotations: vec![ann("controller", vec![str_arg(prefix)])],
            name: name.into(),
            implements: vec![],
            constructor: None,
            methods,
            fields: vec![],
            span: span(),
        }
    }

    #[test]
    fn builds_simple_route_table() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(controller(
                "UserController",
                "/users",
                vec![method("list", vec![ann("get", vec![str_arg("/")])], vec![])],
            ))],
        };
        let builder = RouteTableBuilder::new();
        let table = builder.build(&ast).expect("should build");
        assert_eq!(table.get_routes().len(), 1);
    }

    #[test]
    fn constructs_full_paths_from_prefix_and_method_path() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(controller(
                "UserController",
                "/users",
                vec![method(
                    "get",
                    vec![ann("get", vec![str_arg("/:id")])],
                    vec![param_with_param("id", "id")],
                )],
            ))],
        };
        let builder = RouteTableBuilder::new();
        let table = builder.build(&ast).expect("should build");
        assert_eq!(table.get_routes()[0].path, "/users/:id");
    }

    #[test]
    fn detects_duplicate_routes() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(controller(
                    "A",
                    "/users",
                    vec![method("m1", vec![ann("get", vec![str_arg("/")])], vec![])],
                )),
                TopLevelItem::Class(controller(
                    "B",
                    "/users",
                    vec![method("m2", vec![ann("get", vec![str_arg("/")])], vec![])],
                )),
            ],
        };
        let builder = RouteTableBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, RouteError::DuplicateRoute { .. })));
    }

    #[test]
    fn validates_path_parameters() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(controller(
                "A",
                "/users",
                vec![method(
                    "get",
                    vec![ann("get", vec![str_arg("/:id")])],
                    vec![param_with_param("id", "id")],
                )],
            ))],
        };
        let builder = RouteTableBuilder::new();
        assert!(builder.build(&ast).is_ok());
    }

    #[test]
    fn errors_on_unbound_path_param() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(controller(
                "A",
                "/users",
                vec![method(
                    "get",
                    vec![ann("get", vec![str_arg("/:id")])],
                    vec![],
                )],
            ))],
        };
        let builder = RouteTableBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors
            .iter()
            .any(|e| matches!(e, RouteError::UnboundPathParam { param, .. } if param == "id")));
    }

    #[test]
    fn errors_on_unused_param_annotation() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(controller(
                "A",
                "/users",
                vec![method(
                    "get",
                    vec![ann("get", vec![str_arg("/:id")])],
                    vec![param_with_param("other", "other")],
                )],
            ))],
        };
        let builder = RouteTableBuilder::new();
        let errors = builder.build(&ast).expect_err("should fail");
        assert!(errors.iter().any(
            |e| matches!(e, RouteError::UnusedParamAnnotation { param, .. } if param == "other")
        ));
    }

    #[test]
    fn supports_multiple_controllers_with_different_prefixes() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![
                TopLevelItem::Class(controller(
                    "A",
                    "/users",
                    vec![method("list", vec![ann("get", vec![str_arg("/")])], vec![])],
                )),
                TopLevelItem::Class(controller(
                    "B",
                    "/posts",
                    vec![method("list", vec![ann("get", vec![str_arg("/")])], vec![])],
                )),
            ],
        };
        let builder = RouteTableBuilder::new();
        let table = builder.build(&ast).expect("should build");
        assert_eq!(table.get_routes().len(), 2);
    }

    #[test]
    fn supports_all_http_methods() {
        let ast = SourceFile {
            path: PathBuf::from("test.rw"),
            items: vec![TopLevelItem::Class(controller(
                "A",
                "/api",
                vec![
                    method("g", vec![ann("get", vec![str_arg("/g")])], vec![]),
                    method("p", vec![ann("post", vec![str_arg("/p")])], vec![]),
                    method("u", vec![ann("put", vec![str_arg("/u")])], vec![]),
                    method("d", vec![ann("delete", vec![str_arg("/d")])], vec![]),
                    method("pa", vec![ann("patch", vec![str_arg("/pa")])], vec![]),
                ],
            ))],
        };
        let builder = RouteTableBuilder::new();
        let table = builder.build(&ast).expect("should build");
        let methods = table
            .get_routes()
            .iter()
            .map(|r| r.method)
            .collect::<Vec<_>>();
        assert!(methods.contains(&HttpMethod::Get));
        assert!(methods.contains(&HttpMethod::Post));
        assert!(methods.contains(&HttpMethod::Put));
        assert!(methods.contains(&HttpMethod::Delete));
        assert!(methods.contains(&HttpMethod::Patch));
    }
}
