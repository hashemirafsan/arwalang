use std::process::Command;

use clap::Args;

use super::build::{execute_build, BuildArgs};

/// CLI options for `arwa run`.
#[derive(Debug, Clone, Args)]
pub struct RunArgs {
    #[command(flatten)]
    pub build: BuildArgs,

    /// Arguments forwarded to generated executable (use after `--`).
    #[arg(last = true)]
    pub forward_args: Vec<String>,
}

/// Builds project and runs produced executable.
pub fn execute_run(args: &RunArgs) -> Result<(), String> {
    let output = execute_build(&args.build)?;

    let status = Command::new(&output)
        .args(&args.forward_args)
        .status()
        .map_err(|err| format!("run: failed to execute '{}': {err}", output.display()))?;

    if !status.success() {
        return Err(format!("run: process exited with status {status}"));
    }

    Ok(())
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

    use super::{execute_build, execute_run, RunArgs};

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

        let dist = base.join("dist");
        let args = RunArgs {
            build: BuildArgs {
                input: Some(input.clone()),
                dist: dist.clone(),
                name: Some("runapp".to_string()),
            },
            forward_args: vec![],
        };

        execute_run(&args).expect("run should succeed");

        let output = dist.join("runapp");
        let object = dist.join("runapp.o");
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
        let args = RunArgs {
            build: BuildArgs {
                input: Some(input.clone()),
                dist: dist.clone(),
                name: Some("runargsapp".to_string()),
            },
            forward_args: vec!["--example".to_string(), "value".to_string()],
        };

        execute_run(&args).expect("run with forwarded args should succeed");

        let output = dist.join("runargsapp");
        let object = dist.join("runargsapp.o");
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
}
