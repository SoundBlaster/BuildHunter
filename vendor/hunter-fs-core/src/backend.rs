//! Path-based filesystem access boundary. The host supplies guards and policy.
use std::{fs, io, path::Path};

/// Minimal native metadata/listing interface, without a threading requirement.
/// Implementations preserve input paths and OS errors. symlink_metadata must not
/// follow the final component; intermediate components/read_dir remain path-based.
/// This interface does not enforce access permissions, no-download policy or an
/// atomic/descriptor-anchored tree. Hosts may wrap it with guarded probes/backends.
pub trait FilesystemBackend {
    type Entries: IntoIterator<Item = io::Result<fs::DirEntry>>;
    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata>;
    fn read_dir(&self, path: &Path) -> io::Result<Self::Entries>;
    /// Optional scan-scoped fallback for coordinator measurement. None preserves
    /// the original one-path-at-a-time backend. Some(Err) is authoritative and
    /// must never cause pathname fallback. The handle is acquired lazily, at the
    /// first missing directory/file result, and retained for that selected pass.
    fn prepare_directory(
        &self,
        _path: &Path,
    ) -> Option<io::Result<Box<dyn crate::DirectoryAccess + '_>>> {
        None
    }
}

/// Standard filesystem operations with no policy, caching or extra observations.
#[derive(Clone, Copy, Debug, Default)]
pub struct StdFilesystem;
impl FilesystemBackend for StdFilesystem {
    type Entries = fs::ReadDir;
    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(path)
    }
    fn read_dir(&self, path: &Path) -> io::Result<Self::Entries> {
        fs::read_dir(path)
    }
}

/// Owned root observation for host initialization. Entry includes symlinks and
/// special files; the host decides admission/measurement and retains metadata.
#[derive(Debug)]
pub enum RootObservation {
    Directory(fs::Metadata),
    Entry(fs::Metadata),
}

/// Observe the selected root exactly once, returning unchanged errors and metadata.
/// No normalization, enumeration, follow-up type lookup or stop check occurs here.
/// Host checks access/cloud admission before calling; initial metadata observation
/// even on pre-cancelled scans is preserved. Classification/scheduling/status and
/// backend/platform guard lifetime remain host-owned; this is not full scan startup.
pub fn observe_root<B: FilesystemBackend + ?Sized>(
    backend: &B,
    root: &Path,
) -> io::Result<RootObservation> {
    let metadata = backend.symlink_metadata(root)?;
    Ok(if metadata.is_dir() {
        RootObservation::Directory(metadata)
    } else {
        RootObservation::Entry(metadata)
    })
}

#[cfg(test)]
mod tests {
    use crate::{FilesystemBackend, RootObservation, StdFilesystem, observe_root};
    use std::{
        cell::RefCell,
        fs, io,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "hunter-backend-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct Borrowed<'a> {
        result: RefCell<Option<io::Result<fs::Metadata>>>,
        seen: &'a RefCell<Vec<PathBuf>>,
    }
    impl FilesystemBackend for Borrowed<'_> {
        type Entries = std::iter::Empty<io::Result<fs::DirEntry>>;
        fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
            self.seen.borrow_mut().push(path.to_owned());
            self.result
                .borrow_mut()
                .take()
                .expect("one observation only")
        }
        fn read_dir(&self, _: &Path) -> io::Result<Self::Entries> {
            panic!("root observation must not enumerate")
        }
    }
    #[test]
    fn directory_and_file_roots_keep_their_observed_metadata() {
        let tree = Tree::new();
        let file = tree.0.join("file");
        fs::write(&file, b"1234567").unwrap();
        let RootObservation::Directory(metadata) = observe_root(&StdFilesystem, &tree.0).unwrap()
        else {
            panic!("directory")
        };
        assert!(metadata.is_dir());
        let RootObservation::Entry(metadata) = observe_root(&StdFilesystem, &file).unwrap() else {
            panic!("file")
        };
        assert!(metadata.is_file());
        assert_eq!(metadata.len(), 7);
    }
    #[test]
    fn root_error_and_literal_path_are_preserved_without_retry() {
        let seen = RefCell::new(Vec::new());
        let backend = Borrowed {
            result: RefCell::new(Some(Err(io::Error::from_raw_os_error(13)))),
            seen: &seen,
        };
        let path = Path::new("missing/../selected");
        let error = observe_root(&backend, path).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(13));
        assert_eq!(*seen.borrow(), [path.to_owned()]);
    }
    #[test]
    fn root_observation_borrows_backend_and_never_enumerates_or_normalizes() {
        let tree = Tree::new();
        let seen = RefCell::new(Vec::new());
        let backend = Borrowed {
            result: RefCell::new(Some(fs::symlink_metadata(&tree.0))),
            seen: &seen,
        };
        let path = Path::new("literal/../selected");
        assert!(matches!(
            observe_root(&backend, path),
            Ok(RootObservation::Directory(_))
        ));
        assert_eq!(*seen.borrow(), [path.to_owned()]);
    }
    #[test]
    fn standard_directory_backend_returns_original_entries_and_open_error() {
        let tree = Tree::new();
        fs::write(tree.0.join("file"), b"x").unwrap();
        fs::create_dir(tree.0.join("directory")).unwrap();
        let mut entries = StdFilesystem
            .read_dir(&tree.0)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        let mut expected = vec![tree.0.join("file"), tree.0.join("directory")];
        expected.sort();
        assert_eq!(entries, expected);
        assert_eq!(
            StdFilesystem
                .read_dir(&tree.0.join("missing"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
    }
    #[cfg(unix)]
    #[test]
    fn final_symlinks_are_entries_including_directory_and_dangling_targets() {
        let tree = Tree::new();
        fs::write(tree.0.join("file"), b"x").unwrap();
        fs::create_dir(tree.0.join("directory")).unwrap();
        for target in ["file", "directory", "missing"] {
            let link = tree.0.join(format!("link-{target}"));
            std::os::unix::fs::symlink(target, &link).unwrap();
            let RootObservation::Entry(metadata) = observe_root(&StdFilesystem, &link).unwrap()
            else {
                panic!("must not follow root link")
            };
            assert!(metadata.file_type().is_symlink());
        }
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn non_utf8_root_name_is_not_converted() {
        use std::os::unix::ffi::OsStrExt;
        let tree = Tree::new();
        let file = tree.0.join(std::ffi::OsStr::from_bytes(b"raw-\xff"));
        fs::write(&file, b"raw").unwrap();
        let RootObservation::Entry(metadata) = observe_root(&StdFilesystem, &file).unwrap() else {
            panic!("file")
        };
        assert_eq!(metadata.len(), 3);
    }
}
