use crate::{
    catalog::{CatalogSnapshot, ClientProfile, presentation_kind},
    catalog_bridge::{BHCatalogScanEvent, bh_scan_catalog_v1},
    client_scan::{CatalogScanEvent, scan_client},
    *,
};
use serde_json::Value;
use std::collections::BTreeSet;

#[path = "../tests/support/catalog_fixture.rs"]
mod catalog_fixture;

#[cfg(target_os = "macos")]
#[test]
fn catalog_clients_skip_devicefs_without_marker_facts_or_diagnostics() {
    for profile in [ClientProfile::CliV1, ClientProfile::GuiV1] {
        let snapshot = CatalogSnapshot::basic(profile, &[]).unwrap();
        for root in [
            "/Users/buildhunter-devicefs-test/Library/Developer/CoreDevice/DeviceFS",
            "/Users/buildhunter-devicefs-test/Library/Developer/CoreDevice/DeviceFS/device/AppDataContainers",
        ] {
            let mut events = Vec::new();
            let report = scan_client(
                Path::new(root),
                &snapshot,
                ScanOptions::default(),
                true,
                &ScanControl::new(),
                |event| events.push(event.clone()),
            );
            assert_eq!(report.status, ScanStatus::Incomplete);
            assert!(report.artifacts.is_empty() && report.diagnostics.is_empty());
            assert_eq!(report.marker_observations, 0);
            assert_eq!(report.warnings.len(), 1);
            assert!(report.warnings[0].message.contains("CoreDevice/DeviceFS"));
            assert!(matches!(
                events.last(),
                Some(CatalogScanEvent::Finished(ScanStatus::Incomplete))
            ));
        }
    }
}

#[test]
fn similarly_named_local_devicefs_directory_is_still_scanned() {
    let tree = tests::TempTree::new();
    let local = Path::new("project/Library/Developer/CoreDevice/DeviceFS/.build");
    fs::create_dir_all(tree.0.join(local)).unwrap();
    fs::write(tree.0.join(local).join("object.o"), b"local fixture").unwrap();
    let snapshot = CatalogSnapshot::basic(ClientProfile::GuiV1, &[]).unwrap();
    let report = scan_client(
        &tree.0,
        &snapshot,
        ScanOptions::default(),
        true,
        &ScanControl::new(),
        |_| {},
    );
    assert_eq!(report.status, ScanStatus::Completed);
    assert!(report.warnings.is_empty() && report.diagnostics.is_empty());
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(report.artifacts[0].relative_path, local);
    assert!(report.artifacts[0].bytes > 0);
}

#[test]
fn two_hundred_targets_scan_dynamic_names_and_only_probe_relevant_markers() {
    let tree = tests::TempTree::new();
    catalog_fixture::prepare_synthetic(&tree.0, 200, 1024);
    let full = catalog_fixture::snapshot(ClientProfile::GuiV1, 200, false, &[]);
    let excluded =
        catalog_fixture::snapshot(ClientProfile::GuiV1, 200, false, &["example.cache-137"]);
    assert_eq!(full.catalog().targets().len(), 200);
    assert_eq!(
        full.catalog().catalog_digest(),
        excluded.catalog().catalog_digest()
    );
    for (snapshot, expected) in [(&full, 200), (&excluded, 199)] {
        let mut completions = BTreeSet::new();
        let report = scan_client(
            &tree.0,
            snapshot,
            ScanOptions::default(),
            true,
            &ScanControl::new(),
            |event| {
                if let CatalogScanEvent::ArtifactCompleted { id, .. } = event {
                    assert!(completions.insert(*id));
                }
            },
        );
        assert_eq!(report.status, ScanStatus::Completed);
        assert!(report.warnings.is_empty() && report.diagnostics.is_empty());
        assert_eq!(report.artifacts.len(), expected);
        assert_eq!(completions.len(), expected);
        assert_eq!(report.marker_observations, expected as u64);
        assert_eq!(catalog_fixture::semantic_rows(&report).len(), expected);
        for artifact in &report.artifacts {
            let name = artifact.relative_path.to_str().unwrap();
            assert_eq!(artifact.target.id, format!("example.{name}"));
            assert_eq!(artifact.bytes, independent_bytes(&tree.0.join(name), false));
            assert!(!artifact.partial && !artifact.nested);
        }
        assert_eq!(
            report
                .artifacts
                .iter()
                .any(|a| a.target.id == "example.cache-137"),
            expected == 200
        );
    }
}

fn fixture(case: &Value) -> tests::TempTree {
    let tree = tests::TempTree::new();
    for name in case["directories"].as_array().unwrap() {
        fs::create_dir_all(tree.0.join(name.as_str().unwrap())).unwrap();
    }
    for (name, contents) in case["files"].as_object().unwrap() {
        let path = tree.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents.as_str().unwrap()).unwrap();
    }
    #[cfg(unix)]
    for (name, target) in case["symlinks"].as_object().unwrap() {
        let path = tree.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target.as_str().unwrap(), path).unwrap();
    }
    tree
}

fn independent_bytes(path: &Path, apparent: bool) -> u64 {
    let metadata = fs::symlink_metadata(path).unwrap();
    if metadata.is_symlink() {
        return 0;
    }
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::fs::MetadataExt;
        if apparent {
            metadata.len()
        } else {
            metadata.blocks() * 512
        }
    };
    #[cfg(not(unix))]
    let bytes = {
        let _ = apparent;
        metadata.len()
    };
    bytes
        + if metadata.is_dir() {
            fs::read_dir(path)
                .unwrap()
                .map(|entry| independent_bytes(&entry.unwrap().path(), apparent))
                .sum::<u64>()
        } else {
            0
        }
}

#[test]
fn catalog_clients_match_both_frozen_profiles_and_exclusions() {
    let corpus: Value =
        serde_json::from_str(include_str!("../tests/fixtures/migration-parity-v1.json")).unwrap();
    for case in corpus["cases"].as_array().unwrap() {
        if !cfg!(unix) && case["unixOnly"] == true {
            continue;
        }
        let tree = fixture(case);
        let root = tree.0.join(case["scanRoot"].as_str().unwrap());
        for run in case["runs"].as_array().unwrap() {
            if run["client"] == "cli" && run["exitCode"] == 2 {
                continue;
            } // CLI preflight is outside this API.
            let profile = if run["client"] == "cli" {
                ClientProfile::CliV1
            } else {
                ClientProfile::GuiV1
            };
            let base = CatalogSnapshot::basic(profile, &[]).unwrap();
            let mut excluded = run["excluded"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap())
                .collect::<BTreeSet<_>>();
            let language = run["language"].as_str().unwrap();
            if language != "all" {
                excluded.extend(
                    base.catalog()
                        .targets()
                        .iter()
                        .filter(|target| target.ecosystem != language)
                        .map(|target| target.id.as_str()),
                );
            }
            let snapshot =
                CatalogSnapshot::basic(profile, &excluded.into_iter().collect::<Vec<_>>()).unwrap();
            for apparent in [false, true] {
                let mut events = Vec::new();
                let report = scan_client(
                    &root,
                    &snapshot,
                    ScanOptions {
                        apparent_size: apparent,
                    },
                    run["includeEnvironments"].as_bool().unwrap(),
                    &ScanControl::new(),
                    |event| events.push(event.clone()),
                );
                let label = format!("{} / {} / apparent={apparent}", case["id"], run["id"]);
                assert!(report.diagnostics.is_empty(), "{label}");
                let expected = run["expected"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|key| &case["rows"][key.as_str().unwrap()])
                    .collect::<Vec<_>>();
                assert_eq!(report.artifacts.len(), expected.len(), "{label}");
                let mut total = 0;
                for row in expected {
                    let artifact = report
                        .artifacts
                        .iter()
                        .find(|a| {
                            a.relative_path.to_string_lossy().replace('\\', "/")
                                == row["path"].as_str().unwrap()
                        })
                        .unwrap_or_else(|| panic!("missing {}: {label}", row["path"]));
                    assert_eq!(
                        artifact.target.id,
                        row["filterID"].as_str().unwrap(),
                        "{label}"
                    );
                    assert_eq!(
                        artifact.target.ecosystem,
                        row["language"].as_str().unwrap(),
                        "{label}"
                    );
                    assert_eq!(
                        presentation_kind(profile, &artifact.target),
                        row["kind"].as_str().unwrap(),
                        "{label}"
                    );
                    assert_eq!(artifact.nested, row["nested"].as_bool().unwrap(), "{label}");
                    let absolute = if artifact.relative_path == Path::new(".") {
                        root.clone()
                    } else {
                        root.join(&artifact.relative_path)
                    };
                    let bytes = independent_bytes(&absolute, apparent);
                    assert_eq!(artifact.bytes, bytes, "{label}");
                    assert!(!artifact.partial, "{label}");
                    if !artifact.nested {
                        total += bytes;
                    }
                }
                assert_eq!(report.total_bytes(), total, "{label}");
                assert_eq!(
                    report.warnings.len() as u64,
                    run["warnings"].as_u64().unwrap(),
                    "{label}"
                );
                if run["status"] == "completed" {
                    assert_eq!(report.status, ScanStatus::Completed, "{label}");
                } else {
                    assert!(
                        matches!(report.status, ScanStatus::Incomplete | ScanStatus::Failed),
                        "{label}"
                    );
                }
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| matches!(event, CatalogScanEvent::Finished(_)))
                        .count(),
                    1,
                    "{label}"
                );
                assert!(
                    matches!(events.last(),Some(CatalogScanEvent::Finished(status)) if *status==report.status),
                    "{label}"
                );
                for artifact in &report.artifacts {
                    assert_eq!(events.iter().filter(|event|matches!(event,CatalogScanEvent::ArtifactCompleted{id,..} if *id==artifact.id)).count(),1,"{label}");
                }
            }
        }
    }
}

#[derive(Default)]
struct CEvents {
    kinds: Vec<u32>,
    targets: Vec<String>,
    metadata: Option<Value>,
    completed: usize,
    status: Option<u32>,
    control: *mut ScanControl,
    cancel: bool,
}
unsafe extern "C" fn copy_catalog_event(context: *mut c_void, event: *const BHCatalogScanEvent) {
    let state = unsafe { &mut *context.cast::<CEvents>() };
    let event = unsafe { &*event };
    assert_eq!(event.version, 1);
    state.kinds.push(event.base.event_type);
    match event.base.event_type {
        6 => {
            state.metadata = Some(
                serde_json::from_slice(unsafe {
                    std::slice::from_raw_parts(event.metadata, event.metadata_len)
                })
                .unwrap(),
            )
        }
        1 => {
            state.targets.push(
                String::from_utf8(
                    unsafe { std::slice::from_raw_parts(event.target_id, event.target_id_len) }
                        .to_vec(),
                )
                .unwrap(),
            );
            if state.cancel {
                unsafe {
                    bh_scan_control_cancel(state.control);
                }
            }
        }
        2 => state.completed += 1,
        4 => state.status = Some(event.base.status),
        _ => {}
    }
}

#[test]
fn catalog_event_abi_copies_identity_before_scan_and_preserves_cancel_completion() {
    let tree = tests::TempTree::new();
    fs::create_dir_all(tree.0.join(".build/nested")).unwrap();
    fs::write(tree.0.join(".build/nested/o"), b"payload").unwrap();
    for cancel in [false, true] {
        let snapshot = CatalogSnapshot::basic(ClientProfile::GuiV1, &["future.framework"]).unwrap();
        let root = path_bytes(&tree.0);
        let control = bh_scan_control_create();
        let mut events = CEvents {
            control,
            cancel,
            ..CEvents::default()
        };
        let status = unsafe {
            bh_scan_catalog_v1(
                control,
                &snapshot,
                root.as_ptr(),
                root.len(),
                1,
                0,
                Some(copy_catalog_event),
                (&mut events as *mut CEvents).cast(),
            )
        };
        unsafe {
            bh_scan_control_destroy(control);
        }
        assert_eq!(events.kinds.first(), Some(&6));
        assert_eq!(events.kinds.last(), Some(&4));
        assert_eq!(events.kinds.iter().filter(|kind| **kind == 4).count(), 1);
        assert_eq!(events.targets, vec!["swift.build"]);
        assert_eq!(events.completed, 1);
        assert_eq!(events.status, Some(if cancel { 1 } else { 0 }));
        assert_eq!(status, if cancel { 1 } else { 0 });
        let metadata = events.metadata.unwrap();
        assert_eq!(
            metadata["excludedTargetIDs"],
            serde_json::json!(["future.framework"])
        );
        assert_eq!(
            metadata["digest"],
            snapshot.catalog().catalog_digest().to_string()
        );
    }
}

#[test]
fn catalog_diagnostics_remain_typed_and_do_not_hide_valid_descendants() {
    use buildhunter_plugin_policy::{Origin, PluginCatalog, PluginInput};
    let mut documents = Vec::new();
    for (id, name) in [
        ("first", ".ambiguous"),
        ("second", ".ambiguous"),
        ("child", "child"),
    ] {
        documents.push(serde_json::to_vec(&serde_json::json!({
            "format":"buildhunter.plugin/1-draft.1","id":format!("example.{id}"),"version":"1.0.0","title":id,"description":"Test catalog",
            "program":{"format":"rifle.program/1-draft.1","facts":[{"id":"name","type":"string"}],"rules":[{"id":"match","when":{"fact":"name","op":"eq","value":name}}]},
            "bindings":{"name":{"source":"node.name"}},"targets":[{"id":format!("example.{id}"),"title":id,"ecosystem":"arbitrary","kind":"cache","ruleIds":["match"]}]
        })).unwrap());
    }
    let inputs = documents
        .iter()
        .map(|bytes| PluginInput {
            bytes,
            origin: Origin::External,
        })
        .collect::<Vec<_>>();
    let snapshot = CatalogSnapshot::from_catalog(
        ClientProfile::GuiV1,
        PluginCatalog::load(&inputs, &[]).unwrap(),
    );
    let tree = tests::TempTree::new();
    fs::create_dir_all(tree.0.join(".ambiguous/child")).unwrap();
    fs::write(tree.0.join(".ambiguous/child/data"), b"payload").unwrap();
    for cancel in [false, true] {
        let control = ScanControl::new();
        let mut events = Vec::new();
        let report = scan_client(
            &tree.0,
            &snapshot,
            ScanOptions::default(),
            false,
            &control,
            |event| {
                if cancel && matches!(event, CatalogScanEvent::Diagnostic(_)) {
                    control.cancel();
                }
                events.push(event.clone());
            },
        );
        assert_eq!(report.diagnostics.len(), 1);
        let diagnostic = crate::catalog_bridge::diagnostic_json(&report.diagnostics[0]);
        assert_eq!(diagnostic["kind"], "ambiguous");
        assert_eq!(
            diagnostic["matchedTargetIDs"],
            serde_json::json!(["example.first", "example.second"])
        );
        assert_eq!(
            report.status,
            if cancel {
                ScanStatus::Cancelled
            } else {
                ScanStatus::Incomplete
            }
        );
        assert_eq!(report.artifacts.len(), usize::from(!cancel));
        if !cancel {
            assert_eq!(report.artifacts[0].target.ecosystem, "arbitrary");
        }
        assert!(
            matches!(events.last(),Some(CatalogScanEvent::Finished(status)) if *status==report.status)
        );
    }
}
