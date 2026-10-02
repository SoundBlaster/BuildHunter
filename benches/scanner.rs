//! Dependency-free, release-mode scanner benchmarks. Fixture creation is not timed.
use build_hunter::{
    ScanControl, ScanEvent, ScanOptions, ScanStatus, rust_cli_policy, scan_with_policy,
};
use std::{env, fs, path::Path, time::Instant};

const SCENARIOS: [(&str, usize, usize); 4] = [
    ("source-tree", 16, 1),
    ("many-roots", 4096, 1),
    ("pruned-git", 0, 20),
    ("cancel", 1, 20),
];

fn write_files(path: &Path, count: usize) {
    fs::create_dir_all(path).unwrap();
    let payload = [42; 1024];
    for index in 0..count {
        fs::write(path.join(format!("file-{index}")), payload).unwrap();
    }
}

fn prepare(root: &Path) {
    assert!(!root.exists(), "fixture directory must be new");
    for project in 0..16 {
        let project = root.join("source-tree").join(format!("project-{project}"));
        write_files(&project.join(".build"), 64);
        for module in 0..8 {
            write_files(&project.join(format!("Sources/module-{module}")), 32);
        }
    }
    for project in 0..4096 {
        write_files(
            &root
                .join("many-roots")
                .join(format!("project-{project}/.pytest_cache")),
            1,
        );
    }
    for module in 0..128 {
        write_files(
            &root
                .join("pruned-git/.git")
                .join(format!("module-{module}")),
            32,
        );
    }
    write_files(&root.join("cancel/.build"), 4096);
}

fn sample(root: &Path, scenario: &str, expected_roots: usize, repetitions: usize) {
    let mut elapsed = 0;
    let mut first_artifact = 0;
    let mut cancellation_latency = 0;
    let mut policy_calls = 0;
    for _ in 0..repetitions {
        let control = ScanControl::new();
        let mut first = None;
        let mut cancelled_at = None;
        let start = Instant::now();
        let report = scan_with_policy(
            &root.join(scenario),
            ScanOptions::default(),
            &control,
            |facts| {
                policy_calls += 1;
                rust_cli_policy(facts, false)
            },
            |event| {
                if matches!(event, ScanEvent::ArtifactDiscovered(_)) && first.is_none() {
                    first = Some(start.elapsed().as_nanos());
                    if scenario == "cancel" {
                        cancelled_at = Some(Instant::now());
                        control.cancel();
                    }
                }
            },
        );
        elapsed += start.elapsed().as_nanos();
        cancellation_latency += cancelled_at.map_or(0, |cancelled| cancelled.elapsed().as_nanos());
        first_artifact += first.unwrap_or(0);
        assert!(report.warnings.is_empty());
        assert_eq!(report.artifacts.len(), expected_roots);
        if scenario == "cancel" {
            assert_eq!(report.status, ScanStatus::Cancelled);
            assert!(report.artifacts[0].partial);
        } else {
            assert_eq!(report.status, ScanStatus::Completed);
            assert!(report.artifacts.iter().all(|artifact| !artifact.partial));
            let minimum_payload = match scenario {
                "source-tree" => 16 * 64 * 1024,
                "many-roots" => 4096 * 1024,
                _ => 0,
            };
            assert!(report.total_bytes() >= minimum_payload);
        }
    }
    let repetitions = repetitions as u128;
    println!(
        "{scenario},{},{},{},{},{}",
        elapsed / repetitions,
        first_artifact / repetitions,
        cancellation_latency / repetitions,
        expected_roots,
        policy_calls as u128 / repetitions
    );
}

fn main() {
    let args: Vec<_> = env::args().skip(1).filter(|arg| arg != "--bench").collect();
    match args.as_slice() {
        [flag, path] if flag == "--prepare" => prepare(Path::new(path)),
        [flag, path] if flag == "--fixture" => {
            println!(
                "scenario,elapsed_ns,first_artifact_ns,cancellation_latency_ns,artifacts,policy_calls"
            );
            for (scenario, roots, repetitions) in SCENARIOS {
                sample(Path::new(path), scenario, roots, repetitions);
            }
        }
        [] => {
            let root =
                env::temp_dir().join(format!("build-hunter-performance-{}", std::process::id()));
            prepare(&root);
            println!(
                "scenario,elapsed_ns,first_artifact_ns,cancellation_latency_ns,artifacts,policy_calls"
            );
            for _ in 0..6 {
                for (scenario, roots, repetitions) in SCENARIOS {
                    sample(&root, scenario, roots, repetitions);
                }
            }
            fs::remove_dir_all(root).unwrap();
        }
        _ => panic!("Usage: scanner [--prepare PATH | --fixture PATH]"),
    }
}
