mod profile_report;
use build_hunter::{
    ScanControl, ScanOptions,
    catalog::{CatalogSnapshot, ClientProfile, presentation_kind},
    client_scan::{CatalogScanEvent, scan_client},
};
#[cfg(test)]
use build_hunter::{rust_cli_policy, scan_with_policy};
use std::{collections::BTreeSet, env, fs, path::PathBuf, process, time::Instant};

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

fn parse_exclusion(
    value: &str,
    excluded: &mut BTreeSet<String>,
    catalog: &CatalogSnapshot,
) -> Result<(), String> {
    if value.is_empty() {
        return Err("--exclude requires a non-empty filter ID or language alias".into());
    }
    for id in value.split(',') {
        if id.is_empty() {
            return Err("--exclude contains an empty filter ID".into());
        }
        if catalog.target(id).is_some() {
            excluded.insert(id.to_owned());
            continue;
        }
        let aliases = catalog
            .catalog()
            .targets()
            .iter()
            .filter(|target| target.ecosystem == id)
            .map(|target| target.id.clone())
            .collect::<Vec<_>>();
        if aliases.is_empty() {
            return Err(format!("Unknown exclusion filter: {id}"));
        }
        excluded.extend(aliases);
    }
    Ok(())
}

fn print_filter_catalog(json: bool, catalog: &CatalogSnapshot) {
    // Retain legacy descriptor hints for older callers, but catalog identity,
    // membership, titles and selection all come from the validated plugins.
    let hints: Vec<serde_json::Value> =
        serde_json::from_str(build_hunter::search_filter_catalog_json()).unwrap_or_default();
    let mut descriptors = Vec::new();
    let mut targets = catalog.catalog().targets().iter().collect::<Vec<_>>();
    targets.sort_by(|a, b| {
        hints
            .iter()
            .position(|hint| hint["id"] == a.id)
            .unwrap_or(usize::MAX)
            .cmp(
                &hints
                    .iter()
                    .position(|hint| hint["id"] == b.id)
                    .unwrap_or(usize::MAX),
            )
            .then(a.id.cmp(&b.id))
    });
    for target in targets {
        let group = match target.ecosystem.as_str() {
            "swift" => "Swift",
            "rust" => "Rust",
            "python" => "Python",
            other => other,
        };
        if !json {
            println!("{}\t{}\t{}", target.id, group, target.title);
            continue;
        }
        let mut descriptor = hints
            .iter()
            .find(|hint| hint["id"] == target.id)
            .cloned()
            .unwrap_or_else(
                || serde_json::json!({"nodeNames":[],"suffixes":[],"requiresVenvMarker":false}),
            );
        descriptor["id"] = serde_json::json!(target.id);
        descriptor["title"] = serde_json::json!(target.title);
        descriptor["group"] = serde_json::json!(group);
        descriptor["ecosystem"] = serde_json::json!(target.ecosystem);
        descriptor["catalogDigest"] =
            serde_json::json!(catalog.catalog().catalog_digest().to_string());
        descriptors.push(descriptor);
    }
    if json {
        println!("{}", serde_json::Value::Array(descriptors));
    }
}

fn main() {
    let available = CatalogSnapshot::basic(ClientProfile::CliV1, &[]).unwrap_or_else(|error| {
        eprintln!("Invalid bundled catalog: {error}");
        process::exit(1);
    });
    let mut language = "all".to_owned();
    let mut environments = false;
    let mut apparent = false;
    let mut json = false;
    let mut list_filters = false;
    let mut exclusions = BTreeSet::new();
    let mut root = None;
    let mut profile_path = None;
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => {
                println!(
                    "build-hunter [PATH] [--language all|swift|rust|python] [--exclude ID[,ID...]] [--list-filters] [--json] [--apparent] [--include-envs] [--profile FILE]\n\nRead-only recursive scan. Default PATH: current directory.\nSizes: allocated bytes on Unix; apparent bytes on other platforms.\nSymlinks, .git and virtual environments are skipped by default.\nNested artifacts are listed but counted once in total.\n--profile FILE saves throughput counters and rates as JSON; FILE must not exist.\nMeasured bytes/s describes metadata sizing, not disk reads.\nExclusions select recognized roots; nested artifacts remain discoverable.\nHard links are counted per pathname; sizes are not guaranteed reclaimable space.\nRust custom target directories and global caches are not auto-discovered."
                );
                return;
            }
            Some("--list-filters") => list_filters = true,
            Some("--exclude") => {
                let value = args.next().and_then(|value| value.into_string().ok());
                let Some(value) = value else {
                    eprintln!("--exclude requires a non-empty filter ID or language alias");
                    process::exit(2);
                };
                if value.starts_with('-') {
                    eprintln!("--exclude requires a non-empty filter ID or language alias");
                    process::exit(2);
                }
                if let Err(error) = parse_exclusion(&value, &mut exclusions, &available) {
                    eprintln!("{error}");
                    process::exit(2);
                }
            }
            Some(value) if value.starts_with("--exclude=") => {
                let value = &value["--exclude=".len()..];
                if let Err(error) = parse_exclusion(value, &mut exclusions, &available) {
                    eprintln!("{error}");
                    process::exit(2);
                }
            }
            Some("--language") => {
                language = args
                    .next()
                    .and_then(|v| v.into_string().ok())
                    .unwrap_or_default();
            }
            Some("--profile") => {
                let Some(path) = args
                    .next()
                    .filter(|value| !value.to_string_lossy().starts_with('-') && !value.is_empty())
                else {
                    eprintln!("--profile requires an output file path");
                    process::exit(2);
                };
                profile_path = Some(PathBuf::from(path));
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
    if language != "all"
        && !available
            .catalog()
            .targets()
            .iter()
            .any(|target| target.ecosystem == language)
    {
        eprintln!(
            "Invalid --language; use all or a catalog ecosystem: {}",
            available
                .catalog()
                .targets()
                .iter()
                .map(|target| target.ecosystem.as_str())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>()
                .join(", ")
        );
        process::exit(2);
    }
    if list_filters {
        print_filter_catalog(json, &available);
        return;
    }
    // Root canonicalization and profile preflight can encounter evicted objects
    // before the scanner installs its own per-thread protection.
    let _materialization_guard = match build_hunter::NonMaterializingIoGuard::enter() {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("Could not disable cloud downloads: {error}");
            process::exit(1);
        }
    };
    let root = match fs::canonicalize(root.unwrap_or_else(|| PathBuf::from("."))) {
        Ok(path) if path.is_dir() => path,
        _ => {
            eprintln!("PATH must be an accessible directory");
            process::exit(2);
        }
    };
    // Fail fast for existing paths, but create only after scanning so the output
    // cannot affect artifact sizes when it is saved inside the selected tree.
    if let Some(path) = &profile_path
        && fs::symlink_metadata(path).is_ok()
    {
        eprintln!("Profile output already exists: {}", path.display());
        process::exit(2);
    }
    let mut profile = profile_report::ProfileReport::default();
    let start = Instant::now();
    let control = ScanControl::new();
    control.set_profiling(profile_path.is_some());
    if language != "all" {
        exclusions.extend(
            available
                .catalog()
                .targets()
                .iter()
                .filter(|target| target.ecosystem != language)
                .map(|target| target.id.clone()),
        );
    }
    let excluded = exclusions.iter().map(String::as_str).collect::<Vec<_>>();
    let snapshot =
        CatalogSnapshot::basic(ClientProfile::CliV1, &excluded).unwrap_or_else(|error| {
            eprintln!("Invalid scan catalog: {error}");
            process::exit(2);
        });
    let mut recorded_first_finding = false;
    let report = scan_client(
        &root,
        &snapshot,
        ScanOptions {
            apparent_size: apparent,
        },
        environments,
        &control,
        |event| {
            if profile_path.is_some()
                && !recorded_first_finding
                && matches!(event, CatalogScanEvent::ArtifactDiscovered(_))
            {
                recorded_first_finding = true;
                profile.record_first_finding(start.elapsed());
            }
            if let CatalogScanEvent::Profile(sample) = event {
                profile.record(*sample);
            }
        },
    );
    profile.set_resource_usage(control.resource_usage_json());
    if let Some(path) = &profile_path {
        use std::io::Write;
        // create_new also protects a report created by another process during the scan.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap_or_else(|error| {
                eprintln!("Could not create profile {}: {error}", path.display());
                process::exit(2);
            });
        let status = match report.status {
            build_hunter::ScanStatus::Completed => "completed",
            build_hunter::ScanStatus::CompletedWithChanges => "completed_with_changes",
            build_hunter::ScanStatus::Cancelled => "cancelled",
            build_hunter::ScanStatus::Incomplete => "incomplete",
            build_hunter::ScanStatus::Failed => "failed",
        };
        let size_mode = if apparent || !cfg!(unix) {
            "apparent"
        } else {
            "allocated"
        };
        if let Err(error) = file
            .write_all(
                profile
                    .json(&root.to_string_lossy(), status, size_mode)
                    .as_bytes(),
            )
            .and_then(|_| file.flush())
        {
            eprintln!("Could not write profile: {error}");
            process::exit(2);
        }
    }
    let mut rows = report.artifacts;
    rows.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then(a.relative_path.cmp(&b.relative_path))
    });
    let total: u64 = rows.iter().filter(|r| !r.nested).map(|r| r.bytes).sum();
    if json {
        println!(
            "{{\"schema_version\":2,\"catalog\":{},\"scan_budgets\":{},\"root\":{},\"total_bytes\":{total},\"elapsed_seconds\":{:.3},\"artifacts\":[",
            snapshot.metadata_json(),
            control.resource_usage_json(),
            quoted(&root.to_string_lossy()),
            start.elapsed().as_secs_f64()
        );
        for (i, row) in rows.iter().enumerate() {
            println!(
                "{}{}",
                if i == 0 { "" } else { "," },
                serde_json::json!({
                    "path":root.join(&row.relative_path).to_string_lossy(),"relative_path":row.relative_path.to_string_lossy(),
                    "language":row.target.ecosystem,"kind":presentation_kind(ClientProfile::CliV1,&row.target),
                    "target_id":row.target.id,"bytes":row.bytes,"nested":row.nested,"partial":row.partial,
                })
            );
        }
        println!(
            "],\"errors\":[{}],\"diagnostics\":[{}]}}",
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
                .chain(report.diagnostics.iter().map(|diagnostic| quoted(&format!("{}: {}",root.join(&diagnostic.relative_path).display(),build_hunter::catalog_bridge::diagnostic_json(diagnostic)))))
                .collect::<Vec<_>>()
                .join(","),
            report.diagnostics.iter().map(|diagnostic|serde_json::json!({"relative_path":diagnostic.relative_path.to_string_lossy(),"classification":build_hunter::catalog_bridge::diagnostic_json(diagnostic)}).to_string()).collect::<Vec<_>>().join(",")
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
                "{:>14}  {:<7}  {:<11}  {}{}{}",
                human(row.bytes),
                row.target.ecosystem,
                presentation_kind(ClientProfile::CliV1, &row.target),
                row.relative_path.display(),
                if row.nested { " [nested]" } else { "" },
                if row.partial {
                    " [partial / changed]"
                } else {
                    ""
                }
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
    if !json {
        for diagnostic in &report.diagnostics {
            eprintln!(
                "Warning: {}: {}",
                root.join(&diagnostic.relative_path).display(),
                build_hunter::catalog_bridge::diagnostic_json(diagnostic)
            );
        }
    }
    if matches!(
        report.status,
        build_hunter::ScanStatus::Incomplete | build_hunter::ScanStatus::Failed
    ) {
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
