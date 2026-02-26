use std::fs;
use std::path::PathBuf;
use std::process::Command;

use clap::Args;

use super::build::{execute_build, BuildArgs};

/// CLI options for `arwa run`.
#[derive(Debug, Clone, Args)]
pub struct RunArgs {
    #[command(flatten)]
    pub build: BuildArgs,

    /// Host interface for server bind.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port for server bind.
    #[arg(long, default_value_t = 3000)]
    pub port: u16,

    /// Maximum number of served requests before exit.
    #[arg(long, hide = true)]
    pub max_requests: Option<usize>,

    /// Arguments forwarded to generated executable (use after `--`).
    #[arg(last = true)]
    pub forward_args: Vec<String>,
}

/// Builds project and runs produced executable.
pub fn execute_run(args: &RunArgs) -> Result<(), String> {
    let output = execute_build_for_run(&args.build)?;
    let addr = format!("{}:{}", args.host, args.port);

    let mut command = Command::new(&output);
    command
        .env("ARWA_RUNTIME_SERVE", "1")
        .env("ARWA_RUNTIME_ADDR", &addr)
        .args(&args.forward_args);
    if let Some(max_requests) = args.max_requests {
        command.env("ARWA_RUNTIME_MAX_REQUESTS", max_requests.to_string());
    }

    let status = command
        .status()
        .map_err(|err| format!("run: failed to execute '{}': {err}", output.display()))?;

    cleanup_temp_build_artifacts(&output);

    if !status.success() {
        return Err(format!("run: process exited with status {status}"));
    }

    Ok(())
}

fn execute_build_for_run(build_args: &BuildArgs) -> Result<PathBuf, String> {
    let temp_dist = std::env::temp_dir().join(format!(
        "arwa-run-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|err| format!("run: clock error while creating temp output: {err}"))?
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dist).map_err(|err| {
        format!(
            "run: failed creating temp build dir '{}': {err}",
            temp_dist.display()
        )
    })?;

    let mut run_build_args = build_args.clone();
    run_build_args.dist = temp_dist;
    execute_build(&run_build_args)
}

fn cleanup_temp_build_artifacts(executable: &PathBuf) {
    let _ = fs::remove_file(executable);

    let object_path = executable.with_extension("o");
    let _ = fs::remove_file(&object_path);

    if let Some(parent) = executable.parent() {
        let _ = fs::remove_dir(parent);
    }
}

#[cfg(test)]
fn reserve_free_port() -> u16 {
    use std::net::TcpListener;

    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral listener")
        .local_addr()
        .expect("read ephemeral listener addr")
        .port()
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    use std::fs;

    use crate::cli::build::BuildArgs;
    use crate::cli::cwd_test_lock;

    use super::{execute_build, execute_run, reserve_free_port, RunArgs};

    fn request_with_retry(port: u16, path: &str) -> Result<String, String> {
        let mut response = String::new();
        for _ in 0..200 {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(mut stream) => {
                    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n");
                    stream
                        .write_all(request.as_bytes())
                        .map_err(|err| format!("write request failed: {err}"))?;
                    stream
                        .shutdown(std::net::Shutdown::Write)
                        .map_err(|err| format!("shutdown write failed: {err}"))?;
                    stream
                        .read_to_string(&mut response)
                        .map_err(|err| format!("read response failed: {err}"))?;
                    return Ok(response);
                }
                Err(_) => thread::sleep(Duration::from_millis(25)),
            }
        }
        Err("timed out waiting for runtime server".to_string())
    }

    fn minimal_app_source() -> &'static str {
        r#"
module App {
  provide UserController
  control UserController
}

#[injectable]
#[controller("/users")]
class UserController {
  #[get("/")]
  fn list(res: Result<Int, HttpError>): Result<Int, HttpError> {
    return res
  }
}
"#
    }

    #[test]
    fn run_builds_and_executes_binary() {
        let _cwd_guard = cwd_test_lock().lock().expect("acquire cwd lock");

        let unique = format!(
            "arwa-cli-run-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create temp dir");

        let input = base.join("main.rw");
        fs::write(&input, minimal_app_source()).expect("write source");

        let port = reserve_free_port();
        let args = RunArgs {
            build: BuildArgs {
                input: Some(input.clone()),
                dist: base.join("dist"),
                name: Some("runapp".to_string()),
            },
            host: "127.0.0.1".to_string(),
            port,
            max_requests: Some(1),
            forward_args: vec![],
        };

        let join = thread::spawn(move || execute_run(&args));
        let response = request_with_retry(port, "/users").expect("request should succeed");
        assert!(response.starts_with("HTTP/1.1 200 OK"));

        let run_result = join.join().expect("join run thread");
        run_result.expect("run should succeed");

        assert!(
            !base.join("dist").exists(),
            "run should not persist dist artifacts"
        );
        fs::remove_file(input).expect("cleanup input");
        fs::remove_dir(base).expect("cleanup base");
    }

    #[test]
    fn generated_binary_serves_http_response() {
        let unique = format!(
            "arwa-cli-http-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create temp dir");

        let input = base.join("main.rw");
        fs::write(&input, minimal_app_source()).expect("write source");

        let dist = base.join("dist");
        let output = execute_build(&BuildArgs {
            input: Some(input.clone()),
            dist: dist.clone(),
            name: Some("httpapp".to_string()),
        })
        .expect("build should succeed");

        let probe = TcpListener::bind("127.0.0.1:0").expect("bind probe listener");
        let addr = probe.local_addr().expect("read probe addr");
        drop(probe);

        let mut child = Command::new(&output)
            .env("ARWA_RUNTIME_SERVE", "1")
            .env("ARWA_RUNTIME_ADDR", addr.to_string())
            .env("ARWA_RUNTIME_MAX_REQUESTS", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn generated binary");

        let mut response = String::new();
        let mut connected = false;
        for _ in 0..200 {
            match TcpStream::connect(addr) {
                Ok(mut stream) => {
                    connected = true;
                    stream
                        .write_all(b"GET /users HTTP/1.1\r\nHost: localhost\r\n\r\n")
                        .expect("write request");
                    stream
                        .shutdown(std::net::Shutdown::Write)
                        .expect("shutdown write");
                    stream.read_to_string(&mut response).expect("read response");
                    break;
                }
                Err(_) => {
                    if let Some(status) = child.try_wait().expect("poll child status") {
                        let output = child
                            .wait_with_output()
                            .expect("collect child output after early exit");
                        panic!(
                            "server exited early with status {status}; stdout='{}' stderr='{}'",
                            String::from_utf8_lossy(&output.stdout),
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                    thread::sleep(Duration::from_millis(25));
                }
            }
        }

        assert!(connected, "server never accepted connection");
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("handler:UserController.list"));

        let status = child.wait().expect("wait for child");
        assert!(status.success(), "server process failed: {status}");

        let object = dist.join("httpapp.o");
        if object.exists() {
            fs::remove_file(object).expect("cleanup object");
        }
        if output.exists() {
            fs::remove_file(output).expect("cleanup output");
        }
        fs::remove_file(input).expect("cleanup input");
        fs::remove_dir(dist).expect("cleanup dist");
        fs::remove_dir(base).expect("cleanup base");
    }

    #[test]
    fn run_accepts_forwarded_args() {
        let unique = format!(
            "arwa-cli-run-forward-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create temp dir");

        let input = base.join("main.rw");
        fs::write(&input, minimal_app_source()).expect("write source");

        let dist = base.join("dist");
        let port = reserve_free_port();
        let args = RunArgs {
            build: BuildArgs {
                input: Some(input.clone()),
                dist: base.join("dist"),
                name: Some("runargsapp".to_string()),
            },
            host: "127.0.0.1".to_string(),
            port,
            max_requests: Some(1),
            forward_args: vec!["--example".to_string(), "value".to_string()],
        };

        let join = thread::spawn(move || execute_run(&args));
        let response = request_with_retry(port, "/users").expect("request should succeed");
        assert!(response.starts_with("HTTP/1.1 200 OK"));

        join.join()
            .expect("join run thread")
            .expect("run with forwarded args should succeed");

        assert!(!dist.exists(), "run should not persist dist artifacts");
        fs::remove_file(input).expect("cleanup input");
        fs::remove_dir(base).expect("cleanup base");
    }
}
