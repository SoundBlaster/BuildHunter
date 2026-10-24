use crate::*;
use std::{collections::BTreeSet, sync::mpsc, time::Duration};

fn classify(facts: &CandidateFacts) -> CandidateAction {
    rust_cli_policy(facts, false)
}
fn terminal(events: &[ScanEvent], expected: ScanStatus) {
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, ScanEvent::Finished(_)))
            .count(),
        1
    );
    assert!(matches!(events.last(),Some(ScanEvent::Finished(status)) if *status==expected));
    let discovered: BTreeSet<_> = events
        .iter()
        .filter_map(|e| {
            if let ScanEvent::ArtifactDiscovered(a) = e {
                Some(a.id)
            } else {
                None
            }
        })
        .collect();
    let completed: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let ScanEvent::ArtifactCompleted { id, .. } = e {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(completed.len(), discovered.len());
    assert_eq!(completed.into_iter().collect::<BTreeSet<_>>(), discovered);
}

#[test]
fn finding_count_and_payload_limits_preserve_prefix_and_terminal_order() {
    let tree = tests::TempTree::new();
    for name in ["a.pyc", "b.pyc", "c.pyc"] {
        fs::write(tree.0.join(name), b"data").unwrap();
    }
    for (limits, reason, count) in [
        (
            ScanLimits {
                findings: 1,
                ..Default::default()
            },
            ScanLimit::Findings,
            1,
        ),
        (
            ScanLimits {
                findings_weight: 1,
                ..Default::default()
            },
            ScanLimit::FindingsWeight,
            0,
        ),
    ] {
        let control = ScanControl::with_limits(limits);
        let mut events = Vec::new();
        let report = scan_with_policy(&tree.0, ScanOptions::default(), &control, classify, |e| {
            events.push(e.clone())
        });
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert_eq!(control.resource_limit(), Some(reason));
        assert_eq!(report.artifacts.len(), count);
        assert_eq!(report.warnings.len(), 1);
        assert!(
            report.warnings[0]
                .message
                .starts_with("Resource limit reached:")
        );
        terminal(&events, ScanStatus::Incomplete);
    }
}

#[test]
fn one_large_directory_stops_at_an_explicit_listing_limit() {
    let tree = tests::TempTree::new();
    for i in 0..100 {
        fs::write(tree.0.join(format!("{i}.pyc")), b"data").unwrap();
    }
    for (limits, reason) in [
        (
            ScanLimits {
                listing_entries: 2,
                ..Default::default()
            },
            ScanLimit::ListingEntries,
        ),
        (
            ScanLimits {
                listing_weight: 1,
                ..Default::default()
            },
            ScanLimit::ListingWeight,
        ),
    ] {
        let control = ScanControl::with_limits(limits);
        let mut events = Vec::new();
        let report = scan_with_policy(&tree.0, ScanOptions::default(), &control, classify, |e| {
            events.push(e.clone())
        });
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert_eq!(control.resource_limit(), Some(reason));
        let usage = control.resource_usage_json();
        assert!(usage["peakListingEntries"].as_u64().unwrap() <= limits.listing_entries as u64);
        assert!(usage["peakListingWeight"].as_u64().unwrap() <= limits.listing_weight as u64);
        terminal(&events, ScanStatus::Incomplete);
    }
}

#[test]
fn rejected_work_never_waits_for_a_full_response_channel() {
    let tree = tests::TempTree::new();
    for i in 0..100 {
        fs::create_dir(tree.0.join(format!("dir{i}"))).unwrap();
    }
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        // A root request fits; every child is rejected by estimated path/state weight.
        let root_weight = tree.0.capacity() * 2
            + std::mem::size_of::<DirNode>()
            + std::mem::size_of::<ListTask>();
        let control = ScanControl::with_limits(ScanLimits {
            queued_weight: root_weight,
            response_capacity: 0,
            ..Default::default()
        });
        let mut events = Vec::new();
        let report = scan_with_policy(&tree.0, ScanOptions::default(), &control, classify, |e| {
            events.push(e.clone())
        });
        tx.send((report.status, control.resource_limit(), events))
            .unwrap();
    });
    let (status, limit, events) = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("bounded producers must join without coordinator deadlock");
    assert_eq!(status, ScanStatus::Incomplete);
    assert_eq!(limit, Some(ScanLimit::QueuedWeight));
    terminal(&events, ScanStatus::Incomplete);
}

#[test]
fn cancellation_while_bounded_producers_are_busy_finishes_and_joins() {
    let tree = tests::TempTree::new();
    let root = tree.0.join(".build");
    fs::create_dir(&root).unwrap();
    for i in 0..100 {
        fs::create_dir(root.join(format!("dir{i}"))).unwrap();
    }
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let control = ScanControl::with_limits(ScanLimits {
            response_capacity: 1,
            ..Default::default()
        });
        let mut events = Vec::new();
        let report = scan_with_policy(&root, ScanOptions::default(), &control, classify, |e| {
            if matches!(e, ScanEvent::ArtifactDiscovered(_)) {
                std::thread::sleep(Duration::from_millis(20));
                control.cancel();
            }
            events.push(e.clone());
        });
        tx.send((report.status, events)).unwrap();
    });
    let (status, events) = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("cancel must release producers before join");
    assert_eq!(status, ScanStatus::Cancelled);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ScanEvent::ArtifactCompleted { partial: true, .. }))
    );
    terminal(&events, ScanStatus::Cancelled);
}

#[test]
fn warning_overflow_keeps_a_reserved_limit_summary_and_partial_completion() {
    let tree = tests::TempTree::new();
    let root = tree.0.join(".build");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("data"), b"payload").unwrap();
    let probes = Probes {
        hook: Some(Box::new(|point, path| {
            if point == ProbePoint::Metadata && path.file_name() == Some(OsStr::new("data")) {
                Err(std::io::Error::from_raw_os_error(13))
            } else {
                Ok(())
            }
        })),
        ..Probes::default()
    };
    for (limits, reason) in [
        (
            ScanLimits {
                warnings: 0,
                ..Default::default()
            },
            ScanLimit::Warnings,
        ),
        (
            ScanLimits {
                warning_weight: 1,
                ..Default::default()
            },
            ScanLimit::WarningWeight,
        ),
    ] {
        let control = ScanControl::with_limits(limits);
        let mut events = Vec::new();
        let report = scan_with_probes(
            &root,
            ScanOptions::default(),
            &control,
            classify,
            |e| events.push(e.clone()),
            &probes,
        );
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert_eq!(control.resource_limit(), Some(reason));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.artifacts[0].partial);
        terminal(&events, ScanStatus::Incomplete);
    }
}

#[test]
fn ordinary_scan_within_budgets_has_no_truncation_or_false_partial() {
    let tree = tests::TempTree::new();
    for i in 0..100 {
        fs::write(tree.0.join(format!("{i}.pyc")), b"data").unwrap();
    }
    let control = ScanControl::new();
    let mut events = Vec::new();
    let report = scan_with_policy(
        &tree.0,
        ScanOptions {
            apparent_size: true,
        },
        &control,
        classify,
        |e| events.push(e.clone()),
    );
    assert_eq!(report.status, ScanStatus::Completed);
    assert_eq!(control.resource_limit(), None);
    assert_eq!(report.artifacts.len(), 100);
    assert_eq!(report.total_bytes(), 400);
    assert!(report.warnings.is_empty());
    assert!(report.artifacts.iter().all(|a| !a.partial));
    terminal(&events, ScanStatus::Completed);
}

#[test]
fn catalog_diagnostics_stop_before_cloning_or_emitting_over_budget_payloads() {
    use crate::catalog::{CatalogSnapshot, ClientProfile};
    use crate::client_scan::{CatalogScanEvent, scan_client};
    use buildhunter_plugin_policy::{Origin, PluginCatalog, PluginInput};
    let document=serde_json::to_vec(&serde_json::json!({
        "format":"buildhunter.plugin/1-draft.1","id":"example.ambiguous","version":"1.0.0",
        "title":"Diagnostic fixture","description":"Budget regression",
        "program":{"format":"rifle.program/1-draft.1",
            "facts":[{"id":"kind","type":"string"}],
            "rules":[{"id":"files","when":{"fact":"kind","op":"eq","value":"regular_file"}},
                {"id":"files2","when":{"fact":"kind","op":"eq","value":"regular_file"}}]},
        "bindings":{"kind":{"source":"node.kind"}},
        "targets":[{"id":"example.one","title":"One","ecosystem":"example","kind":"cache","ruleIds":["files"]},
                   {"id":"example.two","title":"Two","ecosystem":"example","kind":"cache","ruleIds":["files2"]}]
    })).unwrap();
    let catalog = PluginCatalog::load(
        &[PluginInput {
            bytes: &document,
            origin: Origin::External,
        }],
        &[],
    )
    .unwrap();
    let snapshot = CatalogSnapshot::from_catalog(ClientProfile::GuiV1, catalog);
    let tree = tests::TempTree::new();
    for i in 0..10 {
        fs::write(tree.0.join(format!("{i}.txt")), b"data").unwrap();
    }
    for (limits, reason, count) in [
        (
            ScanLimits {
                diagnostics: 1,
                ..Default::default()
            },
            ScanLimit::Diagnostics,
            1,
        ),
        (
            ScanLimits {
                diagnostic_weight: 1,
                ..Default::default()
            },
            ScanLimit::DiagnosticWeight,
            0,
        ),
    ] {
        let control = ScanControl::with_limits(limits);
        let mut emitted = 0;
        let mut terminal = None;
        let report = scan_client(
            &tree.0,
            &snapshot,
            ScanOptions::default(),
            false,
            &control,
            |event| match event {
                CatalogScanEvent::Diagnostic(_) => emitted += 1,
                CatalogScanEvent::Finished(status) => terminal = Some(*status),
                _ => {}
            },
        );
        assert_eq!(report.status, ScanStatus::Incomplete);
        assert_eq!(terminal, Some(report.status));
        assert_eq!(control.resource_limit(), Some(reason));
        assert_eq!(report.diagnostics.len(), count);
        assert_eq!(emitted, count);
        assert_eq!(report.warnings.len(), 1);
    }
}

#[cfg(unix)]
#[test]
fn scoped_backend_uses_the_same_listing_retention_limits_without_path_fallback() {
    let tree = tests::TempTree::new();
    for i in 0..10 {
        fs::write(tree.0.join(format!("{i}.pyc")), b"data").unwrap();
    }
    let _guard = NonMaterializingIoGuard::enter().unwrap();
    let access = hunter_fs::ScopedFilesystem::open(&tree.0).unwrap();
    let control = ScanControl::with_limits(ScanLimits {
        listing_entries: 1,
        ..Default::default()
    });
    let mut events = Vec::new();
    let report = scan_with_candidate_access(
        &tree.0,
        ScanOptions::default(),
        &control,
        &access,
        |_, facts, _| classify(facts),
        |e| events.push(e.clone()),
    );
    assert_eq!(report.status, ScanStatus::Incomplete);
    assert_eq!(control.resource_limit(), Some(ScanLimit::ListingEntries));
    assert!(report.artifacts.is_empty());
    terminal(&events, ScanStatus::Incomplete);
}
