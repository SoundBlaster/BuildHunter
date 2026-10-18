//! Typed preparation/measurement seam; native/batch policy and guards are host-owned.
use crate::MeasuredSize;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// One prepared directory pass. State is owned and may borrow external host resources.
/// The host decides batching, fallbacks and error decoration for each requested path.
pub trait FileMeasurementBackend {
    fn measure(&mut self, path: &Path) -> io::Result<MeasuredSize>;
}

/// Synchronous directory observation followed by preparation of original-position
/// file slots. There are no Send/Sync, boxing or static-lifetime requirements.
/// Prepared state can borrow existing external resources but not the temporary
/// mutable borrow of this backend. Host retains guards through its use and drop.
pub trait DirectoryMeasurementBackend {
    type Prepared: FileMeasurementBackend;
    fn directory_metadata(&mut self, path: &Path) -> io::Result<fs::Metadata>;
    /// Always called after directory metadata, including error/pre-cancelled cases.
    /// Path/slots are borrowed only for this call; retain owned data if needed later.
    fn prepare_files(&mut self, path: &Path, slots: &[Option<PathBuf>]) -> Self::Prepared;
}

#[cfg(test)]
mod tests {
    use crate::{DirectoryMeasurementBackend, DirectoryTask, FileMeasurementBackend, MeasuredSize};
    use std::{
        cell::{Cell, RefCell},
        fs, io,
        path::{Path, PathBuf},
        sync::atomic::{AtomicUsize, Ordering},
    };
    struct Tree(PathBuf);
    impl Tree {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let p = std::env::temp_dir().join(format!(
                "hunter-measure-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&p).unwrap();
            fs::create_dir(p.join("dir")).unwrap();
            fs::write(p.join("a"), b"data").unwrap();
            fs::write(p.join("b"), b"data").unwrap();
            Self(p)
        }
        fn task(&self) -> DirectoryTask<String> {
            DirectoryTask {
                id: "owned id".into(),
                path: self.0.clone(),
                measure: true,
                context: (),
            }
        }
        fn entries(&self) -> Vec<(PathBuf, fs::FileType)> {
            ["dir", "a", "b"]
                .into_iter()
                .map(|name| {
                    let p = self.0.join(name);
                    let k = fs::symlink_metadata(&p).unwrap().file_type();
                    (p, k)
                })
                .collect()
        }
    }
    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    struct Backend<'a> {
        log: &'a RefCell<Vec<String>>,
        stopped: &'a Cell<bool>,
        dropped: &'a Cell<bool>,
        directory_error: bool,
        cancel_on_file: bool,
        panic_on_file: bool,
    }
    struct Prepared<'a> {
        log: &'a RefCell<Vec<String>>,
        stopped: &'a Cell<bool>,
        dropped: &'a Cell<bool>,
        cancel: bool,
        panic: bool,
    }
    impl FileMeasurementBackend for Prepared<'_> {
        fn measure(&mut self, path: &Path) -> io::Result<MeasuredSize> {
            self.log
                .borrow_mut()
                .push(path.file_name().unwrap().to_string_lossy().into_owned());
            assert!(!self.panic, "injected measurement panic");
            if self.cancel {
                self.stopped.set(true);
            }
            if path.ends_with("b") {
                Err(io::Error::from_raw_os_error(13))
            } else {
                Ok(MeasuredSize {
                    apparent: 7,
                    allocated: 512,
                })
            }
        }
    }
    impl Drop for Prepared<'_> {
        fn drop(&mut self) {
            self.dropped.set(true);
        }
    }
    impl<'a> DirectoryMeasurementBackend for Backend<'a> {
        type Prepared = Prepared<'a>;
        fn directory_metadata(&mut self, path: &Path) -> io::Result<fs::Metadata> {
            self.log.borrow_mut().push("directory".into());
            if self.directory_error {
                Err(io::Error::from_raw_os_error(5))
            } else {
                fs::symlink_metadata(path)
            }
        }
        fn prepare_files(&mut self, _: &Path, slots: &[Option<PathBuf>]) -> Self::Prepared {
            self.log
                .borrow_mut()
                .push(format!("prepare:{}", slots.len()));
            Prepared {
                log: self.log,
                stopped: self.stopped,
                dropped: self.dropped,
                cancel: self.cancel_on_file,
                panic: self.panic_on_file,
            }
        }
    }
    fn backend<'a>(
        log: &'a RefCell<Vec<String>>,
        stop: &'a Cell<bool>,
        drop: &'a Cell<bool>,
    ) -> Backend<'a> {
        Backend {
            log,
            stopped: stop,
            dropped: drop,
            directory_error: false,
            cancel_on_file: false,
            panic_on_file: false,
        }
    }
    #[test]
    fn typed_backend_preserves_order_slots_identity_and_file_errors() {
        let tree = Tree::new();
        let task = tree.task();
        let log = RefCell::new(vec![]);
        let stop = Cell::new(false);
        let dropped = Cell::new(false);
        let result = task
            .measurement_plan(&tree.entries(), false)
            .unwrap()
            .measure_with(&mut backend(&log, &stop, &dropped), || stop.get());
        assert_eq!(*log.borrow(), ["directory", "prepare:3", "a", "b"]);
        assert_eq!(result.id, "owned id");
        assert!(result.directory.unwrap().is_dir());
        assert_eq!(result.files.len(), 3);
        assert!(result.files[0].is_none());
        assert_eq!(
            result.files[1].as_ref().unwrap().as_ref().unwrap().apparent,
            7
        );
        assert_eq!(
            result.files[2]
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap_err()
                .raw_os_error(),
            Some(13)
        );
        assert!(dropped.get());
    }
    #[test]
    fn precancelled_directory_error_still_prepares_before_stopping() {
        let tree = Tree::new();
        let task = tree.task();
        let log = RefCell::new(vec![]);
        let stop = Cell::new(true);
        let dropped = Cell::new(false);
        let mut b = backend(&log, &stop, &dropped);
        b.directory_error = true;
        let result = task
            .measurement_plan(&tree.entries(), false)
            .unwrap()
            .measure_with(&mut b, || stop.get());
        assert_eq!(*log.borrow(), ["directory", "prepare:3"]);
        assert!(result.files.is_empty());
        assert_eq!(result.directory.unwrap_err().raw_os_error(), Some(5));
        assert!(dropped.get());
    }
    #[test]
    fn cancellation_keeps_original_position_prefix_and_drops_prepared_state() {
        let tree = Tree::new();
        let task = tree.task();
        let log = RefCell::new(vec![]);
        let stop = Cell::new(false);
        let dropped = Cell::new(false);
        let mut b = backend(&log, &stop, &dropped);
        b.cancel_on_file = true;
        let result = task
            .measurement_plan(&tree.entries(), false)
            .unwrap()
            .measure_with(&mut b, || stop.get());
        assert_eq!(*log.borrow(), ["directory", "prepare:3", "a"]);
        assert_eq!(result.files.len(), 2);
        assert!(result.files[0].is_none());
        assert!(dropped.get());
    }
    #[test]
    fn empty_and_omitted_slots_still_prepare_without_file_calls() {
        let tree = Tree::new();
        let task = tree.task();
        for entries in [vec![], vec![tree.entries().remove(0)]] {
            let log = RefCell::new(vec![]);
            let stop = Cell::new(false);
            let dropped = Cell::new(false);
            let result = task
                .measurement_plan(&entries, false)
                .unwrap()
                .measure_with(&mut backend(&log, &stop, &dropped), || false);
            assert_eq!(
                *log.borrow(),
                ["directory".to_owned(), format!("prepare:{}", entries.len())]
            );
            assert_eq!(result.files.len(), entries.len());
            assert!(result.files.iter().all(Option::is_none));
            assert!(dropped.get());
        }
    }
    #[test]
    fn file_callback_panic_propagates_and_releases_prepared_state() {
        let tree = Tree::new();
        let task = tree.task();
        let log = RefCell::new(vec![]);
        let stop = Cell::new(false);
        let dropped = Cell::new(false);
        let mut b = backend(&log, &stop, &dropped);
        b.panic_on_file = true;
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            task.measurement_plan(&tree.entries(), false)
                .unwrap()
                .measure_with(&mut b, || false)
        }));
        assert!(r.is_err());
        assert_eq!(*log.borrow(), ["directory", "prepare:3", "a"]);
        assert!(dropped.get());
    }
}
