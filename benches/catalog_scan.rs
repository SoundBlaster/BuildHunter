//! End-to-end Release comparison. Includes acquisition, dispatch, traversal and profiling.
#[path = "../tests/support/catalog_fixture.rs"]
mod catalog_fixture;

use build_hunter::{
    ResourceHistory, ScanControl, ScanOptions, ScanStatus,
    catalog::{CatalogSnapshot, ClientProfile},
    client_scan::{CatalogScanEvent, scan_client},
};
use serde_json::{Value, json};
use std::{fs, path::Path, time::Instant};

fn scan(root: &Path, snapshot: &CatalogSnapshot) -> (Value, Vec<Value>) {
    let control = ScanControl::new();
    control.set_profiling(true);
    let start = Instant::now();
    let mut first_us = None;
    let mut resources = ResourceHistory::default();
    let report = scan_client(
        root,
        snapshot,
        ScanOptions::default(),
        true,
        &control,
        |event| match event {
            CatalogScanEvent::ArtifactDiscovered(_) => {
                first_us.get_or_insert(start.elapsed().as_micros());
            }
            CatalogScanEvent::Profile(sample) => resources.record(*sample),
            _ => {}
        },
    );
    let elapsed_us = start.elapsed().as_micros();
    assert_eq!(report.status, ScanStatus::Completed);
    assert!(report.warnings.is_empty() && report.diagnostics.is_empty());
    let summary: Value = serde_json::from_str(&resources.summary().json()).unwrap();
    (
        json!({"elapsed_us":elapsed_us,"first_finding_us":first_us,
        "artifacts":report.artifacts.len(),"marker_observations":report.marker_observations,
        "resources":summary}),
        catalog_fixture::semantic_rows(&report),
    )
}

fn median(values: &mut [u64]) -> u64 {
    values.sort_unstable();
    values[values.len() / 2]
}

fn main() {
    let root =
        std::env::temp_dir().join(format!("buildhunter-catalog-bench-{}", std::process::id()));
    assert!(!root.exists(), "Use a fresh fixture path");
    fs::create_dir_all(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    let dynamic = root.join("dynamic");
    catalog_fixture::prepare_synthetic(&dynamic, 200, 1024);
    let dynamic_catalog = catalog_fixture::snapshot(ClientProfile::GuiV1, 200, false, &[]);
    let (dynamic_sample, dynamic_rows) = scan(&dynamic, &dynamic_catalog);
    assert_eq!(dynamic_rows.len(), 200);
    assert_eq!(dynamic_sample["marker_observations"], 200);
    println!(
        "{}",
        json!({"scenario":"dynamic-200","sample":dynamic_sample})
    );

    let representative = root.join("representative");
    for project in 0..128 {
        let project = representative.join(format!("project-{project}"));
        for directory in [
            ".build",
            "target",
            ".pytest_cache",
            "Sources/a",
            "Sources/b",
            "Sources/c",
        ] {
            fs::create_dir_all(project.join(directory)).unwrap();
            for index in 0..4 {
                fs::write(
                    project.join(directory).join(format!("file-{index}")),
                    [23; 1024],
                )
                .unwrap();
            }
        }
        fs::write(project.join("Cargo.toml"), b"[package]").unwrap();
    }
    let basic = CatalogSnapshot::basic(ClientProfile::GuiV1, &[]).unwrap();
    let large = catalog_fixture::snapshot(ClientProfile::GuiV1, 187, true, &[]);
    assert_eq!(large.catalog().targets().len(), 200);
    let (_, oracle) = scan(&representative, &basic);
    assert_eq!(oracle.len(), 384);
    let _ = scan(&representative, &large); // warm both snapshots/OS caches
    let mut times: [Vec<u64>; 2] = Default::default();
    for round in 0..7 {
        for index in if round % 2 == 0 { [0, 1] } else { [1, 0] } {
            let snapshot = if index == 0 { &basic } else { &large };
            let (sample, rows) = scan(&representative, snapshot);
            assert_eq!(
                rows, oracle,
                "Expanded catalog must preserve all findings and byte sizes"
            );
            times[index].push(sample["elapsed_us"].as_u64().unwrap());
            println!(
                "{}",
                json!({"scenario":if index==0 {"basic-13"} else {"expanded-200"},"round":round,"sample":sample})
            );
        }
    }
    let baseline = median(&mut times[0]);
    let expanded = median(&mut times[1]);
    let mut deviations = times[0]
        .iter()
        .map(|x| x.abs_diff(baseline))
        .collect::<Vec<_>>();
    // Broad regression guard; independent semantic/probe assertions are the stable gate.
    let limit = baseline
        .saturating_mul(2)
        .saturating_add(20_000)
        .saturating_add(median(&mut deviations).saturating_mul(3));
    println!(
        "{}",
        json!({"gate":"catalog-scan/1","baseline_median_us":baseline,"expanded_median_us":expanded,
        "limit_us":limit,"passed":expanded<=limit,"targets":200,"repetitions":7,
        "limits":"Warm synthetic filesystem; sampled process RSS; no UI or whole-process budget acceptance"})
    );
    assert!(
        expanded <= limit,
        "200-target end-to-end regression gate exceeded"
    );
}
