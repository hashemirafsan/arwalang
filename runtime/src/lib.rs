#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Runtime route metadata emitted by the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntry {
    pub method: &'static str,
    pub path: &'static str,
    pub handler_fn: &'static str,
}

/// Runtime DI registry entry emitted by the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiEntry {
    pub token: &'static str,
    pub factory_fn: &'static str,
}

/// Runtime lifecycle metadata emitted by the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineEntry {
    pub handler_fn: &'static str,
    pub guards: &'static [&'static str],
    pub pipes: &'static [&'static str],
    pub interceptors: &'static [&'static str],
}

/// Simplified runtime request envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub path: String,
}

/// Simplified runtime response envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub status: u16,
    pub body: String,
}

/// Runtime container for static compiler-emitted tables.
#[derive(Debug, Clone)]
pub struct Runtime<'a> {
    routes: &'a [RouteEntry],
    di_registry: &'a [DiEntry],
    pipelines: &'a [PipelineEntry],
}

impl<'a> Runtime<'a> {
    /// Creates runtime facade from static compiler output tables.
    pub fn new(
        routes: &'a [RouteEntry],
        di_registry: &'a [DiEntry],
        pipelines: &'a [PipelineEntry],
    ) -> Self {
        Self {
            routes,
            di_registry,
            pipelines,
        }
    }

    /// Resolves route entry for incoming request.
    pub fn resolve_route(&self, request: &Request) -> Option<&RouteEntry> {
        self.routes
            .iter()
            .find(|route| route.method == request.method && route.path == request.path)
    }

    /// Resolves static lifecycle pipeline metadata by handler function.
    pub fn resolve_pipeline(&self, handler_fn: &str) -> Option<&PipelineEntry> {
        self.pipelines
            .iter()
            .find(|pipeline| pipeline.handler_fn == handler_fn)
    }

    /// Resolves DI factory symbol by token.
    pub fn resolve_factory(&self, token: &str) -> Option<&str> {
        self.di_registry
            .iter()
            .find(|entry| entry.token == token)
            .map(|entry| entry.factory_fn)
    }

    /// Executes a minimal request dispatch against static tables.
    pub fn dispatch(&self, request: &Request) -> Response {
        let Some(route) = self.resolve_route(request) else {
            return Response {
                status: 404,
                body: "Not Found".to_string(),
            };
        };

        let _pipeline = self.resolve_pipeline(route.handler_fn);

        Response {
            status: 200,
            body: format!("handler:{}", route.handler_fn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DiEntry, PipelineEntry, Request, RouteEntry, Runtime};

    static ROUTES: &[RouteEntry] = &[RouteEntry {
        method: "GET",
        path: "/users",
        handler_fn: "UserController.list",
    }];

    static DI: &[DiEntry] = &[DiEntry {
        token: "UserService",
        factory_fn: "factory::UserService",
    }];

    static GUARDS: &[&str] = &["AuthGuard"];
    static PIPES: &[&str] = &[];
    static INTERCEPTORS: &[&str] = &["LoggingInterceptor"];
    static PIPELINES: &[PipelineEntry] = &[PipelineEntry {
        handler_fn: "UserController.list",
        guards: GUARDS,
        pipes: PIPES,
        interceptors: INTERCEPTORS,
    }];

    #[test]
    fn resolves_route_for_exact_method_and_path() {
        let runtime = Runtime::new(ROUTES, DI, PIPELINES);
        let request = Request {
            method: "GET".to_string(),
            path: "/users".to_string(),
        };

        let route = runtime.resolve_route(&request).expect("route must exist");
        assert_eq!(route.handler_fn, "UserController.list");
    }

    #[test]
    fn resolves_pipeline_by_handler_name() {
        let runtime = Runtime::new(ROUTES, DI, PIPELINES);
        let pipeline = runtime
            .resolve_pipeline("UserController.list")
            .expect("pipeline must exist");

        assert_eq!(pipeline.guards, &["AuthGuard"]);
        assert_eq!(pipeline.interceptors, &["LoggingInterceptor"]);
    }

    #[test]
    fn resolves_factory_by_token() {
        let runtime = Runtime::new(ROUTES, DI, PIPELINES);
        let factory = runtime
            .resolve_factory("UserService")
            .expect("factory must exist");
        assert_eq!(factory, "factory::UserService");
    }

    #[test]
    fn dispatches_to_handler_for_known_route() {
        let runtime = Runtime::new(ROUTES, DI, PIPELINES);
        let request = Request {
            method: "GET".to_string(),
            path: "/users".to_string(),
        };

        let response = runtime.dispatch(&request);
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "handler:UserController.list");
    }

    #[test]
    fn returns_not_found_for_missing_route() {
        let runtime = Runtime::new(ROUTES, DI, PIPELINES);
        let request = Request {
            method: "GET".to_string(),
            path: "/missing".to_string(),
        };

        let response = runtime.dispatch(&request);
        assert_eq!(response.status, 404);
        assert_eq!(response.body, "Not Found");
    }
}
