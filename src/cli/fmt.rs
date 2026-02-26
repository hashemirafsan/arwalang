use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;

/// CLI options for `arwa fmt`.
#[derive(Debug, Clone, Args)]
pub struct FmtArgs {
    /// Optional file/directory path to format (defaults to current directory).
    #[arg(value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Check mode: fail if formatting changes would be required.
    #[arg(long)]
    pub check: bool,
}

/// Formats `.rw` files under target path.
pub fn execute_fmt(args: &FmtArgs) -> Result<usize, String> {
    let root = args.path.clone().unwrap_or_else(|| PathBuf::from("."));
    let files = collect_rw_files(&root)?;
    let mut changed = 0usize;
    let mut needs_changes = Vec::new();

    for file in files {
        let original = fs::read_to_string(&file)
            .map_err(|err| format!("fmt: failed reading '{}': {err}", file.display()))?;
        let formatted = format_rw_source(&original);

        if formatted != original {
            if args.check {
                needs_changes.push(file.display().to_string());
            } else {
                fs::write(&file, formatted)
                    .map_err(|err| format!("fmt: failed writing '{}': {err}", file.display()))?;
                changed += 1;
            }
        }
    }

    if args.check && !needs_changes.is_empty() {
        return Err(format!(
            "fmt: {} file(s) need formatting:\n{}",
            needs_changes.len(),
            needs_changes.join("\n")
        ));
    }

    Ok(changed)
}

fn collect_rw_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    visit_rw_files(root, &mut out)?;
    out.sort();
    Ok(out)
}

fn visit_rw_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) == Some("rw") {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }

    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path)
        .map_err(|err| format!("fmt: failed reading dir '{}': {err}", path.display()))?
    {
        let entry = entry.map_err(|err| format!("fmt: failed reading dir entry: {err}"))?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            visit_rw_files(&entry_path, out)?;
        } else if entry_path.extension().and_then(|e| e.to_str()) == Some("rw") {
            out.push(entry_path);
        }
    }
    Ok(())
}

fn format_rw_source(source: &str) -> String {
    let mut lines: Vec<String> = source
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect();

    sort_leading_imports(&mut lines);

    let mut formatted = Vec::new();
    let mut indent = 0usize;
    for raw in lines {
        let line = raw.trim();
        if line.is_empty() {
            formatted.push(String::new());
            continue;
        }

        if line.starts_with('}') && indent > 0 {
            indent -= 1;
        }

        formatted.push(format!("{}{}", "  ".repeat(indent), line));

        let opens = line.chars().filter(|ch| *ch == '{').count();
        let closes = line.chars().filter(|ch| *ch == '}').count();
        if opens > closes {
            indent += opens - closes;
        } else if closes > opens {
            indent = indent.saturating_sub(closes - opens);
        }
    }

    let mut output = formatted.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn sort_leading_imports(lines: &mut [String]) {
    let mut first = None;
    let mut last = None;

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            if first.is_none() {
                first = Some(idx);
            }
            last = Some(idx);
        } else if !trimmed.is_empty() && first.is_some() {
            break;
        }
    }

    if let (Some(start), Some(end)) = (first, last) {
        let mut imports: Vec<String> = lines[start..=end]
            .iter()
            .filter(|line| line.trim().starts_with("import "))
            .cloned()
            .collect();
        imports.sort();

        let mut cursor = 0usize;
        for line in &mut lines[start..=end] {
            if line.trim().starts_with("import ") {
                *line = imports[cursor].clone();
                cursor += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{execute_fmt, FmtArgs};

    #[test]
    fn fmt_formats_rw_file_in_place() {
        let unique = format!(
            "arwa-fmt-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create base dir");
        let file = base.join("main.rw");

        fs::write(
            &file,
            "import Z\nimport A\nmodule App {\nprovide C\ncontrol C\n}\n",
        )
        .expect("write source");

        let changed = execute_fmt(&FmtArgs {
            path: Some(base.clone()),
            check: false,
        })
        .expect("fmt should pass");
        assert_eq!(changed, 1);

        let formatted = fs::read_to_string(&file).expect("read formatted source");
        assert!(formatted.contains("import A\nimport Z"));
        assert!(formatted.contains("  provide C"));

        fs::remove_file(file).expect("cleanup source");
        fs::remove_dir(base).expect("cleanup base");
    }

    #[test]
    fn fmt_check_reports_needed_changes() {
        let unique = format!(
            "arwa-fmt-check-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be valid")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        fs::create_dir_all(&base).expect("create base dir");
        let file = base.join("main.rw");

        fs::write(&file, "module App {\nprovide X\n}\n").expect("write source");

        let err = execute_fmt(&FmtArgs {
            path: Some(base.clone()),
            check: true,
        })
        .expect_err("check mode should fail when file needs formatting");
        assert!(err.contains("need formatting"));

        fs::remove_file(file).expect("cleanup source");
        fs::remove_dir(base).expect("cleanup base");
    }
}
