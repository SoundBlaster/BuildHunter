//! Read-only, bounded real-tree phase measurement for the bulk-metadata experiment.
use build_hunter::{
    ResourceHistory, ScanControl, ScanEvent, ScanOptions, rust_cli_policy, scan_with_policy,
};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert!(
        args.len() >= 2,
        "Usage: profile_tree PATH [--apparent] [--environments] [--cancel-on-first] [--report] [--no-profile]"
    );
    for flag in &args[2..] {
        assert!(
            matches!(
                flag.as_str(),
                "--apparent" | "--environments" | "--cancel-on-first" | "--report" | "--no-profile"
            ),
            "Unknown option: {flag}"
        );
    }
    let control = ScanControl::new();
    control.set_profiling(!args.iter().any(|arg| arg == "--no-profile"));
    let start = Instant::now();
    let mut first_ns = None;
    let mut cancelled_at = None;
    let mut last_progress = Duration::ZERO;
    let mut last_sample = None;
    let mut resources = ResourceHistory::default();
    let mut samples = std::collections::VecDeque::new();
    let report = scan_with_policy(
        Path::new(&args[1]),
        ScanOptions {
            apparent_size: args.iter().any(|arg| arg == "--apparent"),
        },
        &control,
        |facts| rust_cli_policy(facts, args.iter().any(|arg| arg == "--environments")),
        |event| {
            if matches!(event, ScanEvent::ArtifactDiscovered(_)) && first_ns.is_none() {
                first_ns = Some(start.elapsed().as_nanos());
                if args.iter().any(|arg| arg == "--cancel-on-first") {
                    cancelled_at = Some(Instant::now());
                    control.cancel();
                }
            }
            if let ScanEvent::Profile(sample) = event {
                last_sample = Some(*sample);
                resources.record(*sample);
                if samples.len() == 4096 {
                    samples.pop_front();
                }
                samples.push_back(*sample);
                if start.elapsed().saturating_sub(last_progress) >= Duration::from_secs(5) {
                    last_progress = start.elapsed();
                    eprintln!(
                        "{}s: {} entries, {} artifacts",
                        last_progress.as_secs(),
                        sample.entries,
                        sample.artifacts
                    );
                }
                if start.elapsed() > Duration::from_secs(120) {
                    control.cancel();
                }
            }
        },
    );
    let timings = control.work_timings();
    println!(
        "{{\"status\":\"{:?}\",\"elapsed_ns\":{},\"first_artifact_ns\":{},\"cancellation_latency_ns\":{},\"artifacts\":{},\"warnings\":{},\"bytes\":{},\"entries\":{},\"listing_worker_ns\":{},\"measurement_worker_ns\":{},\"bulk_directories\":{},\"bulk_fallbacks\":{},\"resources\":{},\"resource_samples\":[{}],\"report\":{}}}",
        report.status,
        start.elapsed().as_nanos(),
        first_ns.unwrap_or(0),
        cancelled_at.map_or(0, |cancelled: Instant| cancelled.elapsed().as_nanos()),
        report.artifacts.len(),
        report.warnings.len(),
        report.total_bytes(),
        last_sample.map_or(0, |sample| sample.entries),
        timings.listing_ns,
        timings.measurement_ns,
        timings.bulk_directories,
        timings.bulk_fallbacks,
        resources.summary().json(),
        samples
            .iter()
            .map(|sample| format!(
                "{{\"elapsed_us\":{},{}}}",
                sample.elapsed_us,
                sample.resource_json_fields()
            ))
            .collect::<Vec<_>>()
            .join(","),
        if args.iter().any(|arg| arg == "--report") {
            canonical_report(&report)
        } else {
            "null".into()
        }
    );
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn path_hex(path: &Path) -> String {
    #[cfg(unix)]
    {
        hex(path.as_os_str().as_bytes())
    }
    #[cfg(not(unix))]
    {
        hex(path.to_string_lossy().as_bytes())
    }
}

fn canonical_report(report: &build_hunter::ScanReport) -> String {
    let mut rows: Vec<_> = report
        .artifacts
        .iter()
        .map(|row| {
            format!(
                "[\"{}\",\"{}\",\"{}\",{},{},{}]",
                path_hex(&row.relative_path),
                row.language,
                row.kind,
                row.bytes,
                row.nested,
                row.partial
            )
        })
        .collect();
    rows.sort();
    let mut warnings: Vec<_> = report
        .warnings
        .iter()
        .map(|warning| {
            format!(
                "[\"{}\",\"{}\"]",
                path_hex(&warning.relative_path),
                hex(warning.message.as_bytes())
            )
        })
        .collect();
    warnings.sort();
    format!(
        "{{\"status\":\"{:?}\",\"rows\":[{}],\"warnings\":[{}]}}",
        report.status,
        rows.join(","),
        warnings.join(",")
    )
}
