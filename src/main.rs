use build_hunter::{ScanControl, ScanOptions, rust_cli_policy, scan_with_policy};
use std::{env, fs, path::PathBuf, process, time::Instant};

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
    let mut language = "all".to_owned();
    let mut environments = false;
    let mut apparent = false;
    let mut json = false;
    let mut root = None;
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
                language = args
                    .next()
                    .and_then(|v| v.into_string().ok())
                    .unwrap_or_default();
            }
            Some("--json") => json = true,
            Some("--apparent") => apparent = true,
            Some("--include-envs") => environments = true,
            _ if arg.to_string_lossy().starts_with('-') || root.is_some() => {
                eprintln!("Unexpected argument: {}", arg.to_string_lossy());
                process::exit(2);
            }
            _ => root = Some(PathBuf::from(arg)),
        }
    }
    if !["all", "swift", "rust", "python"].contains(&language.as_str()) {
        eprintln!("Invalid --language; use all, swift, rust or python");
        process::exit(2);
    }
    let root = match fs::canonicalize(root.unwrap_or_else(|| PathBuf::from("."))) {
        Ok(path) if path.is_dir() => path,
        _ => {
            eprintln!("PATH must be an accessible directory");
            process::exit(2);
        }
    };
    let start = Instant::now();
    let control = ScanControl::new();
    let report = scan_with_policy(
        &root,
        ScanOptions {
            apparent_size: apparent,
        },
        &control,
        |facts| {
            let action = rust_cli_policy(facts, environments);
            match action {
                build_hunter::CandidateAction::Classify(classification)
                    if language != "all"
                        && classification.language
                            != match language.as_str() {
                                "swift" => 1,
                                "rust" => 2,
                                "python" => 3,
                                _ => 0,
                            } =>
                {
                    build_hunter::CandidateAction::Traverse
                }
                other => other,
            }
        },
        |_| {},
    );
    let mut rows = report.artifacts;
    rows.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then(a.relative_path.cmp(&b.relative_path))
    });
    let total: u64 = rows.iter().filter(|r| !r.nested).map(|r| r.bytes).sum();
    if json {
        println!(
            "{{\"root\":{},\"total_bytes\":{total},\"elapsed_seconds\":{:.3},\"artifacts\":[",
            quoted(&root.to_string_lossy()),
            start.elapsed().as_secs_f64()
        );
        for (i, row) in rows.iter().enumerate() {
            println!(
                "{}{{\"path\":{},\"language\":{},\"kind\":{},\"bytes\":{},\"nested\":{}}}",
                if i == 0 { "" } else { "," },
                quoted(&root.join(&row.relative_path).to_string_lossy()),
                quoted(row.language),
                quoted(row.kind),
                row.bytes,
                row.nested
            );
        }
        println!(
            "],\"errors\":[{}]}}",
            report
                .warnings
                .iter()
                .map(|warning| {
                    quoted(&format!(
                        "{}: {}",
                        root.join(&warning.relative_path).display(),
                        warning.message
                    ))
                })
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
        for row in &rows {
            println!(
                "{:>14}  {:<7}  {:<11}  {}{}",
                human(row.bytes),
                row.language,
                row.kind,
                row.relative_path.display(),
                if row.nested { " [nested]" } else { "" }
            );
        }
        println!(
            "\n{} artifacts; total: {} ({total} bytes); {:.3}s",
            rows.len(),
            human(total),
            start.elapsed().as_secs_f64()
        );
        for warning in &report.warnings {
            eprintln!(
                "Warning: {}: {}",
                root.join(&warning.relative_path).display(),
                warning.message
            );
        }
    }
    if !report.warnings.is_empty() {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn escapes_json() {
        assert_eq!(quoted("a\n\"\\"), "\"a\\u000a\\\"\\\\\"");
    }

    #[test]
    fn shared_scanner_preserves_nested_and_language_filter_behavior() {
        let root = env::temp_dir().join(format!(
            "build-hunter-cli-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join(".build/__pycache__")).unwrap();
        fs::write(root.join(".build/__pycache__/x.pyc"), b"123456").unwrap();
        fs::create_dir_all(root.join("target")).unwrap();
        fs::create_dir_all(root.join("dist")).unwrap();
        fs::write(root.join("Cargo.toml"), "").unwrap();
        fs::write(root.join("pyproject.toml"), "").unwrap();
        let run = |filter: u32| {
            let control = ScanControl::new();
            scan_with_policy(
                &root,
                ScanOptions {
                    apparent_size: true,
                },
                &control,
                |facts| match rust_cli_policy(facts, false) {
                    build_hunter::CandidateAction::Classify(c)
                        if filter != 0 && c.language != filter =>
                    {
                        build_hunter::CandidateAction::Traverse
                    }
                    action => action,
                },
                |_| {},
            )
        };
        let swift = run(1);
        assert_eq!(swift.artifacts.len(), 1);
        assert_eq!(swift.artifacts[0].language, "swift");
        let all = run(0);
        assert_eq!(
            all.artifacts
                .iter()
                .filter(|artifact| artifact.nested)
                .count(),
            2
        );
        assert!(all.total_bytes() >= 6);
        assert!(all.warnings.is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}
