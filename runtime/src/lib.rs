#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

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

/// Runtime-owned route entry built from linked payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedRouteEntry {
    pub method: String,
    pub path: String,
    pub handler_fn: String,
}

/// Runtime-owned DI entry built from linked payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedDiEntry {
    pub token: String,
    pub factory_fn: String,
}

/// Runtime-owned lifecycle entry built from linked payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedPipelineEntry {
    pub handler_fn: String,
    pub guards: Vec<String>,
    pub pipes: Vec<String>,
    pub interceptors: Vec<String>,
}

/// Runtime-owned table state used for dispatch bootstrap.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OwnedRuntime {
    pub routes: Vec<OwnedRouteEntry>,
    pub di_registry: Vec<OwnedDiEntry>,
    pub pipelines: Vec<OwnedPipelineEntry>,
}

impl OwnedRuntime {
    /// Resolves route entry for incoming request.
    pub fn resolve_route(&self, request: &Request) -> Option<&OwnedRouteEntry> {
        self.routes
            .iter()
            .find(|route| route.method == request.method && route.path == request.path)
    }

    /// Resolves static lifecycle pipeline metadata by handler function.
    pub fn resolve_pipeline(&self, handler_fn: &str) -> Option<&OwnedPipelineEntry> {
        self.pipelines
            .iter()
            .find(|pipeline| pipeline.handler_fn == handler_fn)
    }

    /// Resolves DI factory symbol by token.
    pub fn resolve_factory(&self, token: &str) -> Option<&str> {
        self.di_registry
            .iter()
            .find(|entry| entry.token == token)
            .map(|entry| entry.factory_fn.as_str())
    }

    /// Executes a minimal request dispatch against linked tables.
    pub fn dispatch(&self, request: &Request) -> Response {
        let Some(route) = self.resolve_route(request) else {
            return Response {
                status: 404,
                body: "Not Found".to_string(),
            };
        };

        let _pipeline = self.resolve_pipeline(&route.handler_fn);

        Response {
            status: 200,
            body: format!("handler:{}", route.handler_fn),
        }
    }
}

impl From<LinkedTables> for OwnedRuntime {
    fn from(value: LinkedTables) -> Self {
        Self {
            routes: value
                .routes
                .into_iter()
                .map(|route| OwnedRouteEntry {
                    method: route.method,
                    path: route.path,
                    handler_fn: route.handler_fn,
                })
                .collect(),
            di_registry: value
                .di_registry
                .into_iter()
                .map(|entry| OwnedDiEntry {
                    token: entry.token,
                    factory_fn: entry.factory_fn,
                })
                .collect(),
            pipelines: value
                .pipelines
                .into_iter()
                .map(|pipeline| OwnedPipelineEntry {
                    handler_fn: pipeline.handler_fn,
                    guards: pipeline.guards,
                    pipes: pipeline.pipes,
                    interceptors: pipeline.interceptors,
                })
                .collect(),
        }
    }
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
    let runtime = bootstrap_runtime_from_linked_tables();
    let _probe = bootstrap_dispatch_probe(&runtime);

    if should_start_http_server() {
        let addr = std::env::var("ARWA_RUNTIME_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
        let max_requests = std::env::var("ARWA_RUNTIME_MAX_REQUESTS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok());
        let _ = run_http_server(&runtime, &addr, max_requests);
    }

    0
}

fn should_start_http_server() -> bool {
    std::env::var("ARWA_RUNTIME_SERVE").ok().as_deref() == Some("1")
}

/// Builds runtime-owned state from compiler-emitted linked payload tables.
pub fn bootstrap_runtime_from_linked_tables() -> OwnedRuntime {
    OwnedRuntime::from(load_linked_tables())
}

fn bootstrap_dispatch_probe(runtime: &OwnedRuntime) -> Option<Response> {
    let route = runtime.routes.first()?;
    let request = Request {
        method: route.method.clone(),
        path: route.path.clone(),
    };
    Some(runtime.dispatch(&request))
}

/// Parses minimal HTTP/1.1 request line into runtime request.
pub fn parse_http_request(raw: &str) -> Option<Request> {
    let line = raw.lines().next()?;
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;
    let _version = parts.next()?;
    let path = target.split('?').next().unwrap_or(target).to_string();
    Some(Request { method, path })
}

/// Formats runtime response as minimal HTTP/1.1 payload.
pub fn format_http_response(response: &Response) -> String {
    let status_text = match response.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };

    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response.status,
        status_text,
        response.body.len(),
        response.body
    )
}

/// Runs a minimal HTTP server loop; bounded when max_requests is set.
pub fn run_http_server(
    runtime: &OwnedRuntime,
    bind_addr: &str,
    max_requests: Option<usize>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind_addr)?;
    run_http_server_on_listener(runtime, listener, max_requests)
}

/// Runs HTTP server loop using an existing listener.
pub fn run_http_server_on_listener(
    runtime: &OwnedRuntime,
    listener: TcpListener,
    max_requests: Option<usize>,
) -> std::io::Result<()> {
    let mut handled = 0usize;
    loop {
        if let Some(limit) = max_requests {
            if handled >= limit {
                break;
            }
        }
        let (mut stream, _) = listener.accept()?;
        handle_client(runtime, &mut stream)?;
        handled += 1;
    }
    Ok(())
}

fn handle_client(runtime: &OwnedRuntime, stream: &mut TcpStream) -> std::io::Result<()> {
    let mut buf = [0_u8; 8192];
    let size = stream.read(&mut buf)?;
    let raw = String::from_utf8_lossy(&buf[..size]);

    let response = if let Some(request) = parse_http_request(&raw) {
        runtime.dispatch(&request)
    } else {
        Response {
            status: 400,
            body: "Bad Request".to_string(),
        }
    };

    let payload = format_http_response(&response);
    stream.write_all(payload.as_bytes())?;
    stream.flush()?;
    Ok(())
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
    use std::io::{Read, Write};
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::thread;

    use super::{
        CompilerTableCounts, DiEntry, LinkedDiEntry, LinkedPipelineEntry, LinkedRouteEntry,
        LinkedTables, OwnedRuntime, PipelineEntry, Request, RouteEntry, Runtime,
        bootstrap_runtime_from_linked_tables, format_http_response, load_compiler_table_counts,
        load_linked_tables, parse_http_request, run_http_server_on_listener,
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

    #[test]
    fn owned_runtime_builds_from_linked_tables_and_dispatches() {
        let linked = LinkedTables {
            routes: vec![LinkedRouteEntry {
                method: "GET".to_string(),
                path: "/health".to_string(),
                handler_fn: "HealthController.check".to_string(),
            }],
            di_registry: vec![LinkedDiEntry {
                token: "HealthService".to_string(),
                factory_fn: "factory::HealthService".to_string(),
            }],
            pipelines: vec![LinkedPipelineEntry {
                handler_fn: "HealthController.check".to_string(),
                guards: vec!["AuthGuard".to_string()],
                pipes: vec![],
                interceptors: vec!["LogInterceptor".to_string()],
            }],
        };

        let runtime = OwnedRuntime::from(linked);
        let req = Request {
            method: "GET".to_string(),
            path: "/health".to_string(),
        };
        let res = runtime.dispatch(&req);

        assert_eq!(res.status, 200);
        assert_eq!(res.body, "handler:HealthController.check");
        assert_eq!(
            runtime.resolve_factory("HealthService"),
            Some("factory::HealthService")
        );
    }

    #[test]
    fn bootstrap_runtime_from_linked_tables_uses_fallback_in_tests() {
        let runtime = bootstrap_runtime_from_linked_tables();
        assert!(runtime.routes.is_empty());
        assert!(runtime.di_registry.is_empty());
        assert!(runtime.pipelines.is_empty());
    }

    #[test]
    fn parses_http_request_line_into_runtime_request() {
        let raw = "GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let req = parse_http_request(raw).expect("request should parse");
        assert_eq!(req.method, "GET");
        assert_eq!(req.path, "/health");
    }

    #[test]
    fn formats_http_response_payload() {
        let payload = format_http_response(&super::Response {
            status: 200,
            body: "hello".to_string(),
        });
        assert!(payload.starts_with("HTTP/1.1 200 OK"));
        assert!(payload.contains("Content-Length: 5"));
        assert!(payload.ends_with("hello"));
    }

    #[test]
    fn serves_one_http_request_over_tcp() {
        let runtime = OwnedRuntime::from(LinkedTables {
            routes: vec![LinkedRouteEntry {
                method: "GET".to_string(),
                path: "/health".to_string(),
                handler_fn: "HealthController.check".to_string(),
            }],
            di_registry: vec![],
            pipelines: vec![],
        });

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");

        let server_runtime = runtime.clone();
        let server_thread = thread::spawn(move || {
            run_http_server_on_listener(&server_runtime, listener, Some(1))
                .expect("server should run");
        });

        let mut client = TcpStream::connect(addr).expect("connect client");
        client
            .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("write request");
        client.shutdown(Shutdown::Write).expect("shutdown write");

        let mut response = String::new();
        client.read_to_string(&mut response).expect("read response");

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("handler:HealthController.check"));

        server_thread.join().expect("join server thread");
    }
}
