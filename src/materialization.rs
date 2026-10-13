//! Thread-scoped protection against downloading evicted File Provider objects.

#[cfg(target_os = "macos")]
mod darwin {
    use std::{io, marker::PhantomData, rc::Rc};

    // Public Darwin constants from <sys/resource.h>.
    const MATERIALIZE_DATALESS_FILES: i32 = 3;
    const THREAD: i32 = 1;
    pub(super) const OFF: i32 = 1;

    unsafe extern "C" {
        fn getiopolicy_np(iotype: i32, scope: i32) -> i32;
        fn setiopolicy_np(iotype: i32, scope: i32, policy: i32) -> i32;
    }

    pub(super) fn current() -> io::Result<i32> {
        // SAFETY: these are documented Darwin type/scope constants; no pointers.
        let policy = unsafe { getiopolicy_np(MATERIALIZE_DATALESS_FILES, THREAD) };
        if policy == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(policy)
        }
    }

    fn set(policy: i32) -> io::Result<()> {
        // SAFETY: policy is OFF or a value obtained from getiopolicy_np.
        if unsafe { setiopolicy_np(MATERIALIZE_DATALESS_FILES, THREAD, policy) } == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Must be dropped on the thread whose policy it changed, including unwind.
    pub struct Guard {
        previous: i32,
        _same_thread: PhantomData<Rc<()>>,
    }

    impl Guard {
        pub fn enter() -> io::Result<Self> {
            let previous = current()?;
            set(OFF)?;
            Ok(Self {
                previous,
                _same_thread: PhantomData,
            })
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            // This valid policy was read on this same thread. Never panic in Drop.
            let _ = set(self.previous);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{ScanControl, ScanEvent, ScanOptions, rust_cli_policy, scan_with_policy};

        #[test]
        fn guard_restores_policy_after_nested_use_and_unwind() {
            let before = current().unwrap();
            let result = std::panic::catch_unwind(|| {
                let _outer = Guard::enter().unwrap();
                assert_eq!(current().unwrap(), OFF);
                {
                    let _inner = Guard::enter().unwrap();
                    assert_eq!(current().unwrap(), OFF);
                }
                assert_eq!(current().unwrap(), OFF);
                panic!("exercise policy restoration");
            });
            assert!(result.is_err());
            assert_eq!(current().unwrap(), before);
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
}

#[cfg(target_os = "macos")]
pub use darwin::Guard;

#[cfg(all(test, target_os = "macos"))]
pub(crate) fn current_policy() -> i32 {
    darwin::current().unwrap()
}

#[cfg(not(target_os = "macos"))]
pub struct Guard;

#[cfg(not(target_os = "macos"))]
impl Guard {
    pub fn enter() -> std::io::Result<Self> {
        Ok(Self)
    }
}

/// Keep errno/kind intact except when we can identify a dataless object explicitly.
pub(crate) fn describe_error(path: &std::path::Path, error: std::io::Error) -> std::io::Error {
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        const SF_DATALESS: u32 = 0x4000_0000;
        if std::fs::symlink_metadata(path)
            .is_ok_and(|metadata| metadata.st_flags() & SF_DATALESS != 0)
        {
            return dataless_error(error);
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = path;
    error
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
