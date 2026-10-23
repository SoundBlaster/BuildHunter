//! Routing/lifetime tests. This deliberate std-backed spy is not a no-follow backend.
use crate::*;
use hunter_fs_core::{DirectoryAccess, DirectoryEvent, EntryMetadata, ScanAccess};
use std::{
    io,
    sync::{Mutex, atomic::AtomicUsize},
};

struct Access {
    root: PathBuf,
    fail: bool,
    prepared: Mutex<Vec<PathBuf>>,
    live: AtomicUsize,
}
impl ScanAccess for Access {
    fn root_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        assert_eq!(
            path, self.root,
            "root-only metadata must not serve entry fallback"
        );
        fs::symlink_metadata(path)
    }
    fn visit_directory(
        &self,
        path: &Path,
        stop: &mut dyn FnMut() -> bool,
        emit: &mut dyn FnMut(DirectoryEvent),
    ) -> DirectoryVisit {
        hunter_fs_core::visit_directory(path, fs::read_dir(path), stop, emit)
    }
    fn prepare_directory(&self, path: &Path) -> io::Result<Box<dyn DirectoryAccess + '_>> {
        self.prepared.lock().unwrap().push(path.to_owned());
        if self.fail {
            return Err(io::Error::from_raw_os_error(13));
        }
        self.live.fetch_add(1, Ordering::Relaxed);
        Ok(Box::new(Prepared {
            owner: self,
            path: path.to_owned(),
        }))
    }
}
struct Prepared<'a> {
    owner: &'a Access,
    path: PathBuf,
}
impl Drop for Prepared<'_> {
    fn drop(&mut self) {
        self.owner.live.fetch_sub(1, Ordering::Relaxed);
    }
}
impl DirectoryAccess for Prepared<'_> {
    fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(&self.path)
    }
    fn entry_metadata(&self, name: &OsStr) -> io::Result<EntryMetadata> {
        let metadata = fs::symlink_metadata(self.path.join(name))?;
        Ok(EntryMetadata {
            file_type: metadata.file_type().into(),
            size: MeasuredSize {
                apparent: 987,
                allocated: 1024,
            },
        })
    }
}
fn access(root: &Path, fail: bool) -> Access {
    Access {
        root: root.to_owned(),
        fail,
        prepared: Mutex::new(vec![]),
        live: AtomicUsize::new(0),
    }
}
fn policy(_: &Path, facts: &CandidateFacts, _: CandidateNodeKind) -> CandidateAction {
    if facts.node_name == b"chosen" {
        CandidateAction::Classify(ArtifactClassificationCode {
            language: 0,
            kind: 0,
        })
    } else {
        CandidateAction::Traverse
    }
}

#[test]
fn custom_access_serves_worker_and_coordinator_measurements_and_releases_before_finished() {
    let tree = tests::TempTree::new();
    fs::create_dir_all(tree.0.join("chosen/sub")).unwrap();
    fs::write(tree.0.join("chosen/first"), b"real").unwrap();
    fs::write(tree.0.join("chosen/sub/second"), b"xx").unwrap();
    let directories = [tree.0.join("chosen"), tree.0.join("chosen/sub")];
    let directory_bytes: u64 = directories
        .iter()
        .map(|p| fs::symlink_metadata(p).unwrap().len())
        .sum();
    let access = access(&tree.0, false);
    let owner = std::thread::current().id();
    let mut events = vec![];
    let report = scan_with_candidate_access(
        &tree.0,
        ScanOptions {
            apparent_size: true,
        },
        &ScanControl::new(),
        &access,
        policy,
        |event| {
            assert_eq!(std::thread::current().id(), owner);
            if matches!(event, ScanEvent::Finished(_)) {
                assert_eq!(access.live.load(Ordering::Relaxed), 0);
            }
            events.push(event.clone());
        },
    );
    assert_eq!(report.status, ScanStatus::Completed);
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(
        report.artifacts[0].bytes,
        directory_bytes + 2 * 987,
        "native sizes would be 4 and 2"
    );
    let mut prepared = access.prepared.lock().unwrap().clone();
    prepared.sort();
    assert_eq!(prepared, directories);
    assert!(matches!(
        events.last(),
        Some(ScanEvent::Finished(ScanStatus::Completed))
    ));
}

#[test]
fn custom_access_failure_cannot_be_repaired_by_readable_native_paths() {
    let tree = tests::TempTree::new();
    fs::create_dir(tree.0.join("chosen")).unwrap();
    fs::write(tree.0.join("chosen/readable"), b"must not be counted").unwrap();
    let access = access(&tree.0, true);
    let report = scan_with_candidate_access(
        &tree.0,
        ScanOptions {
            apparent_size: true,
        },
        &ScanControl::new(),
        &access,
        policy,
        |_| {},
    );
    assert_eq!(report.status, ScanStatus::Incomplete);
    assert_eq!(report.artifacts.len(), 1);
    assert_eq!(report.artifacts[0].bytes, 0);
    assert!(report.artifacts[0].partial);
    assert!(!report.warnings.is_empty());
    assert_eq!(*access.prepared.lock().unwrap(), [tree.0.join("chosen")]);
    assert_eq!(access.live.load(Ordering::Relaxed), 0);
}
