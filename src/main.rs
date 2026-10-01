#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
    time::Instant,
};

struct Row {
    path: PathBuf,
    language: &'static str,
    kind: &'static str,
    bytes: u64,
    nested: bool,
}
struct Scanner {
    rows: Vec<Row>,
    errors: Vec<String>,
    language: String,
    environments: bool,
    apparent: bool,
}

fn classify(path: &Path, environments: bool) -> Option<(&'static str, &'static str)> {
    let name = path.file_name()?.to_str()?;
    let parent = path.parent()?;
    let python = matches!(name, "build" | "dist")
        && ["pyproject.toml", "setup.py", "setup.cfg"]
            .iter()
            .any(|f| parent.join(f).is_file());
    match name {
        ".build" => Some(("swift", "build")),
        "target" if parent.join("Cargo.toml").is_file() => Some(("rust", "build")),
        "__pycache__" => Some(("python", "bytecode")),
        ".pytest_cache" | ".mypy_cache" | ".ruff_cache" | ".pytype" | ".tox" | ".nox" => {
            Some(("python", "cache"))
        }
        "build" | "dist" if python => Some(("python", "build")),
        ".venv" | "venv" if environments && path.join("pyvenv.cfg").is_file() => {
            Some(("python", "environment"))
        }
        _ if name.ends_with(".egg-info") => Some(("python", "metadata")),
        _ if (name.ends_with(".pyc") || name.ends_with(".pyo")) && path.is_file() => {
            Some(("python", "bytecode"))
        }
        _ => None,
    }
}

impl Scanner {
    fn walk(&mut self, path: &Path, inside: bool) -> u64 {
        let metadata = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) => {
                self.errors.push(format!("{}: {e}", path.display()));
                return 0;
            }
        };
        if metadata.file_type().is_symlink() {
            return 0;
        }
        let found = classify(path, self.environments)
            .filter(|(lang, _)| self.language == "all" || self.language == *lang);
        // Skip source-control data and virtual environments unless explicitly requested.
        if !inside
            && found.is_none()
            && metadata.is_dir()
            && (path.file_name().is_some_and(|n| n == ".git") || path.join("pyvenv.cfg").is_file())
        {
            return 0;
        }
        let active = inside || found.is_some();
        #[cfg(unix)]
        let mut bytes = if self.apparent {
            metadata.len()
        } else {
            metadata.blocks() * 512
        };
        #[cfg(not(unix))]
        let mut bytes = metadata.len();
        if metadata.is_dir() {
            match fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => {
                                bytes = bytes.saturating_add(self.walk(&entry.path(), active))
                            }
                            Err(e) => self.errors.push(format!("{}: {e}", path.display())),
                        }
                    }
                }
                Err(e) => self.errors.push(format!("{}: {e}", path.display())),
            }
        }
        if let Some((language, kind)) = found {
            self.rows.push(Row {
                path: path.to_owned(),
                language,
                kind,
                bytes,
                nested: inside,
            });
        }
        bytes
    }
}

fn human(bytes: u64) -> String {
    let mut size = bytes as f64;
    let mut unit = "B";
    for next in ["KiB", "MiB", "GiB", "TiB", "PiB"] {
        if size < 1024.0 {
            break;
        }
        size /= 1024.0;
        unit = next;
    }
    format!("{size:.2} {unit}")
}
fn quoted(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn main() {
    let mut scanner = Scanner {
        rows: vec![],
        errors: vec![],
        language: "all".into(),
        environments: false,
        apparent: false,
    };
    let mut root = None;
    let mut json = false;
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "build-hunter [PATH] [--language all|swift|rust|python] [--json] [--apparent] [--include-envs]\n\nRead-only recursive scan. Default PATH: current directory.\nSizes: allocated bytes on Unix; apparent bytes on other platforms.\nSymlinks, .git and virtual environments are skipped by default.\nNested artifacts are listed but counted once in total.\nHard links are counted per pathname; sizes are not guaranteed reclaimable space.\nRust custom target directories and global caches are not auto-discovered."
                );
                return;
            }
            Some("--language") => {
                scanner.language = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .unwrap_or_default();
            }
            Some("--json") => json = true,
            Some("--apparent") => scanner.apparent = true,
            Some("--include-envs") => scanner.environments = true,
            _ if arg.to_string_lossy().starts_with('-') || root.is_some() => {
                eprintln!("Unexpected argument: {}", arg.to_string_lossy());
                process::exit(2);
            }
            _ => root = Some(PathBuf::from(arg)),
        }
    }
    if !["all", "swift", "rust", "python"].contains(&scanner.language.as_str()) {
        eprintln!("Invalid --language; use all, swift, rust or python");
        process::exit(2);
    }
    let root = match fs::canonicalize(root.unwrap_or_else(|| PathBuf::from("."))) {
        Ok(p) if p.is_dir() => p,
        _ => {
            eprintln!("PATH must be an accessible directory");
            process::exit(2);
        }
    };
    let start = Instant::now();
    scanner.walk(&root, false);
    scanner
        .rows
        .sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.path.cmp(&b.path)));
    let total: u64 = scanner
        .rows
        .iter()
        .filter(|r| !r.nested)
        .map(|r| r.bytes)
        .sum();
    if json {
        println!(
            "{{\"root\":{},\"total_bytes\":{total},\"elapsed_seconds\":{:.3},\"artifacts\":[",
            quoted(&root.to_string_lossy()),
            start.elapsed().as_secs_f64()
        );
        for (i, r) in scanner.rows.iter().enumerate() {
            println!(
                "{}{{\"path\":{},\"language\":{},\"kind\":{},\"bytes\":{},\"nested\":{}}}",
                if i == 0 { "" } else { "," },
                quoted(&r.path.to_string_lossy()),
                quoted(r.language),
                quoted(r.kind),
                r.bytes,
                r.nested
            );
        }
        println!(
            "],\"errors\":[{}]}}",
            scanner
                .errors
                .iter()
                .map(|s| quoted(s))
                .collect::<Vec<_>>()
                .join(",")
        );
    } else {
        println!(
            "Root: {}\n{:>14}  {:<7}  {:<11}  Path",
            root.display(),
            "Size",
            "Language",
            "Kind"
        );
        for r in &scanner.rows {
            println!(
                "{:>14}  {:<7}  {:<11}  {}{}",
                human(r.bytes),
                r.language,
                r.kind,
                r.path.strip_prefix(&root).unwrap_or(&r.path).display(),
                if r.nested { " [nested]" } else { "" }
            );
        }
        println!(
            "\n{} artifacts; total: {} ({total} bytes); {:.3}s",
            scanner.rows.len(),
            human(total),
            start.elapsed().as_secs_f64()
        );
        for error in &scanner.errors {
            eprintln!("Warning: {error}");
        }
    }
    if !scanner.errors.is_empty() {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn escapes_json() {
        assert_eq!(quoted("a\n\"\\"), "\"a\\u000a\\\"\\\\\"");
    }
    #[test]
    fn nested_and_language_totals() {
        let root = env::temp_dir().join(format!("build-hunter-test-{}", process::id()));
        fs::create_dir_all(root.join(".build/__pycache__")).unwrap();
        fs::write(root.join(".build/__pycache__/x.pyc"), b"123456").unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::create_dir_all(root.join("dist")).unwrap();
        assert!(classify(&root.join("target"), false).is_none());
        assert!(classify(&root.join("dist"), false).is_none());
        fs::write(root.join("Cargo.toml"), "").unwrap();
        fs::write(root.join("pyproject.toml"), "").unwrap();
        assert_eq!(
            classify(&root.join("target"), false),
            Some(("rust", "build"))
        );
        assert_eq!(
            classify(&root.join("dist"), false),
            Some(("python", "build"))
        );
        let mut s = Scanner {
            rows: vec![],
            errors: vec![],
            language: "swift".into(),
            environments: false,
            apparent: true,
        };
        s.walk(&root, false);
        assert_eq!(s.rows.len(), 1);
        assert!(s.rows[0].bytes >= 6);
        s.rows.clear();
        s.language = "all".into();
        s.walk(&root, false);
        assert_eq!(s.rows.iter().filter(|r| r.nested).count(), 2);
        assert!(s.errors.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
