//! Deterministic scanner lifecycle witnesses, separate from OS/UI timing.
use super::*;

#[test]
fn golden_lifecycle_preserves_status_partial_and_terminal_order() {
    for line in include_str!("../tests/fixtures/migration-lifecycle-v1.tsv")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let columns: Vec<_> = line.split('\t').collect();
        let scenario = columns[0];
        let tree = tests::TempTree::new();
        let file = tree.0.join(".build/object.o");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"fixture payload").unwrap();
        let directory_bytes_before_fault =
            fs::symlink_metadata(file.parent().unwrap()).unwrap().len();
        let root = if scenario == "missing-root" {
            tree.0.join("missing")
        } else {
            tree.0.clone()
        };
        let fault_file = file.clone();
        let fault = scenario.to_owned();
        let acted = AtomicBool::new(false);
        let probes = Probes {
            hook: Some(Box::new(move |point, path| {
                if point == ProbePoint::Metadata
                    && path == fault_file
                    && !acted.swap(true, Ordering::SeqCst)
                {
                    if fault == "vanished-file" {
                        fs::remove_file(path)?;
                    }
                    if fault == "denied-file" {
                        return Err(std::io::ErrorKind::PermissionDenied.into());
                    }
                }
                Ok(())
            })),
            ..Probes::default()
        };
        let control = ScanControl::new();
        let mut events = Vec::new();
        let report = scan_with_probes(
            &root,
            ScanOptions {
                apparent_size: true,
            },
            &control,
            |facts| rust_cli_policy(facts, false),
            |event| {
                if scenario == "cancel-on-discovery"
                    && matches!(event, ScanEvent::ArtifactDiscovered(_))
                {
                    control.cancel();
                }
                events.push(event.clone());
            },
            &probes,
        );
        assert_eq!(
            status_code(report.status).to_string(),
            columns[1],
            "{scenario}"
        );
        assert_eq!(report.warnings.len().to_string(), columns[2], "{scenario}");
        assert_eq!(report.artifacts.len().to_string(), columns[3], "{scenario}");
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, ScanEvent::Finished(_)))
                .count(),
            1
        );
        assert!(
            matches!(events.last(), Some(ScanEvent::Finished(status)) if *status==report.status)
        );
        let discovered: Vec<_> = events
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
                if let ScanEvent::ArtifactCompleted { id, bytes, partial } = e {
                    Some((*id, *bytes, *partial))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(discovered.len(), completed.len(), "{scenario}");
        for artifact in &report.artifacts {
            assert_eq!(artifact.relative_path, Path::new(".build"));
            assert_eq!(artifact.partial.to_string(), columns[4], "{scenario}");
            assert_eq!(
                completed
                    .iter()
                    .filter(|(id, _, _)| *id == artifact.id)
                    .count(),
                1
            );
            assert!(completed.contains(&(artifact.id, artifact.bytes, artifact.partial)));
        }
        if scenario == "denied-file" || scenario == "vanished-file" {
            assert_eq!(report.total_bytes(), directory_bytes_before_fault);
        }
    }
}
