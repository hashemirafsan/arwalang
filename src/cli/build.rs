use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;

use crate::annotations::AnnotationProcessor;
use crate::codegen;
use crate::di::graph::DiGraphBuilder;
use crate::ir::IrGenerator;
use crate::lexer::lexer::Lexer;
use crate::lifecycle::pipeline::PipelineBuilder;
use crate::modules::graph::ModuleGraphBuilder;
use crate::parser::ast::SourceFile;
use crate::parser::parser::Parser;
use crate::resolver::Resolver;
use crate::routes::registry::RouteTableBuilder;
use crate::typechecker::TypeChecker;

/// CLI options for `arwa build`.
#[derive(Debug, Clone, Args)]
pub struct BuildArgs {
    /// Input source file.
    #[arg(value_name = "INPUT")]
    pub input: Option<PathBuf>,

    /// Output dist directory.
    #[arg(long, default_value = "dist")]
    pub dist: PathBuf,

    /// Override output binary name.
    #[arg(long)]
    pub name: Option<String>,
}

/// Executes full build pipeline and returns output binary path.
pub fn execute_build(args: &BuildArgs) -> Result<PathBuf, String> {
    let inputs = resolve_input_paths(args.input.clone())?;
    let ast = load_and_validate_sources(&inputs)?;

    let mut ir = IrGenerator::new()
        .generate_ir(&ast)
        .map_err(|err| format!("ir: {err}"))?;

    if let Some(name) = &args.name {
        ir.name = name.clone();
    }

    codegen::compile_to_executable(&ir, &args.dist).map_err(|err| format!("codegen: {err}"))
}

/// Loads and validates multiple source files through phases 1-9.
pub fn load_and_validate_sources(paths: &[PathBuf]) -> Result<SourceFile, String> {
    if paths.is_empty() {
        return Err("io: no source files provided".to_string());
    }

    let mut merged = SourceFile {
        path: paths[0].clone(),
        items: Vec::new(),
    };

    for path in paths {
        let ast = parse_source_file(path)?;
        merged.items.extend(ast.items);
    }

    validate_semantics(&merged)?;
    Ok(merged)
}

fn parse_source_file(path: &Path) -> Result<SourceFile, String> {
    let source = fs::read_to_string(path)
        .map_err(|err| format!("io: failed reading '{}': {err}", path.display()))?;

    let mut lexer = Lexer::new(source, path.to_path_buf());
    let (tokens, lex_errors) = lexer.tokenize_all();
    if !lex_errors.is_empty() {
        return Err(format_stage_errors("lexer", &lex_errors));
    }

    let mut parser = Parser::new(tokens);
    let ast = parser
        .parse_source_file()
        .map_err(|errors| format_stage_errors("parser", &errors))?;

    Ok(ast)
}

/// Resolves effective source file paths from CLI option or project defaults.
pub fn resolve_input_paths(input: Option<PathBuf>) -> Result<Vec<PathBuf>, String> {
    if let Some(path) = input {
        return Ok(vec![path]);
    }

    let mut src_files = discover_rw_files(&PathBuf::from("src"))?;
    if !src_files.is_empty() {
        src_files.sort();
        return Ok(src_files);
    }

    for candidate in [PathBuf::from("main.rw"), PathBuf::from("src/main.rw")] {
        if candidate.exists() {
            return Ok(vec![candidate]);
        }
    }

    Err("io: no input file provided and none of 'src/main.rw' or 'main.rw' exists".to_string())
}

fn discover_rw_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    visit_dir_for_rw(root, &mut files)?;
    Ok(files)
}

fn visit_dir_for_rw(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir)
        .map_err(|err| format!("io: failed reading directory '{}': {err}", dir.display()))?
    {
        let entry = entry.map_err(|err| format!("io: failed reading directory entry: {err}"))?;
        let path = entry.path();

        if path.is_dir() {
            visit_dir_for_rw(&path, out)?;
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "rw")
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(())
}

fn validate_semantics(ast: &SourceFile) -> Result<(), String> {
    let mut resolver = Resolver::new();
    if let Err(errors) = resolver.resolve_source_file(ast) {
        return Err(format_stage_errors("resolver", &errors));
    }

    let mut checker = TypeChecker::new();
    if let Err(errors) = checker.check_source_file(ast) {
        return Err(format_stage_errors("typechecker", &errors));
    }

    let mut annotations = AnnotationProcessor::new();
    if let Err(errors) = annotations.process_source_file(ast) {
        return Err(format_stage_errors("annotations", &errors));
    }

    let mut di = DiGraphBuilder::new();
    if let Err(errors) = di.build(ast) {
        return Err(format_stage_errors("di", &errors));
    }

    let mut module_graph = ModuleGraphBuilder::new();
    if let Err(errors) = module_graph.build(ast) {
        return Err(format_stage_errors("modules", &errors));
    }

    let route_table = RouteTableBuilder::new()
        .build(ast)
        .map_err(|errors| format_stage_errors("routes", &errors))?;

    if let Err(errors) = PipelineBuilder::new().build(&route_table, ast) {
        return Err(format_stage_errors("lifecycle", &errors));
    }

    Ok(())
}

fn format_stage_errors<T: Display>(stage: &str, errors: &[T]) -> String {
    let details = errors
        .iter()
        .map(|err| format!("- {err}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{stage}: {} error(s)\n{details}", errors.len())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    use super::{execute_build, BuildArgs};

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
    fn build_generates_executable_artifact() {
        let unique = format!(
            "arwa-cli-build-test-{}",
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
        let args = BuildArgs {
            input: Some(input.clone()),
            dist: dist.clone(),
            name: Some("miniapp".to_string()),
        };

        let output = execute_build(&args).expect("build should succeed");
        assert!(output.exists());
        assert_eq!(output, dist.join(PathBuf::from("miniapp")));

        let object = dist.join("miniapp.o");
        if object.exists() {
            fs::remove_file(object).expect("cleanup object");
        }
        fs::remove_file(&output).expect("cleanup output");
        fs::remove_file(&input).expect("cleanup input");
        fs::remove_dir(&dist).expect("cleanup dist");
        fs::remove_dir(&base).expect("cleanup base");
    }

    #[test]
    fn build_discovers_sources_from_src_directory_by_default() {
        let unique = format!(
            "arwa-cli-build-discovery-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        let src = base.join("src");
        fs::create_dir_all(&src).expect("create src dir");

        fs::write(
            src.join("main.rw"),
            r#"
module App {
  provide UserController
  control UserController
}
"#,
        )
        .expect("write main module");

        fs::write(
            src.join("user_controller.rw"),
            r#"
#[injectable]
#[controller("/users")]
class UserController {
  #[get("/")]
  fn list(res: Result<Int, HttpError>): Result<Int, HttpError> {
    return res
  }
}
"#,
        )
        .expect("write controller");

        let old_cwd = env::current_dir().expect("read cwd");
        env::set_current_dir(&base).expect("set cwd to test project");

        let args = BuildArgs {
            input: None,
            dist: PathBuf::from("dist"),
            name: Some("discoverapp".to_string()),
        };

        let output = execute_build(&args).expect("build should succeed from discovered sources");
        assert!(output.exists());

        let object = base.join("dist/discoverapp.o");
        if object.exists() {
            fs::remove_file(object).expect("cleanup object");
        }
        if output.exists() {
            fs::remove_file(&output).expect("cleanup output");
        }

        env::set_current_dir(old_cwd).expect("restore cwd");
        fs::remove_file(src.join("main.rw")).expect("cleanup main source");
        fs::remove_file(src.join("user_controller.rw")).expect("cleanup controller source");
        fs::remove_dir(base.join("dist")).expect("cleanup dist");
        fs::remove_dir(src).expect("cleanup src dir");
        fs::remove_dir(base).expect("cleanup base");
    }
}
