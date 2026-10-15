//! Directory-task data and measurement planning. Host owns filesystem policy/backends.
use crate::MeasuredSize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct DirectoryTask<Id, Context = ()> {
    pub id: Id,
    pub path: PathBuf,
    pub measure: bool,
    pub context: Context,
}
#[derive(Debug, Default)]
pub struct DirectoryListing<Context = ()> {
    pub children: Vec<(PathBuf, fs::FileType)>,
    pub errors: Vec<(PathBuf, io::Error)>,
    pub context: Context,
}
pub type FileMeasurements = Vec<Option<io::Result<MeasuredSize>>>;
#[derive(Debug)]
pub struct DirectoryMeasurement<Id> {
    pub id: Id,
    pub directory: io::Result<fs::Metadata>,
    pub files: FileMeasurements,
}
#[derive(Debug)]
pub enum DirectoryResult<Id, Context = ()> {
    Listed {
        id: Id,
        listing: DirectoryListing<Context>,
        measuring: bool,
    },
    Measured(DirectoryMeasurement<Id>),
}

/// Borrows only the job's directory path; owns the selected child slots and ID.
/// Slot positions refer to the original listing, including skipped directories/links.
#[derive(Debug)]
pub struct DirectoryMeasurementPlan<'a, Id> {
    id: Id,
    path: &'a Path,
    files: Vec<Option<PathBuf>>,
}
impl<Id: Clone, Context> DirectoryTask<Id, Context> {
    /// Host may request speculative prefetch in addition to the job's measurement intent.
    /// Neither the context nor filesystem is inspected to decide whether to measure.
    pub fn measurement_plan(
        &self,
        children: &[(PathBuf, fs::FileType)],
        prefetch: bool,
    ) -> Option<DirectoryMeasurementPlan<'_, Id>> {
        (self.measure || prefetch).then(|| DirectoryMeasurementPlan {
            id: self.id.clone(),
            path: &self.path,
            files: children
                .iter()
                .map(|(path, kind)| (!kind.is_dir() && !kind.is_symlink()).then(|| path.clone()))
                .collect(),
        })
    }
}
impl<Id> DirectoryMeasurementPlan<'_, Id> {
    /// Host publishes Listed before calling this method. Directory metadata runs first,
    /// then backend preparation, even if already cancelled or directory metadata failed.
    /// Check stop before each original slot; return a prefix on cancellation. The host
    /// must retain its access/materialization guards through all callbacks and completion.
    /// Active filesystem calls are not interrupted; this does not create a snapshot.
    pub fn measure<D, P, M, S>(
        self,
        directory: D,
        prepare: P,
        mut should_stop: S,
    ) -> DirectoryMeasurement<Id>
    where
        D: FnOnce(&Path) -> io::Result<fs::Metadata>,
        P: FnOnce(&Path, &[Option<PathBuf>]) -> M,
        M: FnMut(&Path) -> io::Result<MeasuredSize>,
        S: FnMut() -> bool,
    {
        let directory = directory(self.path);
        let mut measure = prepare(self.path, &self.files);
        let mut files = Vec::with_capacity(self.files.len());
        for slot in self.files {
            if should_stop() {
                break;
            }
            files.push(slot.map(|path| measure(&path)));
        }
        DirectoryMeasurement {
            id: self.id,
            directory,
            files,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::{Cell, RefCell},
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let p = std::env::temp_dir().join(format!(
                "hunter-plan-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            Self(p)
        }
        fn task(&self, measure: bool) -> DirectoryTask<usize, u32> {
            DirectoryTask {
                id: 42,
                path: self.0.clone(),
                measure,
                context: 17,
            }
        }
        fn entries(&self) -> Vec<(PathBuf, fs::FileType)> {
            fs::create_dir(self.0.join("dir")).unwrap();
            fs::write(self.0.join("a"), b"a").unwrap();
            fs::write(self.0.join("b"), b"b").unwrap();
            ["dir", "a", "b"]
                .into_iter()
                .map(|name| {
                    let p = self.0.join(name);
                    let kind = fs::symlink_metadata(&p).unwrap().file_type();
                    (p, kind)
                })
                .collect()
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn intent_and_host_prefetch_select_plan_without_context_policy() {
        let t = Tree::new();
        let entries = t.entries();
        for (intent, prefetch, expected) in [
            (false, false, false),
            (false, true, true),
            (true, false, true),
            (true, true, true),
        ] {
            assert_eq!(
                t.task(intent)
                    .measurement_plan(&entries, prefetch)
                    .is_some(),
                expected
            );
        }
    }
    #[test]
    fn slot_alignment_and_directory_prepare_file_order_are_preserved() {
        let t = Tree::new();
        let entries = t.entries();
        let task = t.task(true);
        let log = RefCell::new(Vec::new());
        let measured = task.measurement_plan(&entries, false).unwrap().measure(
            |path| {
                log.borrow_mut().push("directory".to_owned());
                fs::symlink_metadata(path)
            },
            |_, slots| {
                assert_eq!(slots.len(), 3);
                assert!(slots[0].is_none());
                log.borrow_mut().push("prepare".into());
                |path: &Path| {
                    let name = path.file_name().unwrap().to_str().unwrap();
                    log.borrow_mut().push(name.to_owned());
                    if name == "b" {
                        Err(io::Error::from_raw_os_error(13))
                    } else {
                        Ok(MeasuredSize {
                            apparent: 7,
                            allocated: 512,
                        })
                    }
                }
            },
            || false,
        );
        assert_eq!(measured.id, 42);
        assert!(measured.directory.is_ok());
        assert_eq!(measured.files.len(), 3);
        assert!(measured.files[0].is_none());
        assert_eq!(
            measured.files[1]
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .apparent,
            7
        );
        assert_eq!(
            measured.files[2]
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap_err()
                .raw_os_error(),
            Some(13)
        );
        assert_eq!(*log.borrow(), ["directory", "prepare", "a", "b"]);
    }
    #[test]
    fn cancellation_keeps_original_slot_prefix() {
        let t = Tree::new();
        let entries = t.entries();
        let task = t.task(true);
        let stop = Cell::new(false);
        let measured = task.measurement_plan(&entries, false).unwrap().measure(
            |path| fs::symlink_metadata(path),
            |_, _| {
                |_: &Path| {
                    stop.set(true);
                    Ok(MeasuredSize {
                        apparent: 1,
                        allocated: 512,
                    })
                }
            },
            || stop.get(),
        );
        assert_eq!(measured.files.len(), 2);
        assert!(measured.files[0].is_none());
        assert!(measured.files[1].is_some());
    }
    #[test]
    fn cancelled_plan_still_prepares_after_directory_error_but_does_not_measure_slots() {
        let t = Tree::new();
        let entries = t.entries();
        let task = t.task(true);
        let prepared = Cell::new(false);
        let measured = task.measurement_plan(&entries, false).unwrap().measure(
            |_| Err(io::Error::from_raw_os_error(13)),
            |_, _| {
                prepared.set(true);
                |_: &Path| -> io::Result<MeasuredSize> { panic!("cancelled slot") }
            },
            || true,
        );
        assert!(prepared.get());
        assert_eq!(measured.directory.unwrap_err().raw_os_error(), Some(13));
        assert!(measured.files.is_empty());
    }
    #[cfg(unix)]
    #[test]
    fn symbolic_link_and_directory_slots_do_not_invoke_backend() {
        let t = Tree::new();
        let mut entries = t.entries();
        std::os::unix::fs::symlink("a", t.0.join("link")).unwrap();
        let p = t.0.join("link");
        entries.push((p.clone(), fs::symlink_metadata(p).unwrap().file_type()));
        let task = t.task(true);
        let calls = Cell::new(0);
        let measured = task.measurement_plan(&entries, false).unwrap().measure(
            |path| fs::symlink_metadata(path),
            |_, _| {
                |_: &Path| {
                    calls.set(calls.get() + 1);
                    Ok(MeasuredSize {
                        apparent: 1,
                        allocated: 1,
                    })
                }
            },
            || false,
        );
        assert_eq!(calls.get(), 2);
        assert_eq!(measured.files.len(), 4);
        assert!(measured.files[3].is_none());
    }
}
