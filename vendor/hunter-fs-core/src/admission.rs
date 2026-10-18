//! Host-selected root admission followed by a single backend observation.
use crate::{FilesystemBackend, RootObservation, observe_root};
use std::{io, path::Path};

/// Owned outcome without interpreting host policy or deciding scan status.
#[derive(Debug)]
pub enum RootAdmission<Reason> {
    /// Host preflight rejected the root; no backend observation took place.
    Skipped(Reason),
    /// Original no-follow-final-component metadata, ready for host scheduling.
    Observed(RootObservation),
    /// The unchanged backend observation error, separate from host rejection.
    Unavailable(io::Error),
}

/// Invoke host preflight once on the original path before observing metadata.
/// A reason returns immediately. Otherwise delegate to `observe_root` exactly once.
/// No enumeration, cancellation check, normalization, retry or guard acquisition
/// is added. The callback may perform its own effects; its panic propagates before
/// any backend access. Host owns guards, reason meaning, warnings, status and work.
/// Neither callback, reason nor backend needs to be Send, Clone or static.
pub fn admit_root<B: FilesystemBackend + ?Sized, Reason>(
    backend: &B,
    root: &Path,
    preflight: impl FnOnce(&Path) -> Option<Reason>,
) -> RootAdmission<Reason> {
    if let Some(reason) = preflight(root) {
        return RootAdmission::Skipped(reason);
    }
    match observe_root(backend, root) {
        Ok(observation) => RootAdmission::Observed(observation),
        Err(error) => RootAdmission::Unavailable(error),
    }
}

#[cfg(test)]
mod tests {
    use crate::{FilesystemBackend, RootAdmission, RootObservation, admit_root};
    use std::{
        cell::RefCell,
        fs, io,
        path::{Path, PathBuf},
        rc::Rc,
    };

    struct Probe<'a> {
        calls: &'a RefCell<Vec<&'static str>>,
        metadata: RefCell<Option<io::Result<fs::Metadata>>>,
    }
    impl FilesystemBackend for Probe<'_> {
        type Entries = std::iter::Empty<io::Result<fs::DirEntry>>;
        fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
            assert_eq!(path, Path::new("literal/../selected"));
            self.calls.borrow_mut().push("metadata");
            self.metadata
                .borrow_mut()
                .take()
                .expect("one metadata call")
        }
        fn read_dir(&self, _: &Path) -> io::Result<Self::Entries> {
            panic!("admission never enumerates")
        }
    }
    fn probe<'a>(
        calls: &'a RefCell<Vec<&'static str>>,
        result: io::Result<fs::Metadata>,
    ) -> Probe<'a> {
        Probe {
            calls,
            metadata: RefCell::new(Some(result)),
        }
    }
    #[test]
    fn skipped_reason_can_borrow_non_send_state_without_metadata() {
        let calls = RefCell::new(Vec::new());
        let backend = probe(&calls, Err(io::Error::from_raw_os_error(13)));
        let reason = Rc::new(RefCell::new(String::from("host policy")));
        let root = Path::new("literal/../selected");
        let outcome = admit_root(&backend, root, |path| {
            assert!(std::ptr::eq(root, path));
            calls.borrow_mut().push("preflight");
            Some(&reason)
        });
        let RootAdmission::Skipped(returned) = outcome else {
            panic!("skip")
        };
        assert!(std::ptr::eq(returned, &reason));
        assert_eq!(*calls.borrow(), ["preflight"]);
        assert!(backend.metadata.borrow().is_some());
    }
    #[test]
    fn allowed_preflight_precedes_one_observation_and_keeps_metadata() {
        for (path, directory) in [
            (PathBuf::from(env!("CARGO_MANIFEST_DIR")), true),
            (
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
                false,
            ),
        ] {
            let calls = RefCell::new(Vec::new());
            let metadata = fs::symlink_metadata(path).unwrap();
            let length = metadata.len();
            let backend = probe(&calls, Ok(metadata));
            let outcome = admit_root(&backend, Path::new("literal/../selected"), |_| {
                calls.borrow_mut().push("preflight");
                None::<()>
            });
            match outcome {
                RootAdmission::Observed(RootObservation::Directory(m)) if directory => {
                    assert!(m.is_dir())
                }
                RootAdmission::Observed(RootObservation::Entry(m)) if !directory => {
                    assert_eq!(m.len(), length)
                }
                _ => panic!("original root kind"),
            }
            assert_eq!(*calls.borrow(), ["preflight", "metadata"]);
        }
    }
    #[test]
    fn unavailable_preserves_original_error_payload_without_retry() {
        #[derive(Debug)]
        struct Error;
        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("opaque")
            }
        }
        impl std::error::Error for Error {}
        let original = io::Error::other(Error);
        let address = original.get_ref().unwrap() as *const _;
        let calls = RefCell::new(Vec::new());
        let backend = probe(&calls, Err(original));
        let RootAdmission::Unavailable(error) =
            admit_root(&backend, Path::new("literal/../selected"), |_| None::<()>)
        else {
            panic!("unavailable")
        };
        assert!(std::ptr::eq(error.get_ref().unwrap(), address));
        assert!(error.get_ref().unwrap().is::<Error>());
        assert_eq!(*calls.borrow(), ["metadata"]);
    }
    #[test]
    fn preflight_panic_performs_no_observation() {
        let calls = RefCell::new(Vec::new());
        let backend = probe(&calls, Err(io::Error::from_raw_os_error(13)));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            admit_root(
                &backend,
                Path::new("literal/../selected"),
                |_| -> Option<()> { panic!("host") },
            );
        }));
        assert!(result.is_err());
        assert!(calls.borrow().is_empty());
    }
}
