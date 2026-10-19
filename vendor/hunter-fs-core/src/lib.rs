//! Portable one-directory enumeration and metadata size primitives.
//! Product policy and native/batch backend selection remain host-owned.
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// An owned filesystem observation. Entry types are obtained without following links.
#[derive(Debug)]
pub enum DirectoryEvent {
    Entry {
        path: PathBuf,
        file_type: fs::FileType,
    },
    Error {
        path: PathBuf,
        error: io::Error,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectoryVisit {
    Completed,
    Cancelled,
    OpenFailed,
}

/// Consume a host-opened listing, retaining its order and raw paths. The host owns
/// permission/materialization policy and can supply a failed open or iterator for tests.
/// Errors emit at their original position; entry failures use the directory path,
/// type lookup failures use the child path. Cancellation is checked before each
/// entry's type lookup; already emitted observations remain owned by the consumer.
/// This pathname primitive is not a descriptor-bound or atomic tree traversal.
pub fn visit_directory<I>(
    path: &Path,
    entries: io::Result<I>,
    mut should_stop: impl FnMut() -> bool,
    mut emit: impl FnMut(DirectoryEvent),
) -> DirectoryVisit
where
    I: IntoIterator<Item = io::Result<fs::DirEntry>>,
{
    let entries = match entries {
        Ok(entries) => entries,
        Err(error) => {
            emit(DirectoryEvent::Error {
                path: path.to_owned(),
                error,
            });
            return DirectoryVisit::OpenFailed;
        }
    };
    for entry in entries {
        if should_stop() {
            return DirectoryVisit::Cancelled;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                emit(DirectoryEvent::Error {
                    path: path.to_owned(),
                    error,
                });
                continue;
            }
        };
        let child = entry.path();
        match entry.file_type() {
            Ok(file_type) => emit(DirectoryEvent::Entry {
                path: child,
                file_type,
            }),
            Err(error) => emit(DirectoryEvent::Error { path: child, error }),
        }
    }
    DirectoryVisit::Completed
}

/// Apparent size is metadata length. Allocated size on Unix is st_blocks * 512
/// with saturation; other platforms preserve the legacy metadata-length fallback.
/// Does not deduplicate hard links or perform I/O; the host supplies metadata.
pub fn metadata_bytes(metadata: &fs::Metadata, apparent_size: bool) -> u64 {
    #[cfg(unix)]
    {
        if apparent_size {
            metadata.len()
        } else {
            metadata.blocks().saturating_mul(512)
        }
    }
    #[cfg(not(unix))]
    {
        let _ = apparent_size;
        metadata.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeasuredSize {
    pub apparent: u64,
    pub allocated: u64,
}
impl From<fs::Metadata> for MeasuredSize {
    fn from(metadata: fs::Metadata) -> Self {
        Self {
            apparent: metadata_bytes(&metadata, true),
            allocated: metadata_bytes(&metadata, false),
        }
    }
}
impl MeasuredSize {
    pub fn bytes(self, apparent: bool) -> u64 {
        if apparent {
            self.apparent
        } else {
            self.allocated
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let p = std::env::temp_dir().join(format!(
                "hunter-core-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn enumerates_directory_file_and_link_without_following_link_target() {
        let t = Tree::new();
        fs::create_dir(t.0.join("dir")).unwrap();
        fs::write(t.0.join("file"), b"data").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("missing", t.0.join("link")).unwrap();
        let mut events = Vec::new();
        let outcome = visit_directory(&t.0, fs::read_dir(&t.0), || false, |e| events.push(e));
        assert_eq!(outcome, DirectoryVisit::Completed);
        assert!(events.iter().any(|e|matches!(e,DirectoryEvent::Entry{path,file_type} if path==&t.0.join("dir") && file_type.is_dir())));
        assert!(events.iter().any(|e|matches!(e,DirectoryEvent::Entry{path,file_type} if path==&t.0.join("file") && file_type.is_file())));
        #[cfg(unix)]
        assert!(events.iter().any(|e|matches!(e,DirectoryEvent::Entry{path,file_type} if path==&t.0.join("link") && file_type.is_symlink())));
        assert!(
            events
                .iter()
                .all(|e| matches!(e, DirectoryEvent::Entry { .. }))
        );
    }
    #[test]
    fn failed_open_preserves_path_and_os_error_even_if_cancelled() {
        let path = Path::new("missing");
        let mut events = Vec::new();
        assert_eq!(
            visit_directory(
                path,
                Err::<fs::ReadDir, _>(io::Error::from_raw_os_error(13)),
                || true,
                |e| events.push(e)
            ),
            DirectoryVisit::OpenFailed
        );
        assert_eq!(events.len(), 1);
        assert!(
            matches!(&events[0],DirectoryEvent::Error{path:p,error} if p==path && error.raw_os_error()==Some(13))
        );
    }
    #[test]
    fn cancellation_retains_only_the_emitted_prefix() {
        let t = Tree::new();
        for name in ["a", "b", "c"] {
            fs::write(t.0.join(name), b"x").unwrap();
        }
        let seen = Cell::new(0);
        assert_eq!(
            visit_directory(
                &t.0,
                fs::read_dir(&t.0),
                || seen.get() == 1,
                |_| seen.set(seen.get() + 1)
            ),
            DirectoryVisit::Cancelled
        );
        assert_eq!(seen.get(), 1);
        assert_eq!(
            visit_directory(
                &t.0,
                fs::read_dir(&t.0),
                || true,
                |_| panic!("must not emit")
            ),
            DirectoryVisit::Cancelled
        );
    }
    #[test]
    fn iteration_errors_do_not_discard_later_entries() {
        let t = Tree::new();
        fs::write(t.0.join("file"), b"x").unwrap();
        let iterator = std::iter::once(Err(io::Error::from_raw_os_error(13)))
            .chain(fs::read_dir(&t.0).unwrap());
        let mut events = Vec::new();
        assert_eq!(
            visit_directory(&t.0, Ok(iterator), || false, |e| events.push(e)),
            DirectoryVisit::Completed
        );
        assert_eq!(events.len(), 2);
        assert!(
            matches!(&events[0],DirectoryEvent::Error{path,error} if path==&t.0 && error.raw_os_error()==Some(13))
        );
        assert!(matches!(&events[1],DirectoryEvent::Entry{path,..} if path==&t.0.join("file")));
    }
    #[test]
    fn empty_directory_completes_without_observations() {
        let t = Tree::new();
        assert_eq!(
            visit_directory(&t.0, fs::read_dir(&t.0), || false, |_| panic!("empty")),
            DirectoryVisit::Completed
        );
    }
    #[test]
    fn sparse_file_sizes_match_platform_metadata() {
        let t = Tree::new();
        let p = t.0.join("sparse");
        fs::File::create(&p)
            .unwrap()
            .set_len(32 * 1024 * 1024)
            .unwrap();
        let m = fs::symlink_metadata(&p).unwrap();
        let actual = MeasuredSize::from(m.clone());
        assert_eq!(actual.bytes(true), m.len());
        #[cfg(unix)]
        assert_eq!(actual.bytes(false), m.blocks().saturating_mul(512));
        #[cfg(not(unix))]
        assert_eq!(actual.bytes(false), m.len());
    }
    #[test]
    fn hard_links_remain_individually_measured() {
        let t = Tree::new();
        let a = t.0.join("a");
        let b = t.0.join("b");
        fs::write(&a, b"payload").unwrap();
        fs::hard_link(&a, &b).unwrap();
        let a = MeasuredSize::from(fs::symlink_metadata(a).unwrap());
        let b = MeasuredSize::from(fs::symlink_metadata(b).unwrap());
        assert_eq!(a, b);
        assert_eq!(a.bytes(true) + b.bytes(true), 14);
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn raw_names_survive_directory_enumeration() {
        use std::os::unix::ffi::OsStrExt;
        let t = Tree::new();
        let name = std::ffi::OsStr::from_bytes(b"raw-\xff");
        fs::write(t.0.join(name), b"x").unwrap();
        let mut paths = Vec::new();
        visit_directory(
            &t.0,
            fs::read_dir(&t.0),
            || false,
            |e| {
                if let DirectoryEvent::Entry { path, .. } = e {
                    paths.push(path)
                }
            },
        );
        assert_eq!(paths, vec![t.0.join(name)]);
    }
}

mod work;
pub use work::{ShutdownGuard, WorkQueue, spawn_workers, worker_count};

mod task;
pub use task::{
    DirectoryListing, DirectoryMeasurement, DirectoryMeasurementPlan, DirectoryResult,
    DirectoryTask, FileMeasurements,
};

mod children;
pub use children::{ChildDecision, ChildTraversal, DirectoryRequest, PlannedChild};

mod ledger;
pub use ledger::DirectoryTaskLedger;

mod subtree;
pub use subtree::{SubtreeCompletion, SubtreeProgress};

mod execution;
pub use execution::{CoordinatorExit, ScanCoordinator, drive_scan};

mod backend;
pub use backend::{FilesystemBackend, RootObservation, StdFilesystem, observe_root};

mod worker;
pub use worker::{DirectoryWorker, run_directory_worker};

mod listing;
pub use listing::collect_directory;

mod measurement;
pub use measurement::{DirectoryMeasurementBackend, FileMeasurementBackend};

mod session;
pub use session::run_scoped_workers;

mod admission;
pub use admission::{RootAdmission, admit_root};

mod lifetime;
pub use lifetime::{GuardedSetupError, with_guarded_backend};

mod materialization;
pub use materialization::NonMaterializingIoGuard;

mod assembly;
pub use assembly::{ScanSession, with_scan_session};

mod coordinator;
pub use coordinator::{TraversalCoordinator, TraversalHandler, TraversalOutcome, TraversalTasks};
