//! Product error decoration and compatibility alias for the Hunter thread guard.
pub use hunter_fs_core::NonMaterializingIoGuard as Guard;

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn current_policy() -> i32 {
    Guard::current_policy().unwrap()
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use crate::{ScanControl, ScanEvent, ScanOptions, rust_cli_policy, scan_with_policy};
    const OFF: i32 = 1;
    fn current() -> std::io::Result<i32> {
        Guard::current_policy()
    }
    #[test]
    fn scan_protects_coordinator_and_restores_it_after_cancel() {
        let tree = crate::tests::TempTree::new();
        std::fs::create_dir(tree.0.join(".build")).unwrap();
        let before = current().unwrap();
        let control = ScanControl::new();
        let report = scan_with_policy(
            &tree.0,
            ScanOptions::default(),
            &control,
            |facts| {
                assert_eq!(current().unwrap(), OFF);
                rust_cli_policy(facts, false)
            },
            |event| {
                assert_eq!(current().unwrap(), OFF);
                if matches!(event, ScanEvent::ArtifactDiscovered(_)) {
                    control.cancel();
                }
            },
        );
        assert_eq!(report.status, crate::ScanStatus::Cancelled);
        assert_eq!(current().unwrap(), before);
    }

    #[test]
    fn scan_restores_coordinator_after_callback_panics() {
        let tree = crate::tests::TempTree::new();
        std::fs::create_dir(tree.0.join(".build")).unwrap();
        let before = current().unwrap();
        let result = std::panic::catch_unwind(|| {
            scan_with_policy(
                &tree.0,
                ScanOptions::default(),
                &ScanControl::new(),
                |facts| rust_cli_policy(facts, false),
                |_| {
                    assert_eq!(current().unwrap(), OFF);
                    panic!("exercise callback unwind");
                },
            )
        });
        assert!(result.is_err());
        assert_eq!(current().unwrap(), before);
    }

    #[test]
    fn every_filesystem_probe_has_downloads_disabled() {
        let tree = crate::tests::TempTree::new();
        std::fs::create_dir_all(tree.0.join(".build/nested")).unwrap();
        std::fs::write(tree.0.join(".build/nested/object.o"), b"object").unwrap();
        let before = current().unwrap();
        let report = scan_with_policy(
            &tree.0,
            ScanOptions::default(),
            &ScanControl::new(),
            |facts| rust_cli_policy(facts, false),
            |_| {},
        );
        assert_eq!(report.status, crate::ScanStatus::Completed);
        assert!(!report.probes.listing_threads.is_empty());
        assert!(!report.probes.materialization_policies.is_empty());
        assert!(
            report
                .probes
                .materialization_policies
                .iter()
                .all(|policy| *policy == OFF)
        );
        assert_eq!(current().unwrap(), before);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorScope {
    Native,
    Scoped,
}

/// A scoped backend owns all diagnostic metadata too. Its errors are never
/// decorated using a pathname probe, even when the displayed path still exists.
pub(crate) fn describe_error(
    path: &std::path::Path,
    error: std::io::Error,
    scope: ErrorScope,
) -> std::io::Error {
    describe_error_with(path, error, scope, |path| {
        #[cfg(target_os = "macos")]
        {
            use std::os::macos::fs::MetadataExt;
            const SF_DATALESS: u32 = 0x4000_0000;
            std::fs::symlink_metadata(path)
                .is_ok_and(|metadata| metadata.st_flags() & SF_DATALESS != 0)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = path;
            false
        }
    })
}

fn describe_error_with(
    path: &std::path::Path,
    error: std::io::Error,
    scope: ErrorScope,
    native_is_dataless: impl FnOnce(&std::path::Path) -> bool,
) -> std::io::Error {
    if scope == ErrorScope::Scoped {
        return error;
    }
    #[cfg(target_os = "macos")]
    if native_is_dataless(path) {
        return dataless_error(error);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (path, native_is_dataless);
    error
}

#[cfg(all(test, target_os = "macos"))]
#[test]
fn scoped_errors_never_probe_or_relabel_a_replaced_native_path() {
    use std::{cell::Cell, io, path::Path};
    let attempts = Cell::new(0);
    for code in [2, 13, 35] {
        let original = io::Error::from_raw_os_error(code);
        let message = original.to_string();
        let result = describe_error_with(
            Path::new("selected/replaced/foreign"),
            original,
            ErrorScope::Scoped,
            |_| {
                attempts.set(attempts.get() + 1);
                true // Simulate unrelated native metadata claiming SF_DATALESS.
            },
        );
        assert_eq!(result.raw_os_error(), Some(code));
        assert_eq!(result.to_string(), message);
        assert!(!is_dataless_error(&result));
    }
    assert_eq!(
        attempts.get(),
        0,
        "scoped errors must perform no native diagnostic lookup"
    );
}

#[cfg(target_os = "macos")]
#[derive(Debug)]
struct DatalessError(std::io::Error);

#[cfg(target_os = "macos")]
impl std::fmt::Display for DatalessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "Skipped dataless cloud object; contents were not downloaded: {}",
            self.0
        )
    }
}

#[cfg(target_os = "macos")]
impl std::error::Error for DatalessError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn dataless_error(error: std::io::Error) -> std::io::Error {
    std::io::Error::new(error.kind(), DatalessError(error))
}

pub(crate) fn is_dataless_error(error: &std::io::Error) -> bool {
    #[cfg(target_os = "macos")]
    {
        error
            .get_ref()
            .is_some_and(|source| source.is::<DatalessError>())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = error;
        false
    }
}
