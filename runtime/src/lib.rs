#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Compiler-emitted route entry payload loaded from linked metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedRouteEntry {
    pub method: String,
    pub path: String,
    pub handler_fn: String,
}

/// Compiler-emitted DI entry payload loaded from linked metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedDiEntry {
    pub token: String,
    pub factory_fn: String,
}

/// Compiler-emitted lifecycle entry payload loaded from linked metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedPipelineEntry {
    pub handler_fn: String,
    pub guards: Vec<String>,
    pub pipes: Vec<String>,
    pub interceptors: Vec<String>,
}

/// Full linked payload tables produced by codegen.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LinkedTables {
    pub routes: Vec<LinkedRouteEntry>,
    pub di_registry: Vec<LinkedDiEntry>,
    pub pipelines: Vec<LinkedPipelineEntry>,
}

/// Snapshot of compiler-emitted static table sizes linked into the binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompilerTableCounts {
    pub route_count: u64,
    pub di_count: u64,
    pub pipeline_count: u64,
}

impl CompilerTableCounts {
    /// Returns total number of static registry entries in all tables.
    pub fn total_entries(self) -> u64 {
        self.route_count + self.di_count + self.pipeline_count
    }
}

#[cfg(not(test))]
unsafe extern "C" {
    #[link_name = "__arwa_route_count"]
    static ARWA_ROUTE_COUNT: u64;
    #[link_name = "__arwa_di_count"]
    static ARWA_DI_COUNT: u64;
    #[link_name = "__arwa_pipeline_count"]
    static ARWA_PIPELINE_COUNT: u64;

    #[link_name = "__arwa_routes_json"]
    static ARWA_ROUTES_JSON: u8;
    #[link_name = "__arwa_routes_json_len"]
    static ARWA_ROUTES_JSON_LEN: u64;

    #[link_name = "__arwa_di_json"]
    static ARWA_DI_JSON: u8;
    #[link_name = "__arwa_di_json_len"]
    static ARWA_DI_JSON_LEN: u64;

    #[link_name = "__arwa_pipelines_json"]
    static ARWA_PIPELINES_JSON: u8;
    #[link_name = "__arwa_pipelines_json_len"]
    static ARWA_PIPELINES_JSON_LEN: u64;
}

/// Runtime process entrypoint linked into generated executables.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    arwa_runtime_start()
}

/// Starts runtime bootstrap lifecycle for generated binaries.
#[no_mangle]
pub extern "C" fn arwa_runtime_start() -> i32 {
    let linked_tables = load_compiler_table_counts();
    let _ = linked_tables.total_entries();
    let linked_payload = load_linked_tables();
    let _ = linked_payload.routes.len() + linked_payload.di_registry.len() + linked_payload.pipelines.len();
    0
}

/// Reads compiler-emitted static table sizes from linked object symbols.
#[cfg(not(test))]
pub fn load_compiler_table_counts() -> CompilerTableCounts {
    unsafe {
        CompilerTableCounts {
            route_count: ARWA_ROUTE_COUNT,
            di_count: ARWA_DI_COUNT,
            pipeline_count: ARWA_PIPELINE_COUNT,
        }
    }
}

/// Reads compiler-emitted linked table payloads from binary symbols.
#[cfg(not(test))]
pub fn load_linked_tables() -> LinkedTables {
    unsafe {
        LinkedTables {
            routes: parse_json_blob::<Vec<LinkedRouteEntry>>(
                &ARWA_ROUTES_JSON,
                ARWA_ROUTES_JSON_LEN as usize,
            ),
            di_registry: parse_json_blob::<Vec<LinkedDiEntry>>(
                &ARWA_DI_JSON,
                ARWA_DI_JSON_LEN as usize,
            ),
            pipelines: parse_json_blob::<Vec<LinkedPipelineEntry>>(
                &ARWA_PIPELINES_JSON,
                ARWA_PIPELINES_JSON_LEN as usize,
            ),
        }
    }
}

#[cfg(not(test))]
unsafe fn parse_json_blob<T>(head: &u8, len: usize) -> T
where
    T: for<'de> Deserialize<'de> + Default,
{
    if len == 0 {
        return T::default();
    }
    let ptr = head as *const u8;
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    serde_json::from_slice(bytes).unwrap_or_default()
}

/// Test fallback when compiler symbols are not linked.
#[cfg(test)]
pub fn load_compiler_table_counts() -> CompilerTableCounts {
    CompilerTableCounts::default()
}

/// Test fallback when compiler symbols are not linked.
#[cfg(test)]
pub fn load_linked_tables() -> LinkedTables {
    LinkedTables::default()
}

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
    use super::{
        CompilerTableCounts, DiEntry, PipelineEntry, Request, RouteEntry, Runtime,
        load_compiler_table_counts, load_linked_tables,
    };

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

    #[test]
    fn compiler_table_counts_total_entries_works() {
        let counts = CompilerTableCounts {
            route_count: 2,
            di_count: 3,
            pipeline_count: 4,
        };
        assert_eq!(counts.total_entries(), 9);
    }

    #[test]
    fn test_fallback_for_compiler_table_counts_is_zero() {
        let counts = load_compiler_table_counts();
        assert_eq!(counts.route_count, 0);
        assert_eq!(counts.di_count, 0);
        assert_eq!(counts.pipeline_count, 0);
    }

    #[test]
    fn test_fallback_for_linked_tables_is_empty() {
        let tables = load_linked_tables();
        assert!(tables.routes.is_empty());
        assert!(tables.di_registry.is_empty());
        assert!(tables.pipelines.is_empty());
    }
}
