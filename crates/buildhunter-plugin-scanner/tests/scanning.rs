#![cfg(unix)]
use build_hunter::{ScanControl, ScanOptions, ScanStatus};
use buildhunter_plugin_policy::{Origin, PluginCatalog, PluginInput};
use buildhunter_plugin_scanner::{PluginScanEvent, scan};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const FORGE: &[u8] = include_bytes!("../../../docs/contracts/plugin-v1/examples/synthetic.json");
struct Tree(PathBuf);
impl Tree {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "buildhunter-plugin-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn file(&self, p: &str, bytes: &[u8]) {
        let p = self.0.join(p);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }
}
impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn catalog(excluded: &[&str]) -> PluginCatalog {
    PluginCatalog::load(
        &[PluginInput {
            bytes: FORGE,
            origin: Origin::External,
        }],
        excluded,
    )
    .unwrap()
}
#[test]
fn arbitrary_marker_and_name_reach_scanner_and_preserve_target_metadata() {
    let t = Tree::new();
    t.file("forge.project", b"project");
    t.file("forge-out/object", b"12345");
    let mut events = Vec::new();
    let r = scan(
        &t.0,
        &catalog(&[]),
        ScanOptions {
            apparent_size: true,
        },
        &ScanControl::new(),
        |e| events.push(e.clone()),
    )
    .unwrap();
    assert_eq!(r.status, ScanStatus::Completed);
    assert_eq!(r.artifacts.len(), 1);
    assert_eq!(r.artifacts[0].target.id, "example.forge-output");
    assert_eq!(r.artifacts[0].target.ecosystem, "example");
    assert!(r.artifacts[0].bytes >= 5);
    assert!(events.iter().any(|e|matches!(e,PluginScanEvent::ArtifactDiscovered(a) if a.target.id=="example.forge-output")));
}
#[test]
fn excluded_parent_keeps_traversing_descendants() {
    let t = Tree::new();
    t.file("forge-out/project/forge.project", b"p");
    t.file("forge-out/project/forge-out/object", b"child");
    let r = scan(
        &t.0,
        &catalog(&[]),
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert_eq!(r.artifacts.len(), 1);
    assert_eq!(
        r.artifacts[0].relative_path,
        PathBuf::from("forge-out/project/forge-out")
    );
    let r = scan(
        &t.0,
        &catalog(&["example.forge-output"]),
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert!(r.artifacts.is_empty());
    assert_eq!(r.marker_observations, 0);
}
#[test]
fn parent_of_selected_root_is_unknown_and_not_read() {
    let t = Tree::new();
    t.file("forge.project", b"p");
    t.file("forge-out/object", b"data");
    let r = scan(
        &t.0.join("forge-out"),
        &catalog(&[]),
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert!(r.artifacts.is_empty());
    assert_eq!(r.diagnostics.len(), 1);
    assert!(
        r.diagnostics[0]
            .classification
            .unavailable
            .iter()
            .any(|f| f.reason == "out_of_scope")
    );
    assert_eq!(r.marker_observations, 0);
}
#[test]
fn symlink_marker_is_false_and_linked_candidate_is_never_followed() {
    use std::os::unix::fs::symlink;
    let t = Tree::new();
    t.file("actual", b"p");
    t.file("forge-out/object", b"data");
    symlink(t.0.join("actual"), t.0.join("forge.project")).unwrap();
    symlink(t.0.join("forge-out"), t.0.join("linked")).unwrap();
    let r = scan(
        &t.0,
        &catalog(&[]),
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert!(r.artifacts.is_empty());
    assert!(r.diagnostics.is_empty());
}
#[test]
fn cancellation_from_discovery_keeps_partial_result_and_finishes_once() {
    let t = Tree::new();
    t.file("forge.project", b"p");
    t.file("forge-out/object", b"data");
    let control = ScanControl::new();
    let mut finished = 0;
    let r = scan(&t.0, &catalog(&[]), ScanOptions::default(), &control, |e| {
        if matches!(e, PluginScanEvent::ArtifactDiscovered(_)) {
            control.cancel();
        }
        if matches!(e, PluginScanEvent::Finished(_)) {
            finished += 1;
        }
    })
    .unwrap();
    assert_eq!(r.status, ScanStatus::Cancelled);
    assert_eq!(finished, 1);
    assert!(r.artifacts[0].partial);
}

fn load_documents(documents: &[Vec<u8>], excluded: &[&str]) -> PluginCatalog {
    let inputs: Vec<_> = documents
        .iter()
        .map(|bytes| PluginInput {
            bytes,
            origin: Origin::External,
        })
        .collect();
    PluginCatalog::load(&inputs, excluded).unwrap()
}
fn edited(f: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut v: serde_json::Value = serde_json::from_slice(FORGE).unwrap();
    f(&mut v);
    serde_json::to_vec(&v).unwrap()
}
#[test]
fn file_artifact_and_own_marker_only_rules_are_not_gated_by_legacy_names() {
    let t = Tree::new();
    t.file("note.bin", b"1234567");
    t.file("unusual/cache.signal", b"s");
    t.file("unusual/object", b"data");
    let file = edited(|v| {
        v["id"] = "example.file".into();
        v["targets"][0]["id"] = "example.file-output".into();
        v["program"]["facts"] =
            serde_json::json!([{"id":"name","type":"string"},{"id":"kind","type":"string"}]);
        v["bindings"] =
            serde_json::json!({"name":{"source":"node.name"},"kind":{"source":"node.kind"}});
        v["program"]["rules"][0]["when"] = serde_json::json!({"all":[{"fact":"kind","op":"eq","value":"regular_file"},{"fact":"name","op":"eq","value":"note.bin"}]});
    });
    let own = edited(|v| {
        v["id"] = "example.own".into();
        v["targets"][0]["id"] = "example.own-cache".into();
        v["program"]["facts"] = serde_json::json!([{"id":"project","type":"boolean"}]);
        v["bindings"] =
            serde_json::json!({"project":{"source":"marker","scope":"own","name":"cache.signal"}});
        v["program"]["rules"][0]["when"] =
            serde_json::json!({"fact":"project","op":"eq","value":true});
    });
    let r = scan(
        &t.0,
        &load_documents(&[file, own], &[]),
        ScanOptions {
            apparent_size: true,
        },
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert_eq!(r.artifacts.len(), 2);
    assert_eq!(
        r.artifacts
            .iter()
            .find(|a| a.target.id == "example.file-output")
            .unwrap()
            .bytes,
        7
    );
    assert!(
        r.artifacts
            .iter()
            .any(|a| a.relative_path == std::path::Path::new("unusual"))
    );
}
#[test]
fn excluding_outer_target_preserves_distinct_enabled_inner_target() {
    let t = Tree::new();
    t.file("outer/cache.signal", b"s");
    t.file("outer/forge.project", b"p");
    t.file("outer/forge-out/object", b"data");
    let outer = edited(|v| {
        v["id"] = "example.outer".into();
        v["targets"][0]["id"] = "example.outer-cache".into();
        v["program"]["facts"] = serde_json::json!([{"id":"project","type":"boolean"}]);
        v["bindings"] =
            serde_json::json!({"project":{"source":"marker","scope":"own","name":"cache.signal"}});
        v["program"]["rules"][0]["when"] =
            serde_json::json!({"fact":"project","op":"eq","value":true});
    });
    let catalog = load_documents(&[FORGE.to_vec(), outer], &["example.outer-cache"]);
    let r = scan(
        &t.0,
        &catalog,
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert_eq!(r.artifacts.len(), 1);
    assert_eq!(r.artifacts[0].target.id, "example.forge-output");
    assert!(!r.artifacts[0].nested);
}
#[test]
fn ambiguous_parent_emits_diagnostic_and_still_finds_child() {
    let t = Tree::new();
    t.file("forge.project", b"p");
    t.file("forge-out/forge.project", b"p");
    t.file("forge-out/forge-out/object", b"data");
    let conflicting = edited(|v| {
        v["id"] = "example.conflict".into();
        v["targets"][0]["id"] = "example.conflict-output".into();
        v["program"]["rules"][0]["when"]["all"][2] =
            serde_json::json!({"fact":"project","op":"eq","value":false});
        v["bindings"]["project"]["scope"] = "own".into();
        v["bindings"]["project"]["name"] = "stop.signal".into();
    });
    t.file("forge-out/forge-out/stop.signal", b"s");
    let r = scan(
        &t.0,
        &load_documents(&[FORGE.to_vec(), conflicting], &[]),
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert_eq!(r.diagnostics.len(), 1);
    assert!(matches!(
        r.diagnostics[0].classification.decision,
        buildhunter_plugin_policy::Decision::Ambiguous
    ));
    assert_eq!(r.artifacts.len(), 1);
    assert_eq!(
        r.artifacts[0].relative_path,
        PathBuf::from("forge-out/forge-out")
    );
}
#[test]
fn nested_totals_are_counted_once_and_match_legacy_rust_sizing() {
    const RUST: &[u8] = include_bytes!("../../../docs/contracts/plugin-v1/examples/rust.json");
    let t = Tree::new();
    t.file("Cargo.toml", b"p");
    t.file("target/Cargo.toml", b"p");
    t.file("target/target/object", b"data");
    let catalog = PluginCatalog::load(
        &[PluginInput {
            bytes: RUST,
            origin: Origin::Builtin,
        }],
        &[],
    )
    .unwrap();
    let r = scan(
        &t.0,
        &catalog,
        ScanOptions {
            apparent_size: true,
        },
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    let legacy = build_hunter::scan_with_policy(
        &t.0,
        ScanOptions {
            apparent_size: true,
        },
        &ScanControl::new(),
        |f| build_hunter::rust_cli_policy(f, false),
        |_| {},
    );
    assert_eq!(r.artifacts.len(), 2);
    assert!(r.artifacts.iter().any(|a| a.nested));
    assert_eq!(r.total_bytes(), legacy.total_bytes());
}
#[test]
fn git_is_a_host_prune_even_when_children_match_plugins() {
    let t = Tree::new();
    t.file(".git/forge.project", b"p");
    t.file(".git/forge-out/object", b"data");
    let r = scan(
        &t.0,
        &catalog(&[]),
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert!(r.artifacts.is_empty());
    assert!(r.diagnostics.is_empty());
}
#[test]
#[cfg(target_os = "linux")]
fn invalid_utf8_name_is_unknown_with_lossless_path() {
    use std::os::unix::ffi::OsStringExt;
    let t = Tree::new();
    // Keep other conjuncts true: Unknown AND false is correctly NoMatch.
    t.file("forge.project", b"project");
    let name = std::ffi::OsString::from_vec(vec![b'x', 0xff]);
    fs::create_dir(t.0.join(&name)).unwrap();
    let r = scan(
        &t.0,
        &catalog(&[]),
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert!(r.artifacts.is_empty());
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].relative_path, PathBuf::from(name));
    assert!(
        r.diagnostics[0]
            .classification
            .unavailable
            .iter()
            .any(|f| f.reason == "invalid_name")
    );
}

#[test]
fn report_pins_catalog_identity_and_unknown_exclusions() {
    let t = Tree::new();
    let catalog = catalog(&["example.future-filter"]);
    let r = scan(
        &t.0,
        &catalog,
        ScanOptions::default(),
        &ScanControl::new(),
        |_| {},
    )
    .unwrap();
    assert_eq!(r.catalog_digest, catalog.catalog_digest().to_string());
    assert!(r.exclusions.contains("example.future-filter"));
}

#[test]
fn diagnostic_only_scan_can_cancel_before_descendants_are_processed() {
    use buildhunter_plugin_policy::Decision;
    for ambiguous in [false, true] {
        let t = Tree::new();
        for index in 0..64 {
            t.file(&format!("forge-out/child-{index}/object"), b"data");
        }
        let documents = if ambiguous {
            ["example.first", "example.second"]
                .iter()
                .map(|id| {
                    edited(|v| {
                        v["id"] = (*id).into();
                        v["targets"][0]["id"] = format!("{id}-output").into();
                        v["program"]["facts"] = serde_json::json!([{"id":"kind","type":"string"}]);
                        v["bindings"] = serde_json::json!({"kind":{"source":"node.kind"}});
                        v["program"]["rules"][0]["when"] =
                            serde_json::json!({"fact":"kind","op":"eq","value":"directory"});
                    })
                })
                .collect::<Vec<_>>()
        } else {
            vec![FORGE.to_vec()]
        };
        let catalog = load_documents(&documents, &[]);
        let control = ScanControl::new();
        let mut events = Vec::new();
        let report = scan(
            &t.0.join("forge-out"),
            &catalog,
            ScanOptions::default(),
            &control,
            |event| {
                if matches!(event, PluginScanEvent::Diagnostic(_)) {
                    control.cancel();
                }
                events.push(event.clone());
            },
        )
        .unwrap();
        assert_eq!(report.status, ScanStatus::Cancelled);
        assert!(report.artifacts.is_empty());
        assert!(report.warnings.is_empty());
        assert_eq!(
            report.diagnostics.len(),
            1,
            "must stop before classifying children"
        );
        assert_eq!(
            events.len(),
            2,
            "only the immediate diagnostic and terminal event"
        );
        assert!(matches!(&events[0], PluginScanEvent::Diagnostic(d)
            if if ambiguous { matches!(d.classification.decision, Decision::Ambiguous) }
               else { matches!(d.classification.decision, Decision::Indeterminate) }));
        assert!(matches!(
            events[1],
            PluginScanEvent::Finished(ScanStatus::Cancelled)
        ));
    }
}
